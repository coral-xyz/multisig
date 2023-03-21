/**
 * How to use
 * 1. Build the program `anchor build -- --features devnet`
 * 2. Start local validator `solana-test-validator`
 * 3. Deploy the program:
    solana program deploy\
      --program-id ./target/deploy/mean_multisig-keypair.json\
      --url localhost\
         target/deploy/mean_multisig.so
 * 4. Run tests:
    anchor test\
      --skip-local-validator\
      --skip-deploy\
      --provider.cluster localnet\
      -- --features devnet
 */
import * as anchor from "@coral-xyz/anchor";
import { AnchorError, AnchorProvider, BN, Program } from '@coral-xyz/anchor';
import { Keypair, LAMPORTS_PER_SOL, PublicKey, SystemProgram, Transaction } from '@solana/web3.js';
import { MeanMultisig } from "../target/types/mean_multisig";
import { assert, expect } from "chai";

// type Transaction = anchor.IdlAccounts<MeanMultisig>["transaction"];
type TransactionAccount = anchor.IdlTypes<MeanMultisig>["TransactionAccount"];
type Owner = anchor.IdlTypes<MeanMultisig>["Owner"];

const MEAN_MULTISIG_OPS = new PublicKey("3TD6SWY9M1mLY2kZWJNavPLhwXvcRsWdnZLRaMzERJBw");
const ACCOUNT_REPLACEMENT_PLACEHOLDER = new PublicKey("NewPubkey1111111111111111111111111111111111");

describe("multisig", async () => {

    const provider = anchor.getProvider() as AnchorProvider;
    anchor.setProvider(provider);

    const setupInfo: { name: string, pubkey: PublicKey }[] = [];

    const program = anchor.workspace.MeanMultisig as Program<MeanMultisig>;
    setupInfo.push({ name: "PROGRAM", pubkey: program.programId });

    const payer = (program.provider as AnchorProvider).wallet.publicKey;
    setupInfo.push({ name: "PAYER", pubkey: payer });

    const [settings] = await PublicKey.findProgramAddress([Buffer.from(anchor.utils.bytes.utf8.encode("settings"))], program.programId);
    setupInfo.push({ name: "SETTINGS", pubkey: settings });

    const [programData] = await PublicKey.findProgramAddress([program.programId.toBytes()], new anchor.web3.PublicKey("BPFLoaderUpgradeab1e11111111111111111111111"));
    setupInfo.push({ name: "PROGRAMDATA", pubkey: programData });

    const multisig = Keypair.generate();
    setupInfo.push({ name: "MULTISIG", pubkey: multisig.publicKey });

    const transaction = Keypair.generate();
    setupInfo.push({ name: "TRANSACTION", pubkey: transaction.publicKey });

    const [txDetailAddress] = await PublicKey.findProgramAddress(
        [multisig.publicKey.toBuffer(), transaction.publicKey.toBuffer()],
        program.programId
    );
    console.log(`TXDETAILADDRESS: ${txDetailAddress}`);
    setupInfo.push({ name: "TXDETAILADDRESS", pubkey: txDetailAddress });

    const owner1Key = Keypair.generate();
    setupInfo.push({ name: "OWNER1", pubkey: owner1Key.publicKey });

    const owner2Key = Keypair.generate();
    setupInfo.push({ name: "OWNER2", pubkey: owner2Key.publicKey });

    const owner3Key = Keypair.generate();
    setupInfo.push({ name: "OWNER3", pubkey: owner3Key.publicKey });

    const nonOwnerKey = Keypair.generate();
    setupInfo.push({ name: "NONOWNER", pubkey: nonOwnerKey.publicKey });

    console.table(setupInfo.map(r => new Object({ NAME: r.name, PUBKEY: r.pubkey.toBase58() })));

    before(async () => {
        await fundWallet(provider, owner1Key);
        await fundWallet(provider, owner2Key);
        await fundWallet(provider, owner3Key);
        await fundWallet(provider, nonOwnerKey);

        await program.methods.initSettings().accounts({
            payer: payer,
            authority: payer,
            program: program.programId,
            programData,
            settings,
            systemProgram: SystemProgram.programId,
        }).rpc({ commitment: "confirmed" });

        const settingsAccount = await program.account.settings.fetch(
            settings,
            'confirmed'
        );

        assert.exists(settingsAccount);
        assert.ok(settingsAccount.authority.equals(payer));
        assert.ok(settingsAccount.opsAccount.equals(MEAN_MULTISIG_OPS));
        assert.ok(settingsAccount.createMultisigFee.eq(new BN(20_000_000)));
        assert.ok(settingsAccount.createTransactionFee.eq(new BN(20_000_000)));
    });

    // separate init settings is not needed anymore because it is done as part
    // of the 'before' hook
    // it("init settings", async () => {
    //     await program.methods.initSettings().accounts({
    //         payer: payer,
    //         authority: payer,
    //         program: program.programId,
    //         programData,
    //         settings,
    //         systemProgram: SystemProgram.programId,
    //     }).rpc({ commitment: "confirmed" });

    //     const settingsAccount = await program.account.settings.fetch(
    //         settings,
    //         'confirmed'
    //     );

    //     assert.exists(settingsAccount);
    //     assert.ok(settingsAccount.authority.equals(payer));
    //     assert.ok(settingsAccount.opsAccount.equals(MEAN_MULTISIG_OPS));
    //     assert.ok(settingsAccount.createMultisigFee.eq(new BN(20_000_000)));
    //     assert.ok(settingsAccount.createTransactionFee.eq(new BN(20_000_000)));
    // });

    it("creates multisig account", async () => {

        const label = 'Test';
        const threshold = new BN(2);
        const owners = [
            {
                address: owner1Key.publicKey,
                name: "owner1"
            },
            {
                address: owner2Key.publicKey,
                name: "owner2"
            },
            {
                address: owner3Key.publicKey,
                name: "owner3"
            }
        ];

        // create multisig
        const [, nonce] = await PublicKey.findProgramAddress(
            [multisig.publicKey.toBuffer()],
            program.programId
        );

        await program.methods
            .createMultisig(owners, new BN(threshold), nonce, label)
            .accounts({
                proposer: owner1Key.publicKey,
                multisig: multisig.publicKey,
                settings,
                opsAccount: MEAN_MULTISIG_OPS,
                systemProgram: SystemProgram.programId,
            })
            .signers([owner1Key, multisig])
            .rpc();
    });

    it("creates proposal", async () => {

        // create transaction with multiple instructions
        const ix1 = SystemProgram.transfer({
            fromPubkey: provider.wallet.publicKey,
            lamports: 10 * LAMPORTS_PER_SOL,
            toPubkey: nonOwnerKey.publicKey
        });

        const txSize = 1200;
        const createIx = await program.account.transaction.createInstruction(
            transaction,
            txSize
        );

        const title = 'Test transaction';
        const description = "This is a test transaction";
        type Instruction = {
            programId: PublicKey,
            accounts: {
                pubkey: PublicKey,
                isSigner: boolean,
                isWritable: boolean,
            }[],
            data: Buffer | undefined,
        }
        const operation = 1;
        let transactionInstruction: Instruction =
        {
            programId: ix1.programId,
            accounts: ix1.keys.map(key => ({ pubkey: key.pubkey, isSigner: key.isSigner, isWritable: key.isWritable })),
            data: ix1.data
        };

        await program.methods
            .createTransaction(
                ix1.programId,
                ix1.keys.map(key => ({ pubkey: key.pubkey, isSigner: key.isSigner, isWritable: key.isWritable })),
                ix1.data,
                operation,
                title,
                description,
                new BN((new Date().getTime() / 1000) + 3600),
                new BN(0),
                255
            )
            .preInstructions([createIx])
            .accounts({
                multisig: multisig.publicKey,
                transaction: transaction.publicKey,
                transactionDetail: txDetailAddress,
                proposer: owner1Key.publicKey,
                settings,
                opsAccount: MEAN_MULTISIG_OPS,
                systemProgram: SystemProgram.programId,
            })
            .signers([transaction, owner1Key])
            .rpc({
                commitment: 'confirmed',
            });
    });

    it("approves proposal (1 out of 2 needed)", async () => {
        await program.methods
            .approve()
            .accounts({
                multisig: multisig.publicKey,
                transaction: transaction.publicKey,
                transactionDetail: txDetailAddress,
                owner: owner1Key.publicKey,
                systemProgram: SystemProgram.programId,
            })
            .signers([owner1Key])
            .rpc(
                {
                    commitment: 'confirmed',
                }
            );
    });

    it("fails to approve by non onwer", async () => {
        await program.methods
            .approve()
            .accounts({
                multisig: multisig.publicKey,
                transaction: transaction.publicKey,
                transactionDetail: txDetailAddress,
                owner: owner1Key.publicKey,
                systemProgram: SystemProgram.programId,
            })
            .signers([owner1Key])
            .rpc(
                {
                    commitment: 'confirmed',
                }
            );
    });

    it("fails to execute proposal when not enough approvals", async () => {
        const txAccount = await program.account.transaction.fetch(
            transaction.publicKey,
            'confirmed'
        );

        const [multisigSigner] = await PublicKey.findProgramAddress(
            [multisig.publicKey.toBuffer()],
            program.programId
        );

        let remainingAccounts: {
            pubkey: PublicKey;
            isSigner: boolean;
            isWritable: boolean;
        }[] = (txAccount.accounts as TransactionAccount[])
            // Change the signer status on the vendor signer since it's signed by the program, not the client.
            .map((meta) =>
                meta.pubkey.equals(multisigSigner)
                    ? { ...meta, isSigner: false }
                    : meta
            )
            .concat({
                pubkey: txAccount.programId,
                isWritable: false,
                isSigner: false,
            });

        let expectedError: AnchorError | null = null;

        try {
            await program.methods
                .executeTransaction()
                .accounts({
                    multisig: multisig.publicKey,
                    multisigSigner: multisigSigner,
                    transaction: transaction.publicKey,
                    transactionDetail: txDetailAddress,
                    payer: nonOwnerKey.publicKey,
                    systemProgram: SystemProgram.programId,
                })
                .signers([nonOwnerKey]) // anyone can execute once it reaches the approval threshold
                .remainingAccounts(remainingAccounts)
                .rpc(
                    {
                        commitment: 'confirmed',
                    }
                );
            assert.fail("The statements above should fail");
        } catch (error) {
            // console.log(error);
            expectedError = error as AnchorError;
        }
        assert.isNotEmpty(expectedError);
        assert.equal(expectedError?.error.errorCode.code, 'NotEnoughSigners');
        assert.equal(expectedError?.error.errorCode.number, 6002);
        assert.equal(expectedError?.error.errorMessage, 'Not enough owners signed this transaction.');
    });

    it("approves proposal (2 out of 2 needed)", async () => {
        await program.methods
            .approve()
            .accounts({
                multisig: multisig.publicKey,
                transaction: transaction.publicKey,
                transactionDetail: txDetailAddress,
                owner: owner2Key.publicKey,
                systemProgram: SystemProgram.programId,
            })
            .signers([owner2Key])
            .rpc(
                {
                    commitment: 'confirmed',
                }
            );
    });

    it("rejects proposal", async () => {
        await program.methods
            .reject()
            .accounts({
                multisig: multisig.publicKey,
                transaction: transaction.publicKey,
                transactionDetail: txDetailAddress,
                owner: owner3Key.publicKey,
                systemProgram: SystemProgram.programId,
            })
            .signers([owner3Key])
            .rpc(
                {
                    commitment: 'confirmed',
                }
            );
    });

    it("executes proposal", async () => {
        const txAccount = await program.account.transaction.fetch(
            transaction.publicKey,
            'confirmed'
        );

        const [multisigSigner] = await PublicKey.findProgramAddress(
            [multisig.publicKey.toBuffer()],
            program.programId
        );

        let remainingAccounts: {
            pubkey: PublicKey;
            isSigner: boolean;
            isWritable: boolean;
        }[] = (txAccount.accounts as TransactionAccount[])
            // Change the signer status on the vendor signer since it's signed by the program, not the client.
            .map((meta) =>
                meta.pubkey.equals(multisigSigner)
                    ? { ...meta, isSigner: false }
                    : meta
            )
            .concat({
                pubkey: txAccount.programId,
                isWritable: false,
                isSigner: false,
            });

        const balanceBefore = await program.provider.connection
            .getBalance(nonOwnerKey.publicKey, 'confirmed');

        await program.methods
            .executeTransaction()
            .accounts({
                multisig: multisig.publicKey,
                multisigSigner: multisigSigner,
                transaction: transaction.publicKey,
                transactionDetail: txDetailAddress,
                payer: nonOwnerKey.publicKey,
                systemProgram: SystemProgram.programId,
            })
            .signers([nonOwnerKey]) // anyone can execute once it reaches the approval threshold
            .remainingAccounts(remainingAccounts)
            .rpc(
                {
                    commitment: 'confirmed',
                }
            );

        const balanceAfter = await program.provider.connection
            .getBalance(nonOwnerKey.publicKey, 'confirmed');
        const balanceIncrement = balanceAfter - balanceBefore;
        assert.equal(
            balanceIncrement,
            10_000_000_000,
            `incorrect balance after to ${nonOwnerKey.publicKey} transfer proposal executed`
        );
    });

    it("fails to create multisig with non-canonical bump", async () => {

        const label = 'Test non-canonical bump';
        const threshold = new BN(2);
        const owners = [
            {
                address: owner1Key.publicKey,
                name: "owner1"
            },
            {
                address: owner2Key.publicKey,
                name: "owner2"
            },
            {
                address: owner3Key.publicKey,
                name: "owner3"
            }
        ];

        const multisigKey = Keypair.generate();

        const [, nonce] = await PublicKey.findProgramAddress(
            [multisigKey.publicKey.toBuffer()],
            program.programId
        );
        // console.log(`canonical bump: ${nonce}`);

        // find next possible bump
        let nextBump = nonce - 1;
        for (nextBump; nextBump >= 0; nextBump--) {
            // console.log(nextBump);
            try {
                const signerAddress = await PublicKey.createProgramAddress(
                    [multisigKey.publicKey.toBuffer(), Buffer.from([nextBump])],
                    program.programId
                );
                // console.log(`bump: ${nextBump} produces an address off-curve: ${signerAddress}`);
                break;
            } catch (error) {
                // console.log(`bump: ${nextBump} produces an address on-curve... skipping`);
                continue;
            }
        }

        if (nextBump < 0) {
            assert.fail("Could not find next bump");
        }

        let expectedError: AnchorError | null = null;

        try {
            // create multisig
            await program.methods
                .createMultisig(owners, new BN(threshold), nextBump, label)
                .accounts({
                    proposer: owner1Key.publicKey,
                    multisig: multisigKey.publicKey,
                    settings,
                    opsAccount: MEAN_MULTISIG_OPS,
                    systemProgram: SystemProgram.programId,
                })
                .signers([owner1Key, multisigKey])
                .rpc();
            assert.fail("The statements above should fail");
        } catch (error) {
            // console.log(error);
            expectedError = error as AnchorError;
        }
        assert.isNotEmpty(expectedError);
        assert.equal(expectedError?.error.errorCode.code, 'InvalidMultisigNonce');
        assert.equal(expectedError?.error.errorCode.number, 6011);
        assert.equal(expectedError?.error.errorMessage, 'Multisig nonce is not valid.');
    });

    it("fails to create multisig with bump that producess on-curve signer address", async () => {

        const label = 'Test invalid bump';
        const threshold = new BN(2);
        const owners = [
            {
                address: owner1Key.publicKey,
                name: "owner1"
            },
            {
                address: owner2Key.publicKey,
                name: "owner2"
            },
            {
                address: owner3Key.publicKey,
                name: "owner3"
            }
        ];

        const multisigKey = Keypair.generate();

        // find next possible bump
        let nextBump = 255;
        for (nextBump; nextBump >= 0; nextBump--) {
            // console.log(nextBump);
            try {
                const signerAddress = await PublicKey.createProgramAddress(
                    [multisigKey.publicKey.toBuffer(), Buffer.from([nextBump])],
                    program.programId
                );
                // console.log(`bump: ${nextBump} produces an address off-curve: ${signerAddress}`);
                continue;
            } catch (error) {
                // console.log(`bump: ${nextBump} produces an address on-curve... skipping`);
                break;
            }
        }

        if (nextBump < 0) {
            assert.fail("Could not find next bump");
        }

        let expectedError: AnchorError | null = null;

        try {
            // create multisig
            await program.methods
                .createMultisig(owners, new BN(threshold), nextBump, label)
                .accounts({
                    proposer: owner1Key.publicKey,
                    multisig: multisigKey.publicKey,
                    settings,
                    opsAccount: MEAN_MULTISIG_OPS,
                    systemProgram: SystemProgram.programId,
                })
                .signers([owner1Key, multisigKey])
                .rpc();
            assert.fail("The statements above should fail");
        } catch (error) {
            // console.log(error);
            expectedError = error as AnchorError;
        }
        assert.isNotEmpty(expectedError);
        assert.equal(expectedError?.error.errorCode.code, 'InvalidMultisigNonce');
        assert.equal(expectedError?.error.errorCode.number, 6011);
        assert.equal(expectedError?.error.errorMessage, 'Multisig nonce is not valid.');
    });

    it("creates proposal with replacements", async () => {

        const multisig = Keypair.generate();
        const transaction = Keypair.generate();
        const [txDetailAddress] = await PublicKey.findProgramAddress(
            [multisig.publicKey.toBuffer(), transaction.publicKey.toBuffer()],
            program.programId
        );
        const owner1Key = Keypair.generate();
        await fundWallet(provider, owner1Key);
        const label = 'Test';
        const threshold = new BN(1);
        const owners = [
            {
                address: owner1Key.publicKey,
                name: "owner1"
            },
        ];
        const [multisigSigner, nonce] = await PublicKey.findProgramAddress(
            [multisig.publicKey.toBuffer()],
            program.programId
        );

        // create transaction with multiple instructions
        const rent = await provider.connection.getMinimumBalanceForRentExemption(100);

        const ix1 = SystemProgram.createAccount({
            /** The account that will transfer lamports to the created account */
            fromPubkey: provider.wallet.publicKey,
            /** Public key of the created account */
            newAccountPubkey: ACCOUNT_REPLACEMENT_PLACEHOLDER,
            /** Amount of lamports to transfer to the created account */
            lamports: rent,
            /** Amount of space in bytes to allocate to the created account */
            space: 100,
            /** Public key of the program to assign as the owner of the created account */
            programId: SystemProgram.programId
        });

        const txSize = 1200;
        const createIx = await program.account.transaction.createInstruction(
            transaction,
            txSize
        );

        const title = 'Test transaction';
        const description = "This is a test transaction";
        const operation = 1;

        try {

            // create multisig
            await program.methods
                .createMultisig(owners, new BN(threshold), nonce, label)
                .accounts({
                    proposer: owner1Key.publicKey,
                    multisig: multisig.publicKey,
                    settings,
                    opsAccount: MEAN_MULTISIG_OPS,
                    systemProgram: SystemProgram.programId,
                })
                .signers([owner1Key, multisig])
                .rpc();

            // create proposal
            await program.methods
                .createTransaction(
                    ix1.programId,
                    ix1.keys.map(key => ({ pubkey: key.pubkey, isSigner: key.isSigner, isWritable: key.isWritable })),
                    ix1.data,
                    operation,
                    title,
                    description,
                    new BN((new Date().getTime() / 1000) + 3600),
                    new BN(0),
                    255
                )
                .preInstructions([createIx])
                .accounts({
                    multisig: multisig.publicKey,
                    transaction: transaction.publicKey,
                    transactionDetail: txDetailAddress,
                    proposer: owner1Key.publicKey,
                    settings,
                    opsAccount: MEAN_MULTISIG_OPS,
                    systemProgram: SystemProgram.programId,
                })
                .signers([transaction, owner1Key])
                .rpc({
                    commitment: 'confirmed',
                });

            // approve proposal
            await program.methods
                .approve()
                .accounts({
                    multisig: multisig.publicKey,
                    transaction: transaction.publicKey,
                    transactionDetail: txDetailAddress,
                    owner: owner1Key.publicKey,
                    systemProgram: SystemProgram.programId,
                })
                .signers([owner1Key])
                .rpc(
                    {
                        commitment: 'confirmed',
                    }
                );

            // execute proposal tx with replacements
            const txAccount = await program.account.transaction.fetch(
                transaction.publicKey,
                'confirmed'
            );

            const replacementKeys: Keypair[] = [];
            let remainingAccounts: {
                pubkey: PublicKey;
                isSigner: boolean;
                isWritable: boolean;
            }[] = (txAccount.accounts as TransactionAccount[])
                // Change the signer status on the vendor signer since it's signed by the program, not the client.
                .map((meta) => {
                    if (meta.pubkey.equals(multisigSigner)) {
                        return { ...meta, isSigner: false }
                    } else if (meta.pubkey.equals(ACCOUNT_REPLACEMENT_PLACEHOLDER)) {
                        const replacementKey = Keypair.generate();
                        replacementKeys.push(replacementKey);
                        return { ...meta, pubkey: replacementKey.publicKey }
                    }

                    return meta
                })
                .concat({
                    pubkey: txAccount.programId,
                    isWritable: false,
                    isSigner: false,
                });

            await program.methods
                .executeTransactionWithReplacements(
                    replacementKeys.map(k => k.publicKey)
                )
                .accounts({
                    multisig: multisig.publicKey,
                    multisigSigner: multisigSigner,
                    transaction: transaction.publicKey,
                    transactionDetail: txDetailAddress,
                    payer: nonOwnerKey.publicKey,
                    systemProgram: SystemProgram.programId,
                })
                .signers([nonOwnerKey]) // anyone can execute once it reaches the approval threshold
                .remainingAccounts(remainingAccounts)
                .signers(replacementKeys)
                .rpc(
                    {
                        commitment: 'confirmed',
                    }
                );


            const createdAccount = await provider.connection
                .getAccountInfo(replacementKeys[0].publicKey, { commitment: 'confirmed' });
            assert.exists(createdAccount);
            assert.equal(createdAccount?.lamports, rent);
            assert.equal(createdAccount?.data.length, 100);
        } catch (error) {
            console.log(error);
            throw error;
        }
    });

    it("fails to create proposal with not enough replacements", async () => {

        const multisig = Keypair.generate();
        const transaction = Keypair.generate();
        const [txDetailAddress] = await PublicKey.findProgramAddress(
            [multisig.publicKey.toBuffer(), transaction.publicKey.toBuffer()],
            program.programId
        );
        const owner1Key = Keypair.generate();
        await fundWallet(provider, owner1Key);
        const label = 'Test';
        const threshold = new BN(1);
        const owners = [
            {
                address: owner1Key.publicKey,
                name: "owner1"
            },
        ];
        const [multisigSigner, nonce] = await PublicKey.findProgramAddress(
            [multisig.publicKey.toBuffer()],
            program.programId
        );

        // create transaction with multiple instructions
        const rent = await provider.connection.getMinimumBalanceForRentExemption(100);

        const ix1 = SystemProgram.createAccount({
            /** The account that will transfer lamports to the created account */
            fromPubkey: provider.wallet.publicKey,
            /** Public key of the created account */
            newAccountPubkey: ACCOUNT_REPLACEMENT_PLACEHOLDER,
            /** Amount of lamports to transfer to the created account */
            lamports: rent,
            /** Amount of space in bytes to allocate to the created account */
            space: 100,
            /** Public key of the program to assign as the owner of the created account */
            programId: SystemProgram.programId
        });

        const txSize = 1200;
        const createIx = await program.account.transaction.createInstruction(
            transaction,
            txSize
        );

        const title = 'Test transaction';
        const description = "This is a test transaction";
        const operation = 1;

        let expectedError: AnchorError | null = null;
        try {

            // create multisig
            await program.methods
                .createMultisig(owners, new BN(threshold), nonce, label)
                .accounts({
                    proposer: owner1Key.publicKey,
                    multisig: multisig.publicKey,
                    settings,
                    opsAccount: MEAN_MULTISIG_OPS,
                    systemProgram: SystemProgram.programId,
                })
                .signers([owner1Key, multisig])
                .rpc();

            // create proposal
            await program.methods
                .createTransaction(
                    ix1.programId,
                    ix1.keys.map(key => ({ pubkey: key.pubkey, isSigner: key.isSigner, isWritable: key.isWritable })),
                    ix1.data,
                    operation,
                    title,
                    description,
                    new BN((new Date().getTime() / 1000) + 3600),
                    new BN(0),
                    255
                )
                .preInstructions([createIx])
                .accounts({
                    multisig: multisig.publicKey,
                    transaction: transaction.publicKey,
                    transactionDetail: txDetailAddress,
                    proposer: owner1Key.publicKey,
                    settings,
                    opsAccount: MEAN_MULTISIG_OPS,
                    systemProgram: SystemProgram.programId,
                })
                .signers([transaction, owner1Key])
                .rpc({
                    commitment: 'confirmed',
                });

            // approve proposal
            await program.methods
                .approve()
                .accounts({
                    multisig: multisig.publicKey,
                    transaction: transaction.publicKey,
                    transactionDetail: txDetailAddress,
                    owner: owner1Key.publicKey,
                    systemProgram: SystemProgram.programId,
                })
                .signers([owner1Key])
                .rpc(
                    {
                        commitment: 'confirmed',
                    }
                );

            // execute proposal tx with replacements
            const txAccount = await program.account.transaction.fetch(
                transaction.publicKey,
                'confirmed'
            );

            const replacementKeys: Keypair[] = [];
            let remainingAccounts: {
                pubkey: PublicKey;
                isSigner: boolean;
                isWritable: boolean;
            }[] = (txAccount.accounts as TransactionAccount[])
                // Change the signer status on the vendor signer since it's signed by the program, not the client.
                .map((meta) => {
                    if (meta.pubkey.equals(multisigSigner)) {
                        return { ...meta, isSigner: false }
                    } else if (meta.pubkey.equals(ACCOUNT_REPLACEMENT_PLACEHOLDER)) {
                        const replacementKey = Keypair.generate();
                        replacementKeys.push(replacementKey);
                        return { ...meta, pubkey: replacementKey.publicKey }
                    }

                    return meta
                })
                .concat({
                    pubkey: txAccount.programId,
                    isWritable: false,
                    isSigner: false,
                });

            await program.methods
                .executeTransactionWithReplacements(
                    []
                )
                .accounts({
                    multisig: multisig.publicKey,
                    multisigSigner: multisigSigner,
                    transaction: transaction.publicKey,
                    transactionDetail: txDetailAddress,
                    payer: nonOwnerKey.publicKey,
                    systemProgram: SystemProgram.programId,
                })
                .signers([nonOwnerKey]) // anyone can execute once it reaches the approval threshold
                .remainingAccounts(remainingAccounts)
                .signers(replacementKeys)
                .rpc(
                    {
                        commitment: 'confirmed',
                    }
                );
            assert.fail("The statement above should fail")
        } catch (error) {
            // console.log(error);
            expectedError = error as AnchorError;
            assert.isNotEmpty(expectedError);
            assert.strictEqual(expectedError?.error.errorCode.code, 'NotEnoughReplacementAccounts');
            assert.strictEqual(expectedError?.error.errorCode.number, 6016);
            assert.strictEqual(expectedError?.error.errorMessage, 'Not enough replacement accounts.');
        }
    });

    it("updates settings", async () => {
        const newAuthority1Key = Keypair.generate();
        const newAuthority2Key = Keypair.generate();
        const newOpsAccountKey = Keypair.generate();
        const newCreateMultisigFee = 50_000_000;
        const newCreateTransactionFee = 60_000_000;

        await program.methods.updateSettings(
            newAuthority1Key.publicKey,
            newOpsAccountKey.publicKey,
            new BN(newCreateMultisigFee),
            new BN(newCreateTransactionFee)
        )
            .accounts({
                authority: payer,
                settings,
                program: program.programId,
                programData,
            }).rpc({ commitment: "confirmed" });

        let settingsAccount = await program.account.settings.fetch(
            settings,
            'confirmed'
        );

        assert.exists(settingsAccount);
        assert.ok(settingsAccount.authority.equals(newAuthority1Key.publicKey));
        assert.ok(settingsAccount.opsAccount.equals(newOpsAccountKey.publicKey));
        assert.equal(settingsAccount.createMultisigFee.toString(), "50000000");
        assert.equal(settingsAccount.createTransactionFee.toString(), "60000000");

        // Asserts that the new settings authority can update the settings
        await program.methods.updateSettings(
            newAuthority2Key.publicKey,
            newOpsAccountKey.publicKey,
            new BN(newCreateMultisigFee),
            new BN(newCreateTransactionFee)
        )
            .accounts({
                authority: newAuthority1Key.publicKey,
                settings,
                program: program.programId,
                programData,
            })
            .signers([newAuthority1Key])
            .rpc({ commitment: "confirmed" });

        settingsAccount = await program.account.settings.fetch(
            settings,
            'confirmed'
        );
        assert.ok(settingsAccount.authority.equals(newAuthority2Key.publicKey));

        // Asserts that the original settings authority continues to be able to
        // update the setings (because it is the program upgrade authority)
        await program.methods.updateSettings(
            payer,
            newOpsAccountKey.publicKey,
            new BN(newCreateMultisigFee),
            new BN(newCreateTransactionFee)
        )
            .accounts({
                authority: payer,
                settings,
                program: program.programId,
                programData,
            })
            // .signers([...]) // signed with payer authomatically by anchor
            .rpc({ commitment: "confirmed" });

        settingsAccount = await program.account.settings.fetch(
            settings,
            'confirmed'
        );
        assert.ok(settingsAccount.authority.equals(payer));

        // Assert that previous settings authority can not update the settings anymore 
        let expectedError: AnchorError | null = null;

        try {
            await program.methods.updateSettings(
                newAuthority1Key.publicKey,
                newOpsAccountKey.publicKey,
                new BN(newCreateMultisigFee),
                new BN(newCreateTransactionFee)
            )
                .accounts({
                    authority: newAuthority1Key.publicKey,
                    settings,
                    program: program.programId,
                    programData,
                })
                .signers([newAuthority1Key])
                .rpc({ commitment: "confirmed" });
            assert.fail("The statements above should fail");
        } catch (error) {
            // console.log(error);
            expectedError = error as AnchorError;
        }
        assert.isNotEmpty(expectedError);
        assert.equal(expectedError?.error.errorCode.code, 'InvalidSettingsAuthority');
        assert.equal(expectedError?.error.errorCode.number, 6015);
        assert.equal(expectedError?.error.errorMessage, 'Invalid settings authority.');

        try {
            await program.methods.updateSettings(
                newAuthority1Key.publicKey,
                newOpsAccountKey.publicKey,
                new BN(10_000_000_001), // <-- passing more than 10 SOL (max fee allowed)
                new BN(newCreateTransactionFee)
            )
                .accounts({
                    authority: payer,
                    settings,
                    program: program.programId,
                    programData,
                })
                .rpc({ commitment: "confirmed" });
            assert.fail("The statements above should fail");
        } catch (error) {
            // console.log(error);
            expectedError = error as AnchorError;
        }
        assert.isNotEmpty(expectedError);
        assert.equal(expectedError?.error.errorCode.code, 'FeeExceedsMaximumAllowed');
        assert.equal(expectedError?.error.errorCode.number, 6017);
        assert.equal(expectedError?.error.errorMessage, 'Fee amount exceeds the maximum allowed.');

        try {
            await program.methods.updateSettings(
                newAuthority1Key.publicKey,
                newOpsAccountKey.publicKey,
                new BN(newCreateMultisigFee),
                new BN(10_000_000_001) // <-- passing more than 10 SOL (max fee allowed)
            )
                .accounts({
                    authority: payer,
                    settings,
                    program: program.programId,
                    programData,
                })
                .rpc({ commitment: "confirmed" });
            assert.fail("The statements above should fail");
        } catch (error) {
            // console.log(error);
            expectedError = error as AnchorError;
        }
        assert.isNotEmpty(expectedError);
        assert.equal(expectedError?.error.errorCode.code, 'FeeExceedsMaximumAllowed');
        assert.equal(expectedError?.error.errorCode.number, 6017);
        assert.equal(expectedError?.error.errorMessage, 'Fee amount exceeds the maximum allowed.');
    });
});

describe("multisig-owners", async () => {
    const provider = anchor.getProvider() as AnchorProvider;
    anchor.setProvider(provider);
    const program = anchor.workspace.MeanMultisig as Program<MeanMultisig>;
    const [settings] = await PublicKey.findProgramAddress([Buffer.from(anchor.utils.bytes.utf8.encode("settings"))], program.programId);

    it("tests the multisig program", async () => {
        const multisig = anchor.web3.Keypair.generate();
        const [multisigSigner, nonce] =
            await anchor.web3.PublicKey.findProgramAddress(
                [multisig.publicKey.toBuffer()],
                program.programId
            );
        const [settings] = await PublicKey.findProgramAddress([Buffer.from(anchor.utils.bytes.utf8.encode("settings"))], program.programId);

        const ownerA = anchor.web3.Keypair.generate();
        const ownerB = anchor.web3.Keypair.generate();
        const ownerC = anchor.web3.Keypair.generate();
        const ownerD = anchor.web3.Keypair.generate();
        const owners2 = [
            {
                address: ownerA.publicKey,
                name: "ownerA"
            },
            {
                address: ownerB.publicKey,
                name: "ownerB"
            },
            {
                address: ownerC.publicKey,
                name: "ownerC"
            }
        ];

        await fundWallet(provider, ownerA);

        const threshold = new anchor.BN(2);

        await program.methods
            .createMultisig(owners2, new BN(threshold), nonce, "Safe2")
            .accounts({
                proposer: ownerA.publicKey,
                multisig: multisig.publicKey,
                settings: settings,
                opsAccount: MEAN_MULTISIG_OPS,
                systemProgram: SystemProgram.programId,
            })
            .signers([ownerA, multisig])
            .rpc();

        let multisigAccount = await program.account.multisigV2.fetch(
            multisig.publicKey
        );
        assert.strictEqual(multisigAccount.nonce, nonce);
        assert.ok(multisigAccount.threshold.eq(new anchor.BN(2)));
        let fetchedOwners = multisigAccount.owners as Owner[];
        assert.ok(fetchedOwners[0].address.equals(ownerA.publicKey));
        assert.ok(fetchedOwners[1].address.equals(ownerB.publicKey));
        assert.ok(fetchedOwners[2].address.equals(ownerC.publicKey));
        assert.ok(multisigAccount.ownerSetSeqno === 0);

        const pid = program.programId;
        const accounts = [
            {
                pubkey: multisig.publicKey,
                isWritable: true,
                isSigner: false,
            },
            {
                pubkey: multisigSigner,
                isWritable: false,
                isSigner: true,
            },
        ];
        const newOwners = [
            {
                address: ownerA.publicKey,
                name: "ownerA"
            },
            {
                address: ownerB.publicKey,
                name: "ownerB"
            },
            {
                address: ownerD.publicKey,
                name: "ownerD"
            }
        ];
        const data = program.coder.instruction.encode("edit_multisig", {
            owners: newOwners,
            threshold: new BN(2),
            label: "Safe2.1"
        });

        const transaction = anchor.web3.Keypair.generate();
        const txSize = 1000; // Big enough, cuz I'm lazy.

        const [txDetailAddress] = await PublicKey.findProgramAddress(
            [multisig.publicKey.toBuffer(), transaction.publicKey.toBuffer()],
            program.programId
        );

        await program.methods
            .createTransaction(
                pid,
                accounts,
                data,
                1,
                "Update owners",
                "Update owners",
                new BN((new Date().getTime() / 1000) + 3600),
                new BN(0),
                255
            )
            .preInstructions(
                [
                    await program.account.transaction.createInstruction(
                        transaction,
                        txSize
                    )
                ])
            .accounts({
                multisig: multisig.publicKey,
                transaction: transaction.publicKey,
                transactionDetail: txDetailAddress,
                proposer: ownerA.publicKey,
                settings: settings,
                opsAccount: MEAN_MULTISIG_OPS,
                systemProgram: SystemProgram.programId,
            })
            .signers([transaction, ownerA])
            .rpc({
                commitment: 'confirmed',
            });

        let txAccount = await program.account.transaction.fetch(
            transaction.publicKey
        );

        assert.ok(txAccount.programId.equals(pid));
        assert.deepStrictEqual(txAccount.accounts, accounts);
        assert.deepStrictEqual(txAccount.data, data);
        assert.ok(txAccount.multisig.equals(multisig.publicKey));
        assert.ok(txAccount.executedOn.eq(new BN(0)));
        assert.ok(txAccount.ownerSetSeqno === 0);

        // Other owner approves transaction.
        await program.methods
            .approve()
            .accounts({
                multisig: multisig.publicKey,
                transaction: transaction.publicKey,
                transactionDetail: txDetailAddress,
                owner: ownerB.publicKey,
                systemProgram: SystemProgram.programId,
            })
            .signers([ownerB])
            .rpc(
                {
                    commitment: 'confirmed',
                }
            );

        // Now that we've reached the threshold, send the transactoin.

        let remainingAccounts2 = [
            {
                pubkey: multisig.publicKey,
                isWritable: true,
                isSigner: false,
            },
            {
                pubkey: multisigSigner,
                isWritable: false,
                isSigner: false,
            },
            {
                pubkey: txAccount.programId,
                isWritable: false,
                isSigner: false,
            },
        ];
        // Or we can do:
        // let remainingAccounts2: {
        //     pubkey: PublicKey;
        //     isSigner: boolean;
        //     isWritable: boolean;
        // }[] = (txAccount2.accounts as TransactionAccount[])
        //     // Change the signer status on the vendor signer since it's signed by the program, not the client.
        //     .map((meta) =>
        //         meta.pubkey.equals(multisigSigner2)
        //             ? { ...meta, isSigner: false }
        //             : meta
        //     )
        //     .concat({
        //         pubkey: txAccount2.programId,
        //         isWritable: false,
        //         isSigner: false,
        //     });

        const randomExecutor = Keypair.generate();

        await program.methods
            .executeTransaction()
            .accounts({
                multisig: multisig.publicKey,
                multisigSigner: multisigSigner,
                transaction: transaction.publicKey,
                transactionDetail: txDetailAddress,
                payer: randomExecutor.publicKey,
                systemProgram: SystemProgram.programId,
            })
            .signers([randomExecutor]) // anyone can execute once it reaches the approval threshold
            .remainingAccounts(remainingAccounts2)
            .rpc(
                {
                    commitment: 'confirmed',
                }
            );

        multisigAccount = await program.account.multisigV2.fetch(multisig.publicKey);

        txAccount = await program.account.transaction.fetch(
            transaction.publicKey
        );

        assert.strictEqual(multisigAccount.nonce, nonce);
        assert.ok(multisigAccount.threshold.eq(new anchor.BN(2)));
        fetchedOwners = multisigAccount.owners as Owner[];
        assert.ok(fetchedOwners[0].address.equals(ownerA.publicKey));
        assert.ok(fetchedOwners[1].address.equals(ownerB.publicKey));
        assert.ok(fetchedOwners[2].address.equals(ownerD.publicKey));
        assert.ok(multisigAccount.ownerSetSeqno === 1);
        assert.ok(txAccount.executedOn.gt(new BN(0)));
    });

    it("assert unique owners", async () => {
        const multisig = anchor.web3.Keypair.generate();
        const ownerA = anchor.web3.Keypair.generate();
        const ownerB = anchor.web3.Keypair.generate();
        const owners = [
            {
                address: ownerA.publicKey,
                name: "ownerA"
            },
            {
                address: ownerB.publicKey,
                name: "ownerB"
            },
            {
                address: ownerA.publicKey,
                name: "ownerA"
            }
        ];

        await fundWallet(provider, ownerA);

        const threshold = new anchor.BN(2);
        try {
            // create multisig
            const [, nonce] = await PublicKey.findProgramAddress(
                [multisig.publicKey.toBuffer()],
                program.programId
            );

            await program.methods
                .createMultisig(owners, new BN(threshold), nonce, "Safe3")
                .accounts({
                    proposer: ownerA.publicKey,
                    multisig: multisig.publicKey,
                    settings,
                    opsAccount: MEAN_MULTISIG_OPS,
                    systemProgram: SystemProgram.programId,
                })
                .signers([ownerA, multisig])
                .rpc();
            assert.fail();
        } catch (err) {
            const error = (err as AnchorError).error;
            assert.strictEqual(error.errorCode.number, 6009);
            assert.strictEqual(error.errorMessage, "Owners must be unique.");
        }
    });
});

async function fundWallet(provider: AnchorProvider, userKey: Keypair) {
    const tx = new Transaction();
    tx.add(SystemProgram.transfer({
        fromPubkey: provider.wallet.publicKey,
        lamports: 1000 * LAMPORTS_PER_SOL,
        toPubkey: userKey.publicKey
    }));
    await provider.sendAndConfirm(tx);
};

function sleep(ms: number) {
    console.log('Sleeping for', ms / 1000, 'seconds');
    return new Promise((resolve) => setTimeout(resolve, ms));
}