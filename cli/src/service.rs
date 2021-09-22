/// Extra business logic built on top of multisig program's core functionality

use std::io::Write;
use anchor_client::{anchor_lang::{AnchorSerialize, InstructionData, ToAccountMetas}, solana_sdk::{bpf_loader_upgradeable, pubkey::Pubkey}};
use anyhow::Result;
use serum_multisig::TransactionAccount;

use crate::gateway::MultisigGateway;


pub struct MultisigService {
    pub program: MultisigGateway
}

struct DynamicInstructionData {
    data: Vec<u8>,
}

fn dynamic<T: InstructionData>(id: T) -> DynamicInstructionData {
    DynamicInstructionData { data: id.data() }
}

impl AnchorSerialize for DynamicInstructionData {
    fn serialize<W: Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        todo!()
    }
}

impl InstructionData for DynamicInstructionData {
    fn data(&self) -> Vec<u8> { 
        self.data.clone()
    }
}

impl MultisigService {
    pub fn propose_set_owners_and_change_threshold(
        &self,
        multisig: Pubkey,
        threshold: Option<u64>,
        owners: Option<Vec<Pubkey>>,
    ) -> Result<Pubkey> {
        let args = match (threshold, owners) {
            (Some(threshold), Some(owners)) => dynamic(
                serum_multisig::instruction::SetOwnersAndChangeThreshold {
                    owners,
                    threshold,
                }),
            (Some(threshold), None) => dynamic(
                serum_multisig::instruction::ChangeThreshold {
                    threshold,
                }),
            (None, Some(owners)) => dynamic(
                serum_multisig::instruction::SetOwners {
                    owners,
                }),
            (None, None) => panic!("At least one change is required"),
        };
        let multisig_signer = self.program.signer(multisig).0;
        self.propose_anchor_instruction(
            multisig,
            serum_multisig::accounts::Auth {
                multisig,
                multisig_signer,
            },
            args
        )
    }

    pub fn propose_anchor_instruction<A: ToAccountMetas, D: InstructionData>(
        &self, multisig: Pubkey, accounts: A, args: D
    ) -> Result<Pubkey> {
        let ixs = self.program.request()
            .accounts(accounts)
            .args(args)
            .instructions()?;
        if ixs.len() != 1 { 
            panic!("Exactly one instruction must be provided: {:?}", ixs);
        }
        let ix = ixs[0].clone();
        self.program.create_transaction(
            multisig,
            self.program.client.id(),
            ix.accounts.iter().map(|account_meta|
                TransactionAccount {
                    pubkey: account_meta.pubkey,
                    is_signer: account_meta.is_signer,
                    is_writable: account_meta.is_writable,
                }
            ).collect(),
            ix.data,
        )
    }

    pub fn propose_upgrade(
        &self,
        multisig: &Pubkey,
        program: &Pubkey,
        buffer: &Pubkey,
    ) -> Result<Pubkey> {
        let signer = self.program.signer(*multisig).0;
        let instruction = bpf_loader_upgradeable::upgrade(
            program,
            buffer,
            &signer,
            &self.program.client.payer(),
        );
        let accounts = instruction.accounts.iter()
            .map(|account_meta| TransactionAccount {
                pubkey: account_meta.pubkey,
                is_signer: false, // multisig-ui does this
                is_writable: account_meta.is_writable,
            })
            .collect::<Vec<TransactionAccount>>();
        self.program.create_transaction(
            *multisig,
            instruction.program_id,
            accounts,
            instruction.data,
        )
    }
}
