use anchor_client::Cluster;
use anyhow::Result;

use clap::Clap;
use cli::{Job, Opts};
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
    let cluster = match &*multisig_config.cluster.to_lowercase() {
        "localnet" | "localhost" => Cluster::Localnet,
        "devnet" => Cluster::Devnet,
        "mainnet" => Cluster::Mainnet,
        rpc => {
            let wss = rpc.replace("https", "wss");
            Cluster::Custom(rpc.to_owned(), wss)
        }
    };
    let service = multisig_client::load_service(
        cluster,
        multisig_config.program_id,
        &*payer,
        &multisig_config,
    )?;
    run_job(cli_opts.job, service, &multisig_config)
}

fn run_job(job: Job, service: MultisigService, config: &MultisigConfig) -> Result<()> {
    match job {
        Job::New(cmd) => {
            let keys = service.program.create_multisig(cmd.threshold, cmd.owners)?;
            println!("{} {}", keys.0, keys.1);
        }
        Job::AddDelegates(cmd) => {
            service.add_delegates(config.multisig, cmd.delegates)?;
        }
        Job::RemoveDelegates(cmd) => {
            service.remove_delegates(config.multisig, cmd.delegates)?;
        }
        Job::ProposeUpgrade(cmd) => {
            let key = service.propose_upgrade(&config.multisig, &cmd.program, &cmd.buffer)?;
            println!("{}", key);
        }
        Job::Approve(cmd) => service.program.approve(config.multisig, cmd.transaction)?,
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
        Job::ProposeSetMarketFlags(cmd) => {
            let mut flags = MarketFlags::empty();
            if cmd.halt_borrows {
                flags |= MarketFlags::HALT_BORROWS;
            }
            if cmd.halt_deposits {
                flags |= MarketFlags::HALT_DEPOSITS;
            }
            if cmd.halt_repays {
                flags |= MarketFlags::HALT_REPAYS;
            }
            let key = service.propose_set_market_flags(config.multisig, cmd.market, flags)?;
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
