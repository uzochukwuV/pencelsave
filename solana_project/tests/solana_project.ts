import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SolanaProject } from "../target/types/solana_project";
import { assert } from "chai";
import { createMint, createAccount, mintTo, getAccount } from "@solana/spl-token";

describe("solana_project", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolanaProject as Program<SolanaProject>;
  const owner = provider.wallet as anchor.Wallet;

  let usdcMint: anchor.web3.PublicKey;
  let ownerTokenAccount: anchor.web3.PublicKey;
  let yieldVaultMock: anchor.web3.PublicKey;

  let profilePda: anchor.web3.PublicKey;
  let routerVaultPda: anchor.web3.PublicKey;
  let lockVaultPda: anchor.web3.PublicKey;

  before(async () => {
    // 1. Setup mock USDC mint
    usdcMint = await createMint(
      provider.connection,
      (owner as any).payer,
      owner.publicKey,
      null,
      6
    );

    // 2. Setup owner token account
    ownerTokenAccount = await createAccount(
      provider.connection,
      (owner as any).payer,
      usdcMint,
      owner.publicKey
    );

    // 3. Setup mock yield vault
    const yieldVaultKeypair = anchor.web3.Keypair.generate();
    yieldVaultMock = await createAccount(
      provider.connection,
      (owner as any).payer,
      usdcMint,
      yieldVaultKeypair.publicKey
    );

    // 4. Mint some initial USDC to owner
    await mintTo(
      provider.connection,
      (owner as any).payer,
      usdcMint,
      ownerTokenAccount,
      owner.publicKey,
      100000000 // 100 USDC
    );

    // Find PDAs
    [profilePda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("profile"), owner.publicKey.toBuffer()],
      program.programId
    );

    [routerVaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("router_vault"), profilePda.toBuffer()],
      program.programId
    );

    [lockVaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("lock_vault"), profilePda.toBuffer()],
      program.programId
    );
  });

  it("Initializes the profile!", async () => {
    const tx = await program.methods
      .initializeProfile(50, 30, 20, new anchor.BN(10000000), 0)
      .accounts({
        owner: owner.publicKey,
        profile: profilePda,
        usdcMint: usdcMint,
        routerVault: routerVaultPda,
        lockVault: lockVaultPda,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .rpc();

    const profile = await program.account.userProfile.fetch(profilePda);
    assert.equal(profile.splitMainPct, 50);
    assert.equal(profile.splitYieldPct, 30);
    assert.equal(profile.splitLockPct, 20);
    assert.equal(profile.weeklyReleaseAmount.toNumber(), 10000000);
  });

  it("Processes a payment!", async () => {
    // Transfer 100 USDC to router vault to simulate payment
    const transferTx = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: owner.publicKey,
        toPubkey: routerVaultPda,
        lamports: await provider.connection.getMinimumBalanceForRentExemption(165),
      })
    );
    // Actually we need SPL transfer
    const { createTransferInstruction } = await import("@solana/spl-token");
    const splTransferIx = createTransferInstruction(
      ownerTokenAccount,
      routerVaultPda,
      owner.publicKey,
      100000000 // 100 USDC
    );
    const splTx = new anchor.web3.Transaction().add(splTransferIx);
    await provider.sendAndConfirm(splTx);

    // Call process payment
    await program.methods
      .processPayment()
      .accounts({
        profile: profilePda,
        routerVault: routerVaultPda,
        lockVault: lockVaultPda,
        yieldVault: yieldVaultMock,
        ownerTokenAccount: ownerTokenAccount,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .rpc();

    // Verify splits
    const routerAcc = await getAccount(provider.connection, routerVaultPda);
    const lockAcc = await getAccount(provider.connection, lockVaultPda);
    const yieldAcc = await getAccount(provider.connection, yieldVaultMock);

    // 100 USDC total -> 50 to main (already in owner acc, net +50 -100 = -50), 30 to yield, 20 to lock
    assert.equal(Number(routerAcc.amount), 0);
    assert.equal(Number(yieldAcc.amount), 30000000);
    assert.equal(Number(lockAcc.amount), 20000000);
  });
});
