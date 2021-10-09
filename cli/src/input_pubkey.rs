use std::{fmt::Display, str::FromStr, sync::Arc};

use anchor_client::solana_sdk::signature::Keypair;
use anchor_lang::prelude::*;

use crate::{expanded_path::ExpandedPath, input_keypair::InputKeypair};

#[derive(Debug, Clone)]
pub enum InputPubkey {
    Pubkey(Pubkey),
    Keypair(InputKeypair),
}

impl InputPubkey {
    pub fn try_as_path(&self) -> Option<&ExpandedPath> {
        match self {
            InputPubkey::Pubkey(_) => None,
            InputPubkey::Keypair(input_keypair) => Some(input_keypair.as_path()),
        }
    }

    pub fn try_as_keypair(&self) -> Option<Arc<Keypair>> {
        match self {
            InputPubkey::Pubkey(_) => None,
            InputPubkey::Keypair(input_keypair) => Some(input_keypair.as_keypair()),
        }
    }

    pub fn as_pubkey(&self) -> Pubkey {
        match self {
            InputPubkey::Pubkey(pubkey) => *pubkey,
            InputPubkey::Keypair(input_keypair) => input_keypair.as_pubkey(),
        }
    }
}

impl FromStr for InputPubkey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if let Ok(pubkey) = Pubkey::from_str(s) {
            Self::Pubkey(pubkey)
        } else {
            Self::Keypair(InputKeypair::from_str(s)?)
        })
    }
}

impl Display for InputPubkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputPubkey::Pubkey(pubkey) => Display::fmt(pubkey, f),
            InputPubkey::Keypair(input_keypair) => Display::fmt(input_keypair, f),
        }
    }
}
