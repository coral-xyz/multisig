/// Thin compatibility layer exposing on-chain program to this app
extern crate anchor_client;
extern crate anyhow;
extern crate rand;
extern crate serum_multisig;

use std::cmp::max;
use std::convert::TryInto;
use std::rc::Rc;

use anchor_client::solana_sdk::instruction::AccountMeta;
use anchor_client::solana_sdk::system_program;
use anchor_client::solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
};
use anchor_client::{Cluster, Program, RequestBuilder};
use anyhow::Result;
use rand::rngs::OsRng;
use serum_multisig::{Multisig, Transaction, TransactionAccount};

use crate::config::MultisigConfig;

pub struct MultisigGateway<'a> {
    pub client: Program,
    pub cluster: Cluster,
    pub payer: Rc<dyn Signer>,
    pub config: &'a MultisigConfig,
}

impl<'a> MultisigGateway<'a> {
    pub fn get_multisig(&self, multisig: Pubkey) -> Result<Multisig> {
        Ok(self.client.account::<Multisig>(multisig)?)
    }

    pub fn get_transaction(&self, tx: Pubkey) -> Result<Transaction> {
        Ok(self.client.account::<Transaction>(tx)?)
    }

    pub fn list_multisigs(&self) -> Result<Vec<(Pubkey, Multisig)>> {
        Ok(self.client.accounts::<Multisig>(vec![])?)
    }

    pub fn list_transactions(
        &self,
        multisig: Option<Pubkey>,
    ) -> Result<Vec<(Pubkey, Transaction)>> {
        Ok(self
            .client
            .accounts::<Transaction>(vec![])?
            .into_iter()
            .filter(|(_, acct)| match multisig {
                Some(ms) => acct.multisig == ms,
                None => true,
            })
            .collect())
    }

    pub fn signer(&self, multisig: Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[&multisig.to_bytes()], &self.client.id())
    }

    pub fn delegate_list(&self, multisig: Pubkey, owner: Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[&multisig.to_bytes(), &owner.to_bytes()],
            &self.client.id(),
        )
    }

    /// max_owners: sets the space/lamports sufficiently large to handle this many owners
    ///  └─> if unset, defaults to allow owner list to grow by 10
    pub fn create_multisig(
        &self,
        threshold: u64,
        owners: Vec<Pubkey>,
        max_owners: Option<usize>,
    ) -> Result<(Pubkey, Pubkey)> {
        let multisig_acct = Keypair::generate(&mut OsRng);
        let (signer, bump) = self.signer(multisig_acct.pubkey());
        let space = 8
            + std::mem::size_of::<Multisig>()
            + std::mem::size_of::<Pubkey>()
                * max(owners.capacity(), max_owners.unwrap_or(owners.len() + 10));
        self.client
            .request()
            .instruction(system_instruction::create_account(
                &&self.payer.pubkey(),
                &multisig_acct.pubkey(),
                self.client
                    .rpc()
                    .get_minimum_balance_for_rent_exemption(space)?,
                space.try_into().unwrap(),
                &&self.client.id(),
            ))
            .accounts(serum_multisig::accounts::CreateMultisig {
                multisig: multisig_acct.pubkey(),
            })
            .args(serum_multisig::instruction::CreateMultisig {
                owners,
                threshold,
                nonce: bump,
            })
            .signer(&multisig_acct)
            .send()?;
        Ok((multisig_acct.pubkey(), signer))
    }

    pub fn create_delegate_list(&self, multisig: Pubkey, delegates: Vec<Pubkey>) -> Result<()> {
        let owner = self.payer.pubkey();
        let (delegate_list, bump) = self.delegate_list(multisig, owner);

        self.client
            .request()
            .accounts(serum_multisig::accounts::CreateDelegateList {
                multisig,
                delegate_list,
                owner,
                system_program: system_program::ID,
            })
            .args(serum_multisig::instruction::CreateDelegateList { bump, delegates })
            .send()?;

        Ok(())
    }

    pub fn set_delegate_list(
        &self,
        multisig: Pubkey,
        delegate_list: Pubkey,
        delegates: Vec<Pubkey>,
    ) -> Result<()> {
        self.client
            .request()
            .accounts(serum_multisig::accounts::SetDelegateList {
                multisig,
                delegate_list,
                authority: self.payer.pubkey(),
            })
            .args(serum_multisig::instruction::SetDelegateList { delegates })
            .send()?;

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
            .unwrap_or_else(|| self.client.request())
            .instruction(system_instruction::create_account(
                &&self.payer.pubkey(),
                &tx_acct.pubkey(),
                self.client
                    .rpc()
                    .get_minimum_balance_for_rent_exemption(1024)?,
                1024,
                &&self.client.id(),
            ))
            .accounts(serum_multisig::accounts::CreateTransaction {
                multisig,
                transaction: tx_acct.pubkey(),
                proposer: self.payer.pubkey(),
            })
            .args(serum_multisig::instruction::CreateTransaction { pid, accs, data });

        match &self.config.delegation {
            None => (),
            Some(d_config) => {
                let (delegate_list, _) = self.delegate_list(multisig, d_config.owner);

                builder = builder.accounts(AccountMeta::new(delegate_list, false));
            }
        }

        builder.signer(&tx_acct).send()?;
        Ok(tx_acct.pubkey())
    }

    pub fn approve(&self, multisig: Pubkey, transaction: Pubkey) -> Result<()> {
        match &self.config.delegation {
            None => {
                self.client
                    .request()
                    .accounts(serum_multisig::accounts::Approve {
                        multisig,
                        transaction,
                        owner: self.payer.pubkey(),
                    })
                    .args(serum_multisig::instruction::Approve {})
                    .send()?;
            }

            Some(d_config) => {
                let (delegate_list, _) = self.delegate_list(multisig, d_config.owner);

                self.client
                    .request()
                    .accounts(serum_multisig::accounts::DelegateApprove {
                        multisig,
                        transaction,
                        delegate_list,
                        delegate: self.payer.pubkey(),
                    })
                    .args(serum_multisig::instruction::DelegateApprove {})
                    .send()?;
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
            .client
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
            .send()?;

        println!("confirmed: {}", sig);
        Ok(())
    }
}
