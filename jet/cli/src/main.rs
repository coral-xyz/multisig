use anyhow::Result;

use clap::Clap;
use cli::{Opts, run_job};
use multisig_client::config;

mod cli;
mod propose;

fn main() -> Result<()> {
    solana_logger::setup_with_default("solana=debug");
    let cli_opts = Opts::parse();
    let multisig_config = config::load(&cli_opts.config)?;
    let multisig = cli_opts.multisig.or(multisig_config.multisig);
    let payer = multisig_client::load_payer(&multisig_config.wallet);
    let service = multisig_client::load_service(&*payer, &multisig_config)?;
    run_job(cli_opts.job, &service, multisig)
}
