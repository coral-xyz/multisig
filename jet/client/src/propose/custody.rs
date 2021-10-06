use anchor_client::{
    anchor_lang::{AnchorDeserialize, AnchorSerialize, InstructionData, ToAccountMetas},
    solana_sdk::{
        bpf_loader_upgradeable, instruction::Instruction,
        loader_upgradeable_instruction::UpgradeableLoaderInstruction, pubkey::Pubkey, signer,
        signer::Signer, system_instruction, system_program, sysvar,
    },
};
use anchor_spl::token::{self, Mint, TokenAccount};

use anyhow::{bail, Result};
use custody::GenerateTokenBumpSeeds;
use jet::state::MarketFlags;
use multisig_client::service::MultisigService;
/// Extra business logic built on top of multisig program's core functionality
use std::{io::Write, path::PathBuf};



pub fn propose_custody_generate_token_mint(
    service: &MultisigService,
    multisig: Pubkey,
    mint_key: PathBuf,
) -> Result<Pubkey> {
    let mint = match signer::keypair::read_keypair_file(&mint_key) {
        Ok(m) => m,
        Err(e) => bail!(
            "couldn't load mint key '{}', because: {:?}",
            mint_key.display(),
            e
        ),
    };
    let signer = Pubkey::find_program_address(&[b"signer"], &custody::id()).0;
    let getaddr = |seed: &[u8]| {
        Pubkey::find_program_address(&[seed, mint.pubkey().as_ref()], &custody::id())
    };
    let seed_vault = getaddr(b"seed-vault");
    let team_vault = getaddr(b"team-vault");
    let d_vault = getaddr(b"d-vault");
    let e_vault = getaddr(b"e-vault");
    println!(
        "mint {}\nsigner {}\nseed {}\nteam {}\nd {}\ne {}",
        mint.pubkey(),
        signer,
        seed_vault.0,
        team_vault.0,
        d_vault.0,
        e_vault.0
    );

    let builder = service
        .program
        .request()
        .instruction(system_instruction::create_account(
            &&service.program.payer.pubkey(),
            &mint.pubkey(),
            service.program
                .client
                .rpc()
                .get_minimum_balance_for_rent_exemption(Mint::LEN)?,
            Mint::LEN as u64,
            &token::ID,
        ))
        .signer(&mint);

    service.propose_anchor_instruction(
        Some(builder),
        multisig,
        custody::id(),
        custody::accounts::GenerateTokenMint {
            mint: mint.pubkey(),
            seed_vault: seed_vault.0,
            team_vault: team_vault.0,
            d_vault: d_vault.0,
            e_vault: e_vault.0,
            payer: service.program.payer.pubkey(),
            rent: sysvar::rent::ID,
            signer,
            system_program: system_program::id(),
            token_program: anchor_spl::token::ID,
        },
        custody::instruction::GenerateTokenMint {
            _bump: GenerateTokenBumpSeeds {
                seed_vault: seed_vault.1,
                team_vault: team_vault.1,
                d_vault: d_vault.1,
                e_vault: e_vault.1,
            },
        },
    )
}

pub fn propose_custody_transfer_tokens(
    service: &MultisigService,
    multisig: Pubkey,
    source: Pubkey,
    target: Pubkey,
    amount: u64,
) -> Result<Pubkey> {
    let custody_signer = Pubkey::find_program_address(&[b"signer"], &custody::id()).0;
    let multisig_signer = service.program.signer(multisig).0;

    service.propose_anchor_instruction(
        None,
        multisig,
        custody::id(),
        custody::accounts::TransferFunds {
            vault: source,
            to: target,
            signer: custody_signer,
            authority: multisig_signer,
            token_program: token::ID,
        },
        custody::instruction::TransferFunds { amount },
    )
}
