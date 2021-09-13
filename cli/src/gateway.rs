/// Thin layer exposing on-chain program to this app

extern crate anchor_client;
extern crate anyhow;
extern crate rand;
extern crate serum_multisig;


use anchor_client::Program;
use anchor_client::solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    sysvar
};
use anyhow::Result;
use rand::rngs::OsRng;

pub struct MultisigGateway {
    pub client: Program,
}

impl MultisigGateway {
    pub fn create_multisig(
        &self,
        threshold: u64,
        owners: Vec<Pubkey>
    ) -> Result<Pubkey> {
        let multisig_acct = Keypair::generate(&mut OsRng);
        let signer_bump = Pubkey::find_program_address(
            &[&multisig_acct.pubkey().to_bytes()],
            &self.client.id(),
        ).1;
        self.client.request()
            .instruction(system_instruction::create_account(
                &&self.client.payer(),
                &multisig_acct.pubkey(),
                self.client.rpc().get_minimum_balance_for_rent_exemption(500)?,
                500,
                &&self.client.id(),
            ))
            .accounts(serum_multisig::accounts::CreateMultisig {
                multisig: multisig_acct.pubkey(),
                rent: sysvar::rent::ID,
            })
            .args(serum_multisig::instruction::CreateMultisig {
                owners,
                threshold,
                nonce: signer_bump,
            })
            .signer(&multisig_acct)
            .send()?;
        Ok(multisig_acct.pubkey())
    }

    pub fn create_transaction(
        &self,
        multisig: Pubkey,
        instruction: Instruction,
    ) -> Result<Pubkey> {
        let tx_acct = Keypair::generate(&mut OsRng);
        self.client.request()
            .instruction(system_instruction::create_account(
                &&self.client.payer(),
                &tx_acct.pubkey(),
                self.client.rpc().get_minimum_balance_for_rent_exemption(500)?,
                500,
                &&self.client.id(),
            ))
            .accounts(serum_multisig::accounts::CreateTransaction {
                multisig,
                transaction: tx_acct.pubkey(),
                proposer: self.client.payer(),
                rent: sysvar::rent::ID,
            })
            .args(serum_multisig::instruction::CreateTransaction {
                pid: instruction.program_id,
                accs: instruction.accounts.iter()
                    .map(|account_meta| account_meta.into())
                    .collect(),
                data: instruction.data,
            })
            .signer(&tx_acct)
            .send()?;
        Ok(tx_acct.pubkey())
    }

    pub fn approve(
        &self,
        multisig: Pubkey,
        transaction: Pubkey,
    ) -> Result<()> {
        self.client.request()
            .accounts(serum_multisig::accounts::Approve {
                multisig,
                transaction,
                owner: self.client.payer(),
            })
            .args(serum_multisig::instruction::Approve {})
            .send()?;
        Ok(())
    }

    pub fn execute(
        &self,
        multisig: Pubkey,
        transaction: Pubkey,
    ) -> Result<()> {
        let multisig_signer = Pubkey::find_program_address(
            &[&multisig.to_bytes()],
            &self.client.id(),
        ).0;
        self.client.request()
            .accounts(serum_multisig::accounts::ExecuteTransaction {
                multisig,
                transaction,
                multisig_signer,
            })
            .args(serum_multisig::instruction::ExecuteTransaction {})
            .send()?;
        Ok(())
    }
}
