// use anchor_client::{
//     anchor_lang::{AnchorDeserialize, AnchorSerialize, InstructionData, ToAccountMetas},
//     solana_sdk::{
//         bpf_loader_upgradeable, instruction::Instruction,
//         loader_upgradeable_instruction::UpgradeableLoaderInstruction, pubkey::Pubkey, signer,
//         signer::Signer, system_instruction, system_program, sysvar,
//     },
// };
// use anchor_spl::token::{self, Mint, TokenAccount};
// use anyhow::{bail, Result};
// use custody::GenerateTokenBumpSeeds;
// use jet::state::MarketFlags;
// use serum_multisig::{DelegateList, Transaction, TransactionAccount};
// /// Extra business logic built on top of multisig program's core functionality
// use std::{io::Write, path::PathBuf};

// use crate::{gateway::MultisigGateway, request_builder::RequestBuilder};

// pub struct MultisigService<'a> {
//     pub program: MultisigGateway<'a>,
// }

// struct DynamicInstructionData {
//     data: Vec<u8>,
// }

// pub fn dynamic<T: InstructionData>(id: T) -> DynamicInstructionData {
//     DynamicInstructionData { data: id.data() }
// }

// impl AnchorSerialize for DynamicInstructionData {
//     fn serialize<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> {
//         todo!()
//     }
// }

// impl InstructionData for DynamicInstructionData {
//     fn data(&self) -> Vec<u8> {
//         self.data.clone()
//     }
// }

// impl<'a> MultisigService<'a> {
//     pub fn add_delegates(&self, multisig: Pubkey, delegates: Vec<Pubkey>) -> Result<()> {
//         let (delegate_list_account, _) = self
//             .program
//             .delegate_list(multisig, self.program.payer.pubkey());

//         let mut existing = match self
//             .program
//             .client
//             .account::<DelegateList>(delegate_list_account)
//         {
//             Ok(list) => list.delegates,
//             Err(_) => {
//                 self.program.create_delegate_list(multisig, delegates)?;
//                 return Ok(());
//             }
//         };

//         existing.extend(delegates);
//         println!("delegates = {:#?}", existing);

//         self.program
//             .set_delegate_list(multisig, delegate_list_account, existing)?;
//         Ok(())
//     }

//     pub fn remove_delegates(&self, multisig: Pubkey, delegates: Vec<Pubkey>) -> Result<()> {
//         let (delegate_list_account, _) = self
//             .program
//             .delegate_list(multisig, self.program.payer.pubkey());

//         let existing = match self
//             .program
//             .client
//             .account::<DelegateList>(delegate_list_account)
//         {
//             Ok(list) => list.delegates,
//             Err(_) => {
//                 // FIXME: silent failure if its a network issue?
//                 return Ok(());
//             }
//         };

//         let new_list = existing
//             .into_iter()
//             .filter(|d| !delegates.contains(d))
//             .collect();
//         println!("delegates = {:#?}", new_list);

//         self.program
//             .set_delegate_list(multisig, delegate_list_account, new_list)?;
//         Ok(())
//     }

//     /// jet protocol instruction
//     pub fn propose_set_market_flags(
//         &self,
//         multisig: Pubkey,
//         market: Pubkey,
//         flags: MarketFlags,
//     ) -> Result<Pubkey> {
//         self.propose_anchor_instruction(
//             None,
//             multisig,
//             custody::id(),
//             jet::accounts::SetMarketFlags {
//                 market,
//                 owner: self.program.signer(multisig).0,
//             },
//             jet::instruction::SetMarketFlags { 
//                 flags: flags.bits()
//             },
//         )
//     }

//     pub fn propose_mint_tokens(
//         &self,
//         multisig: Pubkey,
//         mint: Pubkey,
//         target: Pubkey,
//         amount: u64,
//     ) -> Result<Pubkey> {
//         let signer = self.program.signer(multisig).0;
//         let ix = spl_token::instruction::mint_to(
//             &spl_token::id(),
//             &mint,
//             &target,
//             &signer,
//             &vec![&signer],
//             amount,
//         )?;
//         self.propose_solana_instruction(&multisig, ix)
//     }

//     pub fn propose_transfer_tokens(
//         &self,
//         multisig: Pubkey,
//         source: Pubkey,
//         target: Pubkey,
//         amount: u64,
//     ) -> Result<Pubkey> {
//         let signer = self.program.signer(multisig).0;
//         let ix = spl_token::instruction::transfer(
//             &spl_token::id(),
//             &source,
//             &target,
//             &signer,
//             &vec![&signer],
//             amount,
//         )?;
//         self.propose_solana_instruction(&multisig, ix)
//     }

//     pub fn propose_custody_generate_token_mint(
//         &self,
//         multisig: Pubkey,
//         mint_key: PathBuf,
//     ) -> Result<Pubkey> {
//         let mint = match signer::keypair::read_keypair_file(&mint_key) {
//             Ok(m) => m,
//             Err(e) => bail!(
//                 "couldn't load mint key '{}', because: {:?}",
//                 mint_key.display(),
//                 e
//             ),
//         };
//         let signer = Pubkey::find_program_address(&[b"signer"], &custody::id()).0;
//         let getaddr = |seed: &[u8]| {
//             Pubkey::find_program_address(&[seed, mint.pubkey().as_ref()], &custody::id())
//         };
//         let seed_vault = getaddr(b"seed-vault");
//         let team_vault = getaddr(b"team-vault");
//         let d_vault = getaddr(b"d-vault");
//         let e_vault = getaddr(b"e-vault");
//         println!(
//             "mint {}\nsigner {}\nseed {}\nteam {}\nd {}\ne {}",
//             mint.pubkey(),
//             signer,
//             seed_vault.0,
//             team_vault.0,
//             d_vault.0,
//             e_vault.0
//         );

//         let builder = self
//             .program
//             .request()
//             .instruction(system_instruction::create_account(
//                 &&self.program.payer.pubkey(),
//                 &mint.pubkey(),
//                 self.program
//                     .client
//                     .rpc()
//                     .get_minimum_balance_for_rent_exemption(Mint::LEN)?,
//                 Mint::LEN as u64,
//                 &token::ID,
//             ))
//             .signer(&mint);

//         self.propose_anchor_instruction(
//             Some(builder),
//             multisig,
//             custody::id(),
//             custody::accounts::GenerateTokenMint {
//                 mint: mint.pubkey(),
//                 seed_vault: seed_vault.0,
//                 team_vault: team_vault.0,
//                 d_vault: d_vault.0,
//                 e_vault: e_vault.0,
//                 payer: self.program.payer.pubkey(),
//                 rent: sysvar::rent::ID,
//                 signer,
//                 system_program: system_program::id(),
//                 token_program: anchor_spl::token::ID,
//             },
//             custody::instruction::GenerateTokenMint {
//                 _bump: GenerateTokenBumpSeeds {
//                     seed_vault: seed_vault.1,
//                     team_vault: team_vault.1,
//                     d_vault: d_vault.1,
//                     e_vault: e_vault.1,
//                 },
//             },
//         )
//     }

//     pub fn propose_custody_transfer_tokens(
//         &self,
//         multisig: Pubkey,
//         source: Pubkey,
//         target: Pubkey,
//         amount: u64,
//     ) -> Result<Pubkey> {
//         let custody_signer = Pubkey::find_program_address(&[b"signer"], &custody::id()).0;
//         let multisig_signer = self.program.signer(multisig).0;

//         self.propose_anchor_instruction(
//             None,
//             multisig,
//             custody::id(),
//             custody::accounts::TransferFunds {
//                 vault: source,
//                 to: target,
//                 signer: custody_signer,
//                 authority: multisig_signer,
//                 token_program: token::ID,
//             },
//             custody::instruction::TransferFunds { amount },
//         )
//     }

//     pub fn propose_set_owners_and_change_threshold(
//         &self,
//         multisig: Pubkey,
//         threshold: Option<u64>,
//         owners: Option<Vec<Pubkey>>,
//     ) -> Result<Pubkey> {
//         let args = match (threshold, owners) {
//             (Some(threshold), Some(owners)) => {
//                 dynamic(serum_multisig::instruction::SetOwnersAndChangeThreshold {
//                     owners,
//                     threshold,
//                 })
//             }
//             (Some(threshold), None) => {
//                 dynamic(serum_multisig::instruction::ChangeThreshold { threshold })
//             }
//             (None, Some(owners)) => dynamic(serum_multisig::instruction::SetOwners { owners }),
//             (None, None) => panic!("At least one change is required"),
//         };
//         let multisig_signer = self.program.signer(multisig).0;
//         self.propose_anchor_instruction(
//             None,
//             multisig,
//             self.program.client.id(),
//             serum_multisig::accounts::Auth {
//                 multisig,
//                 multisig_signer,
//             },
//             args,
//         )
//     }

//     pub fn propose_anchor_instruction<A: ToAccountMetas, D: InstructionData>(
//         &self,
//         builder: Option<RequestBuilder>,
//         multisig: Pubkey,
//         pid: Pubkey,
//         accounts: A,
//         args: D,
//     ) -> Result<Pubkey> {
//         let ixs = self
//             .program
//             .request()
//             .accounts(accounts)
//             .args(args)
//             .instructions()?;
//         if ixs.len() != 1 {
//             panic!("Exactly one instruction must be provided: {:?}", ixs);
//         }
//         let ix = ixs[0].clone();
//         self.program.create_transaction(
//             builder,
//             multisig,
//             pid,
//             ix.accounts
//                 .iter()
//                 .map(|account_meta| TransactionAccount {
//                     pubkey: account_meta.pubkey,
//                     is_signer: account_meta.is_signer,
//                     is_writable: account_meta.is_writable,
//                 })
//                 .collect(),
//             ix.data,
//         )
//     }

//     pub fn propose_upgrade(
//         &self,
//         multisig: &Pubkey,
//         program: &Pubkey,
//         buffer: &Pubkey,
//     ) -> Result<Pubkey> {
//         let signer = self.program.signer(*multisig).0;
//         let instruction =
//             bpf_loader_upgradeable::upgrade(program, buffer, &signer, &self.program.payer.pubkey());
//         self.propose_solana_instruction(multisig, instruction)
//     }

//     pub fn propose_solana_instruction(
//         &self,
//         multisig: &Pubkey,
//         instruction: Instruction,
//     ) -> Result<Pubkey> {
//         let accounts = instruction
//             .accounts
//             .iter()
//             .map(|account_meta| TransactionAccount {
//                 pubkey: account_meta.pubkey,
//                 is_signer: false, // multisig-ui does this
//                 is_writable: account_meta.is_writable,
//             })
//             .collect::<Vec<TransactionAccount>>();
//         self.program.create_transaction(
//             None,
//             *multisig,
//             instruction.program_id,
//             accounts,
//             instruction.data,
//         )
//     }

//     pub fn inspect_proposal(&self, proposed_tx: &Transaction) -> Result<()> {
//         match proposed_tx.program_id {
//             pid if pid == bpf_loader_upgradeable::ID => {
//                 let loader_instruction =
//                     bincode::deserialize::<UpgradeableLoaderInstruction>(&proposed_tx.data)?;

//                 match loader_instruction {
//                     UpgradeableLoaderInstruction::Upgrade => {
//                         println!("Proposal to upgrade a program");

//                         let buffer = proposed_tx.accounts[2].pubkey;
//                         let target = proposed_tx.accounts[1].pubkey;

//                         println!("Program to upgrade: {}", target);
//                         println!("Proposed upgrade buffer: {}", buffer);

//                         return Ok(());
//                     }

//                     _ => (),
//                 }
//             }

//             pid if pid == custody::ID => {
//                 let mut instr_hash = [0u8; 8];
//                 instr_hash.copy_from_slice(&proposed_tx.data[..8]);

//                 match instr_hash {
//                     hash if hash == instr_sighash("generate_token_mint") => {
//                         println!("Proposal to generate initial tokens");

//                         let mint = proposed_tx.accounts[0].pubkey;
//                         println!("Proposed mint: {}", mint);

//                         return Ok(());
//                     }

//                     hash if hash == instr_sighash("transfer_funds") => {
//                         println!("Proposal to transfer funds to an account");

//                         let args = custody::instruction::TransferFunds::try_from_slice(
//                             &proposed_tx.data[8..],
//                         )?;
//                         let vault = proposed_tx.accounts[0].pubkey;
//                         let target = proposed_tx.accounts[0].pubkey;

//                         let vault_account = self.program.client.account::<TokenAccount>(vault)?;
//                         let mint_account =
//                             self.program.client.account::<Mint>(vault_account.mint)?;

//                         let base = 10f64.powf(mint_account.decimals as f64);
//                         let proposed_amount = (args.amount as f64) / base;
//                         let vault_amount = (vault_account.amount as f64) / base;

//                         println!("Transferring from: {}", vault);
//                         println!("Custodied amount: {}", vault_amount);
//                         println!(
//                             "Custodied remaining after the transfer: {}",
//                             vault_amount - proposed_amount
//                         );
//                         println!();
//                         println!("**TRANSFER AMOUNT**: {}", proposed_amount);
//                         println!();
//                         println!("**TRANSFER TO**: {}", target);

//                         return Ok(());
//                     }

//                     _ => (),
//                 }
//             }

//             _ => (),
//         }

//         println!("unknown proposal!");
//         Ok(())
//     }
// }

// fn instr_sighash(name: &str) -> [u8; 8] {
//     let preimage = format!("global:{}", name);

//     let mut result = [0u8; 8];

//     result.copy_from_slice(
//         &anchor_client::solana_sdk::hash::hash(preimage.as_bytes()).to_bytes()[..8],
//     );
//     result
// }
