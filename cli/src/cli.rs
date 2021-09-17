use anchor_client::solana_sdk::pubkey::Pubkey;
use clap::{AppSettings, Clap};

#[derive(Clap)]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct Opts {
    #[clap(short, long, default_value = "Anchor.toml")]
    pub config: String,
    #[clap(subcommand)]
    pub job: Job,
}

#[derive(Clap)]
pub enum Job {
    CreateMultisig(CreateMultisig),
    ProposeUpgrade(ProposeUpgrade),
    Approve(Transaction),
    Execute(Transaction),
    GetMultisig(Key),
    GetTransaction(Key),
}

#[derive(Clap, Debug)]
pub struct CreateMultisig {
    pub threshold: u64,
    pub owners: String,
}

#[derive(Clap)]
pub struct ProposeUpgrade {
    pub multisig: Pubkey,
    pub program: Pubkey,
    pub buffer: Pubkey,
}

#[derive(Clap)]
pub struct Transaction {
    pub multisig: Pubkey,
    pub transaction: Pubkey,
}

#[derive(Clap)]
pub struct Key {
    pub key: Pubkey,
}
