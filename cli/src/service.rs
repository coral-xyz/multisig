/// Extra business logic built on top of multisig program's core functionality

use anchor_client::solana_sdk::{bpf_loader_upgradeable, pubkey::Pubkey};
use anyhow::Result;
use serum_multisig::TransactionAccount;

use crate::gateway::MultisigGateway;


pub struct MultisigService {
    pub program: MultisigGateway
}

impl MultisigService {
    pub fn propose_upgrade(
        &self,
        multisig: &Pubkey,
        program: &Pubkey,
        buffer: &Pubkey,
    ) -> Result<Pubkey> {
        let signer = Pubkey::find_program_address(
            &[&multisig.to_bytes()],
            &self.program.client.id(),
        ).0;
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
