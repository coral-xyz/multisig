/// Thin compatibility layer exposing on-chain program to this app
extern crate anchor_client;
extern crate anyhow;
extern crate rand;
extern crate serum_multisig;

use anchor_client::solana_sdk::instruction::AccountMeta;
use anchor_client::solana_sdk::system_program;
use anchor_client::solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction, sysvar,
};
use anchor_client::{Cluster, Program, RequestNamespace};
use anyhow::Result;
use rand::rngs::OsRng;
use serum_multisig::{Transaction, TransactionAccount};

use crate::config::MultisigConfig;
use crate::request_builder::RequestBuilder;

pub struct MultisigGateway<'a> {
    pub client: Program,
    pub cluster: Cluster,
    pub payer: &'a dyn Signer,
    pub config: &'a MultisigConfig,
}

impl<'a> MultisigGateway<'a> {
    pub fn signer(&self, multisig: Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[&multisig.to_bytes()], &self.client.id())
    }

    pub fn delegate_list(&self, multisig: Pubkey, owner: Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[&multisig.to_bytes(), &owner.to_bytes()],
            &self.client.id(),
        )
    }

    pub fn create_multisig(&self, threshold: u64, owners: Vec<Pubkey>) -> Result<(Pubkey, Pubkey)> {
        let multisig_acct = Keypair::generate(&mut OsRng);
        let (signer, bump) = self.signer(multisig_acct.pubkey());
        self.request()
            .instruction(system_instruction::create_account(
                &&self.payer.pubkey(),
                &multisig_acct.pubkey(),
                self.client
                    .rpc()
                    .get_minimum_balance_for_rent_exemption(500)?,
                500,
                &&self.client.id(),
            ))
            .accounts(serum_multisig::accounts::CreateMultisig {
                multisig: multisig_acct.pubkey(),
                signer,
                rent: sysvar::rent::ID,
            })
            .args(serum_multisig::instruction::CreateMultisig {
                owners,
                threshold,
                nonce: bump,
            })
            .signer(&multisig_acct)
            .send(true)?;
        Ok((multisig_acct.pubkey(), signer))
    }

    pub fn create_delegate_list(&self, multisig: Pubkey, delegates: Vec<Pubkey>) -> Result<()> {
        let owner = self.payer.pubkey();
        let (delegate_list, bump) = self.delegate_list(multisig, owner);

        self.request()
            .accounts(serum_multisig::accounts::CreateDelegateList {
                multisig,
                delegate_list,
                owner,
                system_program: system_program::ID,
            })
            .args(serum_multisig::instruction::CreateDelegateList { bump, delegates })
            .send(true)?;

        Ok(())
    }

    pub fn set_delegate_list(
        &self,
        multisig: Pubkey,
        delegate_list: Pubkey,
        delegates: Vec<Pubkey>,
    ) -> Result<()> {
        self.request()
            .accounts(serum_multisig::accounts::SetDelegateList {
                multisig,
                delegate_list,
                authority: self.payer.pubkey(),
            })
            .args(serum_multisig::instruction::SetDelegateList { delegates })
            .send(true)?;

        Ok(())
    }

    pub fn create_transaction(
        &self,
        builder: Option<RequestBuilder>,
        multisig: Pubkey,
        pid: Pubkey,
        accs: Vec<TransactionAccount>,
        data: Vec<u8>,
    ) -> Result<Pubkey> {
        let tx_acct = Keypair::generate(&mut OsRng);
        let mut builder = builder
            .unwrap_or_else(|| self.request())
            .instruction(system_instruction::create_account(
                &&self.payer.pubkey(),
                &tx_acct.pubkey(),
                self.client
                    .rpc()
                    .get_minimum_balance_for_rent_exemption(500)?,
                500,
                &&self.client.id(),
            ))
            .accounts(serum_multisig::accounts::CreateTransaction {
                multisig,
                transaction: tx_acct.pubkey(),
                proposer: self.payer.pubkey(),
                rent: sysvar::rent::ID,
            })
            .args(serum_multisig::instruction::CreateTransaction { pid, accs, data });

        match &self.config.delegation {
            None => (),
            Some(d_config) => {
                let (delegate_list, _) = self.delegate_list(multisig, d_config.owner);

                builder = builder.accounts(AccountMeta::new(delegate_list, false));
            }
        }

        builder.signer(&tx_acct).send(true)?;
        Ok(tx_acct.pubkey())
    }

    pub fn approve(&self, multisig: Pubkey, transaction: Pubkey) -> Result<()> {
        match &self.config.delegation {
            None => {
                self.request()
                    .accounts(serum_multisig::accounts::Approve {
                        multisig,
                        transaction,
                        owner: self.payer.pubkey(),
                    })
                    .args(serum_multisig::instruction::Approve {})
                    .send(true)?;
            }

            Some(d_config) => {
                let (delegate_list, _) = self.delegate_list(multisig, d_config.owner);

                self.request()
                    .accounts(serum_multisig::accounts::DelegateApprove {
                        multisig,
                        transaction,
                        delegate_list,
                        delegate: self.payer.pubkey(),
                    })
                    .args(serum_multisig::instruction::DelegateApprove {})
                    .send(true)?;
            }
        }

        Ok(())
    }

    pub fn execute(&self, multisig: Pubkey, transaction: Pubkey) -> Result<()> {
        let multisig_signer = self.signer(multisig).0;
        let tx: Transaction = self.client.account(transaction)?;
        let account_metas = tx
            .accounts
            .iter()
            .map(|ta| AccountMeta {
                pubkey: ta.pubkey,
                is_signer: if ta.pubkey == multisig_signer {
                    false
                } else {
                    ta.is_signer
                }, // multisig-ui does this
                is_writable: ta.is_writable,
            })
            .collect::<Vec<AccountMeta>>();

        let sig = self
            .request()
            .accounts(serum_multisig::accounts::ExecuteTransaction {
                multisig,
                transaction,
                multisig_signer,
            })
            .args(serum_multisig::instruction::ExecuteTransaction {})
            .accounts(account_metas) // Include the accounts for the instruction to execute
            .accounts(vec![AccountMeta {
                // Also include the program ID that executes the instruction
                pubkey: tx.program_id,
                is_signer: false,
                is_writable: false,
            }])
            .send(true)?;

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
