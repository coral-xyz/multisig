set -euxo pipefail

multisig() { 
    cargo run -- $@
}


# owners='8ry8MXxB1HGCrELqxvhj3KfySE9GDaUgNB9HuBXDsmJR'
owners=FN5e1jY4DL94F74HutsgMvYoeaC9Sa2ve3qtpjZL5HF1
threshold=1

# multisig=$(awk '{print $1}'<<<$(multisig new $threshold $owners))
# echo multisig $multisig
multisig=3MmkfR292hdJjGv8uwC5LHS7EgsQHySgMCmjje6VPp4W
signer=dqpH2uorrd4Xi5NMDN3oEj51iHyjtPbdpqaNBSu37WR

tx=$(multisig propose-generate-token-mint $multisig | tee /dev/tty | tail -n1)
echo tx $tx
echo 'sleeping so network can reconcile account ownership (not sure why this is necessary but it never works immediately)'
sleep 14
multisig approve $multisig $tx
multisig execute $multisig $tx
multisig get $multisig
