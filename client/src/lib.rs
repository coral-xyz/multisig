pub mod cli;
pub mod compat;
pub mod config;
pub mod gateway;
pub mod instruction_data;
pub mod propose;
pub mod service;

use std::rc::Rc;

pub use anchor_client;

use anchor_client::solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Signature,
    signer::{Signer, SignerError},
};
use anyhow::Result;

use clap2::ArgMatches;
use config::MultisigConfig;
use gateway::MultisigGateway;
use service::{MultisigService, ProposalInspector};
use solana_clap_utils::keypair::DefaultSigner;
use solana_remote_wallet::remote_wallet::maybe_wallet_manager;

pub fn load_payer(path: &str) -> LazilyFailingSigner {
    let path = &*shellexpand::tilde(path);
    let mut wallet_manager = maybe_wallet_manager().unwrap();
    let default_signer = DefaultSigner::new("keypair".to_string(), path);
    let arg_matches = ArgMatches::default();
    LazilyFailingSigner {
        signer: default_signer.signer_from_path(&arg_matches, &mut wallet_manager),
    }
}

pub fn load_service<'a>(
    payer: Rc<dyn Signer>,
    config: &'a MultisigConfig,
    inspector: Option<Box<dyn ProposalInspector>>,
) -> Result<MultisigService<'a>> {
    let cluster = config.cluster();
    let connection = anchor_client::Client::new_with_options(
        cluster.clone(),
        payer.clone(),
        CommitmentConfig {
            commitment: config.commitment,
        },
    );
    let client = connection.program(config.program_id);

    Ok(MultisigService {
        program: MultisigGateway {
            client,
            cluster,
            payer,
            config,
        },
        inspector,
    })
}

/// This allows you to instantiate a client with a signer even if there is no signer available
/// That way, you can execute client operations that don't actually use the signer (such as reads)
pub struct LazilyFailingSigner {
    pub signer: Result<Box<dyn Signer>, Box<dyn std::error::Error>>,
}

impl Signer for LazilyFailingSigner {
    fn try_pubkey(&self) -> Result<Pubkey, SignerError> {
        self.signer.as_ref().unwrap().try_pubkey()
    }

    fn try_sign_message(&self, message: &[u8]) -> Result<Signature, SignerError> {
        self.signer.as_ref().unwrap().try_sign_message(message)
    }

    fn is_interactive(&self) -> bool {
        self.signer.as_ref().unwrap().is_interactive()
    }
}
