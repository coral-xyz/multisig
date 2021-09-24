use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use serde_derive::Deserialize;

pub fn load(path: &str) -> Result<MultisigConfig> {
    let path = &*shellexpand::tilde(path);
    let conf_str =
        std::fs::read_to_string(path).expect(&format!("Could not load config at {}", path));
    let config: MultisigConfig = toml::from_str(&conf_str)?;
    return Ok(config);
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct MultisigConfig {
    pub cluster: String,
    pub wallet: String,

    #[serde(with = "serde_with::rust::display_fromstr")]
    pub program_id: Pubkey,

    #[serde(with = "serde_with::rust::display_fromstr")]
    pub multisig: Pubkey,
}
