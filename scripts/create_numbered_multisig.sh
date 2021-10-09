#!/bin/bash
set -e
../target/debug/multisig --cluster $2 create-multisig --multisig-account ../keys/multisig-$1.json \
     --output-multisig-pda ../target/multisig-$1.pubkey  \
     --threshold $3 --owner ${@:4}
echo -----
echo ----- Assign authorities to multisig-$1
echo set authority to $(cat ../target/multisig-$1.pubkey)