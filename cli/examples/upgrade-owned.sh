# Upgrade a program that is already owned by a pre-existing multisig

multisig() { 
    cargo run -- $@
}

program_id=BiEeLpivfo1XDkLyPa5oy86Lk4qgrPHkL2TkuZXXpkes
multisig_key=s4w4PRKf7gYSvHYPVD1F7fxr7aJvduj8UVqocuakZ1e
multisig_signer=7Psodcsu54tuLnm6Fa8QhQ1zbcHzvpwNJWWpakvm6m6W
upgrade_file=/home/drew/mine/jet/code/jet-protocol/target/deploy/jet.so


buffer="$(solana program write-buffer --buffer-authority "$multisig_signer" "$upgrade_file" | awk '{print $2}')"
echo buffer $buffer
transaction="$(multisig propose-upgrade $multisig_key $program_id $buffer)"
echo tx $transaction
multisig approve $multisig_key $transaction
multisig execute $multisig_key $transaction
