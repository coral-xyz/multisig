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
use std::ops::Deref;

pub mod account_replacement_placeholder {
    anchor_lang::declare_id!("NewPubkey1111111111111111111111111111111111");
}

pub const MAX_FEE_LAMPORTS: u64 = 10_000_000_000;

#[cfg(feature = "devnet")]
declare_id!("MMSdTDhtwBs2w4MxGCbqfWLgerMQNbXazizghoh7uMJ");
#[cfg(not(feature = "devnet"))]
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

        let (_expected_signer_address, expected_bump) =
            Pubkey::find_program_address(
                &[ctx.accounts.multisig.key().to_bytes().as_ref()],
                ctx.program_id
            );

        if expected_bump != nonce {
            return Err(ErrorCode::InvalidMultisigNonce.into());
        }

        assert_unique_owners(&owners)?;
        require!(threshold > 0 && threshold <= owners.len() as u64, ErrorCode::InvalidThreshold);
        require!(owners.len() > 0 && owners.len() <= 10, ErrorCode::InvalidOwnersLen);

        let multisig = &mut ctx.accounts.multisig;

        // Convert owners to owners data
        let mut multisig_owners = [OwnerData::default(); 10];

        for i in 0..owners.len() {
            let owner = owners.get(i).unwrap().clone();
            multisig_owners[i] = OwnerData {
                address: owner.address,
                name: string_to_array_32(&owner.name)
            };
        }

        multisig.owners = multisig_owners;

        let clock = Clock::get()?;

        multisig.version = 2;
        multisig.nonce = nonce;
        multisig.threshold = threshold;
        multisig.owner_set_seqno = 0;
        multisig.label = string_to_array_32(&label);
        multisig.created_on = clock.unix_timestamp as u64;
        multisig.pending_txs = 0;

        // Fee
        let pay_fee_ix = solana_program::system_instruction::transfer(
            ctx.accounts.proposer.key, 
            ctx.accounts.ops_account.key,
            ctx.accounts.settings.create_multisig_fee
        );

        solana_program::program::invoke(
            &pay_fee_ix,
            &[
                ctx.accounts.proposer.to_account_info(), 
                ctx.accounts.ops_account.to_account_info(), 
                ctx.accounts.system_program.to_account_info()
            ]
        )?;

        Ok(())
    }

    /// Modify a multisig account data
    pub fn edit_multisig<'info>(
        ctx: Context<EditMultisig>,
        owners: Vec<Owner>,
        threshold: u64,
        label: String

    ) -> Result<()> {

        assert_unique_owners(&owners)?;
        require!(threshold > 0 && threshold <= owners.len() as u64, ErrorCode::InvalidThreshold);
        require!(owners.len() > 0 && owners.len() <= 10, ErrorCode::InvalidOwnersLen);

        let multisig = &mut ctx.accounts.multisig;
        let mut multisig_owners = [OwnerData::default(); 10];
        multisig.threshold = threshold;
        multisig.label = string_to_array_32(&label);

        for i in 0..owners.len() {
            let owner = owners.get(i).unwrap();
            multisig_owners[i] = OwnerData {
                address: owner.address,
                name: string_to_array_32(&owner.name)
            };
        }

        multisig.owners = multisig_owners;        
        multisig.pending_txs = 0;
        multisig.owner_set_seqno = multisig.owner_set_seqno
            .checked_add(1)
            .ok_or(ErrorCode::Overflow)?;

        Ok(())
    }

    /// Creates a new transaction account, automatically signed by the creator,
    /// which must be one of the owners of the multisig.
    pub fn create_transaction(
        ctx: Context<CreateTransaction>,
        pid: Pubkey,
        accs: Vec<TransactionAccount>,
        data: Vec<u8>,
        operation: u8,
        title: String,
        description: String,
        expiration_date: u64,
        _pda_timestamp: u64, // deprecated
        _pda_bump: u8 // deprecated

    ) -> Result<()> {

        let owner_index = ctx
            .accounts
            .multisig
            .owners
            .iter()
            .position(|a| a.address.eq(&ctx.accounts.proposer.key()))
            .ok_or(ErrorCode::InvalidOwner)?;

        let mut signers = Vec::<u8>::new();
        signers.resize(ctx.accounts.multisig.owners.len(), 0u8);
        signers[owner_index] = 1u8;

        let tx = &mut ctx.accounts.transaction;
        let clock = Clock::get()?;
        // Save transaction data
        tx.program_id = pid;
        tx.accounts = accs;
        tx.data = data;
        tx.signers = signers;
        tx.multisig = ctx.accounts.multisig.key();
        tx.executed_on = 0;
        tx.owner_set_seqno = ctx.accounts.multisig.owner_set_seqno;
        tx.created_on = clock.unix_timestamp as u64;
        tx.operation = operation;
        // tx.keypairs = keypairs; // deprecated
        tx.proposer = ctx.accounts.proposer.key();

        let tx_detail = &mut ctx.accounts.transaction_detail;
        // Save transaction detail
        tx_detail.title = string_to_array_64(&title);
        tx_detail.description = string_to_array_512(&description);
        tx_detail.expiration_date = expiration_date;

        // Update multisig pending transactions 
        let multisig = &mut ctx.accounts.multisig; 
        multisig.pending_txs = multisig.pending_txs
            .checked_add(1)
            .ok_or(ErrorCode::Overflow)?;

        // Fee
        let pay_fee_ix = solana_program::system_instruction::transfer(
            ctx.accounts.proposer.key, 
            ctx.accounts.ops_account.key,
            ctx.accounts.settings.create_transaction_fee
        );

        solana_program::program::invoke(
            &pay_fee_ix,
            &[
                ctx.accounts.proposer.to_account_info(), 
                ctx.accounts.ops_account.to_account_info(), 
                ctx.accounts.system_program.to_account_info()
            ]
        )?;

        Ok(())
    }

    /// Cancel a previously voided Tx
    pub fn cancel_transaction(ctx: Context<CancelTransaction>) -> Result<()> {

        let multisig = &mut ctx.accounts.multisig;
        // Update the multisig pending Txs
        if multisig.pending_txs > 0 {
            multisig.pending_txs = multisig.pending_txs
                .checked_sub(1)
                .ok_or(ErrorCode::Overflow)?;
        }

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

        // Transaction has expired already?
        let now = Clock::get()?.unix_timestamp as u64;

        if ctx.accounts.transaction_detail.expiration_date > 0 && 
           ctx.accounts.transaction_detail.expiration_date < now 
        {
            return Err(ErrorCode::AlreadyExpired.into());
        }

        ctx.accounts.transaction.signers[owner_index] = 1u8;

        Ok(())
    }

    /// Rejects a transaction on behalf of an owner of the multisig.
    pub fn reject(ctx: Context<Reject>) -> Result<()> {

        let owner_index = ctx
            .accounts
            .multisig
            .owners
            .iter()
            .position(|a| a.address.eq(&ctx.accounts.owner.key))
            .ok_or(ErrorCode::InvalidOwner)?;

        // Transaction has expired already?
        let now = Clock::get()?.unix_timestamp as u64;

        if ctx.accounts.transaction_detail.expiration_date > 0 && 
           ctx.accounts.transaction_detail.expiration_date < now 
        {
            return Err(ErrorCode::AlreadyExpired.into());
        }

        ctx.accounts.transaction.signers[owner_index] = 2u8;

        Ok(())
    }

    /// Executes the given transaction if threshold owners have signed it.
    pub fn execute_transaction(ctx: Context<ExecuteTransaction>) -> Result<()> {

        // Has this been executed already?
        if ctx.accounts.transaction.executed_on > 0 {
            return Err(ErrorCode::AlreadyExecuted.into());
        }

        // Transaction has expired already?
        let now = Clock::get()?.unix_timestamp as u64;

        if ctx.accounts.transaction_detail.expiration_date > 0 && 
           ctx.accounts.transaction_detail.expiration_date < now 
        {
            return Err(ErrorCode::AlreadyExpired.into());
        }

        // Do we have enough signers.
        let sig_count = ctx
            .accounts
            .transaction
            .signers
            .iter()
            .filter(|&did_sign| *did_sign == 1)
            .count() as u64;

        if sig_count < ctx.accounts.multisig.threshold {
            return Err(ErrorCode::NotEnoughSigners.into());
        }

        // Execute the transaction signed by the multisig.
        let mut ix: Instruction = (*ctx.accounts.transaction).deref().into();
        ix.accounts = ix
            .accounts
            .iter()
            .map(|acc| {
                let mut acc = acc.clone();
                if &acc.pubkey == ctx.accounts.multisig_signer.to_account_info().key {
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
        ctx.accounts.multisig.reload()?;
        // Burn the transaction to ensure one time use.
        ctx.accounts.transaction.executed_on = Clock::get()?.unix_timestamp as u64;

        if ctx.accounts.multisig.pending_txs > 0 {
            ctx.accounts.multisig.pending_txs = ctx.accounts.multisig.pending_txs
                .checked_sub(1)
                .ok_or(ErrorCode::Overflow)?;
        }

        Ok(())
    }

    /// Executes the given transaction if threshold owners have signed it.
    pub fn execute_transaction_with_replacements(
        ctx: Context<ExecuteTransaction>,
        replacement_accounts: Vec<Pubkey>
    ) -> Result<()> {

        // Has this been executed already?
        if ctx.accounts.transaction.executed_on > 0 {
            return Err(ErrorCode::AlreadyExecuted.into());
        }

        // Transaction has expired already?
        let now = Clock::get()?.unix_timestamp as u64;

        if ctx.accounts.transaction_detail.expiration_date > 0 && 
        ctx.accounts.transaction_detail.expiration_date < now 
        {
            return Err(ErrorCode::AlreadyExpired.into());
        }

        // Do we have enough signers.
        let sig_count = ctx
            .accounts
            .transaction
            .signers
            .iter()
            .filter(|&did_sign| *did_sign == 1)
            .count() as u64;

        if sig_count < ctx.accounts.multisig.threshold {
            return Err(ErrorCode::NotEnoughSigners.into());
        }

        // Execute the transaction signed by the multisig.
        let mut ix: Instruction = (*ctx.accounts.transaction).deref().into();
        let mut last_replacement_account_index_used = 0;
        let mut mapped_accounts = vec!();
        
        for acc in ix.accounts.iter() {
            let mut acc = acc.clone();
            if &acc.pubkey == ctx.accounts.multisig_signer.to_account_info().key {
                acc.is_signer = true;
            } else if acc.pubkey == account_replacement_placeholder::ID {
                if last_replacement_account_index_used == replacement_accounts.len() {
                    return Err(ErrorCode::NotEnoughReplacementAccounts.into());
                }
                let replacement_account = replacement_accounts
                    .get(last_replacement_account_index_used)
                    .unwrap();
                acc.pubkey = *replacement_account;
                acc.is_signer = true;
                last_replacement_account_index_used = last_replacement_account_index_used + 1;
            }
            mapped_accounts.push(acc);
        }
        ix.accounts = mapped_accounts;

        let seeds = &[
            ctx.accounts.multisig.to_account_info().key.as_ref(),            
            &[ctx.accounts.multisig.nonce],
        ];

        let signer = &[&seeds[..]];
        let accounts = ctx.remaining_accounts;
        solana_program::program::invoke_signed(&ix, accounts, signer)?;
        ctx.accounts.multisig.reload()?;
        // Burn the transaction to ensure one time use.
        ctx.accounts.transaction.executed_on = Clock::get()?.unix_timestamp as u64;

        if ctx.accounts.multisig.pending_txs > 0 {
            ctx.accounts.multisig.pending_txs = ctx.accounts.multisig.pending_txs
                .checked_sub(1)
                .ok_or(ErrorCode::Overflow)?;
        }

        Ok(())
    }

    pub fn init_settings(ctx: Context<InitSettings>) -> Result<()> {

        ctx.accounts.settings.version = 1u8;
        ctx.accounts.settings.bump = ctx.bumps["settings"];
        ctx.accounts.settings.authority = ctx.accounts.authority.key();
        ctx.accounts.settings.ops_account = "3TD6SWY9M1mLY2kZWJNavPLhwXvcRsWdnZLRaMzERJBw".parse().unwrap();
        ctx.accounts.settings.create_multisig_fee = 20_000_000;
        ctx.accounts.settings.create_transaction_fee = 20_000_000;

        Ok(())
    }

    pub fn update_settings(
        ctx: Context<UpdateSettings>, 
        authority: Pubkey,
        ops_account: Pubkey,
        create_multisig_fee: u64,
        create_transaction_fee: u64,

    ) -> Result<()> {

        if create_multisig_fee > MAX_FEE_LAMPORTS || 
            create_transaction_fee > MAX_FEE_LAMPORTS {
                return Err(ErrorCode::FeeExceedsMaximumAllowed.into());
        }

        ctx.accounts.settings.authority = authority;
        ctx.accounts.settings.ops_account = ops_account;
        ctx.accounts.settings.create_multisig_fee = create_multisig_fee;
        ctx.accounts.settings.create_transaction_fee = create_transaction_fee;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateMultisig<'info> {
    #[account(mut)]
    proposer: Signer<'info>,
    #[account(
        init,
        payer = proposer, 
        space = 8 + 640 + 1 + 1 + 32 + 4 + 8 + 8 + 8, // 710
    )]
    multisig: Box<Account<'info, MultisigV2>>,
    #[account(
        mut, 
        address = settings.ops_account
    )]
    ops_account: SystemAccount<'info>,
    #[account(
        seeds = [b"settings"],
        bump = settings.bump
    )]
    settings: Box<Account<'info, Settings>>,
    system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct EditMultisig<'info> {
    #[account(mut)]
    multisig: Box<Account<'info, MultisigV2>>,
    #[account(
        seeds = [multisig.key().as_ref()],
        bump = multisig.nonce,
    )]
    multisig_signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct CreateTransaction<'info> {
    #[account(mut)]
    multisig: Box<Account<'info, MultisigV2>>,
    #[account(zero, signer)]
    transaction: Box<Account<'info, Transaction>>,
    #[account(
        init,
        payer = proposer,
        seeds = [multisig.key().as_ref(), transaction.key().as_ref()],
        bump,
        space = 8 + 584 // discriminator + account size
    )]
    transaction_detail: Box<Account<'info, TransactionDetail>>,
    // One of the owners. Checked in the handler.
    #[account(mut)]
    proposer: Signer<'info>,
    #[account(
        mut, 
        address = settings.ops_account
    )]
    ops_account: SystemAccount<'info>,
    #[account(
        seeds = [b"settings"],
        bump = settings.bump
    )]
    settings: Box<Account<'info, Settings>>,
    system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct CancelTransaction<'info> {
    #[account(
        mut,
        constraint = multisig.key() == transaction.multisig @ ErrorCode::InvalidMultisig
    )]
    multisig: Box<Account<'info, MultisigV2>>,
    #[account(
        mut,
        close = proposer
    )]
    transaction: Box<Account<'info, Transaction>>,
    #[account(
        mut,
        close = proposer,
        seeds = [multisig.key().as_ref(), transaction.key().as_ref()],
        bump
    )]
    transaction_detail: Box<Account<'info, TransactionDetail>>,
    #[account(
        mut,
        constraint = proposer.key() == transaction.proposer @ ErrorCode::InvalidOwner
    )]
    proposer: Signer<'info>,
    system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct Approve<'info> {
    #[account(
        mut, 
        constraint = multisig.owner_set_seqno == transaction.owner_set_seqno @ ErrorCode::InvalidOwnerSetSeqNumber
    )]
    multisig: Box<Account<'info, MultisigV2>>,
    #[account(
        mut, 
        has_one = multisig,
        constraint = transaction.executed_on == 0 @ ErrorCode::AlreadyExecuted
    )]
    transaction: Box<Account<'info, Transaction>>,
    #[account(
        init_if_needed,
        payer = owner,
        seeds = [multisig.key().as_ref(), transaction.key().as_ref()],
        bump,
        space = 8 + 584 // discriminator + account size
    )]
    transaction_detail: Box<Account<'info, TransactionDetail>>,
    // One of the multisig owners. Checked in the handler.
    #[account(mut)]
    owner: Signer<'info>,
    system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct Reject<'info> {
    #[account(
        mut, 
        constraint = multisig.owner_set_seqno == transaction.owner_set_seqno @ ErrorCode::InvalidOwnerSetSeqNumber
    )]
    multisig: Box<Account<'info, MultisigV2>>,
    #[account(
        mut, 
        has_one = multisig,
        constraint = transaction.executed_on == 0 @ ErrorCode::AlreadyExecuted
    )]
    transaction: Box<Account<'info, Transaction>>,
    #[account(
        init_if_needed,
        payer = owner,
        seeds = [multisig.key().as_ref(), transaction.key().as_ref()],
        bump,
        space = 8 + 584 // discriminator + account size
    )]
    transaction_detail: Box<Account<'info, TransactionDetail>>,
    // One of the multisig owners. Checked in the handler.
    #[account(mut)]
    owner: Signer<'info>,
    system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct ExecuteTransaction<'info> {
    #[account(
        mut,
        constraint = multisig.owner_set_seqno == transaction.owner_set_seqno @ ErrorCode::InvalidOwnerSetSeqNumber
    )]
    multisig: Box<Account<'info, MultisigV2>>,
    /// CHECK: multisig_signer is a PDA program signer. Data is never read or written to
    #[account(
        seeds = [multisig.key().as_ref()],
        bump = multisig.nonce,
    )]
    multisig_signer: UncheckedAccount<'info>,
    #[account(mut, has_one = multisig)]
    transaction: Box<Account<'info, Transaction>>,
    #[account(
        init_if_needed,
        payer = payer,
        seeds = [multisig.key().as_ref(), transaction.key().as_ref()],
        bump,
        space = 8 + 584 // discriminator + account size
    )]
    transaction_detail: Box<Account<'info, TransactionDetail>>,
    #[account(mut)]
    payer: Signer<'info>,
    system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct InitSettings<'info> {
    #[account(mut)]
    payer: Signer<'info>,
    #[account()]
    authority: Signer<'info>,
    #[account(
        init,
        payer=payer,
        seeds = [b"settings"],
        bump,
        space = 200
    )]
    settings: Account<'info, Settings>,
    #[account(constraint = program.programdata_address()? == Some(program_data.key()))]
    program: Program<'info, crate::program::MeanMultisig>,
    #[account(
        constraint = program_data.upgrade_authority_address == Some(authority.key()) @ ErrorCode::InvalidSettingsAuthority
    )]
    program_data: Account<'info, ProgramData>,
    system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct UpdateSettings<'info> {
    #[account()]
    authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"settings"],
        bump = settings.bump
    )]
    settings: Account<'info, Settings>,
    #[account(constraint = program.programdata_address()? == Some(program_data.key()))]
    program: Program<'info, crate::program::MeanMultisig>,
    #[account(
        constraint = authority.key() == settings.authority || 
            program_data.upgrade_authority_address == Some(authority.key())
            @ ErrorCode::InvalidSettingsAuthority
    )]
    program_data: Account<'info, ProgramData>,
}

// #[account]
// pub struct Multisig {
//     pub owners: Vec<Pubkey>,
//     pub threshold: u64,
//     pub nonce: u8,
//     pub owner_set_seqno: u32,
//     pub label: String,
//     pub created_on: u64,
//     pub pending_txs: u64
// }

#[account]
pub struct MultisigV2 {
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
    pub signers: Vec<u8>,
    /// Owner set sequence number.
    pub owner_set_seqno: u32,
    /// Created blocktime 
    pub created_on: u64,
    /// Executed blocktime
    pub executed_on: u64,
    /// Operation number
    pub operation: u8,
    /// [deprecated] Signatures required for the transaction
    // #[deprecated]
    pub keypairs: Vec<[u8; 64]>,
    /// The proposer of the transaction
    pub proposer: Pubkey,
    #[deprecated]
    /// The timestamp used as part of the seed of the PDA account
    pub pda_timestamp: u64,
    #[deprecated]
    /// The bump used to derive the PDA account
    pub pda_bump: u8
}

#[account]
pub struct TransactionDetail {
    /// A short title to identify the transaction
    pub title: [u8; 64],
    /// A long description with more details about the transaction
    pub description: [u8; 512],
    /// Expiration date (timestamp)
    pub expiration_date: u64
}

#[account]
pub struct Settings {
    /// Account version
    pub version: u8,
    /// PDA bump
    pub bump: u8,
    /// Account authority
    pub authority: Pubkey,
    /// Fees account
    pub ops_account: Pubkey,
    /// Fee amount in lamports
    pub create_multisig_fee: u64,
    /// Fee amount in lamports
    pub create_transaction_fee: u64,
}

/// Owner parameter passed on create and edit multisig
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Owner {
    pub address: Pubkey,
    pub name: String
}

/// The owner data saved in the multisig account data
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
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
            !owners.iter().skip(i + 1).any(|item| item.address.eq(&owner.address)),
            ErrorCode::UniqueOwners
        )
    }
    Ok(())
}

fn string_to_array_32<'info>(string: &String) -> [u8; 32] {
    let mut string_data = [0u8; 32];
    string_data[..string.len()].copy_from_slice(&string.as_bytes());    
    string_data
}

fn string_to_array_64<'info>(string: &String) -> [u8; 64] {
    let mut string_data = [0u8; 64];
    string_data[..string.len()].copy_from_slice(&string.as_bytes());    
    string_data
}

fn string_to_array_512<'info>(string: &String) -> [u8; 512] {
    let mut string_data = [0u8; 512];
    string_data[..string.len()].copy_from_slice(&string.as_bytes());    
    string_data
}

#[error_code]
pub enum ErrorCode {
    #[msg("The given owner is not part of this multisig.")]
    InvalidOwner,
    #[msg("Owners length must be non zero.")]
    InvalidOwnersLen,
    #[msg("Not enough owners signed this transaction.")]
    NotEnoughSigners,
    #[msg("Cannot delete a transaction that has been signed by an owner.")]
    TransactionAlreadySigned,
    #[msg("Operation overflow.")]
    Overflow,
    #[msg("Cannot delete a transaction the owner did not create.")]
    UnableToDelete,
    #[msg("The given transaction has already been executed.")]
    AlreadyExecuted,
    #[msg("Transaction proposal has expired.")]
    AlreadyExpired,
    #[msg("Threshold must be less than or equal to the number of owners.")]
    InvalidThreshold,
    #[msg("Owners must be unique.")]
    UniqueOwners,
    #[msg("Owner name must have less than 32 bytes.")]
    OwnerNameTooLong,
    #[msg("Multisig nonce is not valid.")]
    InvalidMultisigNonce,
    #[msg("Multisig version is not valid.")]
    InvalidMultisigVersion,
    #[msg("Multisig owner set secuency number is not valid.")]
    InvalidOwnerSetSeqNumber,
    #[msg("Multisig account is not valid.")]
    InvalidMultisig,
    #[msg("Invalid settings authority.")]
    InvalidSettingsAuthority,
    #[msg("Not enough replacement accounts.")]
    NotEnoughReplacementAccounts,
    #[msg("Fee amount exceeds the maximum allowed.")]
    FeeExceedsMaximumAllowed
}
