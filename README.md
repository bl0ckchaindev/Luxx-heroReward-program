# LUX Hero Rewards Program

A Solana program built with Anchor for distributing 45 billion LUX tokens to top 50 winners over 45 rounds using Merkle tree verification.

## Overview

This program implements a gamified reward system that:
- Holds exactly 45,000,000,000 LUX tokens in a program-owned vault (PDA)
- Distributes tokens to top 50 winners per round using Merkle proofs
- Uses precise mathematical formulas for scoring and distribution
- Provides permissionless claim triggers with Merkle verification
- Maintains strict security with no arbitrary transfers

## Program Architecture

### Core Components

1. **Global State PDA**: Stores program configuration and state
2. **Rewards Vault PDA**: Holds the 45B LUX tokens (Associated Token Account)
3. **Round State PDAs**: Track each round's Merkle root and distribution status
4. **Claim State PDAs**: Prevent double-claiming for each winner
5. **Merkle Tree Verification**: Ensures only valid winners can claim

### Key Features

- **Pre-funding**: One-time 45B LUX transfer from REW to program vault
- **Merkle Tree Distribution**: Secure, permissionless claiming with proofs
- **Round-based System**: 45 rounds with 1B LUX cap per round
- **Publisher System**: Designated publisher posts winners roots
- **No Arbitrary Transfers**: Vault only sends to verified winners

## Program Instructions

### 1. Initialize
```rust
pub fn initialize(
    ctx: Context<Initialize>,
    tge_ts: i64,
    round_len_secs: i64,
) -> Result<()>
```

**Authority**: Governance only  
**Purpose**: Initialize the program with configuration parameters

**Account Structure**:
```rust
pub struct Initialize<'info> {
    pub state: Account<'info, HeroRewardsState>,           // State PDA
    pub governance: Signer<'info>,                        // Governance signer
    pub vault_authority: UncheckedAccount<'info>,         // Vault authority PDA
    pub mint_lux: Account<'info, Mint>,                   // LUX mint account
    pub vault_lux: Account<'info, TokenAccount>,         // Vault ATA (created)
    pub publisher: UncheckedAccount<'info>,              // Publisher address
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}
```

### 2. Post Winners Root
```rust
pub fn post_winners_root(
    ctx: Context<PostWinnersRoot>,
    round: u8,
    root: [u8; 32],
    total: u64,
) -> Result<()>
```

**Authority**: Publisher only  
**Purpose**: Post Merkle root for a round's winners

### 3. Claim Winner
```rust
pub fn claim_winner(
    ctx: Context<ClaimWinner>,
    round: u8,
    index: u32,
    address: Pubkey,
    amount: u64,
    proof: Vec<[u8; 32]>,
) -> Result<()>
```

**Authority**: Permissionless  
**Purpose**: Claim winner allocation using Merkle proof

### 4. Close Round
```rust
pub fn close_round(ctx: Context<CloseRound>, round: u8) -> Result<()>
```

**Authority**: Permissionless  
**Purpose**: Close a round when cap reached or deadline passed

### 5. Set Publisher
```rust
pub fn set_publisher(ctx: Context<SetPublisher>, new_publisher: Pubkey) -> Result<()>
```

**Authority**: Governance only  
**Purpose**: Update the publisher address

### 6. Pause
```rust
pub fn pause(ctx: Context<Pause>, is_paused: bool) -> Result<()>
```

**Authority**: Governance only  
**Purpose**: Pause/unpause the program

## Scoring System

The program works with an off-chain indexer that computes scores using the formula:

```
S = [(0.40)Havg + (0.35)Tscore + (0.25)Padj − (0.50)Sadj] × Dwin
```

Where:
- **Havg**: Time-weighted average of held LUX tokens
- **Tscore**: Holding duration (log₂(days_held + 1))
- **Padj**: Buy price vs market average (lower = better)
- **Sadj**: Penalty for selling/transferring LUX
- **Dwin**: Winner history multiplier (0.40 to 1.00)

## Merkle Tree Structure

Each round's winners are stored in a Merkle tree with:
- **Leaf**: `hash(index, address, amount)`
- **Root**: Posted by publisher for verification
- **Proof**: Required for claiming

## Security Features

### Invariants
1. **Exact Cap**: Each round distributes exactly 1B LUX (or less if cap not reached)
2. **No Double Claims**: Each winner can only claim once per round
3. **Merkle Verification**: Only valid proofs allow claiming
4. **No Arbitrary Transfers**: Vault only sends to verified winners
5. **Round Caps**: Cannot exceed 1B LUX per round

### Access Control
- **Initialize**: Governance only
- **Post Winners Root**: Publisher only
- **Claim Winner**: Permissionless (with valid proof)
- **Close Round**: Permissionless (when conditions met)
- **Set Publisher**: Governance only
- **Pause**: Governance only

## Events

### Initialized
```rust
pub struct Initialized {
    pub tge_ts: i64,
    pub round_len_secs: i64,
    pub total_rounds: u8,
    pub total_units: u128,
    pub publisher: Pubkey,
}
```

### WinnersRootPosted
```rust
pub struct WinnersRootPosted {
    pub round: u8,
    pub root: [u8; 32],
    pub total: u64,
    pub publisher: Pubkey,
}
```

### WinnerPaid
```rust
pub struct WinnerPaid {
    pub round: u8,
    pub index: u32,
    pub address: Pubkey,
    pub amount: u64,
    pub total_claimed: u64,
}
```

### RoundClosed
```rust
pub struct RoundClosed {
    pub round: u8,
    pub total_claimed: u64,
    pub cap_reached: bool,
    pub deadline_passed: bool,
}
```

## Error Codes

- `Unauthorized`: Caller not authorized for operation
- `InvalidRound`: Round number out of bounds
- `InvalidTotal`: Total amount doesn't match round cap
- `DistributionAlreadyStarted`: Round already has winners root
- `DistributionNotStarted`: Round doesn't have winners root yet
- `DistributionFinished`: Round is already closed
- `InvalidProof`: Merkle proof verification failed
- `ExceedsRoundCap`: Claim would exceed round cap
- `RoundNotClosable`: Round cannot be closed yet
- `InvalidWinner`: Winner address doesn't match
- `InvalidClaimState`: Invalid claim state PDA
- `Paused`: Program is paused
- `Overflow`: Arithmetic overflow

## Integration Points

### With Indexer
- **Score Calculation**: Off-chain indexer computes scores using the formula
- **Merkle Tree Generation**: Indexer creates Merkle tree from top 50 winners
- **Root Publishing**: Publisher posts root for each round

### With Website/UI
- **Leaderboard**: Display top 50 winners with scores
- **Claim Interface**: Allow winners to claim with Merkle proofs
- **Round Status**: Show distribution progress and deadlines

## Usage Examples

### Initialize Program
```typescript
await program.methods
  .initialize(
    new anchor.BN(tgeTs),
    new anchor.BN(roundLenSecs)
  )
  .accounts({
    state,
    governance: governance.publicKey,
    vaultAuthority,
    mintLux,
    vaultLux,
    publisher: publisher.publicKey,
    tokenProgram: TOKEN_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
    associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
    rent: SYSVAR_RENT_PUBKEY,
  })
  .signers([governance])
  .rpc();
```

### Post Winners Root
```typescript
await program.methods
  .postWinnersRoot(round, merkleRoot, new anchor.BN(total))
  .accounts({
    state,
    publisher: publisher.publicKey,
    roundState: roundStatePda,
    systemProgram: SystemProgram.programId,
  })
  .signers([publisher])
  .rpc();
```

### Claim Winner
```typescript
await program.methods
  .claimWinner(
    round,
    index,
    winner.publicKey,
    new anchor.BN(amount),
    proof
  )
  .accounts({
    state,
    roundState: roundStatePda,
    claimState: claimStatePda,
    vaultAuthority,
    vaultLux,
    winnerAta,
    winner: winner.publicKey,
    claimer: claimer.publicKey,
    tokenProgram: TOKEN_PROGRAM_ID,
    associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  })
  .signers([claimer])
  .rpc();
```

## Testing

The test suite covers:
- Program initialization
- Winners root posting
- Winner claiming with Merkle proofs
- Round closing
- Publisher management
- Pause functionality
- Error handling and edge cases
- Security constraints and access control

Run tests with:
```bash
anchor test
```

### Environment Variables
Create a `.env` file in the project root with:
```bash
# LUX mint address
LUX_MINT_ADDRESS=your_lux_mint_address_here
```

## Deployment

### Network Configuration
- **Network**: Solana mainnet-beta
- **Owner/Upgrade Authority**: GOV multisig (EWYj67…FYsq) — timelocked
- **Funding Source**: REW multisig (7bSj8Y…wybUK) — one-time 45B LUX to program PDA

### Key Addresses
- **Program ID**: `HERO_REWARDS_PROGRAM_ID_HERE`
- **Vault PDA**: `REWARDS_VAULT_PDA`
- **Governance**: `EWYj67…FYsq`
- **REW Multisig**: `7bSj8Y…wybUK`

## Security Considerations

1. **Merkle Proof Verification**: All claims must provide valid Merkle proofs
2. **Round Caps**: Strict enforcement of 1B LUX per round limit
3. **No Arbitrary Transfers**: Vault can only send to verified winners
4. **Publisher Control**: Only designated publisher can post winners roots
5. **Governance Control**: Only governance can pause or change publisher
6. **Immutable Schedule**: 45 rounds, 60-day cadence, fixed caps

## Monitoring & Operations

- **Round Tracking**: Monitor distribution progress per round
- **Cap Enforcement**: Alert when rounds approach 1B LUX cap
- **Claim Monitoring**: Track successful claims and remaining allocations
- **Publisher Management**: Monitor publisher activity and root posting
- **Security Alerts**: Monitor for unauthorized access attempts

This program provides a secure, transparent, and efficient way to distribute LUX tokens to top performers while maintaining strict caps and preventing abuse through Merkle tree verification.
