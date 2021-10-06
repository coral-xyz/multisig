use std::path::PathBuf;

use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use clap::{AppSettings, Clap};
use ::jet::state::MarketFlags;
use jet_multisig_client::propose::custody;
use jet_multisig_client::propose::jet;
use multisig_client::{cli::{
    CreateMultisig,
    Delegates,
    Edit,
    Key,
    ProposeUpgrade,
    TokenAction,
    Transaction,
    Job as CoreJob
}, config::MultisigConfig, service::MultisigService};

#[derive(Clap)]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct Opts {
    #[clap(short, long, default_value = "~/.config/jet-multisig.toml")]
    pub config: String,

    #[clap(subcommand)]
    pub job: Job,
}

#[derive(Clap)]
pub enum Job {
    New(CreateMultisig),
    AddDelegates(Delegates),
    RemoveDelegates(Delegates),
    Approve(Transaction),
    Execute(Transaction),
    Get,
    GetTransaction(Key),
    InspectProposal(Key),
    ProposeUpgrade(ProposeUpgrade),
    ProposeEdit(Edit),
    ProposeMintTokens(TokenAction),
    ProposeTransferTokens(TokenAction),

    ProposeSetMarketFlags(MarketFlagsOpts),
    ProposeCustodyGenerateTokenMint(GenerateTokens),
    ProposeCustodyTransferTokens(TokenAction),
}


#[derive(Clap)]
pub struct GenerateTokens {
    #[clap(long, short = 'k')]
    pub mint_key: PathBuf,
}

#[derive(Clap)]
pub struct MarketFlagsOpts {
    pub market: Pubkey,

    #[clap(long, short = 'b')]
    pub halt_borrows: bool,

    #[clap(long, short = 'r')]
    pub halt_repays: bool,

    #[clap(long, short = 'd')]
    pub halt_deposits: bool,
}


pub fn run_job(job: Job, service: &MultisigService, config: &MultisigConfig) -> Result<()> {
    let core = |cj| multisig_client::cli::run_job(cj, &service, config);
    match job {
        Job::New(cmd) => core(CoreJob::New(cmd))?,
        Job::AddDelegates(cmd) => core(CoreJob::AddDelegates(cmd))?,
        Job::RemoveDelegates(cmd) => core(CoreJob::RemoveDelegates(cmd))?,
        Job::ProposeUpgrade(cmd) => core(CoreJob::ProposeUpgrade(cmd))?,
        Job::Approve(cmd) => core(CoreJob::Approve(cmd))?,
        Job::Execute(cmd) => core(CoreJob::Execute(cmd))?,
        Job::Get => core(CoreJob::Get)?,
        Job::GetTransaction(cmd) => core(CoreJob::GetTransaction(cmd))?,
        Job::InspectProposal(cmd) => core(CoreJob::InspectProposal(cmd))?,
        Job::ProposeEdit(cmd) => core(CoreJob::ProposeEdit(cmd))?,
        Job::ProposeMintTokens(cmd) => core(CoreJob::ProposeMintTokens(cmd))?,
        Job::ProposeTransferTokens(cmd) => core(CoreJob::ProposeTransferTokens(cmd))?,

        Job::ProposeSetMarketFlags(cmd) => {
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
            let key = jet::propose_set_market_flags(&service, config.multisig, cmd.market, flags)?;
            println!("{}", key);
        }
        Job::ProposeCustodyGenerateTokenMint(cmd) => {
            let key = custody::propose_custody_generate_token_mint(&service, config.multisig, cmd.mint_key)?;
            println!("{}", key);
        }
        Job::ProposeCustodyTransferTokens(cmd) => {
            let key = custody::propose_custody_transfer_tokens(
                &service,
                config.multisig,
                cmd.source,
                cmd.target,
                cmd.amount,
            )?;
            println!("{}", key);
        }
    };
    Ok(())
}
