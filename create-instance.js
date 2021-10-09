const anchor = require("@project-serum/anchor");

const multisig = anchor.web3.Keypair.generate();
const [
    multisigSigner,
    nonce,
] = await anchor.web3.PublicKey.findProgramAddress(
    [multisig.publicKey.toBuffer()],
    program.programId
);
const multisigSize = 200; // Big enough.

const ownerA = anchor.web3.Keypair.generate();
const ownerB = anchor.web3.Keypair.generate();
const ownerC = anchor.web3.Keypair.generate();
const ownerD = anchor.web3.Keypair.generate();
const owners = [ownerA.publicKey, ownerB.publicKey, ownerC.publicKey];

const threshold = new anchor.BN(2);
await program.rpc.createMultisig(owners, threshold, nonce, {
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

