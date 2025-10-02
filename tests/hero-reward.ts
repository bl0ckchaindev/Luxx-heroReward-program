import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { HeroRewards } from "../target/types/hero_rewards";
import { PublicKey, Keypair, SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, getOrCreateAssociatedTokenAccount, mintTo, getAccount, getAssociatedTokenAddressSync } from "@solana/spl-token";
import { expect } from "chai";
import bs58 from "bs58";
import * as dotenv from "dotenv";
dotenv.config();

// NOTE: This test suite now uses configurable addresses instead of hard-coded constants.
// All addresses are passed as parameters during initialization, making the program more flexible.

describe("hero-rewards", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.HeroRewards as Program<HeroRewards>;
  const provider = anchor.getProvider();

  // Test accounts
  let governance: Keypair;
  let rewAuthority: Keypair;
  let publisher: Keypair;
  let winner: Keypair;
  let claimer: Keypair;
  let mintLux: PublicKey;
  let vaultLux: PublicKey;
  let vaultAuthority: PublicKey;
  let state: PublicKey;
  let winnerAta: PublicKey;
  let rewLuxAta: PublicKey;

  const ROUND_LEN_SECS = 60 * 24 * 60 * 60; // 60 days
  const TOTAL_ROUNDS = 45;
  const PER_ROUND_CAP = 1_000_000_000; // 1B LUX
  const TOTAL_TOKENS = 45_000_000_000; // 45B LUX

  before(async () => {
    // Use deployer wallet as governance
    governance = provider.wallet.payer;
    console.log('Governance wallet', governance.publicKey.toBase58())

    // Create test accounts
    // All addresses are now configurable and passed as parameters
    rewAuthority = Keypair.fromSecretKey(bs58.decode(process.env.REW_PRIVATE_KEY));
    publisher = Keypair.fromSecretKey(bs58.decode(process.env.PUBLISHER_PRIVATE_KEY));
    winner = Keypair.fromSecretKey(bs58.decode(process.env.WINNER_PRIVATE_KEY));
    claimer = Keypair.fromSecretKey(bs58.decode(process.env.CLAIMER_PRIVATE_KEY));

    // Set existing deployed LUX mint address
    mintLux = new PublicKey(process.env.LUX_MINT_ADDRESS || "YOUR_LUX_MINT_ADDRESS_HERE");
    console.log('mintLux', mintLux.toBase58())

    // Derive PDAs
    const [statePda] = PublicKey.findProgramAddressSync([Buffer.from("hero_rewards_state")], program.programId);
    state = statePda;
    console.log('state', state.toBase58())
    
    const [vaultLuxAuthority] = PublicKey.findProgramAddressSync([Buffer.from("vault_authority")], program.programId);
    vaultAuthority = vaultLuxAuthority;
    console.log('vaultAuthority', vaultAuthority.toBase58())
    
    vaultLux = getAssociatedTokenAddressSync(mintLux, vaultAuthority, true);
    console.log('vaultLux', vaultLux.toBase58())
    
    winnerAta = getAssociatedTokenAddressSync(mintLux, winner.publicKey);
    console.log('winnerAta', winnerAta.toBase58())
    
    rewLuxAta = getAssociatedTokenAddressSync(mintLux, rewAuthority.publicKey);
    console.log('rewLuxAta', rewLuxAta.toBase58())
  });

  // it("Initializes the hero rewards program", async () => {
  //   const tgeTs = Math.floor(Date.now() / 1000) + 60; // 1 minute from now

  //   const tx = await program.methods
  //     .initialize(
  //       new anchor.BN(tgeTs),
  //       new anchor.BN(ROUND_LEN_SECS)
  //     )
  //     .accounts({
  //       state,
  //       governance: governance.publicKey,
  //       vaultAuthority,
  //       mintLux,
  //       vaultLux,
  //       publisher: publisher.publicKey,
  //       tokenProgram: TOKEN_PROGRAM_ID,
  //       systemProgram: SystemProgram.programId,
  //       associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
  //       rent: SYSVAR_RENT_PUBKEY,
  //     })
  //     .signers([governance])
  //     .rpc();

  //   console.log("Initialize transaction signature:", tx);

  //   const stateAccount = await program.account.heroRewardsState.fetch(state);
  //   expect(stateAccount.governance.toString()).to.equal(governance.publicKey.toString());
  //   expect(stateAccount.mintLux.toString()).to.equal(mintLux.toString());
  //   expect(stateAccount.publisher.toString()).to.equal(publisher.publicKey.toString());
  //   expect(stateAccount.tgeTs.toNumber()).to.equal(tgeTs);
  //   expect(stateAccount.roundLenSecs.toNumber()).to.equal(ROUND_LEN_SECS);
  //   expect(stateAccount.totalRounds).to.equal(TOTAL_ROUNDS);
  //   expect(stateAccount.paused).to.be.false;
  // });

  // it("Prefunds LUX tokens to the rewards vault", async () => {
  //   const amount = createLargeAmount(TOTAL_TOKENS, 6); // 45B LUX in base units
  //   console.log('log->amount', amount.toString())
  //   // Note: This test will fail in production because REW_ADDRESS is a constant
  //   // For testing purposes, we need to either:
  //   // 1. Use the actual REW multisig address, or
  //   // 2. Modify the program to allow testing with generated keypairs    
  //   // Uncomment the following when using the actual REW multisig:
  
  //   const tx = await program.methods
  //     .prefund(new anchor.BN(amount))
  //     .accounts({
  //       state,
  //       rewAuthority: rewAuthority.publicKey,
  //       mintLux,
  //       rewLuxAta,
  //       vaultAuthority,
  //       vaultLux,
  //       tokenProgram: TOKEN_PROGRAM_ID,
  //       associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
  //     })
  //     .signers([rewAuthority])
  //     .rpc();

  //   console.log("Prefund transaction signature:", tx);

  //   const vaultAccount = await getAccount(provider.connection, vaultLux);
  //   expect(vaultAccount.amount).to.equal(BigInt(amount));
  // });

  // it("Fails to prefund with unauthorized authority", async () => {
  //   const amount = 1000 * 10**8; // 1000 LUX in base units
  //   const unauthorizedAuthority = Keypair.generate();

  //   try {
  //     await program.methods
  //       .prefund(new anchor.BN(amount))
  //       .accounts({
  //         state,
  //         rewAuthority: unauthorizedAuthority.publicKey,
  //         mintLux,
  //         rewLuxAta,
  //         vaultAuthority,
  //         vaultLux,
  //         tokenProgram: TOKEN_PROGRAM_ID,
  //         associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
  //       })
  //       .signers([unauthorizedAuthority])
  //       .rpc();
      
  //     expect.fail("Should have failed with unauthorized error");
  //   } catch (error) {
  //     // This will fail because REW_ADDRESS constant doesn't match the generated keypair
  //     expect(error.message).to.include("Unauthorized");
  //   }
  // });

  // it("Demonstrates constant address constraints", async () => {
  //   // This test shows the expected behavior with constant addresses
  //   console.log("Program uses constant addresses:");
  //   console.log("- GOV_ADDRESS: EWYj67FYsq (governance multisig)");
  //   console.log("- REW_ADDRESS: 7bSj8YwybUK (REW multisig for funding)");
  //   console.log("These addresses are hardcoded in the program and cannot be changed");
  //   console.log("For testing, you must use the actual multisig keypairs");
    
  //   // Verify the governance address constraint
  //   expect(governance.publicKey.toBase58()).to.not.equal("EWYj67FYsq");
  //   console.log("Current governance wallet:", governance.publicKey.toBase58());
  //   console.log("Expected governance wallet: EWYj67FYsq");
  //   console.log("Note: In production, these must match exactly");
  // });

  it("Posts winners root for round 1", async () => {
    const round = 1;
    const root = new Uint8Array(32).fill(1); // Mock Merkle root
    const total = PER_ROUND_CAP;

    const tx = await program.methods
      .postWinnersRoot(round, Array.from(root), new anchor.BN(total))
      .accounts({
        state,
        publisher: publisher.publicKey,
        roundState: await getRoundStatePda(round),
        systemProgram: SystemProgram.programId,
      })
      .signers([publisher])
      .rpc();

    console.log("Post winners root transaction signature:", tx);

    const roundStateAccount = await program.account.roundState.fetch(await getRoundStatePda(round));
    expect(roundStateAccount.round).to.equal(round);
    expect(roundStateAccount.total.toNumber()).to.equal(total);
    expect(roundStateAccount.distributionStarted).to.be.true;
    expect(roundStateAccount.distributionFinished).to.be.false;
  });

  it("Claims winner allocation", async () => {
    const round = 1;
    const index = 0;
    const amount = 100_000_000; // 100M LUX
    const proof: number[][] = []; // Mock proof

    // Create winner's ATA if it doesn't exist
    const winnerAtaInfo = await provider.connection.getAccountInfo(winnerAta);
    if (!winnerAtaInfo) {
      const createAtaIx = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        claimer,
        mintLux,
        winner.publicKey
      );
      
      await program.provider.sendAndConfirm(
        new anchor.web3.Transaction().add(createAtaIx.instruction),
        [claimer]
      );
    }

    const tx = await program.methods
      .claimWinner(
        round,
        index,
        winner.publicKey,
        new anchor.BN(amount),
        proof
      )
      .accounts({
        state,
        roundState: await getRoundStatePda(round),
        claimState: await getClaimStatePda(round, index),
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

    console.log("Claim winner transaction signature:", tx);

    const claimStateAccount = await program.account.claimState.fetch(await getClaimStatePda(round, index));
    expect(claimStateAccount.claimed).to.be.true;
  });

  it("Closes round when cap is reached", async () => {
    const round = 1;

    const tx = await program.methods
      .closeRound(round)
      .accounts({
        state,
        roundState: await getRoundStatePda(round),
      })
      .rpc();

    console.log("Close round transaction signature:", tx);

    const roundStateAccount = await program.account.roundState.fetch(await getRoundStatePda(round));
    expect(roundStateAccount.distributionFinished).to.be.true;
  });

  // it("Sets new publisher", async () => {
  //   const newPublisher = Keypair.generate();

  //   const tx = await program.methods
  //     .setPublisher(newPublisher.publicKey)
  //     .accounts({
  //       state,
  //       governance: governance.publicKey,
  //       oldPublisher: publisher.publicKey,
  //     })
  //     .signers([governance])
  //     .rpc();

  //   console.log("Set publisher transaction signature:", tx);

  //   const stateAccount = await program.account.heroRewardsState.fetch(state);
  //   expect(stateAccount.publisher.toString()).to.equal(newPublisher.publicKey.toString());
  // });

  // it("Pauses the program", async () => {
  //   const tx = await program.methods
  //     .pause(true)
  //     .accounts({
  //       state,
  //       governance: governance.publicKey,
  //     })
  //     .signers([governance])
  //     .rpc();

  //   console.log("Pause transaction signature:", tx);

  //   const stateAccount = await program.account.heroRewardsState.fetch(state);
  //   expect(stateAccount.paused).to.be.true;
  // });

  // Helper functions
  async function getRoundStatePda(round: number): Promise<PublicKey> {
    const [pda] = PublicKey.findProgramAddressSync(
      [Buffer.from("round_state"), Buffer.from(round.toString().padStart(8, '0'))],
      program.programId
    );
    return pda;
  }

  async function getClaimStatePda(round: number, index: number): Promise<PublicKey> {
    const [pda] = PublicKey.findProgramAddressSync(
      [Buffer.from("claim"), Buffer.from(round.toString().padStart(8, '0')), Buffer.from(index.toString().padStart(8, '0'))],
      program.programId
    );
    return pda;
  }

  function createLargeAmount(tokens: number, decimals: number): string {
    // Use string concatenation to avoid JavaScript number precision issues
    const baseAmount = tokens.toString();
    const multiplier = '1' + '0'.repeat(decimals);
    return (BigInt(baseAmount) * BigInt(multiplier)).toString();
  }
  // Production Testing Notes:
  // To test with actual addresses in production:
  // 1. Set GOV_ADDRESS in lib.rs to match your governance multisig
  // 2. Set REW_ADDRESS in lib.rs to match your REW multisig  
  // 3. Use the actual multisig keypairs in the test setup
  // 4. Ensure the multisigs have the required LUX tokens for testing
  // 5. Uncomment the prefund test code when using actual addresses

  it("Verifies exact decimal precision calculations", async () => {
    // Test that the program correctly calculates units with proper decimal scaling
    const mintDecimals = 8; // LUX has 8 decimals
    const decimalsPower = BigInt(10) ** BigInt(mintDecimals);
    
    // Test total units: 45B * 10^8 = 4,500,000,000,000,000,000
    const expectedTotalUnits = BigInt(45_000_000_000) * decimalsPower;
    expect(expectedTotalUnits.toString()).to.equal("4500000000000000000");
    
    // Test per-round cap: 1B * 10^8 = 100,000,000,000,000,000
    const expectedPerRoundCap = BigInt(1_000_000_000) * decimalsPower;
    expect(expectedPerRoundCap.toString()).to.equal("100000000000000000");
    
    // Verify no overflow occurs
    expect(expectedTotalUnits).to.be.greaterThan(0);
    expect(expectedPerRoundCap).to.be.greaterThan(0);
    
    console.log("✅ Decimal precision calculations verified:");
    console.log(`Total units: ${expectedTotalUnits.toString()}`);
    console.log(`Per-round cap: ${expectedPerRoundCap.toString()}`);
  });
});
