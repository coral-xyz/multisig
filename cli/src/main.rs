use anchor_client::{
    solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer},
    Cluster,
};
use anyhow::Result;

use clap::Clap;
use clap2::ArgMatches;
use cli::{Job, Opts};
use config::MultisigConfig;
use gateway::MultisigGateway;
use rand::rngs::OsRng;
use serum_multisig::{Multisig, Transaction};
use service::MultisigService;
use solana_clap_utils::keypair::DefaultSigner;
use solana_remote_wallet::remote_wallet::maybe_wallet_manager;

mod cli;
mod config;
mod gateway;
mod request_builder;
mod service;

fn main() -> Result<()> {
    solana_logger::setup_with_default("solana=debug");
    let cli_opts = Opts::parse();
    let multisig_config = config::load(&cli_opts.config)?;
    let payer = load_payer(&multisig_config.wallet);
    let cluster = match &*multisig_config.cluster.to_lowercase() {
        "localnet" | "localhost" => Cluster::Localnet,
        "devnet" => Cluster::Devnet,
        "mainnet" => Cluster::Mainnet,
        rpc => {
            let wss = rpc.replace("https", "wss");
            Cluster::Custom(rpc.to_owned(), wss)
        }
    };
    let service = load_service(cluster, multisig_config.program_id, &*payer)?;
    run_job(cli_opts.job, service, &multisig_config)
}

fn load_payer(path: &str) -> Box<dyn Signer> {
    let path = &*shellexpand::tilde(path);
    let mut wallet_manager = maybe_wallet_manager().unwrap();
    let default_signer = DefaultSigner::new("keypair".to_string(), path);
    let arg_matches = ArgMatches::default();
    default_signer
        .signer_from_path(&arg_matches, &mut wallet_manager)
        .unwrap()
}

fn load_service<'a>(
    cluster: Cluster,
    program_id: Pubkey,
    payer: &'a dyn Signer,
) -> Result<MultisigService<'a>> {
    // todo change anchor to use Signer so we don't need this dummy keypair that we have to be careful not to use
    let keypair = Keypair::generate(&mut OsRng);
    let connection = anchor_client::Client::new(cluster.clone(), keypair);
    let client = connection.program(program_id);

    Ok(MultisigService {
        program: MultisigGateway {
            client,
            cluster,
            payer,
        },
    })
}

fn run_job(job: Job, service: MultisigService, config: &MultisigConfig) -> Result<()> {
    match job {
        Job::New(cmd) => {
            let keys = service.program.create_multisig(cmd.threshold, cmd.owners)?;
            println!("{} {}", keys.0, keys.1);
        }
        Job::ProposeUpgrade(cmd) => {
            let key = service.propose_upgrade(&config.multisig, &cmd.program, &cmd.buffer)?;
            println!("{}", key);
        }
        Job::Approve(cmd) => {
            service.program.approve(config.multisig, cmd.transaction)?;
        }
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
            let tx = service.program.client.account::<Transaction>(cmd.key)?;
            println!("{:#?}", tx);
        }
        Job::InspectProposal(cmd) => {
            let tx = service.program.client.account::<Transaction>(cmd.key)?;
            service.inspect_proposal(&tx)?;
        }
        Job::ProposeEdit(cmd) => {
            let key = service.propose_set_owners_and_change_threshold(
                config.multisig,
                cmd.threshold,
                cmd.owners,
            )?;
            println!("{}", key);
        }
        Job::ProposeMintTokens(cmd) => {
            let key =
                service.propose_mint_tokens(config.multisig, cmd.source, cmd.target, cmd.amount)?;
            println!("{}", key);
        }
        Job::ProposeTransferTokens(cmd) => {
            let key = service.propose_transfer_tokens(
                config.multisig,
                cmd.source,
                cmd.target,
                cmd.amount,
            )?;
            println!("{}", key);
        }
        Job::ProposeCustodyGenerateTokenMint(cmd) => {
            let key = service.propose_custody_generate_token_mint(config.multisig, cmd.mint_key)?;
            println!("{}", key);
        }
        Job::ProposeCustodyTransferTokens(cmd) => {
            let key = service.propose_custody_transfer_tokens(
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
