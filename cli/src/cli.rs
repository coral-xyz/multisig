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
    New(CreateMultisig),
    Approve(Transaction),
    Execute(Transaction),
    Get(Key),
    GetTransaction(Key),
    ProposeUpgrade(ProposeUpgrade),
    ProposeEdit(Edit),    
}

#[derive(Clap, Debug)]
pub struct CreateMultisig {
    pub threshold: u64,
    #[clap(required = true)]
    pub owners: Vec<Pubkey>,
}

#[derive(Clap, Debug)]
pub struct Edit {
    pub multisig: Pubkey,
    #[clap(long)]
    pub threshold: Option<u64>,
    #[clap(long)]
    pub owners: Option<Vec<Pubkey>>,
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
