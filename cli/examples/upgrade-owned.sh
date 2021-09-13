# Upgrade a program that is already owned by a pre-existing multisig

multisig() { 
    cargo run -- $@
}

program_id=DTSXKszUjCuaNTFdRCaxi49RZN2NWWJs3JCokidCtLhH
multisig_key=2XZHQr2mnogMFNF3TzFXUH8p2fbmufWrhL9jjbNM4ma7
upgrade_file=/home/drew/mine/jet/code/jet-protocol/target/deploy/jet.so


buffer="$(solana program write-buffer --buffer-authority "$multisig_key" "$upgrade_file" | awk '{print $2}')"
echo buffer $buffer
transaction="$(multisig propose-upgrade $multisig_key $program_id $buffer)"
echo tx $transaction
multisig approve $multisig_key $transaction
multisig execute $multisig_key $transaction
