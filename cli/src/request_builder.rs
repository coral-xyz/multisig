#![allow(dead_code, unused_variables, unused_imports)]

use anchor_client::anchor_lang;
use anchor_client::anchor_lang::InstructionData;
use anchor_client::anchor_lang::ToAccountMetas;
use anchor_client::solana_client::rpc_client::RpcClient;
use anchor_client::solana_client::rpc_config::RpcSendTransactionConfig;
use anchor_client::solana_sdk::bpf_loader_upgradeable;
use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::commitment_config::CommitmentLevel;
use anchor_client::solana_sdk::instruction::AccountMeta;
use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::rent;
use anchor_client::solana_sdk::signature::Signature;
use anchor_client::solana_sdk::signature::{Keypair, Signer};
use anchor_client::solana_sdk::system_instruction;
use anchor_client::solana_sdk::system_program;
use anchor_client::solana_sdk::sysvar;
use anchor_client::solana_sdk::transaction::Transaction;
use anchor_client::Client;
use anchor_client::ClientError;
use anchor_client::Program;
use anchor_client::RequestNamespace;

/// `RequestBuilder` provides a builder interface to create and send
/// transactions to a cluster.
pub struct RequestBuilder<'a> {
    cluster: String,
    program_id: Pubkey,
    accounts: Vec<AccountMeta>,
    options: CommitmentConfig,
    instructions: Vec<Instruction>,
    payer: &'a dyn Signer,
    // Serialized instruction data for the target RPC.
    instruction_data: Option<Vec<u8>>,
    signers: Vec<&'a dyn Signer>,
    // True if the user is sending a state instruction.
    namespace: RequestNamespace,
}

impl<'a> RequestBuilder<'a> {
    pub fn from(
        program_id: Pubkey,
        cluster: &str,
        payer: &'a dyn Signer,
        options: Option<CommitmentConfig>,
        namespace: RequestNamespace,
    ) -> Self {
        Self {
            program_id,
            payer,
            cluster: cluster.to_string(),
            accounts: Vec::new(),
            options: options.unwrap_or_default(),
            instructions: Vec::new(),
            instruction_data: None,
            signers: Vec::new(),
            namespace,
        }
    }

    pub fn payer(mut self, payer: &'a dyn Signer) -> Self {
        self.payer = payer;
        self
    }

    pub fn cluster(mut self, url: &str) -> Self {
        self.cluster = url.to_string();
        self
    }

    pub fn instruction(mut self, ix: Instruction) -> Self {
        self.instructions.push(ix);
        self
    }

    pub fn program(mut self, program_id: Pubkey) -> Self {
        self.program_id = program_id;
        self
    }

    pub fn accounts(mut self, accounts: impl ToAccountMetas) -> Self {
        let mut metas = accounts.to_account_metas(None);
        self.accounts.append(&mut metas);
        self
    }

    pub fn options(mut self, options: CommitmentConfig) -> Self {
        self.options = options;
        self
    }

    pub fn args(mut self, args: impl InstructionData) -> Self {
        self.instruction_data = Some(args.data());
        self
    }

    /// Invokes the `#[state]`'s `new` constructor.
    pub fn new(mut self, args: impl InstructionData) -> Self {
        assert!(self.namespace == RequestNamespace::State { new: false });
        self.namespace = RequestNamespace::State { new: true };
        self.instruction_data = Some(args.data());
        self
    }

    pub fn signer(mut self, signer: &'a dyn Signer) -> Self {
        self.signers.push(signer);
        self
    }

    pub fn send(self, preflight: bool) -> Result<Signature, ClientError> {
        let accounts = match self.namespace {
            RequestNamespace::State { new } => {
                let mut accounts = match new {
                    false => vec![AccountMeta::new(
                        anchor_lang::__private::state::address(&self.program_id),
                        false,
                    )],
                    true => vec![
                        AccountMeta::new_readonly(self.payer.pubkey(), true),
                        AccountMeta::new(
                            anchor_lang::__private::state::address(&self.program_id),
                            false,
                        ),
                        AccountMeta::new_readonly(
                            Pubkey::find_program_address(&[], &self.program_id).0,
                            false,
                        ),
                        AccountMeta::new_readonly(system_program::ID, false),
                        AccountMeta::new_readonly(self.program_id, false),
                        AccountMeta::new_readonly(sysvar::rent::ID, false), // had to change this for some reason
                    ],
                };
                accounts.extend_from_slice(&self.accounts);
                accounts
            }
            _ => self.accounts,
        };
        let mut instructions = self.instructions;
        if let Some(ix_data) = self.instruction_data {
            instructions.push(Instruction {
                program_id: self.program_id,
                data: ix_data,
                accounts,
            });
        }

        let mut signers = self.signers;
        signers.push(self.payer);

        let rpc_client = RpcClient::new_with_commitment(self.cluster, self.options);

        let tx = {
            let (recent_hash, _fee_calc) = rpc_client.get_recent_blockhash()?;
            Transaction::new_signed_with_payer(
                &instructions,
                Some(&self.payer.pubkey()),
                &signers,
                recent_hash,
            )
        };

        let mut config = RpcSendTransactionConfig::default();
        config.skip_preflight = !preflight;

        // rpc_client
        // .send_transaction_with_config(&tx, config)
        //     .map_err(Into::into);
        let commitment = match config.preflight_commitment {
            Some(c) => c,
            _ => CommitmentLevel::Processed,
        };

        rpc_client
            .send_and_confirm_transaction_with_spinner_and_config(
                &tx,
                CommitmentConfig { commitment },
                config,
            )
            .map_err(Into::into)

        // rpc_client
        //     .send_and_confirm_transaction(&tx)
        //     .map_err(Into::into)
    }

    pub fn instructions(self) -> Result<Vec<Instruction>, ClientError> {
        let accounts = match self.namespace {
            RequestNamespace::State { new } => {
                let mut accounts = match new {
                    false => vec![AccountMeta::new(
                        anchor_lang::__private::state::address(&self.program_id),
                        false,
                    )],
                    true => vec![
                        AccountMeta::new_readonly(self.payer.pubkey(), true),
                        AccountMeta::new(
                            anchor_lang::__private::state::address(&self.program_id),
                            false,
                        ),
                        AccountMeta::new_readonly(
                            Pubkey::find_program_address(&[], &self.program_id).0,
                            false,
                        ),
                        AccountMeta::new_readonly(system_program::ID, false),
                        AccountMeta::new_readonly(self.program_id, false),
                        AccountMeta::new_readonly(sysvar::rent::ID, false), // had to change this for some reason
                    ],
                };
                accounts.extend_from_slice(&self.accounts);
                accounts
            }
            _ => self.accounts,
        };
        let mut instructions = self.instructions;
        if let Some(ix_data) = self.instruction_data {
            instructions.push(Instruction {
                program_id: self.program_id,
                data: ix_data,
                accounts,
            });
        }

        Ok(instructions)
    }

    fn rpc_snd() {}
}
