extern crate clap;
extern crate anchor_client;
extern crate anyhow;
extern crate rand;
extern crate serum_multisig;
extern crate serde_derive;

use std::str::FromStr;

use anchor_client::{Cluster, solana_sdk::{pubkey::Pubkey, signature::read_keypair_file}};
use anyhow::Result;

use clap::Clap;
use cli::{Opts, Job};
use gateway::MultisigGateway;
use serum_multisig::{Multisig, Transaction};
use service::MultisigService;

mod gateway;
mod cli;
mod anchor_toml;
mod service;
mod request_builder;


fn main() -> Result<()> {
    solana_logger::setup_with_default("solana=debug");
    let cli_opts = Opts::parse();
    let anchor_toml = anchor_toml::load(&cli_opts.config)?;
    let program_id = Pubkey::from_str(&match anchor_toml.provider.cluster {
        Cluster::Mainnet => anchor_toml.programs.mainnet.serum_multisig,
        Cluster::Devnet => anchor_toml.programs.devnet.serum_multisig,
        Cluster::Localnet => anchor_toml.programs.localnet.unwrap().serum_multisig,
        _ => panic!("Code currently cannot handle this cluster: {}", anchor_toml.provider.cluster)
    }).expect("Invalid multisig program id");
    let service = load_service(
        anchor_toml.provider.cluster,
        program_id,
        &anchor_toml.provider.wallet,
    )?;
    run_job(cli_opts.job, service)
}

fn load_service(
    cluster: Cluster,
    program_id: Pubkey,
    keypair_path: &str,
) -> Result<MultisigService> {
    let keypair = read_keypair_file(&*shellexpand::tilde(keypair_path))
        .expect(&format!("Invalid keypair file {}", keypair_path));
    let connection = anchor_client::Client::new(cluster.clone(), keypair);
    let client = connection.program(program_id);
    let keypair = read_keypair_file(&*shellexpand::tilde(keypair_path))
        .expect(&format!("Invalid keypair file {}", keypair_path));

    Ok(MultisigService { program: MultisigGateway { client, cluster, keypair } })
}

fn run_job(job: Job, service: MultisigService) -> Result<()> {
    match job {
        Job::CreateMultisig(cmd) => {
            let owners =
                cmd.owners
                    .iter()
                    .map(|s|
                        Pubkey::from_str(s).expect(
                            &format!("Invalid Pubkey: '{}'", s)))
                    .collect();
            let keys = service.program.create_multisig(cmd.threshold, owners)?;
            println!("{} {}", keys.0, keys.1);
        }
        Job::ProposeUpgrade(cmd) => {
            let key = service.propose_upgrade(
                &cmd.multisig, 
                &cmd.program, 
                &cmd.buffer,
            )?;
            println!("{}", key);
        }
        Job::Approve(cmd) => {
            service.program.approve(
                cmd.multisig, 
                cmd.transaction,
            )?;
        }
        Job::Execute(cmd) => {
            service.program.execute(
                cmd.multisig, 
                cmd.transaction,
            )?;
        }
        Job::GetMultisig(cmd) => {
            let ms = service.program.client.account::<Multisig>(cmd.key)?;
            println!("{:?}", ms);
        }
        Job::GetTransaction(cmd) => {
            let tx = service.program.client.account::<Transaction>(cmd.key)?;
            println!("{:?}", tx);
        }
    }
    Ok(())
}
