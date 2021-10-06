use anchor_client::solana_sdk::pubkey::Pubkey;
use jet::state::MarketFlags;
use multisig_client::service::MultisigService;
use anyhow::Result;


pub fn propose_set_market_flags(
    service: &MultisigService,
    multisig: Pubkey,
    market: Pubkey,
    flags: MarketFlags,
) -> Result<Pubkey> {
    service.propose_anchor_instruction(
        None,
        multisig,
        custody::id(),
        jet::accounts::SetMarketFlags {
            market,
            owner: service.program.signer(multisig).0,
        },
        jet::instruction::SetMarketFlags { 
            flags: flags.bits()
        },
    )
}
