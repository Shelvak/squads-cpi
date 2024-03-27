import * as anchor from "@coral-xyz/anchor";
import { splTokenProgram } from "@coral-xyz/spl-token";
import { assert } from "chai";

import {
  getAssociatedTokenAddressSync,
  TOKEN_2022_PROGRAM_ID,
  createTransferInstruction,
  createAssociatedTokenAccountInstruction,
} from "@solana/spl-token";
import {
  SystemProgram,
  TransactionMessage,
  sendAndConfirmTransaction,
  VersionedTransaction,
} from "@solana/web3.js";

import { SqdsCpi } from "../target/types/sqds_cpi";

import * as SQDS from "@sqds/multisig";

const newAccountWithLamports = async (connection, lamports = 100000000000) => {
  const account = anchor.web3.Keypair.generate();
  const signature = await connection.requestAirdrop(
    account.publicKey,
    lamports
  );
  await connection.confirmTransaction(signature);
  return account;
}

const initStakePool = async (program) => {
  const authority = await newAccountWithLamports(program.provider.connection)
  const [stakePoolKey] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      authority.publicKey.toBuffer(),
      Buffer.from("stakePool", "utf-8"),
    ],
    program.programId
  );

  await program.methods
  .initialize()
  .accounts({
    payer: authority.publicKey,
    stakePool: stakePoolKey,
    tokenProgram: TOKEN_2022_PROGRAM_ID,
    rent: anchor.web3.SYSVAR_RENT_PUBKEY,
    systemProgram: anchor.web3.SystemProgram.programId,
  })
  .signers([authority])
  .rpc({skipPreflight: true});

  return { authority, stakePoolKey };
}

describe("Sqds multi cpi", () => {
  const sqdProgramId = new anchor.web3.PublicKey("SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf")
  const program = anchor.workspace.SqdsCpi as anchor.Program<SqdsCpi>;
  const connection = program.provider.connection;

  const tokenProgram = splTokenProgram({ programId: TOKEN_2022_PROGRAM_ID });

  const amount = new anchor.BN(3);
  let authority, stakePoolKey, multisigPda, sqdsVaultPda, transactionPda;

  before(async () => {
    ({ authority, stakePoolKey, } = await initStakePool(program));

    const connection = program.provider.connection;
    // Derive the multisig PDA
    const createKey = anchor.web3.Keypair.generate();
    multisigPda = SQDS.getMultisigPda({
      createKey: createKey.publicKey,
    })[0];

    // Get treasury from Squads program config
    const programConfigPda = SQDS.getProgramConfigPda({programId: sqdProgramId})[0];
    const programConfig = await SQDS.accounts.ProgramConfig.fromAccountAddress(
        connection,
        programConfigPda
    );
    const configTreasury = programConfig.treasury;

    // Create multisig with authority & stakePoolPDA
    const tx = await SQDS.transactions.multisigCreateV2({
      blockhash: (await connection.getLatestBlockhash()).blockhash,
      treasury: configTreasury,
      createKey: createKey.publicKey,
      creator: authority.publicKey,
      multisigPda: multisigPda,
      configAuthority: null,
      timeLock: 0,
      threshold: 1,
      rentCollector: null,
      members: [
        {
          key: authority.publicKey,
          permissions: SQDS.types.Permissions.all(),
        },
        {
          key: stakePoolKey,
          permissions: SQDS.types.Permissions.all(),
        },
      ],
      programId: sqdProgramId,
    });

    tx.sign([authority, createKey])
    await connection.sendTransaction(tx);

    sqdsVaultPda = SQDS.getVaultPda({
      multisigPda: multisigPda,
      index: 0,
    })[0];

    // Airdrop some SOL to the vault
    await connection.confirmTransaction(await connection.requestAirdrop(
      authority.publicKey,
      1_000_000_000
    ));
    await connection.confirmTransaction(await connection.requestAirdrop(
      sqdsVaultPda,
      1_000_000_000
    ));

    // JS full approve "example"

    // The transfer is being signed from the Squads Vault, that is why we use the VaultPda
    const instruction = SystemProgram.transfer({
        fromPubkey: sqdsVaultPda,
        toPubkey: authority.publicKey,
        lamports: 1,
    });

    // @ts-ignore
    // This message contains the instructions that the transaction is going to execute
    const transactionIndex = 1n;
    const transferMessage = new TransactionMessage({
        payerKey: authority.publicKey,
        recentBlockhash: (await connection.getLatestBlockhash()).blockhash,
        instructions: [instruction],
    });

    // This is the first transaction in the multisig
    const signature1 = await SQDS.rpc.vaultTransactionCreate({
        connection,
        feePayer: authority,
        multisigPda: multisigPda,
        creator: authority.publicKey,
        transactionIndex,
        vaultIndex: 0,
        ephemeralSigners: 0,
        transactionMessage: transferMessage,
        memo: "Transfer 0.1 SOL to creator",
        sendOptions: { skipPreflight: true, preflightCommitment: "finalized" },
    });

    console.log("Transaction created: ", signature1);

    const signature2 = await SQDS.rpc.proposalCreate({
        connection,
        feePayer: authority,
        multisigPda: multisigPda,
        creator: authority,
        transactionIndex,
        sendOptions: { skipPreflight: true, preflightCommitment: "finalized" },
    });
    await connection.confirmTransaction(signature2);

    console.log("Transaction proposal created: ", signature2);
    console.log("Transaction proposal approved: ", await SQDS.rpc.proposalApprove({
        connection,
        feePayer: authority,
        multisigPda: multisigPda,
        transactionIndex,
        member: authority,
        memo: "Approve transaction",
        sendOptions: { skipPreflight: true, preflightCommitment: "finalized" },
    }));

    console.log("Transaction proposal executed: ", await SQDS.rpc.vaultTransactionExecute({
      connection,
      feePayer: authority,
      multisigPda: multisigPda,
      transactionIndex,
      member: authority.publicKey,
      sendOptions: { skipPreflight: true, preflightCommitment: "finalized" },
    }));
  });

  it("Works from deposit", async () => {
    const txIndex = 2;
    const [transactionPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        SQDS.utils.toUtfBytes("multisig"),
        multisigPda.toBytes(),
        SQDS.utils.toUtfBytes("transaction"),
        // @ts-ignore
        SQDS.utils.toU64Bytes(txIndex),
      ],
      sqdProgramId
    );

    const [proposalPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        SQDS.utils.toUtfBytes("multisig"),
        multisigPda.toBytes(),
        SQDS.utils.toUtfBytes("transaction"),
        // @ts-ignore
        SQDS.utils.toU64Bytes(txIndex),
        SQDS.utils.toUtfBytes("proposal"),
      ],
      sqdProgramId
    );

    await program.methods
      .deposit(amount)
      .accounts({
        payer: authority.publicKey,
        stakePool: stakePoolKey,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        systemProgram: anchor.web3.SystemProgram.programId,
        transactionPda,
        proposalPda,
        multisigPda,
        sqdsVaultPda,
        sqdsProgram: sqdProgramId,
      })
      .signers([authority])
      .rpc({ skipPreflight: true });
  });
});
