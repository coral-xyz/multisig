use std::path::PathBuf;

use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::read_keypair_file;
use anchor_client::solana_sdk::signature::{Keypair, Signer};
use anchor_client::solana_sdk::system_instruction;
use anchor_client::solana_sdk::sysvar;
use anchor_client::{Client, Cluster, Program};
use clap::Clap;
use multisig::accounts as multisig_accounts;
use multisig::instruction as multisig_instruction;
use rand::rngs::OsRng;

/// Multisig -- interact with a deployed Multisig program.
#[derive(Clap, Debug)]
struct Opts {
    /// Address of the Multisig program.
    #[clap(long)]
    multisig_program_id: Pubkey,

    #[clap(subcommand)]
    subcommand: SubCommand
}

#[derive(Clap, Debug)]
enum SubCommand {
    /// Create a new multisig address.
    CreateMultisig(CreateMultisigOpts)
}

#[derive(Clap, Debug)]
struct CreateMultisigOpts {
    /// How many signatures are needed to approve a transaction.
    #[clap(long)]
    threshold: u64,

    /// The public keys of the multisig owners, who can sign transactions.
    owners: Vec<Pubkey>,
}

/// Read the keypair from ~/.config/solana/id.json.
fn get_user_keypair() -> Keypair {
    let home = std::env::var("HOME").expect("Expected $HOME to be set.");
    let mut path = PathBuf::from(home);
    path.push(".config/solana/id.json");
    read_keypair_file(path).expect("Expected key pair at ~/.config/solana/id.json.")
}

fn main() {
    let opts = Opts::parse();
    let payer = get_user_keypair();
    let client = Client::new_with_options(
        Cluster::Localnet,
        payer,
        CommitmentConfig::confirmed(),
    );
    let program = client.program(opts.multisig_program_id);

    match opts.subcommand {
        SubCommand::CreateMultisig(cmd_opts) => create_multisig(program, cmd_opts),
    }
}

fn create_multisig(program: Program, opts: CreateMultisigOpts) {
    if opts.threshold > opts.owners.len() as u64 {
        println!("Threshold must be at most the number of owners.");
        std::process::exit(1);
    }

    // Before we can make the Multisig program initialize a new multisig
    // account, we need to have a program-owned account to used for that.
    // We generate a temporary key pair for this; after the account is
    // constructed, we no longer need to manipulate it (it is managed by the
    // Multisig program).
    // TODO: Should we save the private key, to allow deleting the multisig
    // account in order to recover the funds?
    let multisig_account = Keypair::generate(&mut OsRng);

    println!("Multisig account: {}", multisig_account.pubkey());

    program
        .request()
        // Create the program-owned account that will hold the multisig data,
        // and fund it from the payer account to make it rent-exempt.
        .instruction(system_instruction::create_account(
            &program.payer(),
            &multisig_account.pubkey(),
            // 352 bytes should be sufficient to hold a multisig state with 10
            // owners. Get the minimum rent-exempt balance for that, and
            // initialize the account with it, funded by the payer.
            // TODO: Ask for confirmation from the user first.
            program
                .rpc()
                .get_minimum_balance_for_rent_exemption(352)
                .expect("Failed to obtain minimum rent-exempt balance."),
            352,
            &program.id(),
        ))
        // Creating the account must be signed by the account itself.
        .signer(&multisig_account)
        .accounts(multisig_accounts::CreateMultisig {
            multisig: multisig_account.pubkey(),
            rent: sysvar::rent::ID,
        })
        .args(multisig_instruction::CreateMultisig {
            owners: opts.owners,
            threshold: opts.threshold,
            nonce: 0,
        })
        .send()
        .expect("Failed to send transaction.");
}
