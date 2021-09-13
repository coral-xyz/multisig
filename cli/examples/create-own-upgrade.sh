# Creates a new multisig, transfer ownership of a program to it, then upgrade the program.
set -euxo pipefail

multisig() { 
    cargo run -- $@
}

# program_id=DTSXKszUjCuaNTFdRCaxi49RZN2NWWJs3JCokidCtLhH #local1
# program_id=BiEeLpivfo1XDkLyPa5oy86Lk4qgrPHkL2TkuZXXpkes
program_id=ChAfyDZz7rGiv6WpoBg9RGcFXRXtr82CC8hFTz7KZhBT
upgrade_file=/home/drew/mine/jet/code/jet-protocol/target/deploy/jet.so
# multisig_owners='7tJksM8pH4HRVCdiyAoNbCKUjszKNNrigq21Jdo4CDNu'
multisig_owners='FN5e1jY4DL94F74HutsgMvYoeaC9Sa2ve3qtpjZL5HF1'
threshold=1
net=d


multisig_keys="$(multisig create-multisig $threshold "$multisig_owners")"
multisig_key="$(awk '{print $1}'<<<$multisig_keys)"
multisig_signer="$(awk '{print $2}'<<<$multisig_keys)"
echo multisig $multisig_key
echo signer $multisig_signer
solana program -u$net set-upgrade-authority $program_id --new-upgrade-authority "$multisig_signer"
buffer="$(solana program -u$net write-buffer --buffer-authority "$multisig_signer" "$upgrade_file" | awk '{print $2}')"
echo buffer $buffer
transaction="$(multisig propose-upgrade $multisig_key $program_id $buffer)"
echo tx $transaction
multisig approve $multisig_key $transaction
# multisig execute $multisig_key $transaction
