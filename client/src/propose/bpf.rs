use anchor_client::solana_sdk::{bpf_loader_upgradeable, pubkey::Pubkey};
use anyhow::Result;

use crate::service::MultisigService;

pub fn propose_upgrade(
    service: &MultisigService,
    multisig: &Pubkey,
    program: &Pubkey,
    buffer: &Pubkey,
) -> Result<Pubkey> {
    let signer = service.program.signer(*multisig).0;
    let instruction = bpf_loader_upgradeable::upgrade(
        program, 
        buffer, 
        &signer, 
        &service.program.payer.pubkey()
    );
    service.propose_solana_instruction(multisig, instruction)
}
