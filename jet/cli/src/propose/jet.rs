use anchor_client::solana_sdk::{
    pubkey::Pubkey, signature::Keypair, signer::Signer, system_program, sysvar::rent,
};
use anchor_spl::token;
use anyhow::Result;
use jet::instructions::init_reserve::InitReserveBumpSeeds;
use jet::state::{MarketFlags, ReserveConfig};
use multisig_client::service::MultisigService;
use rand::rngs::OsRng;
use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct ReserveParameters {
    #[serde(with = "serde_with::rust::display_fromstr")]
    dex_market: Pubkey,
    #[serde(with = "serde_with::rust::display_fromstr")]
    token_mint: Pubkey,
    #[serde(with = "serde_with::rust::display_fromstr")]
    dex_program: Pubkey,
    #[serde(with = "serde_with::rust::display_fromstr")]
    oracle_price: Pubkey,
    #[serde(with = "serde_with::rust::display_fromstr")]
    oracle_product: Pubkey,
    #[serde(with = "serde_with::rust::display_fromstr")]
    quote_token_mint: Pubkey,
    utilization_rate_1: u16,
    utilization_rate_2: u16,
    borrow_rate_0: u16,
    borrow_rate_1: u16,
    borrow_rate_2: u16,
    borrow_rate_3: u16,
    min_collateral_ratio: u16,
    liquidation_premium: u16,
    manage_fee_collection_threshold: u64,
    manage_fee_rate: u16,
    loan_origination_fee: u16,
    liquidation_slippage: u16,
    liquidation_dex_trade_max: u64,
}

pub fn propose_init_reserve(
    service: &MultisigService,
    multisig: Pubkey,
    market: Pubkey,
    params: ReserveParameters,
) -> Result<(Pubkey, Pubkey)> {
    let reserve = Keypair::generate(&mut OsRng);
    let (market_authority, _) = Pubkey::find_program_address(&[market.as_ref()], &jet::id());
    let (vault, vault_bump) =
        Pubkey::find_program_address(&[b"vault", reserve.pubkey().as_ref()], &jet::id());
    let (fee_note_vault, fee_note_vault_bump) =
        Pubkey::find_program_address(&[b"fee-vault", reserve.pubkey().as_ref()], &jet::id());
    let (dex_swap_tokens, dex_swap_tokens_bump) =
        Pubkey::find_program_address(&[b"dex-swap-tokens", reserve.pubkey().as_ref()], &jet::id());
    let (dex_open_orders, dex_open_orders_bump) =
        Pubkey::find_program_address(&[b"dex-open-orders", reserve.pubkey().as_ref()], &jet::id());
    let (deposit_note_mint, deposit_note_mint_bump) = Pubkey::find_program_address(
        &[
            b"deposits",
            reserve.pubkey().as_ref(),
            params.token_mint.as_ref(),
        ],
        &jet::id(),
    );
    let (loan_note_mint, loan_note_mint_bump) = Pubkey::find_program_address(
        &[
            b"loans",
            reserve.pubkey().as_ref(),
            params.token_mint.as_ref(),
        ],
        &jet::id(),
    );
    let bump = InitReserveBumpSeeds {
        vault: vault_bump,
        fee_note_vault: fee_note_vault_bump,
        dex_swap_tokens: dex_swap_tokens_bump,
        dex_open_orders: dex_open_orders_bump,
        deposit_note_mint: deposit_note_mint_bump,
        loan_note_mint: loan_note_mint_bump,
    };
    let builder = service.program.request().signer(&reserve);
    let proposal = service.propose_anchor_instruction(
        Option::Some(builder),
        multisig,
        jet::id(),
        jet::accounts::InitializeReserve {
            market,
            market_authority,
            reserve: reserve.pubkey(),
            vault,
            fee_note_vault,
            dex_swap_tokens,
            dex_open_orders,
            dex_market: params.dex_market,
            token_mint: params.token_mint,
            token_program: token::ID,
            dex_program: params.dex_program,
            oracle_price: params.oracle_price,
            oracle_product: params.oracle_product,
            deposit_note_mint,
            loan_note_mint,
            quote_token_mint: params.quote_token_mint,
            owner: service.program.signer(multisig).0,
            system_program: system_program::id(),
            rent: rent::id(),
        },
        jet::instruction::InitReserve {
            config: ReserveConfig {
                utilization_rate_1: params.utilization_rate_1,
                utilization_rate_2: params.utilization_rate_2,
                borrow_rate_0: params.borrow_rate_0,
                borrow_rate_1: params.borrow_rate_1,
                borrow_rate_2: params.borrow_rate_2,
                borrow_rate_3: params.borrow_rate_3,
                min_collateral_ratio: params.min_collateral_ratio,
                liquidation_premium: params.liquidation_premium,
                manage_fee_collection_threshold: params.manage_fee_collection_threshold,
                manage_fee_rate: params.manage_fee_rate,
                loan_origination_fee: params.loan_origination_fee,
                liquidation_slippage: params.liquidation_slippage,
                liquidation_dex_trade_max: params.liquidation_dex_trade_max,
                _reserved0: 0,
                _reserved1: [0; 24],
            },
            bump,
        },
    )?;
    Ok((proposal, reserve.pubkey()))
}

pub fn propose_set_market_flags(
    service: &MultisigService,
    multisig: Pubkey,
    market: Pubkey,
    flags: MarketFlags,
) -> Result<Pubkey> {
    service.propose_anchor_instruction(
        None,
        multisig,
        jet::id(),
        jet::accounts::SetMarketFlags {
            market,
            owner: service.program.signer(multisig).0,
        },
        jet::instruction::SetMarketFlags {
            flags: flags.bits(),
        },
    )
}

pub fn propose_set_market_owner(
    service: &MultisigService,
    multisig: Pubkey,
    market: Pubkey,
    new_owner: Pubkey,
) -> Result<Pubkey> {
    service.propose_anchor_instruction(
        None,
        multisig,
        jet::id(),
        jet::accounts::SetMarketOwner {
            market,
            owner: service.program.signer(multisig).0,
        },
        jet::instruction::SetMarketOwner { new_owner },
    )
}
