#!/bin/bash
set -e
rm -f /tmp/buffer.json
rm -f /tmp/transaction.json
solana-keygen new --no-bip39-passphrase -o /tmp/buffer.json
solana program write-buffer -v -u $1 ../target/deploy/multisig.so --buffer /tmp/buffer.json
solana program set-buffer-authority  -v -u $1 /tmp/buffer.json --new-buffer-authority `cat ../keys/multisig-1-PDA.pubkey`
../target/debug multisig --cluster $1 propose-upgrade --multisig-address ../keys/self_upgrade_multisig.json --transaction-account /tmp/transaction.json --program-address ../keys/multisig-keypair.json --buffer-address /tmp/buffer.json
echo Show command: multisig --cluster d show-transaction --transaction-address `solana-keygen pubkey /tmp/transaction.json`
echo Approve command: multisig --cluster $1 approve --multisig-address `solana-keygen pubkey ../keys/self_upgrade_multisig.json` --transaction-address `solana-keygen pubkey /tmp/transaction.json`
echo Execute command: multisig --cluster $1 execute-transaction --multisig-address `solana-keygen pubkey ../keys/self_upgrade_multisig.json` --transaction-address `solana-keygen pubkey /tmp/transaction.json`
