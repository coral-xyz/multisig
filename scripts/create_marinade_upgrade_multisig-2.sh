#!/bin/bash
set -e
../target/debug/multisig --cluster $1 create-multisig --multisig-account ../keys/multisig-2.json \
     --output-multisig-pda ../target/multisig-2.pubkey  \
     --threshold $2 --owner ${@:3}
echo -----
echo ----- Assign new-upgrade-authority so marinade-program can be upgraded by multisig-2
echo solana program set-upgrade-authority MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD --new-upgrade-authority $(cat ../target/multisig-2.pubkey)