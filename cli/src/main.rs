use std::rc::Rc;

use anyhow::Result;

use clap::Parser;
use multisig_client::{
    cli::{run_job, Opts},
    config::{self, DelegationConfig, MultisigConfig},
};

fn main() -> Result<()> {
    solana_logger::setup_with_default("solana=debug");
    let cli_opts = Opts::parse();
    let mut multisig_config: MultisigConfig = config::load(&cli_opts.config)?;
    let multisig = cli_opts.multisig.or(multisig_config.multisig);
    let keypair = cli_opts.keypair.unwrap_or(multisig_config.wallet.clone());
    let payer = multisig_client::load_payer(&keypair);
    if let Some(url) = cli_opts.url {
        multisig_config.cluster = url;
    }
    if let Some(owner) = cli_opts.delegated_owner {
        multisig_config.delegation = Some(DelegationConfig { owner });
    }
    let service = multisig_client::load_service(Rc::new(payer), &multisig_config, None)?;
    run_job(cli_opts.job, &service, multisig)
}
