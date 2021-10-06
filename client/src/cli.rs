use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use clap::{AppSettings, Clap};
use serum_multisig::{Multisig, Transaction as SerumTxn};

use crate::service::MultisigService;
use crate::config::MultisigConfig;
use crate::propose::{bpf, multisig, token};


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
}

#[derive(Clap, Debug)]
pub struct CreateMultisig {
    pub threshold: u64,
    #[clap(required = true)]
    pub owners: Vec<Pubkey>,
}

#[derive(Clap, Debug)]
pub struct Delegates {
    pub delegates: Vec<Pubkey>,
}

#[derive(Clap, Debug)]
pub struct Edit {
    #[clap(long)]
    pub threshold: Option<u64>,
    #[clap(long)]
    pub owners: Option<Vec<Pubkey>>,
}

#[derive(Clap)]
pub struct ProposeUpgrade {
    pub program: Pubkey,
    pub buffer: Pubkey,
}

#[derive(Clap)]
pub struct TokenAction {
    #[clap(long, short)]
    pub source: Pubkey,

    #[clap(long, short)]
    pub target: Pubkey,

    #[clap(long, short)]
    pub amount: u64,
}

#[derive(Clap)]
pub struct Transaction {
    pub transaction: Pubkey,
}

#[derive(Clap)]
pub struct Key {
    pub key: Pubkey,
}


pub fn run_job(job: Job, service: &MultisigService, config: &MultisigConfig) -> Result<()> {
    match job {
        Job::New(cmd) => {
            let keys = service.program.create_multisig(cmd.threshold, cmd.owners)?;
            println!("{} {}", keys.0, keys.1);
        }
        Job::AddDelegates(cmd) => {
            service.add_delegates(config.multisig, cmd.delegates)?;
        }
        Job::RemoveDelegates(cmd) => {
            service.remove_delegates(config.multisig, cmd.delegates)?;
        }
        Job::ProposeUpgrade(cmd) => {
            let key = bpf::propose_upgrade(&service, &config.multisig, &cmd.program, &cmd.buffer)?;
            println!("{}", key);
        }
        Job::Approve(cmd) => service.program.approve(config.multisig, cmd.transaction)?,
        Job::Execute(cmd) => {
            service.program.execute(config.multisig, cmd.transaction)?;
        }
        Job::Get => {
            let ms = service
                .program
                .client
                .account::<Multisig>(config.multisig)?;
            let signer = service.program.signer(config.multisig).0;
            println!("{:#?}", ms);
            println!("signer = {:?}", signer);
        }
        Job::GetTransaction(cmd) => {
            let tx = service.program.client.account::<SerumTxn>(cmd.key)?;
            println!("{:#?}", tx);
        }
        Job::InspectProposal(cmd) => {
            let tx = service.program.client.account::<SerumTxn>(cmd.key)?;
            service.inspect_proposal(&tx)?;
        }
        Job::ProposeEdit(cmd) => {
            let key = multisig::propose_set_owners_and_change_threshold(
                &service,
                config.multisig,
                cmd.threshold,
                cmd.owners,
            )?;
            println!("{}", key);
        }
        Job::ProposeMintTokens(cmd) => {
            let key =
                token::propose_mint_tokens(&service, config.multisig, cmd.source, cmd.target, cmd.amount)?;
            println!("{}", key);
        }
        Job::ProposeTransferTokens(cmd) => {
            let key = token::propose_transfer_tokens(
                &service,
                config.multisig,
                cmd.source,
                cmd.target,
                cmd.amount,
            )?;
            println!("{}", key);
        }
    }
    Ok(())
}
