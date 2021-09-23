use anchor_client::{
    anchor_lang::{AnchorSerialize, InstructionData, ToAccountMetas},
    solana_sdk::{
        bpf_loader_upgradeable, instruction::Instruction, pubkey::Pubkey, signature::Keypair,
        signer, signer::Signer, system_instruction, system_program, sysvar,
    },
};
use anchor_spl::token::{self, Mint};
use anyhow::{bail, Result};
use custody::GenerateTokenBumpSeeds;
use rand::rngs::OsRng;
use serum_multisig::TransactionAccount;
/// Extra business logic built on top of multisig program's core functionality
use std::{io::Write, path::PathBuf};

use crate::{gateway::MultisigGateway, request_builder::RequestBuilder};

pub struct MultisigService<'a> {
    pub program: MultisigGateway<'a>,
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

impl<'a> MultisigService<'a> {
    pub fn propose_mint_tokens(
        &self,
        multisig: Pubkey,
        mint: Pubkey,
        target: Pubkey,
        amount: u64,
    ) -> Result<Pubkey> {
        let signer = self.program.signer(multisig).0;
        let ix = spl_token::instruction::mint_to(
            &spl_token::id(),
            &mint,
            &target,
            &signer,
            &vec![&signer],
            amount,
        )?;
        self.propose_solana_instruction(&multisig, ix)
    }

    pub fn propose_transfer_tokens(
        &self,
        multisig: Pubkey,
        source: Pubkey,
        target: Pubkey,
        amount: u64,
    ) -> Result<Pubkey> {
        let signer = self.program.signer(multisig).0;
        let ix = spl_token::instruction::transfer(
            &spl_token::id(),
            &source,
            &target,
            &signer,
            &vec![&signer],
            amount,
        )?;
        self.propose_solana_instruction(&multisig, ix)
    }

    pub fn propose_custody_generate_token_mint(
        &self,
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

        let builder = self
            .program
            .request()
            .instruction(system_instruction::create_account(
                &&self.program.payer.pubkey(),
                &mint.pubkey(),
                self.program
                    .client
                    .rpc()
                    .get_minimum_balance_for_rent_exemption(Mint::LEN)?,
                Mint::LEN as u64,
                &token::ID,
            ))
            .signer(&mint);

        self.propose_anchor_instruction(
            Some(builder),
            multisig,
            custody::id(),
            custody::accounts::GenerateTokenMint {
                mint: mint.pubkey(),
                seed_vault: seed_vault.0,
                team_vault: team_vault.0,
                d_vault: d_vault.0,
                e_vault: e_vault.0,
                payer: self.program.payer.pubkey(),
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
        &self,
        multisig: Pubkey,
        source: Pubkey,
        target: Pubkey,
        amount: u64,
    ) -> Result<Pubkey> {
        let custody_signer = Pubkey::find_program_address(&[b"signer"], &custody::id()).0;
        let multisig_signer = self.program.signer(multisig).0;

        self.propose_anchor_instruction(
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

    pub fn propose_set_owners_and_change_threshold(
        &self,
        multisig: Pubkey,
        threshold: Option<u64>,
        owners: Option<Vec<Pubkey>>,
    ) -> Result<Pubkey> {
        let args = match (threshold, owners) {
            (Some(threshold), Some(owners)) => {
                dynamic(serum_multisig::instruction::SetOwnersAndChangeThreshold {
                    owners,
                    threshold,
                })
            }
            (Some(threshold), None) => {
                dynamic(serum_multisig::instruction::ChangeThreshold { threshold })
            }
            (None, Some(owners)) => dynamic(serum_multisig::instruction::SetOwners { owners }),
            (None, None) => panic!("At least one change is required"),
        };
        let multisig_signer = self.program.signer(multisig).0;
        self.propose_anchor_instruction(
            None,
            multisig,
            self.program.client.id(),
            serum_multisig::accounts::Auth {
                multisig,
                multisig_signer,
            },
            args,
        )
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

    pub fn propose_upgrade(
        &self,
        multisig: &Pubkey,
        program: &Pubkey,
        buffer: &Pubkey,
    ) -> Result<Pubkey> {
        let signer = self.program.signer(*multisig).0;
        let instruction =
            bpf_loader_upgradeable::upgrade(program, buffer, &signer, &self.program.payer.pubkey());
        self.propose_solana_instruction(multisig, instruction)
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
}
