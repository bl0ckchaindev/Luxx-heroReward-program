use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer, Mint};
use anchor_spl::associated_token::AssociatedToken;
use std::str::FromStr;

declare_id!("62syzcwvnS56yKHakNx2rr4JBd5BJmgJ7jDMK3SiipbM");

// Constants
const TOTAL_ROUNDS: u8 = 45;
const ROUND_LEN_DAYS: u64 = 60;
const PER_ROUND_CAP_HUMAN: u64 = 1_000_000_000; // 1B LUX per round (human readable)
const TOTAL_TOKENS_HUMAN: u64 = 45_000_000_000; // 45B LUX total (human readable)

#[program]
pub mod hero_rewards {
    use super::*;

    /// Initialize the hero rewards program
    /// Only callable by governance
    pub fn initialize(
        ctx: Context<Initialize>,
        tge_ts: i64,
        round_len_secs: i64,
    ) -> Result<()> {
        let state = &mut ctx.accounts.state;
        let mint_info = ctx.accounts.mint_lux.to_account_info();
        let mint_data = Mint::try_deserialize(&mut &mint_info.data.borrow()[..])?;
        
        // Calculate total units: 45B * 10^decimals with proper overflow protection
        let decimals_power = 10u128.pow(mint_data.decimals as u32);
        let total_units = (TOTAL_TOKENS_HUMAN as u128)
            .checked_mul(decimals_power)
            .ok_or(HeroRewardsError::Overflow)?;

        // Calculate per-round cap: 1B * 10^decimals with proper overflow protection
        let per_round_cap = (PER_ROUND_CAP_HUMAN as u128)
            .checked_mul(decimals_power)
            .ok_or(HeroRewardsError::Overflow)?;

        state.governance = ctx.accounts.governance.key();
        state.mint_lux = ctx.accounts.mint_lux.key();
        state.vault_lux = ctx.accounts.vault_lux.key();
        state.tge_ts = tge_ts;
        state.round_len_secs = round_len_secs;
        state.total_rounds = TOTAL_ROUNDS;
        state.total_units = total_units;
        state.per_round_cap = per_round_cap;
        state.publisher = ctx.accounts.publisher.key();
        state.paused = false;

        emit!(Initialized {
            tge_ts,
            round_len_secs,
            total_rounds: TOTAL_ROUNDS,
            total_units,
            publisher: ctx.accounts.publisher.key(),
        });

        Ok(())
    }

    /// Prefund LUX tokens to the rewards vault
    /// Only callable by REW multisig
    pub fn prefund(
        ctx: Context<Prefund>,
        amount: u64,
    ) -> Result<()> {
        require!(amount > 0, HeroRewardsError::InvalidAmount);

        let cpi_accounts = Transfer {
            from: ctx.accounts.rew_lux_ata.to_account_info(),
            to: ctx.accounts.vault_lux.to_account_info(),
            authority: ctx.accounts.rew_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        
        token::transfer(cpi_ctx, amount)?;

        emit!(Prefunded {
            amount,
            vault: ctx.accounts.vault_lux.key(),
        });

        Ok(())
    }

    /// Post winners root for a round
    /// Only callable by publisher
    pub fn post_winners_root(
        ctx: Context<PostWinnersRoot>,
        round: u8,
        root: [u8; 32],
        total: u64,
    ) -> Result<()> {
        require!(round > 0 && round <= TOTAL_ROUNDS, HeroRewardsError::InvalidRound);
        require!(total == PER_ROUND_CAP, HeroRewardsError::InvalidTotal);
        require!(!ctx.accounts.state.paused, HeroRewardsError::Paused);

        let round_state = &mut ctx.accounts.round_state;
        require!(!round_state.distribution_started, HeroRewardsError::DistributionAlreadyStarted);

        round_state.round = round;
        round_state.root = root;
        round_state.total = total;
        round_state.distribution_started = true;
        round_state.distribution_finished = false;

        emit!(WinnersRootPosted {
            round,
            root,
            total,
            publisher: ctx.accounts.publisher.key(),
        });

        Ok(())
    }

    /// Claim winner allocation
    /// Permissionless - anyone can call for any winner
    pub fn claim_winner(
        ctx: Context<ClaimWinner>,
        round: u8,
        index: u32,
        address: Pubkey,
        amount: u64,
        proof: Vec<[u8; 32]>,
    ) -> Result<()> {
        require!(round > 0 && round <= TOTAL_ROUNDS, HeroRewardsError::InvalidRound);
        require!(!ctx.accounts.state.paused, HeroRewardsError::Paused);

        let round_state = &ctx.accounts.round_state;
        require!(round_state.distribution_started, HeroRewardsError::DistributionNotStarted);
        require!(!round_state.distribution_finished, HeroRewardsError::DistributionFinished);

        // Verify Merkle proof
        let leaf = hash_leaf(index, address, amount);
        require!(verify_merkle_proof(leaf, proof, round_state.root), HeroRewardsError::InvalidProof);

        // Check if already claimed
        let claim_bump = ctx.bumps.claim_state;
        let claim_seeds = &[
            b"claim",
            &round.to_le_bytes()[..],
            &index.to_le_bytes()[..],
            &[claim_bump],
        ];
        let claim_pda = Pubkey::create_program_address(claim_seeds, ctx.program_id)
            .map_err(|_| HeroRewardsError::InvalidClaimState)?;
        require!(ctx.accounts.claim_state.key() == claim_pda, HeroRewardsError::InvalidClaimState);

        // Check round cap
        let current_claimed = round_state.claimed_amount
            .checked_add(amount)
            .ok_or(HeroRewardsError::Overflow)?;
        require!(current_claimed <= round_state.total, HeroRewardsError::ExceedsRoundCap);

        // Update round state
        let round_state = &mut ctx.accounts.round_state;
        round_state.claimed_amount = current_claimed;

        // Mark as claimed
        let claim_state = &mut ctx.accounts.claim_state;
        claim_state.claimed = true;

        // Transfer tokens to winner
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault_lux.to_account_info(),
            to: ctx.accounts.winner_ata.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        
        token::transfer(cpi_ctx, amount)?;

        emit!(WinnerPaid {
            round,
            index,
            address,
            amount,
            total_claimed: current_claimed,
        });

        Ok(())
    }

    /// Close a round
    /// Can be called by anyone when cap is reached or deadline passed
    pub fn close_round(ctx: Context<CloseRound>, round: u8) -> Result<()> {
        require!(round > 0 && round <= TOTAL_ROUNDS, HeroRewardsError::InvalidRound);
        require!(!ctx.accounts.state.paused, HeroRewardsError::Paused);

        let round_state = &mut ctx.accounts.round_state;
        require!(round_state.distribution_started, HeroRewardsError::DistributionNotStarted);
        require!(!round_state.distribution_finished, HeroRewardsError::DistributionFinished);

        // Check if cap reached or deadline passed
        let current_time = Clock::get()?.unix_timestamp;
        let round_end_time = ctx.accounts.state.tge_ts + (round as i64 - 1) * ctx.accounts.state.round_len_secs;
        let deadline_passed = current_time > round_end_time;
        let cap_reached = round_state.claimed_amount >= round_state.total;

        require!(cap_reached || deadline_passed, HeroRewardsError::RoundNotClosable);

        round_state.distribution_finished = true;

        emit!(RoundClosed {
            round,
            total_claimed: round_state.claimed_amount,
            cap_reached,
            deadline_passed,
        });

        Ok(())
    }

    /// Set publisher (governance only)
    pub fn set_publisher(ctx: Context<SetPublisher>, new_publisher: Pubkey) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.publisher = new_publisher;

        emit!(PublisherUpdated {
            old_publisher: ctx.accounts.old_publisher.key(),
            new_publisher,
        });

        Ok(())
    }

    /// Pause/unpause the program (governance only)
    pub fn pause(ctx: Context<Pause>, is_paused: bool) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.paused = is_paused;

        emit!(Paused { is_paused });

        Ok(())
    }
}

// Account structs
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = governance,
        space = 8 + HeroRewardsState::INIT_SPACE,
        seeds = [b"hero_rewards_state"],
        bump
    )]
    pub state: Account<'info, HeroRewardsState>,

    #[account(mut)]
    pub governance: Signer<'info>,

    /// CHECK: PDA authority for vault
    #[account(
        seeds = [b"vault_authority"],
        bump
    )]
    pub vault_authority: UncheckedAccount<'info>,
    
    /// CHECK: LUX mint address
    pub mint_lux: Account<'info, Mint>,

    #[account(
        init,
        payer = governance,
        associated_token::mint = mint_lux,
        associated_token::authority = vault_authority
    )]
    pub vault_lux: Account<'info, TokenAccount>,
    
    /// CHECK: Publisher address (can be set by governance)
    pub publisher: UncheckedAccount<'info>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct Prefund<'info> {
    #[account(
        seeds = [b"hero_rewards_state"],
        bump
    )]
    pub state: Account<'info, HeroRewardsState>,

    /// CHECK: REW multisig authority for funding
    pub rew_authority: Signer<'info>,

    /// CHECK: LUX mint address
    pub mint_lux: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint_lux,
        associated_token::authority = rew_authority
    )]
    pub rew_lux_ata: Account<'info, TokenAccount>,

    /// CHECK: PDA authority for vault
    #[account(
        seeds = [b"vault_authority"],
        bump
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = mint_lux,
        associated_token::authority = vault_authority
    )]
    pub vault_lux: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
#[instruction(round: u8)]
pub struct PostWinnersRoot<'info> {
    #[account(
        seeds = [b"hero_rewards_state"],
        bump
    )]
    pub state: Account<'info, HeroRewardsState>,

    /// CHECK: Publisher address
    #[account(
        mut,
        constraint = publisher.key() == state.publisher @ HeroRewardsError::Unauthorized
    )]
    pub publisher: Signer<'info>,

    #[account(
        init,
        payer = publisher,
        space = 8 + RoundState::INIT_SPACE,
        seeds = [b"round_state", &round.to_le_bytes()[..]],
        bump
    )]
    pub round_state: Account<'info, RoundState>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(round: u8, index: u32, address: Pubkey)]
pub struct ClaimWinner<'info> {
    #[account(
        seeds = [b"hero_rewards_state"],
        bump
    )]
    pub state: Account<'info, HeroRewardsState>,

    #[account(
        seeds = [b"round_state", &round.to_le_bytes()[..]],
        bump
    )]
    pub round_state: Account<'info, RoundState>,

    #[account(
        init_if_needed,
        payer = claimer,
        space = 8 + ClaimState::INIT_SPACE,
        seeds = [b"claim", &round.to_le_bytes()[..], &index.to_le_bytes()[..]],
        bump
    )]
    pub claim_state: Account<'info, ClaimState>,

    /// CHECK: PDA authority for vault
    #[account(
        seeds = [b"vault_authority"],
        bump
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub vault_lux: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = state.mint_lux,
        associated_token::authority = address
    )]
    pub winner_ata: Account<'info, TokenAccount>,

    /// CHECK: Winner address
    #[account(
        constraint = winner.key() == address @ HeroRewardsError::InvalidWinner
    )]
    pub winner: UncheckedAccount<'info>,

    #[account(mut)]
    pub claimer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(round: u8)]
pub struct CloseRound<'info> {
    #[account(
        seeds = [b"hero_rewards_state"],
        bump
    )]
    pub state: Account<'info, HeroRewardsState>,

    #[account(
        mut,
        seeds = [b"round_state", &round.to_le_bytes()[..]],
        bump
    )]
    pub round_state: Account<'info, RoundState>,
}

#[derive(Accounts)]
pub struct SetPublisher<'info> {
    #[account(
        mut,
        seeds = [b"hero_rewards_state"],
        bump,
        constraint = state.governance == governance.key() @ HeroRewardsError::Unauthorized
    )]
    pub state: Account<'info, HeroRewardsState>,

    pub governance: Signer<'info>,

    /// CHECK: Old publisher address
    pub old_publisher: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Pause<'info> {
    #[account(
        mut,
        seeds = [b"hero_rewards_state"],
        bump,
        constraint = state.governance == governance.key() @ HeroRewardsError::Unauthorized
    )]
    pub state: Account<'info, HeroRewardsState>,

    pub governance: Signer<'info>,
}

// State accounts
#[account]
#[derive(InitSpace)]
pub struct HeroRewardsState {
    pub governance: Pubkey,
    pub mint_lux: Pubkey,
    pub vault_lux: Pubkey,
    pub tge_ts: i64,
    pub round_len_secs: i64,
    pub total_rounds: u8,
    pub total_units: u128,
    pub per_round_cap: u128,       // Per-round cap (1B * 10^decimals)
    pub publisher: Pubkey,
    pub paused: bool,
}

#[account]
#[derive(InitSpace)]
pub struct RoundState {
    pub round: u8,
    pub root: [u8; 32],
    pub total: u64,
    pub claimed_amount: u64,
    pub distribution_started: bool,
    pub distribution_finished: bool,
}

#[account]
#[derive(InitSpace)]
pub struct ClaimState {
    pub claimed: bool,
}

// Events
#[event]
pub struct Initialized {
    pub tge_ts: i64,
    pub round_len_secs: i64,
    pub total_rounds: u8,
    pub total_units: u128,
    pub publisher: Pubkey,
}

#[event]
pub struct Prefunded {
    pub amount: u64,
    pub vault: Pubkey,
}

#[event]
pub struct WinnersRootPosted {
    pub round: u8,
    pub root: [u8; 32],
    pub total: u64,
    pub publisher: Pubkey,
}

#[event]
pub struct WinnerPaid {
    pub round: u8,
    pub index: u32,
    pub address: Pubkey,
    pub amount: u64,
    pub total_claimed: u64,
}

#[event]
pub struct RoundClosed {
    pub round: u8,
    pub total_claimed: u64,
    pub cap_reached: bool,
    pub deadline_passed: bool,
}

#[event]
pub struct PublisherUpdated {
    pub old_publisher: Pubkey,
    pub new_publisher: Pubkey,
}

#[event]
pub struct Paused {
    pub is_paused: bool,
}

// Error codes
#[error_code]
pub enum HeroRewardsError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Invalid round")]
    InvalidRound,
    #[msg("Invalid total amount")]
    InvalidTotal,
    #[msg("Distribution already started")]
    DistributionAlreadyStarted,
    #[msg("Distribution not started")]
    DistributionNotStarted,
    #[msg("Distribution finished")]
    DistributionFinished,
    #[msg("Invalid Merkle proof")]
    InvalidProof,
    #[msg("Exceeds round cap")]
    ExceedsRoundCap,
    #[msg("Round not closable")]
    RoundNotClosable,
    #[msg("Invalid winner")]
    InvalidWinner,
    #[msg("Invalid claim state")]
    InvalidClaimState,
    #[msg("Paused")]
    Paused,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Invalid amount")]
    InvalidAmount,
}

// Merkle tree utilities
fn hash_leaf(index: u32, address: Pubkey, amount: u64) -> [u8; 32] {
    let mut data = Vec::new();
    data.extend_from_slice(&index.to_le_bytes());
    data.extend_from_slice(address.as_ref());
    data.extend_from_slice(&amount.to_le_bytes());
    
    use anchor_lang::solana_program::keccak;
    keccak::hash(&data).to_bytes()
}

fn verify_merkle_proof(leaf: [u8; 32], proof: Vec<[u8; 32]>, root: [u8; 32]) -> bool {
    let mut current = leaf;
    
    for sibling in proof {
        let mut combined = Vec::new();
        if current < sibling {
            combined.extend_from_slice(&current);
            combined.extend_from_slice(&sibling);
        } else {
            combined.extend_from_slice(&sibling);
            combined.extend_from_slice(&current);
        }
        
        use anchor_lang::solana_program::keccak;
        current = keccak::hash(&combined).to_bytes();
    }
    
    current == root
}
