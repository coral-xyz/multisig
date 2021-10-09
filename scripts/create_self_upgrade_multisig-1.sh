#!/bin/bash
set -e
../target/debug/multisig --cluster $1 create-multisig --multisig-account ../keys/self_upgrade_multisig-1.json \
     --output-multisig-pda ../target/self_upgrade_multisig_pda.pubkey  \
     --threshold $2 --owner ${@:3}
echo -----
echo ----- Assign new-upgrade-authority so marinade-multisig-program is updated by self_upgrade_multisig_pda
echo solana program set-upgrade-authority H88LfRBiJLZ7wYkHGuwkKTaijfQxexq8JvzUndu7fyjL --new-upgrade-authority $(cat ../target/self_upgrade_multisig_pda.pubkey)
