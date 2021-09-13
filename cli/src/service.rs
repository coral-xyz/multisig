use anchor_client::solana_sdk::{bpf_loader_upgradeable, pubkey::Pubkey};
use anyhow::Result;

use crate::gateway::MultisigGateway;


pub struct MultisigService {
    pub program: MultisigGateway
}

impl MultisigService {
    pub fn upgrade_program(
        &self,
        multisig: &Pubkey,
        program: &Pubkey,
        buffer: &Pubkey,
        authority: &Pubkey,
        spill: &Pubkey,
    ) -> Result<Pubkey> {
        let instruction = bpf_loader_upgradeable::upgrade(
            program,
            buffer,
            authority,
            spill,
        );
        self.program.create_transaction(*multisig, instruction)
    }
}
