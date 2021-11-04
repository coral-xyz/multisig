pub mod cli;
pub mod config;
pub mod gateway;
pub mod instruction_data;
pub mod propose;
pub mod request_builder;
pub mod service;

use anchor_client::solana_sdk::{signature::Keypair, signer::Signer};
use anyhow::Result;

use clap2::ArgMatches;
use config::MultisigConfig;
use gateway::MultisigGateway;
use rand::rngs::OsRng;
use service::MultisigService;
use solana_clap_utils::keypair::DefaultSigner;
use solana_remote_wallet::remote_wallet::maybe_wallet_manager;

pub fn load_payer(path: &str) -> Box<dyn Signer> {
    let path = &*shellexpand::tilde(path);
    let mut wallet_manager = maybe_wallet_manager().unwrap();
    let default_signer = DefaultSigner::new("keypair".to_string(), path);
    let arg_matches = ArgMatches::default();
    default_signer
        .signer_from_path(&arg_matches, &mut wallet_manager)
        .unwrap()
}

pub fn load_service<'a>(
    payer: &'a dyn Signer,
    config: &'a MultisigConfig,
) -> Result<MultisigService<'a>> {
    // todo change anchor to use Signer so we don't need this dummy keypair that we have to be careful not to use
    let keypair = Keypair::generate(&mut OsRng);
    let cluster = config.cluster();
    let connection = anchor_client::Client::new(cluster.clone(), keypair);
    let client = connection.program(config.program_id);

    Ok(MultisigService {
        program: MultisigGateway {
            client,
            cluster,
            payer,
            config,
        },
    })
}
