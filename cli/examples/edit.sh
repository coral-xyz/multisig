set -euxo pipefail

. $(dirname ${BASH_SOURCE[0]})/build.sh

me=$(solana address)
initial_owners=$me
initial_threshold=1
final_owners="--owners 11111111111111111111111111111111 --owners $me"
final_threshold=2

multisig="$(awk 'END{print $1}'<<<$(multisig admin new $initial_threshold $initial_owners))"
echo multisig $multisig
tx=$(multisig -m $multisig propose multisig edit $final_owners --threshold $final_threshold | tail -n1)
echo tx $tx
# echo 'sleeping so network can reconcile account ownership (not sure why this is necessary but it never works immediately)'
# sleep 14
multisig -m $multisig admin approve $tx
multisig -m $multisig admin execute $tx
multisig -m $multisig admin get
