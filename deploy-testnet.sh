#!/bin/bash
set -e
cargo build 
anchor build
cp target/idl/multisig.json res/
solana program deploy -v -u testnet --program-id keys/multisig-keypair.json target/deploy/multisig.so 
