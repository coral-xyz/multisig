use anchor_client::Cluster;
use anyhow::Result;

use clap::Clap;
use cli::{Job, Opts, run_job};
use multisig_client::config::{self, MultisigConfig};
use jet::state::MarketFlags;
use serum_multisig::{Multisig, Transaction};
use multisig_client::service::MultisigService;

mod cli;

fn main() -> Result<()> {
    solana_logger::setup_with_default("solana=debug");
    let cli_opts = Opts::parse();
    let multisig_config = config::load(&cli_opts.config)?;
    let payer = multisig_client::load_payer(&multisig_config.wallet);
    let service = multisig_client::load_service(&*payer, &multisig_config)?;
    run_job(cli_opts.job, &service, &multisig_config)
}
