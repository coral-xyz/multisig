use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;

use crate::{instruction_data::dynamic, service::MultisigService};

pub fn propose_set_owners_and_change_threshold(
    service: &MultisigService,
    multisig: Pubkey,
    threshold: Option<u64>,
    owners: Option<Vec<Pubkey>>,
) -> Result<Pubkey> {
    let args = match (threshold, owners) {
        (Some(threshold), Some(owners)) => {
            dynamic(serum_multisig::instruction::SetOwnersAndChangeThreshold { owners, threshold })
        }
        (Some(threshold), None) => {
            dynamic(serum_multisig::instruction::ChangeThreshold { threshold })
        }
        (None, Some(owners)) => dynamic(serum_multisig::instruction::SetOwners { owners }),
        (None, None) => panic!("At least one change is required"),
    };
    let multisig_signer = service.program.signer(multisig).0;
    service.propose_anchor_instruction(
        None,
        multisig,
        service.program.client.id(),
        serum_multisig::accounts::Auth {
            multisig,
            multisig_signer,
        },
        args,
    )
}
