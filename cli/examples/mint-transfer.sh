set -euxo pipefail

. $(dirname ${BASH_SOURCE[0]})/build.sh

# owners='8ry8MXxB1HGCrELqxvhj3KfySE9GDaUgNB9HuBXDsmJR'
#owners=FN5e1jY4DL94F74HutsgMvYoeaC9Sa2ve3qtpjZL5HF1
owners=FcyC5ouiinjj9pYzKmHVevg91n3pwwrtpCUFvXVap4p1
threshold=1

ms=$(multisig admin new $threshold $owners)
multisig=$(awk '{print $1}'<<<$ms)
signer=$(awk '{print $2}'<<<$ms)
echo multisig $multisig
echo multisig $signer
# multisig=3MmkfR292hdJjGv8uwC5LHS7EgsQHySgMCmjje6VPp4W
# signer=dqpH2uorrd4Xi5NMDN3oEj51iHyjtPbdpqaNBSu37WR

mint=$(spl-token create-token --mint-authority $signer | sed -n 's/Creating token \(.*\)/\1/gp')
ms_token_account=$(spl-token create-account "$mint" --owner $signer | sed -n 's/Creating account \(.*\)/\1/gp')
my_token_account=$(spl-token create-account "$mint" | sed -n 's/Creating account \(.*\)/\1/gp')

tx=$(multisig propose token mint $multisig $mint $ms_token_account 100 | tee /dev/tty | tail -n1)
echo tx $tx
echo 'sleeping so network can reconcile account ownership (not sure why this is necessary but it never works immediately)'
sleep 14
multisig approve $multisig $tx
multisig execute $multisig $tx

tx=$(multisig propose token transfer $multisig $ms_token_account $my_token_account 50 | tee /dev/tty | tail -n1)
echo tx $tx
echo 'sleeping so network can reconcile account ownership (not sure why this is necessary but it never works immediately)'
sleep 14
multisig approve $multisig $tx
multisig execute $multisig $tx
