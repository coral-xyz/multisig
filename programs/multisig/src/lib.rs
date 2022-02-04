//! An example of a multisig to execute arbitrary Solana transactions.
//!
//! This program can be used to allow a multisig to govern anything a regular
//! Pubkey can govern. One can use the multisig as a BPF program upgrade
//! authority, a mint authority, etc.
//!
//! To use, one must first create a `Multisig` account, specifying two important
//! parameters:
//!
//! 1. Owners - the set of addresses that sign transactions for the multisig.
//! 2. Threshold - the number of signers required to execute a transaction.
//!
//! Once the `Multisig` account is created, one can create a `Transaction`
//! account, specifying the parameters for a normal solana transaction.
//!
//! To sign, owners should invoke the `approve` instruction, and finally,
//! the `execute_transaction`, once enough (i.e. `threhsold`) of the owners have
//! signed.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use std::iter::FromIterator;
use std::convert::TryInto;

declare_id!("FF7U7Vj1PpBkTPau7frwLLrUHrjkxTQLsH7U5K3T3B3j");

#[program]
pub mod mean_multisig {

    use super::*;

    /// Initializes a new multisig account with a set of owners and a threshold.
    pub fn create_multisig(
        ctx: Context<CreateMultisig>,
        owners: Vec<Owner>,
        threshold: u64,
        nonce: u8,
        label: String

    ) -> Result<()> {

        assert_unique_owners(&owners)?;
        require!(threshold > 0 && threshold <= owners.len() as u64, InvalidThreshold);
        require!(owners.len() > 0 && owners.len() <= 10, InvalidOwnersLen);

        let multisig = &mut ctx.accounts.multisig;

        // Convert owners to owners data
        multisig.owners = Vec::from_iter(owners.iter().map(|o| {
            let owner = o.clone();
            OwnerData {
                address: owner.address,
                name: string_to_array(&owner.name)
            }
        })).as_slice().try_into().unwrap();

        let clock = Clock::get()?;

        multisig.version = 2;
        multisig.nonce = nonce;
        multisig.threshold = threshold;
        multisig.owner_set_seqno = 0;
        multisig.label = string_to_array(&label);
        multisig.created_on = clock.unix_timestamp as u64;
        multisig.pending_txs = 0;

        Ok(())
    }

    /// Modify a multisig account data
    pub fn edit_multisig<'info>(
        ctx: Context<'_, '_, '_, 'info, EditMultisig<'info>>,
        owners: Vec<Owner>,
        threshold: u64,
        label: String

    ) -> Result<()> {

        assert_unique_owners(&owners)?;
        require!(threshold > 0 && threshold <= owners.len() as u64, InvalidThreshold);
        require!(owners.len() != 10, InvalidOwnersLen);

        let multisig = &mut ctx.accounts.multisig;
        multisig.label = string_to_array(&label);

        let accounts = &mut Auth { 
            multisig: multisig.clone(), 
            multisig_signer: ctx.accounts.multisig_signer.clone()
        };

        let remaining_accounts = ctx.remaining_accounts;

        set_owners_and_change_threshold(
            Context::new(ctx.program_id, accounts, remaining_accounts),
            owners,
            threshold
        )?;

        Ok(())
    }

    /// Creates a new transaction account, automatically signed by the creator,
    /// which must be one of the owners of the multisig.
    pub fn create_transaction(
        ctx: Context<CreateTransaction>,
        pid: Pubkey,
        operation: u8,
        accs: Vec<TransactionAccount>,
        data: Vec<u8>

    ) -> Result<()> {

        let owner_index = ctx
            .accounts
            .multisig
            .owners
            .iter()
            .position(|a| a.address.eq(&ctx.accounts.proposer.key))
            .ok_or(ErrorCode::InvalidOwner)?;

        let mut signers = Vec::new();
        signers.resize(ctx.accounts.multisig.owners.len(), false);
        signers[owner_index] = true;

        let tx = &mut ctx.accounts.transaction;
        let clock = Clock::get()?;

        tx.program_id = pid;
        tx.accounts = accs;
        tx.data = data;
        tx.signers = signers;
        tx.multisig = *ctx.accounts.multisig.to_account_info().key;
        tx.executed_on = 0;
        tx.owner_set_seqno = ctx.accounts.multisig.owner_set_seqno;
        tx.created_on = clock.unix_timestamp as u64;
        tx.operation = operation;

        let multisig = &mut ctx.accounts.multisig; 
        multisig.pending_txs = multisig.pending_txs
            .checked_add(1)
            .ok_or(ErrorCode::Overflow)?;

        Ok(())
    }

    /// Approves a transaction on behalf of an owner of the multisig.
    pub fn approve(ctx: Context<Approve>) -> Result<()> {

        let owner_index = ctx
            .accounts
            .multisig
            .owners
            .iter()
            .position(|a| a.address.eq(&ctx.accounts.owner.key))
            .ok_or(ErrorCode::InvalidOwner)?;

        ctx.accounts.transaction.signers[owner_index] = true;

        Ok(())
    }

    /// Set owners and threshold at once.
    pub fn set_owners_and_change_threshold<'info>(
        ctx: Context<'_, '_, '_, 'info, Auth<'info>>,
        owners: Vec<Owner>,
        threshold: u64,
    ) -> Result<()> {
        set_owners(
            Context::new(ctx.program_id, ctx.accounts, ctx.remaining_accounts),
            owners,
        )?;
        change_threshold(ctx, threshold)
    }

    /// Sets the owners field on the multisig. The only way this can be invoked
    /// is via a recursive call from execute_transaction -> set_owners.
    pub fn set_owners(ctx: Context<Auth>, owners: Vec<Owner>) -> Result<()> {

        assert_unique_owners(&owners)?;
        require!(owners.len() > 0 && owners.len() <= 10, InvalidOwnersLen);

        let multisig = &mut ctx.accounts.multisig;

        if (owners.len() as u64) < multisig.threshold {
            multisig.threshold = owners.len() as u64;
        }

        multisig.owners = Vec::from_iter(owners.iter().map(|o| {
            let owner = o.clone();
            OwnerData {
                address: owner.address,
                name: string_to_array(&owner.name)
            }
        })).as_slice().try_into().unwrap();

        multisig.owner_set_seqno = multisig.owner_set_seqno
            .checked_add(1)
            .ok_or(ErrorCode::Overflow)?;

        Ok(())
    }

    /// Changes the execution threshold of the multisig. The only way this can be
    /// invoked is via a recursive call from execute_transaction ->
    /// change_threshold.
    pub fn change_threshold(ctx: Context<Auth>, threshold: u64) -> Result<()> {

        require!(threshold > 0, InvalidThreshold);

        if threshold > ctx.accounts.multisig.owners.len() as u64 {
            return Err(ErrorCode::InvalidThreshold.into());
        }

        let multisig = &mut ctx.accounts.multisig;
        multisig.threshold = threshold;

        Ok(())
    }

    /// Executes the given transaction if threshold owners have signed it.
    pub fn execute_transaction(ctx: Context<ExecuteTransaction>) -> Result<()> {

        // Has this been executed already?
        if ctx.accounts.transaction.executed_on > 0 {
            return Err(ErrorCode::AlreadyExecuted.into());
        }

        // Do we have enough signers.
        let sig_count = ctx
            .accounts
            .transaction
            .signers
            .iter()
            .filter(|&did_sign| *did_sign)
            .count() as u64;

        if sig_count < ctx.accounts.multisig.threshold {
            return Err(ErrorCode::NotEnoughSigners.into());
        }

        // Execute the transaction signed by the multisig.
        let mut ix: Instruction = (&*ctx.accounts.transaction).into();
        ix.accounts = ix
            .accounts
            .iter()
            .map(|acc| {
                let mut acc = acc.clone();
                if &acc.pubkey == ctx.accounts.multisig_signer.key {
                    acc.is_signer = true;
                }
                acc
            })
            .collect();

        let seeds = &[
            ctx.accounts.multisig.to_account_info().key.as_ref(),
            &[ctx.accounts.multisig.nonce],
        ];
        
        let signer = &[&seeds[..]];
        let accounts = ctx.remaining_accounts;
        solana_program::program::invoke_signed(&ix, accounts, signer)?;
        // Burn the transaction to ensure one time use.
        ctx.accounts.transaction.executed_on = Clock::get()?.unix_timestamp as u64;

        if ctx.accounts.multisig.pending_txs > 0 
        {
            let multisig = &mut ctx.accounts.multisig; 
            multisig.pending_txs = multisig.pending_txs
                .checked_sub(1)
                .ok_or(ErrorCode::Overflow)?;
        }

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateMultisig<'info> {
    proposer: Signer<'info>,
    #[account(init, payer = proposer, space = 8 + 640 + 1 + 1 + 32 + 4 + 8 + 8 + 8)] // 720
    multisig: Account<'info, Multisig>,
    system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(nonce: u8)]
pub struct EditMultisig<'info> {
    #[account(
        mut,
        constraint = multisig.nonce == nonce @ ErrorCode::InvalidMultisigNonce,
        constraint = multisig.version == 2 @ ErrorCode::InvalidMultisigVersion
    )]
    multisig: Account<'info, Multisig>,
    #[account(
        seeds = [multisig.to_account_info().key.as_ref()],
        bump = multisig.nonce,
    )]
    multisig_signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct CreateTransaction<'info> {
    #[account(mut)]
    multisig: Account<'info, Multisig>,
    #[account(zero)]
    transaction: Account<'info, Transaction>,
    // One of the owners. Checked in the handler.
    #[account()]
    proposer: Signer<'info>,
}

#[derive(Accounts)]
pub struct Approve<'info> {
    #[account(constraint = multisig.owner_set_seqno == transaction.owner_set_seqno)]
    multisig: Account<'info, Multisig>,
    #[account(mut, has_one = multisig)]
    transaction: Account<'info, Transaction>,
    // One of the multisig owners. Checked in the handler.
    #[account()]
    owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct Auth<'info> {
    #[account(mut)]
    multisig: Account<'info, Multisig>,
    #[account(
        seeds = [multisig.to_account_info().key.as_ref()],
        bump = multisig.nonce,
    )]
    multisig_signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct ExecuteTransaction<'info> {
    #[account(mut, constraint = multisig.owner_set_seqno == transaction.owner_set_seqno)]
    multisig: Account<'info, Multisig>,
    #[account(
        seeds = [multisig.to_account_info().key.as_ref()],
        bump = multisig.nonce,
    )]
    multisig_signer: AccountInfo<'info>,
    #[account(mut, has_one = multisig)]
    transaction: Account<'info, Transaction>,
}

#[account]
pub struct MultisigOld {
    pub owners: Vec<Pubkey>,
    pub threshold: u64,
    pub nonce: u8,
    pub owner_set_seqno: u32,
    pub label: String,
    pub created_on: u64,
    pub pending_txs: u64
}

#[account]
pub struct Multisig {
    /// multisig account owners
    pub owners: [OwnerData; 10],
    /// multisig account version
    pub version: u8,
    /// multisig nonce
    pub nonce: u8,
    /// multisig label (name or description)
    pub label: [u8; 32],
    /// multisig owner set secuency number
    pub owner_set_seqno: u32,
    /// multisig required signers threshold
    pub threshold: u64,
    /// amount of transaction pending for approval in the multisig
    pub pending_txs: u64,  
    /// created time in seconds
    pub created_on: u64,
}

#[account]
pub struct Transaction {
    /// The multisig account this transaction belongs to.
    pub multisig: Pubkey,
    /// Target program to execute against.
    pub program_id: Pubkey,
    /// Accounts requried for the transaction.
    pub accounts: Vec<TransactionAccount>,
    /// Instruction data for the transaction.
    pub data: Vec<u8>,
    /// signers[index] is true if multisig.owners[index] signed the transaction.
    pub signers: Vec<bool>,
    /// Owner set sequence number.
    pub owner_set_seqno: u32,
    /// Created blocktime 
    created_on: u64,
    /// Executed blocktime
    executed_on: u64,
    /// Operation number
    operation: u8,
}

/// Owner parameter passed on create and edit multisig
#[account]
pub struct Owner {
    pub address: Pubkey,
    pub name: String
}

/// The owner data saved in the multisig account data
#[account]
#[derive(Copy)]
pub struct OwnerData {
    pub address: Pubkey,
    pub name: [u8; 32]
}

/// To support fixed size arrays we need to implement
/// the Default trait for owner data
impl Default for OwnerData {
    fn default() -> Self {
        Self {
            address: Pubkey::default(),
            name: [0u8; 32]
        }
    }
}

impl From<&Transaction> for Instruction {
    fn from(tx: &Transaction) -> Instruction {
        Instruction {
            program_id: tx.program_id,
            accounts: tx.accounts.iter().map(Into::into).collect(),
            data: tx.data.clone(),
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TransactionAccount {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl From<&TransactionAccount> for AccountMeta {
    fn from(account: &TransactionAccount) -> AccountMeta {
        match account.is_writable {
            false => AccountMeta::new_readonly(account.pubkey, account.is_signer),
            true => AccountMeta::new(account.pubkey, account.is_signer),
        }
    }
}

impl From<&AccountMeta> for TransactionAccount {
    fn from(account_meta: &AccountMeta) -> TransactionAccount {
        TransactionAccount {
            pubkey: account_meta.pubkey,
            is_signer: account_meta.is_signer,
            is_writable: account_meta.is_writable,
        }
    }
}

fn assert_unique_owners(owners: &[Owner]) -> Result<()> {
    for (i, owner) in owners.iter().enumerate() {
        require!(
            !owners.iter().skip(i + 1).any(|item| item.address == owner.address),
            UniqueOwners
        )
    }
    Ok(())
}

fn string_to_array<'info>(string: &String) -> [u8; 32] {
    let mut string_data = [b' '; 32];
    string_data[..32].copy_from_slice(&string.as_bytes());    
    string_data
}

#[error]
pub enum ErrorCode {
    /// 6000: The given owner is not part of this multisig.
    #[msg("The given owner is not part of this multisig.")]
    InvalidOwner,
    /// 6001: Owners length must be non zero.
    #[msg("Owners length must be non zero.")]
    InvalidOwnersLen,
    /// 6002: Not enough owners signed this transaction.
    #[msg("Not enough owners signed this transaction.")]
    NotEnoughSigners,
    /// 6003: Cannot delete a transaction that has been signed by an owner.
    #[msg("Cannot delete a transaction that has been signed by an owner.")]
    TransactionAlreadySigned,
    /// 6004: Operation overflow.
    #[msg("Operation overflow")]
    Overflow,
    /// 6005: Cannot delete a transaction the owner did not create.
    #[msg("Cannot delete a transaction the owner did not create.")]
    UnableToDelete,
    /// 6006: The given transaction has already been executed.
    #[msg("The given transaction has already been executed.")]
    AlreadyExecuted,
    /// 6007: Threshold must be less than or equal to the number of owners.
    #[msg("Threshold must be less than or equal to the number of owners.")]
    InvalidThreshold,
    /// 6008: Owners must be unique.
    #[msg("Owners must be unique")]
    UniqueOwners,
    /// 6009: Owner name must have less than 32 bytes.
    #[msg("Owner name must have less than 32 bytes")]
    OwnerNameTooLong,
    /// 6010: Multisig nonce is not valid.
    #[msg("Multisig nonce is not valid")]
    InvalidMultisigNonce,
    /// 6011: Multisig version is not valid.
    #[msg("Multisig version is not valid")]
    InvalidMultisigVersion,
}
