use std::fmt;
use std::fs::File;
use std::io::{Read, Write};
use std::sync::Arc;

use anchor_client::solana_sdk::{
    bpf_loader_upgradeable,
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction, sysvar,
};
use anchor_client::{Client, Cluster, Program};
use anchor_lang::prelude::{AccountMeta, ToAccountMetas};
use anchor_lang::{Discriminator, InstructionData};
use borsh::de::BorshDeserialize;
use borsh::ser::BorshSerialize;
use clap::Clap;
use expanded_path::ExpandedPath;
use input_keypair::InputKeypair;
use input_pubkey::InputPubkey;
use multisig::{accounts as multisig_accounts, TransactionInstruction};
use multisig::{instruction as multisig_instruction, Transaction};
use serde::{Serialize, Serializer};

mod expanded_path;
mod input_keypair;
mod input_pubkey;
mod spl_token_transfer;

/// Multisig -- interact with a deployed Multisig program.
#[derive(Clap, Debug)]
struct Opts {
    /// Address of the Multisig program.
    #[clap(long, default_value = "H88LfRBiJLZ7wYkHGuwkKTaijfQxexq8JvzUndu7fyjL")]
    multisig_program_id: InputPubkey,

    /// The keypair to sign and pay with. [default: ~/.config/solana/id.json]
    #[clap(long, default_value = "~/.config/solana/id.json")]
    keypair_path: InputKeypair,

    /// Cluster to connect to (mainnet, testnet, devnet, localnet, or url).
    #[clap(long, default_value = "localnet")]
    cluster: Cluster,

    /// Output json instead of text to stdout.
    #[clap(long)]
    output_json: bool,

    #[clap(subcommand)]
    subcommand: SubCommand,
}

#[derive(Clap, Debug)]
enum SubCommand {
    /// Create a new multisig address.
    CreateMultisig(CreateMultisigOpts),

    /// Show the owners and threshold of the given multisig.
    ShowMultisig(ShowMultisigOpts),

    /// Show the details of a transaction.
    ShowTransaction(ShowTransactionOpts),

    /// Propose spl-token transfer
    ProposeSplTokenTransfer(spl_token_transfer::SplTokenTransferOptions),

    /// Propose binary transaction from file
    ProposeBinaryTransaction(ProposeBinaryTransactionOpts),

    /// Propose replacing a program with that in the given buffer account.
    ProposeUpgrade(ProposeUpgradeOpts),

    /// Propose replacing the set of owners or threshold of this multisig.
    ProposeChangeMultisig(ProposeChangeMultisigOpts),

    /// Approve a proposed transaction.
    Approve(ApproveOpts),

    /// Execute a transaction that has enough approvals.
    ExecuteTransaction(ExecuteTransactionOpts),
}

#[derive(Clap, Debug)]
struct CreateMultisigOpts {
    #[clap(long)]
    multisig_account: Option<InputKeypair>, // random by default

    #[clap(long)]
    output_multisig_account: Option<ExpandedPath>,

    #[clap(long)]
    output_multisig_pda: Option<ExpandedPath>,

    /// How many signatures are needed to approve a transaction.
    #[clap(long)]
    threshold: u64,

    /// The public keys of the multisig owners, who can sign transactions.
    #[clap(long = "owner", required = true)]
    owners: Vec<Pubkey>,
}

impl CreateMultisigOpts {
    /// Perform a few basic checks to rule out nonsensical multisig settings.
    ///
    /// Exits if validation fails.
    fn validate_or_exit(&self) {
        if self.threshold > self.owners.len() as u64 {
            println!("Threshold must be at most the number of owners.");
            std::process::exit(1);
        }
        if self.threshold == 0 {
            println!("Threshold must be at least 1.");
            std::process::exit(1);
        }
    }
}

#[derive(Clap, Debug)]
struct ProposeBinaryTransactionOpts {
    /// The multisig account whose owners should vote for this proposal.
    #[clap(long)]
    multisig_address: InputPubkey,

    #[clap(long)]
    data: ExpandedPath,

    #[clap(long)]
    transaction_account: Option<InputKeypair>,
}

#[derive(Clap, Debug)]
struct ProposeUpgradeOpts {
    /// The multisig account whose owners should vote for this proposal.
    #[clap(long)]
    multisig_address: InputPubkey,

    /// The program id of the program to upgrade.
    #[clap(long)]
    program_address: InputPubkey,

    /// The address that holds the new program data.
    #[clap(long)]
    buffer_address: InputPubkey,

    /// Account that will receive leftover funds from the buffer account.
    #[clap(long, default_value = "~/.config/solana/id.json")]
    spill_address: InputPubkey,

    #[clap(long)]
    transaction_account: Option<InputKeypair>,
}

#[derive(Clap, Debug)]
struct ProposeChangeMultisigOpts {
    /// The multisig account to modify.
    #[clap(long)]
    multisig_address: InputPubkey,

    // The fields below are the same as for `CreateMultisigOpts`, but we can't
    // just embed a `CreateMultisigOpts`, because Clap does not support that.
    /// How many signatures are needed to approve a transaction.
    #[clap(long)]
    threshold: u64,

    /// The public keys of the multisig owners, who can sign transactions.
    #[clap(long = "owner", required = true)]
    owners: Vec<Pubkey>,

    #[clap(long)]
    transaction_account: Option<InputKeypair>,
}

impl From<&ProposeChangeMultisigOpts> for CreateMultisigOpts {
    fn from(opts: &ProposeChangeMultisigOpts) -> CreateMultisigOpts {
        CreateMultisigOpts {
            threshold: opts.threshold,
            owners: opts.owners.clone(),
            multisig_account: None,
            output_multisig_account: None,
            output_multisig_pda: None,
        }
    }
}

#[derive(Clap, Debug)]
struct ShowMultisigOpts {
    /// The multisig account to display.
    #[clap(long)]
    multisig_address: InputPubkey,
}

#[derive(Clap, Debug)]
struct ShowTransactionOpts {
    /// The transaction to display.
    #[clap(long)]
    transaction_address: InputPubkey,

    #[clap(long)]
    output_data: Option<ExpandedPath>,
}

#[derive(Clap, Debug)]
struct ApproveOpts {
    /// The transaction to approve.
    #[clap(long)]
    transaction_address: InputPubkey,

    #[clap(long)]
    data: Option<ExpandedPath>,
}

#[derive(Clap, Debug)]
struct ExecuteTransactionOpts {
    /// The multisig account whose owners approved this transaction.
    // TODO: Can be omitted, we can obtain it from the transaction account.
    #[clap(long)]
    multisig_address: InputPubkey,

    /// The transaction to execute.
    #[clap(long)]
    transaction_address: InputPubkey,
}

fn print_output<Output: fmt::Display + Serialize>(as_json: bool, output: &Output) {
    if as_json {
        let json_string =
            serde_json::to_string_pretty(output).expect("Failed to serialize output as json.");
        println!("{}", json_string);
    } else {
        println!("{}", output);
    }
}

fn main() {
    let opts = Opts::parse();

    let client = Client::new_with_options(
        opts.cluster,
        Keypair::from_bytes(&opts.keypair_path.as_keypair().to_bytes()).unwrap(),
        CommitmentConfig::confirmed(),
    );
    let program = client.program(opts.multisig_program_id.as_pubkey());

    match opts.subcommand {
        SubCommand::CreateMultisig(cmd_opts) => {
            let output = create_multisig(program, cmd_opts);
            print_output(opts.output_json, &output);
        }
        SubCommand::ShowMultisig(cmd_opts) => {
            let output = show_multisig(program, cmd_opts);
            print_output(opts.output_json, &output);
        }
        SubCommand::ShowTransaction(cmd_opts) => {
            let output = show_transaction(program, cmd_opts);
            print_output(opts.output_json, &output);
        }
        SubCommand::ProposeSplTokenTransfer(cmd_opts) => {
            spl_token_transfer::propose_spl_token_transfer(program, cmd_opts);
        }
        SubCommand::ProposeBinaryTransaction(cmd_opts) => {
            let output = propose_binary_transaction(program, cmd_opts);
            print_output(opts.output_json, &output);
        }
        SubCommand::ProposeUpgrade(cmd_opts) => {
            let output = propose_upgrade(program, cmd_opts);
            print_output(opts.output_json, &output);
        }
        SubCommand::ProposeChangeMultisig(cmd_opts) => {
            let output = propose_change_multisig(program, cmd_opts);
            print_output(opts.output_json, &output);
        }
        SubCommand::Approve(cmd_opts) => approve(program, cmd_opts),
        SubCommand::ExecuteTransaction(cmd_opts) => execute_transaction(program, cmd_opts),
    }
}

fn get_multisig_program_address(program: &Program, multisig_pubkey: &Pubkey) -> (Pubkey, u8) {
    let seeds = [multisig_pubkey.as_ref()];
    Pubkey::find_program_address(&seeds, &program.id())
}

/// Wrapper for `Pubkey` to serialize it as base58 in json, instead of a list of numbers.
struct PubkeyBase58(Pubkey);

impl fmt::Display for PubkeyBase58 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Serialize for PubkeyBase58 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Defer to the Display impl, which formats as base58.
        serializer.collect_str(&self.0)
    }
}

impl From<Pubkey> for PubkeyBase58 {
    fn from(pk: Pubkey) -> PubkeyBase58 {
        PubkeyBase58(pk)
    }
}

impl From<&Pubkey> for PubkeyBase58 {
    fn from(pk: &Pubkey) -> PubkeyBase58 {
        PubkeyBase58(*pk)
    }
}

#[derive(Serialize)]
struct CreateMultisigOutput {
    multisig_address: PubkeyBase58,
    multisig_program_derived_address: PubkeyBase58,
}

impl fmt::Display for CreateMultisigOutput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Multisig address:        {}", self.multisig_address)?;
        writeln!(
            f,
            "Program derived address: {}",
            self.multisig_program_derived_address
        )?;
        writeln!(f, "The multisig can sign on behalf of the derived address.")?;
        Ok(())
    }
}

fn create_multisig(program: Program, opts: CreateMultisigOpts) -> CreateMultisigOutput {
    // Enforce a few basic sanity checks.
    opts.validate_or_exit();

    // Before we can make the Multisig program initialize a new multisig
    // account, we need to have a program-owned account to used for that.
    // We may generate a temporary key pair for this; after the account is
    // constructed, we no longer need to manipulate it (it is managed by the
    // Multisig program). We don't save the private key because the account will
    // be owned by the Multisig program later anyway. Its funds will be locked
    // up forever.
    let multisig_account = if let Some(multisig_account) = opts.multisig_account {
        multisig_account.as_keypair()
    } else {
        Arc::new(Keypair::new())
    };

    if let Some(output_multisig_account) = opts.output_multisig_account {
        File::create(output_multisig_account.as_path())
            .unwrap()
            .write_all(multisig_account.pubkey().to_string().as_bytes()) // base58 representation
            .unwrap();
    }

    // The Multisig program will sign transactions on behalf of a derived
    // account. Return this derived account, so it can be used to set as e.g.
    // the upgrade authority for a program. Because not every derived address is
    // valid, a bump seed is appended to the seeds. It is stored in the `nonce`
    // field in the multisig account, and the Multisig program includes it when
    // deriving its program address.
    let (program_derived_address, nonce) =
        get_multisig_program_address(&program, &multisig_account.pubkey());

    if let Some(output_multisig_pda) = opts.output_multisig_pda {
        File::create(output_multisig_pda.as_path())
            .unwrap()
            .write_all(program_derived_address.to_string().as_bytes()) // base58 representation
            .unwrap();
    }

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
        .signer(multisig_account.as_ref())
        .accounts(multisig_accounts::CreateMultisig {
            multisig: multisig_account.pubkey(),
            rent: sysvar::rent::ID,
        })
        .args(multisig_instruction::CreateMultisig {
            owners: opts.owners,
            threshold: opts.threshold,
            nonce,
        })
        .send()
        .expect("Failed to send transaction.");

    CreateMultisigOutput {
        multisig_address: multisig_account.pubkey().into(),
        multisig_program_derived_address: program_derived_address.into(),
    }
}

#[derive(Serialize)]
struct ShowMultisigOutput {
    multisig_program_derived_address: PubkeyBase58,
    threshold: u64,
    owners: Vec<PubkeyBase58>,
}

impl fmt::Display for ShowMultisigOutput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "Program derived address: {}",
            self.multisig_program_derived_address
        )?;
        writeln!(
            f,
            "Threshold: {} out of {}",
            self.threshold,
            self.owners.len()
        )?;
        writeln!(f, "Owners:")?;
        for owner_pubkey in &self.owners {
            writeln!(f, "  {}", owner_pubkey)?;
        }
        Ok(())
    }
}

fn show_multisig(program: Program, opts: ShowMultisigOpts) -> ShowMultisigOutput {
    let multisig: multisig::Multisig = program
        .account(opts.multisig_address.as_pubkey())
        .expect("Failed to read multisig state from account.");

    let (program_derived_address, _nonce) =
        get_multisig_program_address(&program, &opts.multisig_address.as_pubkey());

    ShowMultisigOutput {
        multisig_program_derived_address: program_derived_address.into(),
        threshold: multisig.threshold,
        owners: multisig.owners.iter().map(PubkeyBase58::from).collect(),
    }
}

#[derive(Serialize)]
struct ShowTransactionSigner {
    owner: PubkeyBase58,
    did_sign: bool,
}

#[derive(Serialize)]
enum ShowTransactionSigners {
    /// The current owners of the multisig are the same as in the transaction,
    /// and these are the owners and whether they signed.
    Current { signers: Vec<ShowTransactionSigner> },

    /// The owners of the multisig have changed since this transaction, so we
    /// cannot know who the signers were any more, only how many signatures it
    /// had.
    Outdated {
        num_signed: usize,
        num_owners: usize,
    },
}

/// If an `Instruction` is a known one, this contains its details.
#[derive(Serialize)]
enum ParsedInstruction {
    BpfLoaderUpgrade {
        program_to_upgrade: PubkeyBase58,
        program_data_address: PubkeyBase58,
        buffer_address: PubkeyBase58,
        spill_address: PubkeyBase58,
    },
    MultisigChange {
        threshold: u64,
        owners: Vec<PubkeyBase58>,
    },
    Unrecognized(Vec<u8>),
}

#[derive(Serialize)]
struct ShowTransactionOutput {
    multisig_address: PubkeyBase58,
    did_execute: bool,
    signers: ShowTransactionSigners,
    // TODO: when using --output-json, the addresses in here get serialized as
    // arrays of numbers instead of base58 strings, because this uses the
    // regular Solana `Pubkey` types. But I don't feel like creating an
    // `Instruction` duplicate just for this purpose right now, we can create
    // one when needed.
    instruction: Instruction,
    parsed_instruction: ParsedInstruction,
}

impl fmt::Display for ShowTransactionOutput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Multisig: {}", self.multisig_address)?;
        writeln!(f, "Did execute: {}", self.did_execute)?;

        match &self.signers {
            ShowTransactionSigners::Current { signers } => {
                writeln!(f, "\nSigners:")?;
                for signer in signers {
                    writeln!(
                        f,
                        "  [{}] {}",
                        if signer.did_sign { 'x' } else { ' ' },
                        signer.owner,
                    )?;
                }
            }
            ShowTransactionSigners::Outdated {
                num_signed,
                num_owners,
            } => {
                writeln!(
                    f,
                    "\nThe owners of the multisig have changed since this transaction was created,"
                )?;
                writeln!(f, "therefore we cannot show the identities of the signers.")?;
                writeln!(
                    f,
                    "The transaction had {} out of {} signatures.",
                    num_signed, num_owners,
                )?;
            }
        }

        writeln!(f, "\nInstruction:")?;
        writeln!(f, "  Program to call: {}", self.instruction.program_id)?;
        writeln!(f, "  Accounts:\n")?;
        for account in &self.instruction.accounts {
            writeln!(
                f,
                "    * {}\n      signer: {}, writable: {}\n",
                account.pubkey, account.is_signer, account.is_writable,
            )?;
        }

        match &self.parsed_instruction {
            ParsedInstruction::BpfLoaderUpgrade {
                program_to_upgrade,
                program_data_address,
                buffer_address,
                spill_address,
            } => {
                writeln!(
                    f,
                    "  This is a bpf_loader_upgradeable::upgrade instruction."
                )?;
                writeln!(f, "    Program to upgrade:      {}", program_to_upgrade)?;
                writeln!(f, "    Program data address:    {}", program_data_address)?;
                writeln!(f, "    Buffer with new program: {}", buffer_address)?;
                writeln!(f, "    Spill address:           {}", spill_address)?;
            }
            ParsedInstruction::MultisigChange { threshold, owners } => {
                writeln!(
                    f,
                    "  This is a multisig::set_owners_and_change_threshold instruction."
                )?;
                writeln!(
                    f,
                    "    New threshold: {} out of {}",
                    threshold,
                    owners.len()
                )?;
                writeln!(f, "    New owners:")?;
                for owner_pubkey in owners {
                    writeln!(f, "      {}", owner_pubkey)?;
                }
            }
            ParsedInstruction::Unrecognized(data) => {
                writeln!(f, "  Unrecognized instruction: {}.", hex::encode(data))?;
            }
        }

        Ok(())
    }
}

fn show_transaction(program: Program, opts: ShowTransactionOpts) -> ShowTransactionOutput {
    let transaction: multisig::Transaction = program
        .account(opts.transaction_address.as_pubkey())
        .expect("Failed to read transaction data from account.");

    if let Some(output_data) = opts.output_data {
        File::create(output_data.as_path())
            .expect("Can not create tx data file")
            .write_all(&transaction.instruction.try_to_vec().unwrap())
            .expect("Can not write tx data");
    }

    // Also query the multisig, to get the owner public keys, so we can display
    // exactly who voted.
    // Note: Although these are separate reads, the result will still be
    // consistent, because the transaction account must be owned by the Multisig
    // program, and the multisig program never modifies the
    // `transaction.multisig` field.
    let multisig: multisig::Multisig = program
        .account(transaction.multisig)
        .expect("Failed to read multisig state from account.");

    let signers = if transaction.owner_set_seqno == multisig.owner_set_seqno {
        // If the owners did not change, match up every vote with its owner.
        ShowTransactionSigners::Current {
            signers: multisig
                .owners
                .iter()
                .cloned()
                .zip(transaction.signers.iter())
                .map(|(owner, did_sign)| ShowTransactionSigner {
                    owner: owner.into(),
                    did_sign: *did_sign,
                })
                .collect(),
        }
    } else {
        // If the owners did change, we no longer know who voted. The best we
        // can do is report how many signatures there were.
        ShowTransactionSigners::Outdated {
            num_signed: transaction
                .signers
                .iter()
                .filter(|&did_sign| *did_sign)
                .count(),
            num_owners: transaction.signers.len(),
        }
    };

    let instr = Instruction::from(&transaction.instruction);

    let parsed_instr = if instr.program_id == bpf_loader_upgradeable::ID
        && bpf_loader_upgradeable::is_upgrade_instruction(&instr.data[..])
    {
        // Account meaning, according to
        // https://docs.rs/solana-sdk/1.5.19/solana_sdk/loader_upgradeable_instruction/enum.UpgradeableLoaderInstruction.html#variant.Upgrade
        ParsedInstruction::BpfLoaderUpgrade {
            program_data_address: instr.accounts[0].pubkey.into(),
            program_to_upgrade: instr.accounts[1].pubkey.into(),
            buffer_address: instr.accounts[2].pubkey.into(),
            spill_address: instr.accounts[3].pubkey.into(),
        }
    } else
    // Try to deserialize the known multisig instructions. The instruction
    // data starts with an 8-byte tag derived from the name of the function,
    // and then the struct data itself, so we need to skip the first 8 bytes
    // when deserializing. See also `anchor_lang::InstructionData::data()`.
    // There doesn't appear to be a way to access the tag through code
    // currently (https://github.com/project-serum/anchor/issues/243), so we
    // hard-code the tag here (it is stable as long as the namespace and
    // function name do not change).
    if instr.program_id == program.id()
        && &instr.data[..8] == &[122, 49, 168, 177, 231, 28, 167, 204]
    {
        if let Ok(instr) =
            multisig_instruction::SetOwnersAndChangeThreshold::try_from_slice(&instr.data[8..])
        {
            ParsedInstruction::MultisigChange {
                threshold: instr.threshold,
                owners: instr.owners.iter().map(PubkeyBase58::from).collect(),
            }
        } else {
            ParsedInstruction::Unrecognized(transaction.instruction.try_to_vec().unwrap())
        }
    } else {
        ParsedInstruction::Unrecognized(transaction.instruction.try_to_vec().unwrap())
    };

    ShowTransactionOutput {
        multisig_address: transaction.multisig.into(),
        did_execute: transaction.did_execute,
        signers,
        instruction: instr,
        parsed_instruction: parsed_instr,
    }
}

#[derive(Serialize)]
struct ProposeInstructionOutput {
    transaction_address: PubkeyBase58,
}

impl fmt::Display for ProposeInstructionOutput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Transaction address: {}", self.transaction_address)
    }
}

/// Propose the given instruction to be approved and executed by the multisig.
pub(crate) fn propose_instruction(
    program: Program,
    multisig_address: Pubkey,
    transaction_account: Arc<Keypair>,
    instruction: Instruction,
) -> ProposeInstructionOutput {


    // get multisig_instance data
    let multisig_data: multisig::Multisig = program
        .account(multisig_address)
        .expect("Failed to read multisig state from account.");


    // The Multisig program expects `multisig::TransactionAccount` instead of
    // `solana_sdk::AccountMeta`. The types are structurally identical,
    // but not nominally, so we need to convert these.
    let accounts: Vec<_> = instruction
        .accounts
        .iter()
        .map(multisig::TransactionAccount::from)
        .collect();

    // Build the data that the account will hold, just to measure its size, so
    // we can allocate an account of the right size.

    let mut signers_flags = Vec::new(); // boolean array, who signed
    signers_flags.resize(multisig_data.owners.len(), false);

    let dummy_tx = multisig::Transaction {
        multisig: multisig_address,
        instruction: TransactionInstruction {
            program_id: instruction.program_id,
            accounts: accounts.clone(),
            data: instruction.data.clone(),
        },
        signers: signers_flags,
        did_execute: false,
        owner_set_seqno: 0,
    };

    // The space used is the serialization of the transaction itself, plus the
    // discriminator that Anchor uses to identify the account type.
    let mut account_bytes = dummy_tx
        .try_to_vec()
        .expect("Failed to serialize dummy transaction.");
    account_bytes.extend(&multisig::Transaction::discriminator()[..]);
    println!(
        "create tx account owned by {}, size {}",
        &program.id(),
        account_bytes.len()
    );
    program
        .request()
        // Create the program-owned account that will hold the transaction data,
        // and fund it from the payer account to make it rent-exempt.
        .instruction(system_instruction::create_account(
            &program.payer(),
            &transaction_account.pubkey(),
            // TODO: Ask for confirmation from the user first before funding the
            // account.
            program
                .rpc()
                .get_minimum_balance_for_rent_exemption(account_bytes.len())
                .expect("Failed to obtain minimum rent-exempt balance."),
            account_bytes.len() as u64,
            &program.id(),
        ))
        // Creating the account must be signed by the account itself.
        .signer(transaction_account.as_ref())
        .accounts(multisig_accounts::CreateTransaction {
            multisig: multisig_address,
            transaction: transaction_account.pubkey(),
            // For convenience, assume that the party that signs the proposal
            // transaction is a member of the multisig owners, and use it as the
            // proposer.
            proposer: program.payer(),
            rent: sysvar::rent::ID,
        })
        .args(multisig_instruction::CreateTransaction {
            pid: instruction.program_id,
            accs: accounts,
            data: instruction.data,
        })
        .send()
        .expect("Failed to send transaction.");

    ProposeInstructionOutput {
        transaction_address: transaction_account.pubkey().into(),
    }
}

fn propose_binary_transaction(
    program: Program,
    opts: ProposeBinaryTransactionOpts,
) -> ProposeInstructionOutput {
    let mut data = Vec::new();
    File::open(opts.data.as_path())
        .expect("Data file open error")
        .read_to_end(&mut data)
        .expect("Error reading transaction data");

    let instruction: Instruction =
        (&TransactionInstruction::try_from_slice(&data).expect("Transaction parse error")).into();

    propose_instruction(
        program,
        opts.multisig_address.as_pubkey(),
        opts.transaction_account
            .as_ref()
            .map_or_else(|| Arc::new(Keypair::new()), InputKeypair::as_keypair),
        instruction,
    )
}

fn propose_upgrade(program: Program, opts: ProposeUpgradeOpts) -> ProposeInstructionOutput {
    let (program_derived_address, _nonce) =
        get_multisig_program_address(&program, &opts.multisig_address.as_pubkey());

    let upgrade_instruction = bpf_loader_upgradeable::upgrade(
        &opts.program_address.as_pubkey(),
        &opts.buffer_address.as_pubkey(),
        // The upgrade authority is the multisig-derived program address.
        &program_derived_address,
        &opts.spill_address.as_pubkey(),
    );

    propose_instruction(
        program,
        opts.multisig_address.as_pubkey(),
        opts.transaction_account
            .as_ref()
            .map_or_else(|| Arc::new(Keypair::new()), InputKeypair::as_keypair),
        upgrade_instruction,
    )
}

fn propose_change_multisig(
    program: Program,
    opts: ProposeChangeMultisigOpts,
) -> ProposeInstructionOutput {
    // Check that the new settings make sense. This check is shared between a
    // new multisig or altering an existing one.
    CreateMultisigOpts::from(&opts).validate_or_exit();

    let (program_derived_address, _nonce) =
        get_multisig_program_address(&program, &opts.multisig_address.as_pubkey());

    let change_data = multisig_instruction::SetOwnersAndChangeThreshold {
        owners: opts.owners,
        threshold: opts.threshold,
    };
    let change_addrs = multisig_accounts::Auth {
        multisig: opts.multisig_address.as_pubkey(),
        multisig_signer: program_derived_address,
    };

    let override_is_signer = None;
    let change_instruction = Instruction {
        program_id: program.id(),
        data: change_data.data(),
        accounts: change_addrs.to_account_metas(override_is_signer),
    };

    propose_instruction(
        program,
        opts.multisig_address.as_pubkey(),
        opts.transaction_account
            .as_ref()
            .map_or_else(|| Arc::new(Keypair::new()), InputKeypair::as_keypair),
        change_instruction,
    )
}

fn approve(program: Program, opts: ApproveOpts) {
    let transaction: Transaction = program
        .account(opts.transaction_address.as_pubkey())
        .expect("Can not read transaction");
    if let Some(data) = opts.data {
        let mut test = Vec::new();
        File::open(data.as_path())
            .expect("Can not open data file")
            .read_to_end(&mut test)
            .expect("Can not read data file");
        if transaction.instruction.try_to_vec().unwrap() != test {
            panic!("Transaction data does not match");
        }
    }
    program
        .request()
        .accounts(multisig_accounts::Approve {
            multisig: transaction.multisig,
            transaction: opts.transaction_address.as_pubkey(),
            // The owner that signs the multisig proposed transaction, should be
            // the public key that signs the entire approval transaction (which
            // is also the payer).
            owner: program.payer(),
        })
        .args(multisig_instruction::Approve)
        .send()
        .expect("Failed to send transaction.");
}

/// Wrapper type needed to implement `ToAccountMetas`.
struct TransactionAccounts {
    accounts: Vec<multisig::TransactionAccount>,
    program_id: Pubkey,
}

impl anchor_lang::ToAccountMetas for TransactionAccounts {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        assert_eq!(
            is_signer, None,
            "Overriding the signer is not implemented, it is not used by RequestBuilder::accounts.",
        );
        let mut account_metas: Vec<_> = self
            .accounts
            .iter()
            .map(|tx_account| {
                let mut account_meta = AccountMeta::from(tx_account);
                // When the program executes the transaction, it uses the account
                // list with the right signers. But when we build the wrapper
                // instruction that calls the multisig::execute_transaction, the
                // signers of the inner instruction should not be signers of the
                // outer one.
                account_meta.is_signer = false;
                account_meta
            })
            .collect();

        // Aside from the accounts that the transaction references, we also need
        // to include the id of the program it calls as a referenced account in
        // the outer instruction.
        let program_is_signer = false;
        account_metas.push(AccountMeta::new_readonly(
            self.program_id,
            program_is_signer,
        ));

        account_metas
    }
}

fn execute_transaction(program: Program, opts: ExecuteTransactionOpts) {
    let (program_derived_address, _nonce) =
        get_multisig_program_address(&program, &opts.multisig_address.as_pubkey());

    // The wrapped instruction can reference additional accounts, that we need
    // to specify in this `multisig::execute_transaction` instruction as well,
    // otherwise `invoke_signed` can fail in `execute_transaction`.
    let transaction: multisig::Transaction = program
        .account(opts.transaction_address.as_pubkey())
        .expect("Failed to read transaction data from account.");
    let tx_inner_accounts = TransactionAccounts {
        accounts: transaction.instruction.accounts,
        program_id: transaction.instruction.program_id,
    };

    program
        .request()
        .accounts(multisig_accounts::ExecuteTransaction {
            multisig: opts.multisig_address.as_pubkey(),
            multisig_signer: program_derived_address,
            transaction: opts.transaction_address.as_pubkey(),
        })
        .accounts(tx_inner_accounts)
        .args(multisig_instruction::ExecuteTransaction)
        .send()
        .expect("Failed to send transaction.");
}
