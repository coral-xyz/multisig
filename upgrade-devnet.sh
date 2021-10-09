#!/bin/bash
rm buffer.json
solana-keygen new --no-bip39-passphrase -o buffer.json
solana program write-buffer -v -u devnet --buffer buffer.json target/deploy/multisig.so
solana program set-buffer-authority -v -u devnet --new-buffer-authority 3uxzYiAYW9UK7L4DT3Cr256ZStLc2G1vbjSHS5PEF9Bs buffer.json