/// Thin compatibility layer exposing on-chain program to this app

extern crate anchor_client;
extern crate anyhow;
extern crate rand;
extern crate serum_multisig;


use anchor_client::solana_sdk::instruction::AccountMeta;
use anchor_client::{Cluster, Program, RequestNamespace};
use anchor_client::solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    sysvar
};
use anyhow::Result;
use rand::rngs::OsRng;
use serum_multisig::{Transaction, TransactionAccount};

use crate::request_builder::RequestBuilder;

pub struct MultisigGateway<'a> {
    pub client: Program,
    pub cluster: Cluster,
    pub payer: &'a dyn Signer,
}

impl<'a> MultisigGateway<'a> {
    pub fn signer(&self, multisig: Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[&multisig.to_bytes()],
            &self.client.id(),
        )
    }

    pub fn create_multisig(
        &self,
        threshold: u64,
        owners: Vec<Pubkey>
    ) -> Result<(Pubkey, Pubkey)> {
        let multisig_acct = Keypair::generate(&mut OsRng);
        let signer = self.signer(multisig_acct.pubkey());
        self.request()
            .instruction(system_instruction::create_account(
                &&self.payer.pubkey(),
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
                nonce: signer.1,
            })
            .signer(&multisig_acct)
            .send()?;
        Ok((multisig_acct.pubkey(), signer.0))
    }

    pub fn create_transaction(
        &self,
        multisig: Pubkey,
        pid: Pubkey,
        accs: Vec<TransactionAccount>,
        data: Vec<u8>,
    ) -> Result<Pubkey> {
        let tx_acct = Keypair::generate(&mut OsRng);
        self.request()
            .instruction(system_instruction::create_account(
                &&self.payer.pubkey(),
                &tx_acct.pubkey(),
                self.client.rpc().get_minimum_balance_for_rent_exemption(500)?,
                500,
                &&self.client.id(),
            ))
            .accounts(serum_multisig::accounts::CreateTransaction {
                multisig,
                transaction: tx_acct.pubkey(),
                proposer: self.payer.pubkey(),
                rent: sysvar::rent::ID,
            })
            .args(serum_multisig::instruction::CreateTransaction {
                pid,
                accs,
                data,
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
        self.request()
            .accounts(serum_multisig::accounts::Approve {
                multisig,
                transaction,
                owner: self.payer.pubkey(),
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
        let multisig_signer = self.signer(multisig).0;
        let tx: Transaction = self.client.account(transaction)?;
        let account_metas = tx.accounts.iter()
            .map(|ta| AccountMeta {
                pubkey: ta.pubkey,
                is_signer: if ta.pubkey == multisig_signer { false } else { ta.is_signer }, // multisig-ui does this
                is_writable: ta.is_writable,
            })
            .collect::<Vec<AccountMeta>>();

        let sig = self.request()
            .accounts(serum_multisig::accounts::ExecuteTransaction {
                multisig,
                transaction,
                multisig_signer,
            })
            .args(serum_multisig::instruction::ExecuteTransaction {})
            .accounts(account_metas) // Include the accounts for the instruction to execute
            .accounts(vec![AccountMeta { // Also include the program ID that executes the instruction
                pubkey: tx.program_id,
                is_signer: false,
                is_writable: false,
            }])
            .send()?;

        println!("confirmed: {}", sig);
        Ok(())
    }

    pub fn request(&self) -> RequestBuilder {
        RequestBuilder::from(
            self.client.id(),
            &self.cluster.url(),
            self.payer,
            None,
            RequestNamespace::Global,
        )
    }
}
