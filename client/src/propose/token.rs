use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;

use crate::service::MultisigService;

pub fn propose_mint_tokens(
    service: &MultisigService,
    multisig: Pubkey,
    mint: Pubkey,
    target: Pubkey,
    amount: u64,
) -> Result<Pubkey> {
    let signer = service.program.signer(multisig).0;
    let ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        &mint,
        &target,
        &signer,
        &vec![&signer],
        amount,
    )?;
    service.propose_solana_instruction(&multisig, ix)
}

pub fn propose_transfer_tokens(
    service: &MultisigService,
    multisig: Pubkey,
    source: Pubkey,
    target: Pubkey,
    amount: u64,
) -> Result<Pubkey> {
    let signer = service.program.signer(multisig).0;
    let ix = spl_token::instruction::transfer(
        &spl_token::id(),
        &source,
        &target,
        &signer,
        &vec![&signer],
        amount,
    )?;
    service.propose_solana_instruction(&multisig, ix)
}
