use anchor_client::Cluster;
use anyhow::Result;
use serde_derive::Deserialize;

pub fn load(path: &str) -> Result<AnchorToml> {
    let conf_str =
        std::fs::read_to_string(path).expect(&format!("Could not load Anchor.toml at {}", path));
    let config: AnchorToml = toml::from_str(&conf_str)?;
    return Ok(config);
}

#[derive(Deserialize, Debug)]
pub struct AnchorToml {
    pub provider: Provider,
    pub programs: GlobalPrograms,
}

#[derive(Deserialize, Debug)]
pub struct Provider {
    pub cluster: Cluster, //todo better parser
    pub wallet: String,
}

#[derive(Deserialize, Debug)]
pub struct GlobalPrograms {
    pub mainnet: ClusterPrograms,
    pub devnet: ClusterPrograms,
    pub localnet: Option<ClusterPrograms>,
}

#[derive(Deserialize, Debug)]
pub struct ClusterPrograms {
    pub serum_multisig: String,
}
