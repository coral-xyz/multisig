/// Extra business logic built on top of multisig program's core functionality

use anchor_client::{
    anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas},
    solana_sdk::{
        bpf_loader_upgradeable, instruction::Instruction,
        loader_upgradeable_instruction::UpgradeableLoaderInstruction, pubkey::Pubkey,
    },
};
use anchor_spl::token::{Mint, TokenAccount};
use anyhow::Result;
use serum_multisig::{DelegateList, Transaction, TransactionAccount};
use crate::{gateway::MultisigGateway, request_builder::RequestBuilder};

pub struct MultisigService<'a> {
    pub program: MultisigGateway<'a>,
}


impl<'a> MultisigService<'a> {
    pub fn add_delegates(&self, multisig: Pubkey, delegates: Vec<Pubkey>) -> Result<()> {
        let (delegate_list_account, _) = self
            .program
            .delegate_list(multisig, self.program.payer.pubkey());

        let mut existing = match self
            .program
            .client
            .account::<DelegateList>(delegate_list_account)
        {
            Ok(list) => list.delegates,
            Err(_) => {
                self.program.create_delegate_list(multisig, delegates)?;
                return Ok(());
            }
        };

        existing.extend(delegates);
        println!("delegates = {:#?}", existing);

        self.program
            .set_delegate_list(multisig, delegate_list_account, existing)?;
        Ok(())
    }

    pub fn remove_delegates(&self, multisig: Pubkey, delegates: Vec<Pubkey>) -> Result<()> {
        let (delegate_list_account, _) = self
            .program
            .delegate_list(multisig, self.program.payer.pubkey());

        let existing = match self
            .program
            .client
            .account::<DelegateList>(delegate_list_account)
        {
            Ok(list) => list.delegates,
            Err(_) => {
                // FIXME: silent failure if its a network issue?
                return Ok(());
            }
        };

        let new_list = existing
            .into_iter()
            .filter(|d| !delegates.contains(d))
            .collect();
        println!("delegates = {:#?}", new_list);

        self.program
            .set_delegate_list(multisig, delegate_list_account, new_list)?;
        Ok(())
    }

    pub fn propose_anchor_instruction<A: ToAccountMetas, D: InstructionData>(
        &self,
        builder: Option<RequestBuilder>,
        multisig: Pubkey,
        pid: Pubkey,
        accounts: A,
        args: D,
    ) -> Result<Pubkey> {
        let ixs = self
            .program
            .request()
            .accounts(accounts)
            .args(args)
            .instructions()?;
        if ixs.len() != 1 {
            panic!("Exactly one instruction must be provided: {:?}", ixs);
        }
        let ix = ixs[0].clone();
        self.program.create_transaction(
            builder,
            multisig,
            pid,
            ix.accounts
                .iter()
                .map(|account_meta| TransactionAccount {
                    pubkey: account_meta.pubkey,
                    is_signer: account_meta.is_signer,
                    is_writable: account_meta.is_writable,
                })
                .collect(),
            ix.data,
        )
    }

    pub fn propose_solana_instruction(
        &self,
        multisig: &Pubkey,
        instruction: Instruction,
    ) -> Result<Pubkey> {
        let accounts = instruction
            .accounts
            .iter()
            .map(|account_meta| TransactionAccount {
                pubkey: account_meta.pubkey,
                is_signer: false, // multisig-ui does this
                is_writable: account_meta.is_writable,
            })
            .collect::<Vec<TransactionAccount>>();
        self.program.create_transaction(
            None,
            *multisig,
            instruction.program_id,
            accounts,
            instruction.data,
        )
    }

    pub fn inspect_proposal(&self, proposed_tx: &Transaction) -> Result<()> {
        match proposed_tx.program_id {
            pid if pid == bpf_loader_upgradeable::ID => {
                let loader_instruction =
                    bincode::deserialize::<UpgradeableLoaderInstruction>(&proposed_tx.data)?;

                match loader_instruction {
                    UpgradeableLoaderInstruction::Upgrade => {
                        println!("Proposal to upgrade a program");

                        let buffer = proposed_tx.accounts[2].pubkey;
                        let target = proposed_tx.accounts[1].pubkey;

                        println!("Program to upgrade: {}", target);
                        println!("Proposed upgrade buffer: {}", buffer);

                        return Ok(());
                    }

                    _ => (),
                }
            }

            pid if pid == custody::ID => {
                let mut instr_hash = [0u8; 8];
                instr_hash.copy_from_slice(&proposed_tx.data[..8]);

                match instr_hash {
                    hash if hash == instr_sighash("generate_token_mint") => {
                        println!("Proposal to generate initial tokens");

                        let mint = proposed_tx.accounts[0].pubkey;
                        println!("Proposed mint: {}", mint);

                        return Ok(());
                    }

                    hash if hash == instr_sighash("transfer_funds") => {
                        println!("Proposal to transfer funds to an account");

                        let args = custody::instruction::TransferFunds::try_from_slice(
                            &proposed_tx.data[8..],
                        )?;
                        let vault = proposed_tx.accounts[0].pubkey;
                        let target = proposed_tx.accounts[0].pubkey;

                        let vault_account = self.program.client.account::<TokenAccount>(vault)?;
                        let mint_account =
                            self.program.client.account::<Mint>(vault_account.mint)?;

                        let base = 10f64.powf(mint_account.decimals as f64);
                        let proposed_amount = (args.amount as f64) / base;
                        let vault_amount = (vault_account.amount as f64) / base;

                        println!("Transferring from: {}", vault);
                        println!("Custodied amount: {}", vault_amount);
                        println!(
                            "Custodied remaining after the transfer: {}",
                            vault_amount - proposed_amount
                        );
                        println!();
                        println!("**TRANSFER AMOUNT**: {}", proposed_amount);
                        println!();
                        println!("**TRANSFER TO**: {}", target);

                        return Ok(());
                    }

                    _ => (),
                }
            }

            _ => (),
        }

        println!("unknown proposal!");
        Ok(())
    }
}

fn instr_sighash(name: &str) -> [u8; 8] {
    let preimage = format!("global:{}", name);

    let mut result = [0u8; 8];

    result.copy_from_slice(
        &anchor_client::solana_sdk::hash::hash(preimage.as_bytes()).to_bytes()[..8],
    );
    result
}
