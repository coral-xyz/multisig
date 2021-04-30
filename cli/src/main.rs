use std::path::PathBuf;

use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::read_keypair_file;
use anchor_client::solana_sdk::signature::{Keypair, Signer};
use anchor_client::solana_sdk::system_instruction;
use anchor_client::solana_sdk::bpf_loader_upgradeable;
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
    CreateMultisig(CreateMultisigOpts),

    /// Show the owners and threshold of the given multisig.
    ShowMultisig(ShowMultisigOpts),

    /// Show the details of a transaction.
    ShowTransaction(ShowTransactionOpts),

    /// Propose replacing a program with that in the given buffer account.
    ProposeUpgrade(ProposeUpgradeOpts),

    /// Approve a proposed transaction.
    Approve(ApproveOpts),
}

#[derive(Clap, Debug)]
struct CreateMultisigOpts {
    /// How many signatures are needed to approve a transaction.
    #[clap(long)]
    threshold: u64,

    /// The public keys of the multisig owners, who can sign transactions.
    #[clap(long = "owner")]
    owners: Vec<Pubkey>,
}

#[derive(Clap, Debug)]
struct ProposeUpgradeOpts {
    /// The multisig account whose owners should vote for this proposal.
    #[clap(long)]
    multisig_address: Pubkey,

    /// The program id of the program to upgrade.
    #[clap(long)]
    program_address: Pubkey,

    /// The address that holds the new program data.
    #[clap(long)]
    buffer_address: Pubkey,

    /// Account that will receive leftover funds from the buffer account.
    #[clap(long)]
    spill_address: Pubkey,
}

#[derive(Clap, Debug)]
struct ShowMultisigOpts {
    /// The multisig account to display.
    #[clap(long)]
    multisig_address: Pubkey,
}

#[derive(Clap, Debug)]
struct ShowTransactionOpts {
    /// The transaction to display.
    #[clap(long)]
    transaction_address: Pubkey,
}

#[derive(Clap, Debug)]
struct ApproveOpts {
    /// The multisig account whose owners should vote for this proposal.
    // TODO: Can be omitted, we can obtain it from the transaction account.
    #[clap(long)]
    multisig_address: Pubkey,

    /// The transaction to approve.
    #[clap(long)]
    transaction_address: Pubkey,
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
        SubCommand::ShowMultisig(cmd_opts) => show_multisig(program, cmd_opts),
        SubCommand::ShowTransaction(cmd_opts) => show_transaction(program, cmd_opts),
        SubCommand::ProposeUpgrade(cmd_opts) => propose_upgrade(program, cmd_opts),
        SubCommand::Approve(cmd_opts) => approve(program, cmd_opts),
    }
}

fn get_multisig_program_address(
    program: &Program,
    multisig_pubkey: &Pubkey,
) -> (Pubkey, u8) {
    let seeds = [
        multisig_pubkey.as_ref(),
    ];
    Pubkey::find_program_address(
        &seeds,
        &program.id(),
    )
}

fn create_multisig(program: Program, opts: CreateMultisigOpts) {
    if opts.threshold > opts.owners.len() as u64 {
        println!("Threshold must be at most the number of owners.");
        std::process::exit(1);
    }
    if opts.threshold == 0 {
        println!("Threshold must be at least 1.");
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

    // The Multisig program will sign transactions on behalf of a derived
    // account. Print this derived account, so it can be used to set as e.g.
    // the upgrade authority for a program. Because not every derived address is
    // valid, a bump seed is appended to the seeds. It is stored in the `nonce`
    // field in the multisig account, and the Multisig program includes it when
    // deriving its program address.
    let (program_derived_address, nonce) = get_multisig_program_address(
        &program,
        &multisig_account.pubkey(),
    );
    // TODO: The address it prints here, is not equal to the one that the web UI
    // displays ... why not?
    println!(
        "Program derived address (use as upgrade authority): {}",
        program_derived_address,
    );

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
            nonce: nonce,
        })
        .send()
        .expect("Failed to send transaction.");
}

fn show_multisig(program: Program, opts: ShowMultisigOpts) {
    let multisig: multisig::Multisig = program
        .account(opts.multisig_address)
        .expect("Failed to read multisig state from account.");

    println!("Threshold: {} out of {}", multisig.threshold, multisig.owners.len());
    println!("Owners:");
    for owner_pubkey in &multisig.owners {
        println!("  {}", owner_pubkey);
    }
}

fn show_transaction(program: Program, opts: ShowTransactionOpts) {
    let transaction: multisig::Transaction = program
        .account(opts.transaction_address)
        .expect("Failed to read transaction data from account.");

    println!("Multisig: {}", transaction.multisig);
    println!("Did execute: {}", transaction.did_execute);

    // Also query the multisig, to get the owner public keys, so we can display
    // exactly who voted.
    // TODO: Is there a way to make the client query from the same block, so
    // that we are sure that we get a consistent view of the data?
    let multisig: multisig::Multisig = program
        .account(transaction.multisig)
        .expect("Failed to read multisig state from account.");

    if transaction.owner_set_seqno == multisig.owner_set_seqno {
        println!("Signers:");
        for (owner_pubkey, did_sign) in multisig.owners.iter().zip(transaction.signers) {
            println!("  [{}] {}", if did_sign { 'x' } else { ' ' }, owner_pubkey);
        }
    } else {
        println!("The owners of the multisig have changed since this transaction was created,");
        println!("therefore we cannot show the identities of the signers.");
        let num_signatures = transaction
            .signers
            .iter()
            .filter(|&did_sign| *did_sign)
            .count();
        println!("It had {} out of {} signatures.", num_signatures, transaction.signers.len());
    }

    // TODO: Print transaction details.
}

fn propose_upgrade(program: Program, opts: ProposeUpgradeOpts) {
    let (program_derived_address, _nonce) = get_multisig_program_address(
        &program,
        &opts.multisig_address,
    );

    let upgrade_instruction = bpf_loader_upgradeable::upgrade(
        &opts.program_address,
        &opts.buffer_address,
        // The upgrade authority is the multisig-derived program address.
        &program_derived_address,
        &opts.spill_address,
    );

    // The program expects `multisig::TransactionAccount` instead of
    // `solana_sdk::AccountMeta`. The types are structurally identical,
    // but not nominally, so we need to convert these.
    let accounts: Vec<_> = upgrade_instruction
        .accounts
        .iter()
        .map(multisig::TransactionAccount::from)
        .collect();

    // The transaction is stored by the Multisig program in yet another account,
    // that we create just for this transaction.
    // TODO: Should we save the private key, to allow deleting the multisig
    // account in order to recover the funds?
    let transaction_account = Keypair::generate(&mut OsRng);
    println!("Transaction account: {}", transaction_account.pubkey());

    program
        .request()
        // Create the program-owned account that will hold the transaction data,
        // and fund it from the payer account to make it rent-exempt.
        .instruction(system_instruction::create_account(
            &program.payer(),
            &transaction_account.pubkey(),
            // TODO: Is there a good way to determine the size of the
            // transaction; can we serialize and measure maybe? For now, assume
            // 500 bytes will be sufficient.
            // TODO: Ask for confirmation from the user first before funding the
            // account.
            program
                .rpc()
                .get_minimum_balance_for_rent_exemption(500)
                .expect("Failed to obtain minimum rent-exempt balance."),
            500,
            &program.id(),
        ))
        // Creating the account must be signed by the account itself.
        .signer(&transaction_account)
        .accounts(multisig_accounts::CreateTransaction {
            multisig: opts.multisig_address,
            transaction: transaction_account.pubkey(),
            // For convenience, assume that the party that signs the proposal
            // transaction is a member of the multisig owners, and use it as the
            // proposer.
            proposer: program.payer(),
            rent: sysvar::rent::ID,
        })
        .args(multisig_instruction::CreateTransaction {
            pid: upgrade_instruction.program_id,
            accs: accounts,
            data: upgrade_instruction.data,
        })
        .send()
        .expect("Failed to send transaction.");
}

fn approve(program: Program, opts: ApproveOpts) {
    program
        .request()
        .accounts(multisig_accounts::Approve {
            multisig: opts.multisig_address,
            transaction: opts.transaction_address,
            // The owner that signs the multisig proposed transaction, should be
            // the public key that signs the entire approval transaction (which
            // is also the payer).
            owner: program.payer(),
        })
        .args(multisig_instruction::Approve)
        .send()
        .expect("Failed to send transaction.");
}
