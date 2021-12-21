use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use clap::Parser;
use paste::paste;

use crate::propose::{bpf, multisig, token};
use crate::service::MultisigService;

pub const MISSING_MULTISIG: &str = "This operation requires a preexisting multisig, but no multisig was specified in the CLI or config file.";

#[derive(Parser)]
pub struct Opts {
    #[clap(short, long, default_value = "~/.config/multisig.toml")]
    pub config: String,

    #[clap(short, long)]
    pub multisig: Option<Pubkey>,

    #[clap(subcommand)]
    pub job: Job,
}

nested_subcommands!(
    Job {
        Admin(MultisigCommand),
        Propose(Proposal),
    }
);

#[derive(Parser)]
pub enum MultisigCommand {
    New(CreateMultisig),
    AddDelegates(Delegates),
    RemoveDelegates(Delegates),
    Approve(Transaction),
    Execute(Transaction),
    Get,
    List,
    GetProposal(Key),
    ListProposals,
    InspectProposal(Key),
}

nested_subcommands! {
    Proposal {
        Multisig(MultisigProposal),
        Bpf(BpfProposal),
        Token(TokenProposal),
    }
}

#[derive(Parser)]
pub enum MultisigProposal {
    Edit(Edit),
}

#[derive(Parser)]
pub enum BpfProposal {
    Upgrade(ProposeUpgrade),
}

#[derive(Parser)]
pub enum TokenProposal {
    Mint(TokenAction),
    Transfer(TokenAction),
}

#[derive(Parser, Debug)]
pub struct CreateMultisig {
    pub threshold: u64,
    #[clap(required = true)]
    pub owners: Vec<Pubkey>,
}

#[derive(Parser, Debug)]
pub struct Delegates {
    pub delegates: Vec<Pubkey>,
}

#[derive(Parser, Debug)]
pub struct Edit {
    #[clap(long)]
    pub threshold: Option<u64>,
    #[clap(long)]
    pub owners: Option<Vec<Pubkey>>,
}

#[derive(Parser)]
pub struct ProposeUpgrade {
    pub program: Pubkey,
    pub buffer: Pubkey,
}

#[derive(Parser)]
pub struct TokenAction {
    #[clap(long, short)]
    pub source: Pubkey,

    #[clap(long, short)]
    pub target: Pubkey,

    #[clap(long, short)]
    pub amount: u64,
}

#[derive(Parser)]
pub struct Transaction {
    pub transaction: Pubkey,
}

#[derive(Parser)]
pub struct Key {
    pub key: Pubkey,
}

// clients that add more proposals should reimplement this function with their own Proposal enum
pub fn run_job(job: Job, service: &MultisigService, multisig: Option<Pubkey>) -> Result<()> {
    match job {
        Job::Admin(cmd) => run_multisig_command(cmd.subcommand, service, multisig),
        Job::Propose(cmd) => {
            let multisig = multisig.expect(MISSING_MULTISIG);
            match cmd.subcommand {
                Proposal::Multisig(cmd) => run_multisig_proposal(cmd.subcommand, service, multisig),
                Proposal::Bpf(cmd) => run_bpf_proposal(cmd.subcommand, service, multisig),
                Proposal::Token(cmd) => run_token_proposal(cmd.subcommand, service, multisig),
            }
        }
    }
}

pub fn run_multisig_command(
    job: MultisigCommand,
    service: &MultisigService,
    multisig: Option<Pubkey>,
) -> Result<()> {
    match job {
        MultisigCommand::New(cmd) => {
            let keys = service.program.create_multisig(cmd.threshold, cmd.owners)?;
            println!("{} {}", keys.0, keys.1);
        }
        MultisigCommand::AddDelegates(cmd) => {
            service.add_delegates(multisig.expect(MISSING_MULTISIG), cmd.delegates)?;
        }
        MultisigCommand::RemoveDelegates(cmd) => {
            service.remove_delegates(multisig.expect(MISSING_MULTISIG), cmd.delegates)?;
        }
        MultisigCommand::Approve(cmd) => service
            .program
            .approve(multisig.expect(MISSING_MULTISIG), cmd.transaction)?,
        MultisigCommand::Execute(cmd) => {
            service
                .program
                .execute(multisig.expect(MISSING_MULTISIG), cmd.transaction)?;
        }
        MultisigCommand::Get => {
            let ms = service.program.get_multisig()?;
            println!("{:#?}", ms);
        }
        MultisigCommand::List => {
            let mss = service.program.list_multisigs()?;
            println!("{:#?}", mss);
        }
        MultisigCommand::GetProposal(cmd) => {
            let tx = service.program.get_transaction(cmd.key)?;
            println!("{:#?}", tx);
        }
        MultisigCommand::ListProposals => {
            let txs = service.program.list_transactions()?;
            println!("{:#?}", txs);
        }
        MultisigCommand::InspectProposal(cmd) => {
            let tx = service.program.get_transaction(cmd.key)?;
            service.inspect_proposal(&tx)?;
        }
    }
    Ok(())
}

pub fn run_multisig_proposal(
    job: MultisigProposal,
    service: &MultisigService,
    multisig: Pubkey,
) -> Result<()> {
    match job {
        MultisigProposal::Edit(cmd) => {
            let key = multisig::propose_set_owners_and_change_threshold(
                &service,
                multisig,
                cmd.threshold,
                cmd.owners,
            )?;
            println!("{}", key);
        }
    }
    Ok(())
}

pub fn run_bpf_proposal(
    job: BpfProposal,
    service: &MultisigService,
    multisig: Pubkey,
) -> Result<()> {
    match job {
        BpfProposal::Upgrade(cmd) => {
            let key = bpf::propose_upgrade(&service, &multisig, &cmd.program, &cmd.buffer)?;
            println!("{}", key);
        }
    }
    Ok(())
}

pub fn run_token_proposal(
    job: TokenProposal,
    service: &MultisigService,
    multisig: Pubkey,
) -> Result<()> {
    match job {
        TokenProposal::Mint(cmd) => {
            let key =
                token::propose_mint_tokens(&service, multisig, cmd.source, cmd.target, cmd.amount)?;
            println!("{}", key);
        }
        TokenProposal::Transfer(cmd) => {
            let key = token::propose_transfer_tokens(
                &service, multisig, cmd.source, cmd.target, cmd.amount,
            )?;
            println!("{}", key);
        }
    }
    Ok(())
}

#[macro_export]
macro_rules! nested_subcommands {
    ($name:ident {
        $($top:ident($bottom:ty)),+$(,)?
    }) => {
        paste! {
            #[derive(Parser)]
            pub enum $name {
                $($top([<$top $bottom>]),)+
            }

            $(
                #[derive(Parser)]
                pub struct [<$top $bottom>] {
                    #[clap(subcommand)]
                    subcommand: $bottom
                }
            )+
        }
    };
}
pub use nested_subcommands;

// todo this would make a more declarative syntax where the nesting is
// represented in a single place instead of multiple structs and enums. it's
// the next step in the evolution of what nested_subcommands does. but it's
// tricky to get it working.
// macro_rules! nested_subcommands2 {
//     ($name:ident {
//         $(> $top:ident{$nested:tt}),*
//         $($top2:ident($bottom:ty)),*
//         $(,)?
//     }) => {
//         paste! {
//             #[derive(Parser)]
//             pub enum $name {
//                 $($top([<$top Middleman>]),)*
//                 $($top2($bottom),)*
//             }

//             $(
//                 #[derive(Parser)]
//                 pub struct [<$top Middleman>] {
//                     #[clap(subcommand)]
//                     subcommand: $top
//                 }

//                 nested_subcommands2! {
//                     $next {
//                         $nested
//                     }
//                 }
//             )*
//         }
//     };
// }
// pub(crate) use nested_subcommands2;
// example usage:
// nested_subcommands2! {
//     MetaJob {
//         > Admin {
//             New(CreateMultisig),
//             AddDelegates(Delegates),
//             RemoveDelegates(Delegates),
//             Approve(Transaction),
//             Execute(Transaction),
//             Get,
//             GetTransaction(Key),
//             InspectProposal(Key),
//         },
//         Propose(Proposal),
//     }
// }
