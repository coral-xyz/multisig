// Migrations are an early feature. Currently, they're nothing more than this
// single deploy script that's invoked from the CLI, injecting a provider
// configured from the workspace's Anchor.toml.


const { PublicKey, SystemProgram } = require('@solana/web3.js');
const anchor = require('@coral-xyz/anchor');


module.exports = async function (provider) {
  // Configure client to use the provider.
  anchor.setProvider(provider);

  // Add your deploy script here.

  // Program client handle.
  const program = anchor.workspace.MeanMultisig;
  console.log(`program ID: ${program.programId}`);
  

  const [settings] = await PublicKey.findProgramAddress(
    [Buffer.from(anchor.utils.bytes.utf8.encode('settings'))],
    program.programId
  );
  const [programData] = await PublicKey.findProgramAddress(
    [program.programId.toBytes()],
    new PublicKey('BPFLoaderUpgradeab1e11111111111111111111111')
  );
  await program.methods
    .initSettings()
    .accounts({
      payer: (program.provider).wallet.publicKey,
      authority: (program.provider).wallet.publicKey,
      program: program.programId,
      programData,
      settings,
      systemProgram: SystemProgram.programId
    })
    .rpc();
}
