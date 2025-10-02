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

## Deployment & Upgradeability

### Upgrade Authority Management

The program's upgrade authority is controlled by the **GOV multisig** with timelock enforcement through **Squads Protocol**.

#### Initial Deployment

1. **Deploy the program:**
   ```bash
   anchor build
   anchor deploy --provider.cluster devnet
   ```

2. **Set upgrade authority to GOV multisig:**
   ```bash
   # Get the deployed program ID
   PROGRAM_ID="62syzcwvnS56yKHakNx2rr4JBd5BJmgJ7jDMK3SiipbM"
   
   # Set upgrade authority to GOV multisig
   solana program set-upgrade-authority $PROGRAM_ID --new-upgrade-authority <GOV_MULTISIG_ADDRESS>
   ```

3. **Verify upgrade authority:**
   ```bash
   solana program show $PROGRAM_ID
   ```

#### Timelock Enforcement

The timelock is enforced through **Squads Protocol**:

1. **Propose upgrade** (requires GOV multisig approval)
2. **Timelock period** (configurable, typically 24-48 hours)
3. **Execute upgrade** (after timelock expires)

**Squads Instruction Flow:**
```bash
# 1. Create upgrade proposal
squads create-proposal --multisig <GOV_MULTISIG> --title "Upgrade Hero-Reward Program"

# 2. Add upgrade instruction
squads add-instruction --proposal <PROPOSAL_ID> \
  --program-id $PROGRAM_ID \
  --accounts <UPGRADE_ACCOUNTS> \
  --data <UPGRADE_DATA>

# 3. Approve proposal (requires threshold signatures)
squads approve-proposal --proposal <PROPOSAL_ID>

# 4. Execute after timelock (automatic)
squads execute-proposal --proposal <PROPOSAL_ID>
```

#### Post-Audit Lock Plan

After successful audit and acceptance, the upgrade authority will be **permanently locked**:

```bash
# Lock upgrade authority (irreversible)
solana program set-upgrade-authority $PROGRAM_ID --new-upgrade-authority 11111111111111111111111111111111
```

**⚠️ WARNING**: This action is **irreversible**. The program will become immutable after this step.

### CI/CD Integration

#### Upgrade Authority Assertion Script

Create `scripts/assert-upgrade-authority.sh`:

```bash
#!/bin/bash
set -e

PROGRAM_ID="62syzcwvnS56yKHakNx2rr4JBd5BJmgJ7jDMK3SiipbM"
EXPECTED_AUTHORITY="<GOV_MULTISIG_ADDRESS>"

echo "🔍 Checking upgrade authority for Hero-Reward Program"
echo "Program ID: $PROGRAM_ID"
echo "Expected Authority: $EXPECTED_AUTHORITY"

# Get current upgrade authority
CURRENT_AUTHORITY=$(solana program show $PROGRAM_ID --output json | jq -r '.upgradeAuthority // "null"')

if [ "$CURRENT_AUTHORITY" = "null" ]; then
    echo "❌ Failed to fetch program information or program not found"
    exit 1
fi

echo "Current Authority: $CURRENT_AUTHORITY"

# Check if upgrade authority matches
if [ "$CURRENT_AUTHORITY" = "$EXPECTED_AUTHORITY" ]; then
    echo "✅ Upgrade authority is correctly set to GOV multisig"
    exit 0
else
    echo "❌ Upgrade authority mismatch!"
    echo "Expected: $EXPECTED_AUTHORITY"
    echo "Current:  $CURRENT_AUTHORITY"
    echo ""
    echo "To fix this, run:"
    echo "solana program set-upgrade-authority $PROGRAM_ID --new-upgrade-authority $EXPECTED_AUTHORITY"
    exit 1
fi
```

#### GitHub Actions Integration

Add to `.github/workflows/ci.yml`:

```yaml
name: CI/CD
on: [push, pull_request]

jobs:
  assert-upgrade-authority:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Setup Solana CLI
        uses: actions/setup-node@v3
        with:
          node-version: '18'
      - name: Install Solana CLI
        run: |
          sh -c "$(curl -sSfL https://release.solana.com/v1.17.0/install)"
          echo "$HOME/.local/share/solana/install/active_release/bin" >> $GITHUB_PATH
      - name: Assert Upgrade Authority
        run: |
          chmod +x scripts/assert-upgrade-authority.sh
          ./scripts/assert-upgrade-authority.sh
        env:
          SOLANA_RPC_URL: ${{ secrets.SOLANA_RPC_URL }}
          SOLANA_KEYPAIR: ${{ secrets.SOLANA_KEYPAIR }}
```

### Deployment Checklist

- [ ] Program deployed to target cluster
- [ ] Upgrade authority set to GOV multisig
- [ ] Upgrade authority verified
- [ ] Squads timelock configured
- [ ] CI/CD pipeline configured
- [ ] Post-audit lock plan documented
- [ ] Emergency procedures documented

### Emergency Procedures

In case of critical bugs requiring immediate upgrade:

1. **Emergency multisig proposal** (bypasses normal timelock)
2. **Immediate upgrade execution** (requires emergency threshold)
3. **Post-upgrade verification** (ensure fix is effective)
4. **Documentation update** (record emergency upgrade)

## Integration Notes

### For Indexers and UIs

#### Event Monitoring

Monitor these events for real-time hero rewards tracking:

```typescript
// Subscribe to hero rewards events
program.addEventListener("Initialized", (event) => {
  console.log("Program initialized:", event);
  // Update program state, total units, publisher
});

program.addEventListener("Prefunded", (event) => {
  console.log("Vault prefunded:", event);
  // Update vault balance display
});

program.addEventListener("WinnersRootPosted", (event) => {
  console.log("Winners root posted:", event);
  // Update round state, enable claiming
});

program.addEventListener("WinnerPaid", (event) => {
  console.log("Winner paid:", event);
  // Update claim status, user balances
});

program.addEventListener("RoundClosed", (event) => {
  console.log("Round closed:", event);
  // Update round status, final statistics
});

program.addEventListener("PublisherUpdated", (event) => {
  console.log("Publisher updated:", event);
  // Update publisher address
});

program.addEventListener("Paused", (event) => {
  console.log("Program paused:", event);
  // Show pause notification
});
```

#### Key Addresses

| Component | Address | Description |
|-----------|---------|-------------|
| **Program ID** | `62syzcwvnS56yKHakNx2rr4JBd5BJmgJ7jDMK3SiipbM` | Hero rewards program |
| **Governance** | GOV Multisig | Program authority (timelock controlled) |
| **Publisher** | Configurable | Address authorized to post winners roots |
| **REW Multisig** | Configurable | Funding source for vault |
| **State PDA** | `[b"hero_rewards_state"]` | Program state account |
| **Vault Authority** | `[b"vault_authority"]` | PDA controlling vault |
| **Round State PDA** | `[b"round_state", round.to_le_bytes()]` | Per-round state |
| **Claim State PDA** | `[b"claim_state", round.to_le_bytes(), index.to_le_bytes()]` | Per-claim state |

#### Required Safety Checks

```typescript
// Verify program ID
const HERO_PROGRAM_ID = "62syzcwvnS56yKHakNx2rr4JBd5BJmgJ7jDMK3SiipbM";
if (program.programId.toString() !== HERO_PROGRAM_ID) {
  throw new Error("Invalid hero rewards program ID");
}

// Verify round bounds
if (round < 1 || round > state.totalRounds) {
  throw new Error("Invalid round number");
}

// Verify round timing
const now = Math.floor(Date.now() / 1000);
const roundStart = state.tgeTs + (round - 1) * state.roundLenSecs;
const roundEnd = roundStart + state.roundLenSecs;

if (now < roundStart) {
  throw new Error("Round not yet started");
}

// Verify round cap
if (roundState.total > state.perRoundCap) {
  throw new Error("Round total exceeds cap");
}

// Verify publisher authority
if (publisher.toString() !== state.publisher.toString()) {
  throw new Error("Unauthorized publisher");
}
```

#### Merkle Proof Verification

```typescript
// Verify Merkle proof client-side before claiming
function verifyMerkleProof(
  leaf: Buffer,
  proof: Buffer[],
  root: Buffer
): boolean {
  let computedHash = leaf;
  
  for (const proofElement of proof) {
    if (computedHash.compare(proofElement) <= 0) {
      computedHash = Buffer.from(
        anchor.utils.sha256.hash(
          Buffer.concat([computedHash, proofElement])
        )
      );
    } else {
      computedHash = Buffer.from(
        anchor.utils.sha256.hash(
          Buffer.concat([proofElement, computedHash])
        )
      );
    }
  }
  
  return computedHash.equals(root);
}

// Create leaf hash for verification
function createLeafHash(index: number, address: PublicKey, amount: BN): Buffer {
  const indexBuffer = Buffer.alloc(4);
  indexBuffer.writeUInt32LE(index);
  
  const amountBuffer = Buffer.alloc(8);
  amountBuffer.writeBigUInt64LE(BigInt(amount.toString()));
  
  return Buffer.from(
    anchor.utils.sha256.hash(
      Buffer.concat([
        indexBuffer,
        address.toBuffer(),
        amountBuffer
      ])
    )
  );
}
```

#### Scoring Integration

For off-chain indexers calculating scores:

```typescript
// Hero scoring formula implementation
function calculateHeroScore(
  havg: number,      // Time-weighted average holdings
  tscore: number,    // Holding duration score
  padj: number,      // Price adjustment score
  sadj: number,      // Selling penalty score
  dwin: number       // Winner history multiplier
): number {
  const sPre = (0.40 * havg) + (0.35 * tscore) + (0.25 * padj) - (0.50 * sadj);
  return sPre > 0 ? sPre * dwin : sPre;
}

// Winner history multiplier calculation
function calculateDwin(pastWins: Array<{round: number, rank: number}>): number {
  let sumDecay = 0;
  
  for (const win of pastWins) {
    if (win.rank === 1) {
      sumDecay += 0.15;
    } else if (win.rank >= 2 && win.rank <= 3) {
      sumDecay += 0.10;
    } else if (win.rank >= 4 && win.rank <= 10) {
      sumDecay += 0.05;
    }
  }
  
  return Math.max(1.0 - sumDecay, 0.40);
}
```

#### UI Integration Guidelines

1. **Leaderboard Display:**
   - Show normalized scores (-1000 to +1000) for UI
   - Use raw scores for actual ranking and payouts
   - Display winner history and Dwin multiplier

2. **Claim Interface:**
   - Verify Merkle proof before submitting transaction
   - Show claim status and remaining time
   - Handle claim state verification

3. **Round Management:**
   - Monitor round start/end times
   - Track distribution progress and caps
   - Show publisher activity and root posting

#### Security Recommendations

1. **Merkle Proof Validation:**
   - Always verify proofs client-side before claiming
   - Validate leaf construction matches program logic
   - Check proof array length and structure

2. **Round Cap Monitoring:**
   - Track claimed amounts vs round cap
   - Alert when approaching 1B LUX limit
   - Monitor for unusual claiming patterns

3. **Publisher Verification:**
   - Verify publisher signatures on roots
   - Monitor for unauthorized root posting attempts
   - Track publisher changes and governance actions

4. **Claim Protection:**
   - Prevent double-claiming attempts
   - Validate winner addresses and amounts
   - Implement claim status caching

#### Testing Checklist

Before mainnet integration:

- [ ] Verify program ID and key addresses
- [ ] Test Merkle proof generation and verification
- [ ] Validate scoring formula implementation
- [ ] Test claim functionality with various proofs
- [ ] Verify round timing and cap enforcement
- [ ] Test publisher authorization
- [ ] Validate event monitoring
- [ ] Test pause/unpause scenarios
- [ ] Verify winner history tracking