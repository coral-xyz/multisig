use std::path::PathBuf;

use custody_anchor_lang::AnchorDeserialize;
use ::jet::state::MarketFlags;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use clap::Parser;
use multisig_anchor_client::Program;
use multisig_anchor_spl::token::Mint;
use multisig_anchor_spl::token::TokenAccount;
use multisig_client::cli::run_bpf_proposal;
use multisig_client::cli::run_multisig_command;
use multisig_client::cli::run_multisig_proposal;
use multisig_client::cli::run_token_proposal;
use multisig_client::cli::MISSING_MULTISIG;
use multisig_client::config::load;
use multisig_client::nested_subcommands;
use multisig_client::service::ProposalInspector;
use multisig_client::{
    cli::{BpfProposal, MultisigCommand, MultisigProposal, TokenAction, TokenProposal},
    service::MultisigService,
};
use paste::paste;
use serum_multisig::Transaction;

use crate::propose::custody as custody_proposals;
use crate::propose::jet;
use crate::propose::jet::ReserveParameters;

#[derive(Parser)]
pub struct Opts {
    #[clap(short, long, default_value = "~/.config/multisig.toml")]
    pub config: String,

    #[clap(short, long)]
    pub multisig: Option<Pubkey>,

    #[clap(subcommand)]
    pub job: Job,
}

nested_subcommands!(
    Job {
        Admin(MultisigCommand),
        Propose(Proposal),
    }
);

nested_subcommands! {
    Proposal {
        Multisig(MultisigProposal),
        Bpf(BpfProposal),
        Token(TokenProposal),
        Jet(JetProposal),
        Custody(CustodyProposal),
    }
}

#[derive(Parser)]
pub enum JetProposal {
    SetMarketFlags(MarketFlagsOpts),
    SetMarketOwner(NewMarketOwner),
    InitReserve(InitReserve),
}

#[derive(Parser)]
pub enum CustodyProposal {
    GenerateTokenMint(GenerateTokens),
    TransferTokens(TokenAction),
}

#[derive(Parser)]
pub struct GenerateTokens {
    #[clap(long, short = 'k')]
    pub mint_key: PathBuf,
}

#[derive(Parser)]
pub struct MarketFlagsOpts {
    pub market: Pubkey,

    #[clap(long, short = 'b')]
    pub halt_borrows: bool,

    #[clap(long, short = 'r')]
    pub halt_repays: bool,

    #[clap(long, short = 'd')]
    pub halt_deposits: bool,
}

#[derive(Parser)]
pub struct NewMarketOwner {
    pub market: Pubkey,
    pub new_owner: Pubkey,
}

#[derive(Parser)]
pub struct InitReserve {
    pub market: Pubkey,
    pub config: String,
}

pub fn run_job(job: Job, service: &MultisigService, multisig: Option<Pubkey>) -> Result<()> {
    match job {
        Job::Admin(cmd) => run_multisig_command(cmd.subcommand, service, multisig),
        Job::Propose(cmd) => {
            let multisig = multisig.expect(MISSING_MULTISIG);
            match cmd.subcommand {
                Proposal::Multisig(cmd) => run_multisig_proposal(cmd.subcommand, service, multisig),
                Proposal::Bpf(cmd) => run_bpf_proposal(cmd.subcommand, service, multisig),
                Proposal::Token(cmd) => run_token_proposal(cmd.subcommand, service, multisig),
                Proposal::Jet(cmd) => run_jet_proposal(cmd.subcommand, service, multisig),
                Proposal::Custody(cmd) => run_custody_proposal(cmd.subcommand, service, multisig),
            }
        }
    }
}

pub fn run_jet_proposal(
    job: JetProposal,
    service: &MultisigService,
    multisig: Pubkey,
) -> Result<()> {
    match job {
        JetProposal::SetMarketFlags(cmd) => {
            let mut flags = MarketFlags::empty();
            if cmd.halt_borrows {
                flags |= MarketFlags::HALT_BORROWS;
            }
            if cmd.halt_deposits {
                flags |= MarketFlags::HALT_DEPOSITS;
            }
            if cmd.halt_repays {
                flags |= MarketFlags::HALT_REPAYS;
            }
            let key = jet::propose_set_market_flags(&service, multisig, cmd.market, flags)?;
            println!("{}", key);
        }
        JetProposal::SetMarketOwner(cmd) => {
            let key = jet::propose_set_market_owner(&service, multisig, cmd.market, cmd.new_owner)?;
            println!("{}", key);
        }
        JetProposal::InitReserve(cmd) => {
            let params: ReserveParameters = load(&cmd.config)?;
            let (proposal, reserve) =
                jet::propose_init_reserve(&service, multisig, cmd.market, params)?;
            println!("{} {}", proposal, reserve);
        }
    }
    Ok(())
}

pub fn run_custody_proposal(
    job: CustodyProposal,
    service: &MultisigService,
    multisig: Pubkey,
) -> Result<()> {
    match job {
        CustodyProposal::GenerateTokenMint(cmd) => {
            let key =
                custody_proposals::propose_custody_generate_token_mint(&service, multisig, cmd.mint_key)?;
            println!("{}", key);
        }
        CustodyProposal::TransferTokens(cmd) => {
            let key = custody_proposals::propose_custody_transfer_tokens(
                &service, multisig, cmd.source, cmd.target, cmd.amount,
            )?;
            println!("{}", key);
        }
    }
    Ok(())
}

pub struct JetProposalInspector;

impl ProposalInspector for JetProposalInspector {
    fn inspect_proposal(&self, program: &Program, proposed_tx: &Transaction) -> Result<bool> {
        match proposed_tx.program_id {
            pid if pid == custody::ID => {
                let mut instr_hash = [0u8; 8];
                instr_hash.copy_from_slice(&proposed_tx.data[..8]);

                match instr_hash {
                    hash if hash == instr_sighash("generate_token_mint") => {
                        println!("Proposal to generate initial tokens");

                        let mint = proposed_tx.accounts[0].pubkey;
                        println!("Proposed mint: {}", mint);

                        return Ok(true);
                    }

                    hash if hash == instr_sighash("transfer_funds") => {
                        println!("Proposal to transfer funds to an account");

                        let args = custody::instruction::TransferFunds::try_from_slice(
                            &proposed_tx.data[8..],
                        )?;
                        let vault = proposed_tx.accounts[0].pubkey;
                        let target = proposed_tx.accounts[1].pubkey;

                        let vault_account = program.account::<TokenAccount>(vault)?;
                        let mint_account =
                            program.account::<Mint>(vault_account.mint)?;

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

                        Ok(true)
                    }

                    _ => Ok(false),
                }
            }

            _ => Ok(false),
        }
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
