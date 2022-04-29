set -euxo pipefail

PROGRAM_ID=JPEngBKGXmLUWAXrqZ66zTUzXNBirh5Lkjpjh7dfbXV

main() {
    echo 'IMPORTANT: this needs to be run from the repo root as test/self-upgrade-test.sh, otherwise it will misbehave'
    avm use 0.21.0

    ==== deploy old multisig to localnet
    start-localnet

    ==== generate owners
    local owner1=$(keygen owner1.json)
    local owner2=$(keygen owner2.json)
    local owner3=$(keygen owner3.json)
    local delegate1=$(keygen delegate1.json)
    local unauthorized=$(keygen unauthorized.json)

    ==== create a multisig with two owners and threshold = 2
    eval $(awk 'END{print "local multisig=" $1 ";", "local signer=" $2}'<<<$(multisig admin new 2 $owner1 $owner2))
    echo old-multisig $multisig
    echo signer $signer

    ==== give upgrade authority for the multisig program to the multisig
    solana -ul program set-upgrade-authority $PROGRAM_ID --new-upgrade-authority $signer

    ==== add a delegate for owner 1
    old-multisig -k $owner1 admin add-delegates $delegate1

    ==== create proposal 1 to upgrade a verifiable build of the new multisig
    local proposal1=$(build-and-propose $owner1)

    ==== approve proposal 1 with owners 2 and 3 and execute
    old-multisig -k $owner2 approve $proposal
    old-multisig -k $owner3 approve $proposal
    old-multisig execute $proposal

    ==== verify the upgrade
    anchor verify $PROGRAM_ID

    ==== create proposal 2 to rollback the multisig program to the old version
    local proposal2=$(multisig -k $deployer propose program upgrade $PROGRAM_ID test/old_multisig.so)

    ==== approve with owner 3
    new-multisig -k $owner3 approve $proposal2

    ==== execution is not allowed
    new-multisig execute $proposal2

    ==== fail to vote on the proposal using invalid wallet
    new-multisig -k $unauthorized approve $proposal2

    ==== execution is not allowed
    new-multisig execute $proposal2

    ==== approve with delegate for owner 1
    new-multisig -k $delegate1 approve $proposal2
    
    ==== execute the proposal
    new-multisig execute $proposal2

    ==== verify the upgrade
    solana program dump JPEngBKGXmLUWAXrqZ66zTUzXNBirh5Lkjpjh7dfbXV dump.so
    assert_eq $(md5sum test/old_multisig.so) $(md5sum dump.so) 'deployed multisig does not match old multisig after rollback'

    ==== clean up
    # prompt-await 'should we clean up test artifacts? [y/n]' y n
    clean_up
}

keygen() { local path=$1
    solana-keygen new -so $path --no-bip39-passphrase >/dev/null
    solana -ul -k owner1.json address
    solana -ul -k owner1.json airdrop 100 >/dev/null
}

new-multisig() {
    target/debug/multisig-cli -ul $@
}

old-multisig() {
    target/debug/old-multisig-cli -ul $@
}

start-localnet() {
    solana-test-validator -r --bpf-program "$PROGRAM_ID" test/old_multisig.so >/dev/null &
    sleep 3
    solana -ul logs &
    trap "clean_up && trap - SIGTERM && kill -- -$$" SIGINT SIGTERM EXIT
}

build-and-propose() { local deployer=$1
    anchor build --verifiable
    solana -ul program write-buffer target/verifiable/multisig.so
    multisig -k $deployer propose program upgrade $PROGRAM_ID target/verifiable/multisig.so
}

prompt-await() { local prompt=$1; local confirm=${3:=y}; local exit=${3:=exit}
    while true; do
        echo $prompt
        read inp
        if [[ "$inp" == "$confirm" ]]; then
            echo confirmed
            break
        fi
        if [[ "$inp" == "$exit" ]]; then
            echo exiting
            exit
        fi
        echo "sorry, $confirm is not valid."
    done
}

assert_eq() { local expected=$1; local actual=$2, local message=$3
    if [[ "$expected" != "$actual" ]]; then
        echo "assertion failed: $message"
        echo "expected: $expected"
        echo "actual: $actual"
        exit 42
    fi
}

clean_up() {
    # prompt-await 'should we clean up test artifacts? [y/n]' y n
    rm dump.so
    rm owner1.json
    rm owner2.json
    rm owner3.json
    rm delegate1.json
    rm unauthorized.json
}

====() {
    set +x
    echo
    echo '=================================='
    echo $@
    echo '=================================='
    set -x
}

main