// Migrations are an early feature. Currently, they're nothing more than this
// single deploy script that's invoked from the CLI, injecting a provider
// configured from the workspace's Anchor.toml.


import { PublicKey, SystemProgram } from '@solana/web3.js';
import * as anchor from "@coral-xyz/anchor";
import { MeanMultisig } from "../target/types/mean_multisig"; 


module.exports = async function (provider: anchor.Provider) {
  // Configure client to use the provider.
  anchor.setProvider(provider);

  // Add your deploy script here.

  // Program client handle.
  const program = anchor.workspace.MeanMultisigas as anchor.Program<MeanMultisig>;
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
      payer: (program.provider as anchor.AnchorProvider).wallet.publicKey,
      authority: (program.provider as anchor.AnchorProvider).wallet.publicKey,
      program: program.programId,
      programData,
      settings,
      systemProgram: SystemProgram.programId
    })
    .rpc();
}
