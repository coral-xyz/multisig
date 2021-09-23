use std::str::FromStr;

use anchor_client::{
    solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer},
    Cluster,
};
use anyhow::Result;

use clap::Clap;
use clap2::ArgMatches;
use cli::{Job, Opts};
use gateway::MultisigGateway;
use rand::rngs::OsRng;
use serum_multisig::{Multisig, Transaction};
use service::MultisigService;
use solana_clap_utils::keypair::DefaultSigner;
use solana_remote_wallet::remote_wallet::maybe_wallet_manager;

mod anchor_toml;
mod cli;
mod gateway;
mod request_builder;
mod service;

fn main() -> Result<()> {
    solana_logger::setup_with_default("solana=debug");
    let cli_opts = Opts::parse();
    let anchor_toml = anchor_toml::load(&cli_opts.config)?;
    let program_id = Pubkey::from_str(&match anchor_toml.provider.cluster {
        Cluster::Mainnet => anchor_toml.programs.mainnet.serum_multisig,
        Cluster::Devnet => anchor_toml.programs.devnet.serum_multisig,
        Cluster::Localnet => anchor_toml.programs.localnet.unwrap().serum_multisig,
        _ => panic!(
            "Code currently cannot handle this cluster: {}",
            anchor_toml.provider.cluster
        ),
    })
    .expect("Invalid multisig program id");
    let payer = load_payer(&anchor_toml.provider.wallet);
    let service = load_service(anchor_toml.provider.cluster, program_id, &*payer)?;
    run_job(cli_opts.job, service)
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

fn run_job(job: Job, service: MultisigService) -> Result<()> {
    match job {
        Job::New(cmd) => {
            let keys = service.program.create_multisig(cmd.threshold, cmd.owners)?;
            println!("{} {}", keys.0, keys.1);
        }
        Job::ProposeUpgrade(cmd) => {
            let key = service.propose_upgrade(&cmd.multisig, &cmd.program, &cmd.buffer)?;
            println!("{}", key);
        }
        Job::Approve(cmd) => {
            service.program.approve(cmd.multisig, cmd.transaction)?;
        }
        Job::Execute(cmd) => {
            service.program.execute(cmd.multisig, cmd.transaction)?;
        }
        Job::Get(cmd) => {
            let ms = service.program.client.account::<Multisig>(cmd.key)?;
            let signer = service.program.signer(cmd.key).0;
            println!("{:?}", ms);
            println!("signer = {:?}", signer);
        }
        Job::GetTransaction(cmd) => {
            let tx = service.program.client.account::<Transaction>(cmd.key)?;
            println!("{:?}", tx);
        }
        Job::ProposeEdit(cmd) => {
            let key = service.propose_set_owners_and_change_threshold(
                cmd.multisig,
                cmd.threshold,
                cmd.owners,
            )?;
            println!("{}", key);
        }
        Job::ProposeMintTokens(cmd) => {
            let key =
                service.propose_mint_tokens(cmd.multisig, cmd.source, cmd.target, cmd.amount)?;
            println!("{}", key);
        }
        Job::ProposeTransferTokens(cmd) => {
            let key = service.propose_transfer_tokens(
                cmd.multisig,
                cmd.source,
                cmd.target,
                cmd.amount,
            )?;
            println!("{}", key);
        }
        Job::ProposeCustodyGenerateTokenMint(cmd) => {
            let key = service.propose_custody_generate_token_mint(cmd.multisig, cmd.mint_key)?;
            println!("{}", key);
        }
        Job::ProposeCustodyTransferTokens(cmd) => {
            let key = service.propose_custody_transfer_tokens(
                cmd.multisig,
                cmd.source,
                cmd.target,
                cmd.amount,
            )?;
            println!("{}", key);
        }
    }
    Ok(())
}
