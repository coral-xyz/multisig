# Creates a new multisig, transfer ownership of a program to it, then upgrade the program.

program_id=DTSXKszUjCuaNTFdRCaxi49RZN2NWWJs3JCokidCtLhH
upgrade_file=/home/drew/mine/jet/code/jet-protocol/target/deploy/jet.so
multisig_owners='7tJksM8pH4HRVCdiyAoNbCKUjszKNNrigq21Jdo4CDNu'
threshold=1


multisig="$(multisig create-multisig $threshold "$multisig_owners")"
solana program set-upgrade-authority $program_id --new-upgrade-authority "$multisig"
solana program write-buffer --buffer-authority "$multisig" "$upgrade_file"
transaction="$(multisig propose-upgrade $multisig $program_id $buffer)"
multisig approve $multisig $transaction
multisig execute $multisig $transaction
