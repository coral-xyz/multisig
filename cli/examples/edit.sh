set -euxo pipefail

. $(dirname ${BASH_SOURCE[0]})/build.sh

# initial_owners='8ry8MXxB1HGCrELqxvhj3KfySE9GDaUgNB9HuBXDsmJR'
initial_owners=FN5e1jY4DL94F74HutsgMvYoeaC9Sa2ve3qtpjZL5HF1
initial_threshold=1
final_owners='--owners 11111111111111111111111111111111 --owners FN5e1jY4DL94F74HutsgMvYoeaC9Sa2ve3qtpjZL5HF1'
final_threshold=2

multisig=$(awk 'END{print $1}'<<<$(multisig admin new $initial_threshold $initial_owners))
echo multisig $multisig
tx=$(multisig -m $multisig propose edit $final_owners --threshold $final_threshold | tail -n1)
echo tx $tx
# echo 'sleeping so network can reconcile account ownership (not sure why this is necessary but it never works immediately)'
# sleep 14
multisig -m $multisig admin approve $tx
multisig -m $multisig admin execute $tx
multisig -m $multisig admin get
