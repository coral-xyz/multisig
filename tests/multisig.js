const anchor = require("@project-serum/anchor");
const assert = require("assert");

describe("multisig", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.SerumMultisig;
  const ownerA = anchor.web3.Keypair.generate();
  const ownerB = anchor.web3.Keypair.generate();
  const ownerC = anchor.web3.Keypair.generate();
  const ownerD = anchor.web3.Keypair.generate();
  const multisig = anchor.web3.Keypair.generate();

  let multisigSigner;
  let multisigSignerNonce;

  let staleTx = anchor.web3.Keypair.generate();

  it("Creates PDA", async () => {
    const [
      _multisigSigner,
      _multisigSignerNonce,
    ] = await anchor.web3.PublicKey.findProgramAddress(
      [multisig.publicKey.toBuffer()],
      program.programId
    );

    multisigSigner = _multisigSigner;
    multisigSignerNonce = _multisigSignerNonce;
  });

  it("Creates the multisig", async () => {
    const multisigSize = 200; // Big enough.
    const owners = [ownerA.publicKey, ownerB.publicKey, ownerC.publicKey];
    const threshold = new anchor.BN(2);
    await program.rpc.createMultisig(owners, threshold, multisigSignerNonce, {
      accounts: {
        multisig: multisig.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      instructions: [
        await program.account.multisig.createInstruction(
          multisig,
          multisigSize
        ),
      ],
      signers: [multisig],
    });

    let multisigAccount = await program.account.multisig.fetch(
      multisig.publicKey
    );
    assert.strictEqual(multisigAccount.nonce, multisigSignerNonce);
    assert.ok(multisigAccount.threshold.eq(new anchor.BN(2)));
    assert.deepStrictEqual(multisigAccount.owners, owners);
    assert.ok(multisigAccount.ownerSetSeqno === 0);
  });

  it("A creates and signs a tx B and C don't agree with", async () => {
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
    const newOwners = [ownerA.publicKey, ownerC.publicKey, ownerD.publicKey];
    const data = program.coder.instruction.encode("set_owners", {
      owners: newOwners,
    });
    const transaction = anchor.web3.Keypair.generate();
    const txSize = 1000; // Big enough, cuz I'm lazy.
    await program.rpc.createTransaction(program.programId, accounts, data, {
      accounts: {
        multisig: multisig.publicKey,
        transaction: transaction.publicKey,
        proposer: ownerA.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      instructions: [
        await program.account.transaction.createInstruction(
          transaction,
          txSize
        ),
      ],
      signers: [transaction, ownerA],
    });

    const txAccount = await program.account.transaction.fetch(
      transaction.publicKey
    );
    assert.ok(txAccount.programId.equals(program.programId));
    assert.deepStrictEqual(txAccount.accounts, accounts);
    assert.deepStrictEqual(txAccount.data, data);
    assert.ok(txAccount.multisig.equals(multisig.publicKey));
    assert.deepStrictEqual(txAccount.didExecute, false);
    assert.ok(txAccount.ownerSetSeqno === 0);

    staleTx = transaction.publicKey;
  });

  it("B and C change the owner set to B, C, D", async () => {
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
    const newOwners = [ownerB.publicKey, ownerC.publicKey, ownerD.publicKey];
    const data = program.coder.instruction.encode("set_owners", {
      owners: newOwners,
    });
    const transaction = anchor.web3.Keypair.generate();
    const txSize = 1000; // Big enough, cuz I'm lazy.
    await program.rpc.createTransaction(program.programId, accounts, data, {
      accounts: {
        multisig: multisig.publicKey,
        transaction: transaction.publicKey,
        proposer: ownerB.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      instructions: [
        await program.account.transaction.createInstruction(
          transaction,
          txSize
        ),
      ],
      signers: [transaction, ownerB],
    });

    const txAccount = await program.account.transaction.fetch(
      transaction.publicKey
    );
    assert.ok(txAccount.programId.equals(program.programId));
    assert.deepStrictEqual(txAccount.accounts, accounts);
    assert.deepStrictEqual(txAccount.data, data);
    assert.ok(txAccount.multisig.equals(multisig.publicKey));
    assert.deepStrictEqual(txAccount.didExecute, false);
    assert.ok(txAccount.ownerSetSeqno === 0);

    // C approves transaction.
    await program.rpc.approve({
      accounts: {
        multisig: multisig.publicKey,
        transaction: transaction.publicKey,
        owner: ownerC.publicKey,
      },
      signers: [ownerC],
    });

    // Now that we've reached the threshold, send the transactoin.
    await program.rpc.executeTransaction({
      accounts: {
        multisig: multisig.publicKey,
        multisigSigner,
        transaction: transaction.publicKey,
      },
      remainingAccounts: program.instruction.setOwners
        .accounts({
          multisig: multisig.publicKey,
          multisigSigner,
        })
        // Change the signer status on the vendor signer since it's signed by the program, not the client.
        .map((meta) =>
          meta.pubkey.equals(multisigSigner)
            ? { ...meta, isSigner: false }
            : meta
        )
        .concat({
          pubkey: program.programId,
          isWritable: false,
          isSigner: false,
        }),
    });

    multisigAccount = await program.account.multisig.fetch(multisig.publicKey);

    assert.strictEqual(multisigAccount.nonce, multisigSignerNonce);
    assert.ok(multisigAccount.threshold.eq(new anchor.BN(2)));
    assert.deepStrictEqual(multisigAccount.owners, newOwners);
    assert.ok(multisigAccount.ownerSetSeqno === 1);
  });

  it("D tries to approve the old stale transaction (created by A) and fails", async () => {
    await assert.rejects(
      async () => {
        await program.rpc.approve({
          accounts: {
            multisig: multisig.publicKey,
            transaction: staleTx,
            owner: ownerD.publicKey,
          },
          signers: [ownerD],
        });
      },
      (err) => {
        // InvaldOwnerSetSeqno error code.
        assert.ok(err.code === 308);
        return true;
      }
    );
  });
});
