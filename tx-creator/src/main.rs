use std::{fs::File, io::Write, str::FromStr};

use borsh::BorshSerialize;
use multisig::{TransactionAccount, TransactionInstruction};
use solana_sdk::{bpf_loader_upgradeable::set_upgrade_authority, pubkey::Pubkey};

fn main() -> anyhow::Result<()> {
    let multisig_id = Pubkey::from_str("H88LfRBiJLZ7wYkHGuwkKTaijfQxexq8JvzUndu7fyjL").unwrap();
    let current_authority =
        Pubkey::from_str("3uxzYiAYW9UK7L4DT3Cr256ZStLc2G1vbjSHS5PEF9Bs").unwrap();
    let new_authority = Pubkey::from_str("FgpxPVJy5oCnze7BMjnPzzh73jwEqyMZe2uvsp8VDPoR").unwrap();
    let instruction = set_upgrade_authority(&multisig_id, &current_authority, Some(&new_authority));

    let transaction = TransactionInstruction {
        program_id: solana_sdk::bpf_loader_upgradeable::ID,
        accounts: instruction
            .accounts
            .iter()
            .map(TransactionAccount::from)
            .collect(),
        data: instruction.data,
    };

    File::create("data.bin")?.write_all(&transaction.try_to_vec()?)?;

    Ok(())
}
