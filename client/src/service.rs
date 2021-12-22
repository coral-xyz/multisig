use crate::gateway::MultisigGateway;
/// Extra business logic built on top of multisig program's core functionality
use anchor_client::{
    anchor_lang::{InstructionData, ToAccountMetas},
    solana_sdk::{
        bpf_loader_upgradeable, instruction::Instruction,
        loader_upgradeable_instruction::UpgradeableLoaderInstruction, pubkey::Pubkey,
    }, RequestBuilder, Program,
};
use anyhow::Result;
use serum_multisig::{DelegateList, Transaction, TransactionAccount};

pub struct MultisigService<'a> {
    pub program: MultisigGateway<'a>,
    pub inspector: Option<Box<dyn ProposalInspector>>,
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
            .client
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
        if let Some(inspector) = &self.inspector {
            if inspector.inspect_proposal(&self.program.client, proposed_tx)? == true {
                return Ok(());
            }
        }

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
            _ => (),
        }

        println!("unknown proposal!");
        Ok(())
        
    }
}

pub trait ProposalInspector {
    fn inspect_proposal(&self, program: &Program, proposed_tx: &Transaction) -> Result<bool>;
}
