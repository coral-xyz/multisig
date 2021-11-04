use anchor_client::{solana_sdk::pubkey::Pubkey, Cluster};
use anyhow::Result;
use serde::de::DeserializeOwned;
use serde_derive::Deserialize;

pub fn load<'a, T>(path: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let path = &*shellexpand::tilde(path);
    let conf_str =
        std::fs::read_to_string(path).expect(&format!("Could not load config at {}", path));
    let config: T = toml::from_str(&conf_str)?;
    return Ok(config);
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct MultisigConfig {
    pub cluster: String,

    pub wallet: String,

    #[serde(with = "serde_with::rust::display_fromstr")]
    pub program_id: Pubkey,

    #[serde(default, with = "optional_display_fromstr")]
    pub multisig: Option<Pubkey>,

    pub delegation: Option<DelegationConfig>,
}

impl MultisigConfig {
    pub fn cluster(&self) -> Cluster {
        match &*self.cluster.to_lowercase() {
            "l" | "localnet" | "localhost" => Cluster::Localnet,
            "d" | "devnet" => Cluster::Devnet,
            "m" | "mainnet" => Cluster::Mainnet,
            rpc => {
                let wss = rpc.replace("https", "wss");
                Cluster::Custom(rpc.to_owned(), wss)
            }
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct DelegationConfig {
    #[serde(with = "serde_with::rust::display_fromstr")]
    pub owner: Pubkey,
}

mod optional_display_fromstr {
    use super::Pubkey;
    use serde::{Deserialize, Deserializer};

    // pub fn serialize<S>(value: &Option<Pubkey>, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    //     #[derive(Serialize)]
    //     struct Helper<'a>(#[serde(with = "serde_with::rust::display_fromstr")] &'a Pubkey);

    //     value.as_ref().map(Helper).serialize(serializer)
    // }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Pubkey>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper(#[serde(with = "serde_with::rust::display_fromstr")] Pubkey);

        let helper = Option::deserialize(deserializer)?;
        Ok(helper.map(|Helper(external)| external))
    }
}
