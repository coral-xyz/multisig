use std::{fmt::Display, str::FromStr, sync::Arc};

use anchor_client::solana_sdk::{
    signature::{read_keypair_file, write_keypair_file, Keypair},
    signer::Signer,
};
use anchor_lang::prelude::*;
use anyhow::anyhow;

use crate::expanded_path::ExpandedPath;

#[derive(Clone)]
pub struct InputKeypair {
    path: ExpandedPath,
    keypair: Arc<Keypair>,
}

impl InputKeypair {
    pub fn as_path(&self) -> &ExpandedPath {
        &self.path
    }

    pub fn as_keypair(&self) -> Arc<Keypair> {
        self.keypair.clone()
    }

    pub fn as_pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }
}

impl std::fmt::Debug for InputKeypair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "InputKeypair({}, {})", self.path, self.as_pubkey())
    }
}

impl FromStr for InputKeypair {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let path = ExpandedPath::from_str(s)?;
        let keypair = if path.exists() {
            if path.is_dir() {
                let keypair = Keypair::new();
                write_keypair_file(&keypair, &path)
                    .map_err(|_| anyhow!("error writing keypair file {}", path))?;
                keypair
            } else {
                read_keypair_file(&path).map_err(|e| anyhow!("Error reading keypair file {}", e))?
            }
        } else {
            let keypair = Keypair::new();
            write_keypair_file(&keypair, &path)
                .map_err(|_| anyhow!("error writing keypair file {}", path))?;
            keypair
        };
        Ok(Self {
            path,
            keypair: Arc::new(keypair),
        })
    }
}

impl Display for InputKeypair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", self.keypair.pubkey(), &self.path)
    }
}
