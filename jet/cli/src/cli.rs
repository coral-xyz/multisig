use std::path::PathBuf;

use ::jet::state::MarketFlags;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use clap::Parser;
use multisig_client::cli::run_bpf_proposal;
use multisig_client::cli::run_multisig_command;
use multisig_client::cli::run_multisig_proposal;
use multisig_client::cli::run_token_proposal;
use multisig_client::cli::MISSING_MULTISIG;
use multisig_client::config::load;
use multisig_client::nested_subcommands;
use multisig_client::{
    cli::{BpfProposal, MultisigCommand, MultisigProposal, TokenAction, TokenProposal},
    service::MultisigService,
};
use paste::paste;

use crate::propose::custody;
use crate::propose::jet;
use crate::propose::jet::ReserveParameters;

#[derive(Parser)]
pub struct Opts {
    #[clap(short, long, default_value = "~/.config/jet-multisig.toml")]
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
                custody::propose_custody_generate_token_mint(&service, multisig, cmd.mint_key)?;
            println!("{}", key);
        }
        CustodyProposal::TransferTokens(cmd) => {
            let key = custody::propose_custody_transfer_tokens(
                &service, multisig, cmd.source, cmd.target, cmd.amount,
            )?;
            println!("{}", key);
        }
    }
    Ok(())
}
