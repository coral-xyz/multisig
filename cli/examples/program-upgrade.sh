# Demo of multisig for program upgrades.
# 1. Optionally deploys a new program.
# 2. Optionally creates a new multisig and transfers ownership of the program to it.
# 3. Upgrades the program using multisig.
set -euxo pipefail

. $(dirname ${BASH_SOURCE[0]})/build.sh

# Config
cluster=l
do_initial_deploy=true
new_multisig=true
new_buffer=true
upgrade_file=/data/projects/jet-v1/target/deploy/jet.so
me=$(solana address)
multisig_owners=$me
threshold=1

# Defaults (may be overridden depending on config)
jet_program_id=93jG7g93VwxxrpBWJ64GFyGaZjkbMca4byTG9MUCdraY
multisig_key=GjHh3Ud74rJdaykXhLy68Pr7YBRb44pA9hd8HoC1H5dn
multisig_signer=74XiSQuXQSixUi2Uz84V64eABCYRbt7JvHGSv9fM1Nvk
buffer=3UoWWJtUxEh37At1u9ixnQEMXfyX2ZvZhhehnrizZmzM

if [[ $do_initial_deploy == true ]]; then
    jet_program_id="$(solana program -u$cluster deploy "$upgrade_file" | awk '{print $3}')"
    echo jet program id: $jet_program_id
fi

if [[ $new_multisig == true ]]; then
    multisig_keys="$(multisig admin new $threshold "$multisig_owners")"
    multisig_key="$(awk 'END{print $1}'<<<$multisig_keys)"
    multisig_signer="$(awk 'END{print $2}'<<<$multisig_keys)"
    echo multisig: $multisig_key
    echo signer: $multisig_signer
    solana program -u$cluster set-upgrade-authority $jet_program_id --new-upgrade-authority "$multisig_signer"
fi

if [[ $new_buffer == true ]]; then
    buffer="$(solana program -u$cluster write-buffer "$upgrade_file" | awk '{print $2}')"
    solana program -u$cluster set-buffer-authority "$buffer" --new-buffer-authority "$multisig_signer"
    echo buffer $buffer
fi

transaction="$(multisig -m $multisig_key propose program upgrade $jet_program_id $buffer | tail -n1)"
echo tx $transaction

multisig -m $multisig_key admin approve $transaction
multisig -m $multisig_key admin execute $transaction
