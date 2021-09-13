# Creates a new multisig, transfer ownership of a program to it, then upgrade the program.

multisig() { 
    cargo run -- $@
}

# program_id=DTSXKszUjCuaNTFdRCaxi49RZN2NWWJs3JCokidCtLhH
program_id=BiEeLpivfo1XDkLyPa5oy86Lk4qgrPHkL2TkuZXXpkes
upgrade_file=/home/drew/mine/jet/code/jet-protocol/target/deploy/jet.so
# multisig_owners='7tJksM8pH4HRVCdiyAoNbCKUjszKNNrigq21Jdo4CDNu'
multisig_owners='FN5e1jY4DL94F74HutsgMvYoeaC9Sa2ve3qtpjZL5HF1'
threshold=1


multisig_key="$(multisig create-multisig $threshold "$multisig_owners")"
echo $multisig_key
solana program -ud set-upgrade-authority $program_id --new-upgrade-authority "$multisig_key"
buffer="$(solana program -ud write-buffer --buffer-authority "$multisig_key" "$upgrade_file" | awk '{print $2}')"
echo buffer $buffer
transaction="$(multisig propose-upgrade $multisig_key $program_id $buffer)"
echo tx $transaction
multisig approve $multisig_key $transaction
multisig execute $multisig_key $transaction
