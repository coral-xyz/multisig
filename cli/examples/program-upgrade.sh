# Demo of multisig for program upgrades.
# 1. Optionally deploys a new program.
# 2. Optionally creates a new multisig and transfers ownership of the program to it.
# 3. Upgrades the program using multisig.
set -euxo pipefail

multisig() { 
    cargo run -- $@
}

# Config
cluster=l
new_multisig=true
do_initial_deploy=true
upgrade_file=/home/drew/mine/jet/code/jet-protocol/target/deploy/jet.so
# multisig_owners='FN5e1jY4DL94F74HutsgMvYoeaC9Sa2ve3qtpjZL5HF1'
multisig_owners=8ry8MXxB1HGCrELqxvhj3KfySE9GDaUgNB9HuBXDsmJR
threshold=1

# Defaults (may be overridden depending on config)
jet_program_id=AKL7ZKUpTGaD2EbMev5zWQkFjUdvLMrFLT8xBhHkmnbu
multisig_key=GjHh3Ud74rJdaykXhLy68Pr7YBRb44pA9hd8HoC1H5dn
multisig_signer=74XiSQuXQSixUi2Uz84V64eABCYRbt7JvHGSv9fM1Nvk


if [[ $do_initial_deploy == true ]]; then
    jet_program_id="$(solana program -u$cluster deploy "$upgrade_file" | awk '{print $3}')"
    echo jet program id: $jet_program_id
fi

if [[ $new_multisig == true ]]; then
    multisig_keys="$(multisig new $threshold "$multisig_owners")"
    multisig_key="$(awk 'END{print $1}'<<<$multisig_keys)"
    multisig_signer="$(awk 'END{print $2}'<<<$multisig_keys)"
    echo multisig: $multisig_key
    echo signer: $multisig_signer
    solana program -u$cluster set-upgrade-authority $jet_program_id --new-upgrade-authority "$multisig_signer"
fi

buffer="$(solana program -u$cluster write-buffer "$upgrade_file" | awk '{print $2}')"
solana program -u$cluster set-buffer-authority "$buffer" --new-buffer-authority "$multisig_signer"
echo buffer $buffer
transaction="$(multisig propose-upgrade $multisig_key $jet_program_id $buffer | tail -n1)"
echo tx $transaction
echo 'sleeping so network can reconcile account ownership (not sure why this is necessary but it never works immediately)'
sleep 14
multisig approve $multisig_key $transaction
multisig execute $multisig_key $transaction
