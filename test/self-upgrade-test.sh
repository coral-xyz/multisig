set -euxo pipefail

# Instructions:
# - save build to test the upgrade *from* to old-multisig.so
# - save build of multisig-cli that is compatible with old-multisig.so to old-multisig-cli
#   (maybe the current build is backwards compatible?)
# - create program.json containing the same key that old-multisig.so expects
# - make sure TEST_PROGRAM_ID is set to the same address as program.json

DEFAULT_PROGRAM_ID=JPEngBKGXmLUWAXrqZ66zTUzXNBirh5Lkjpjh7dfbXV
TEST_PROGRAM_ID=JPEngBKGXmLUWAXrqZ66zTUzXNBirh5Lkjpjh7dfbXV
OLD_BINARY=test/old_multisig-mainnet-copy.so

main() {
    [ -f Anchor.toml ] \
        || (echo this needs to be run from the repo root as test/self-upgrade-test.sh \
            && exit 40)
    avm use 0.21.0

    ~# deploy old multisig to localnet
    start-localnet
    solana -ul program deploy $OLD_BINARY --program-id test/program.json
    sleep 10
    enable-logging

    ~# generate owners
    local owner1=$(keygen owner1.json)
    local owner2=$(keygen owner2.json)
    local owner3=$(keygen owner3.json)
    local delegate1=$(keygen delegate1.json)
    local unauthorized=$(keygen unauthorized.json)

    ~# create a multisig with two owners and threshold = 2
    eval $(awk 'END{print \
        "local multisig=" $1 ";",\
        "local signer=" $2
    }'<<<$(old-multisig admin new 2 $owner1 $owner2))

    ~# give upgrade authority for the multisig program to the multisig
    solana -ul program set-upgrade-authority $TEST_PROGRAM_ID --new-upgrade-authority $signer

    ~# add a delegate for owner 1
    old-multisig -k $owner1 add-delegates $delegate1

    ~# create proposal 1 to upgrade a verifiable build of the new multisig
    local proposal1=$(build-and-propose $owner1)

    ~# approve proposal 1 with owners 2 and 3 and execute
    old-multisig -k $owner2 approve $proposal
    old-multisig -k $owner3 approve $proposal
    old-multisig execute $proposal

    ~# verify the upgrade
    anchor verify $TEST_PROGRAM_ID

    ~# create proposal 2 to rollback the multisig program to the old version
    local proposal2=$(multisig -k $deployer propose program upgrade $TEST_PROGRAM_ID $OLD_BINARY)

    ~# approve with owner 3
    new-multisig -k $owner3 approve $proposal2

    ~# execution is not allowed
    new-multisig execute $proposal2

    ~# fail to vote on the proposal using invalid wallet
    new-multisig -k $unauthorized approve $proposal2

    ~# execution is not allowed
    new-multisig execute $proposal2

    ~# approve with delegate for owner 1
    new-multisig -k $delegate1 approve $proposal2
    
    ~# execute the proposal
    new-multisig execute $proposal2

    ~# verify the upgrade
    solana program dump JPEngBKGXmLUWAXrqZ66zTUzXNBirh5Lkjpjh7dfbXV dump.so
    assert_eq $(md5sum $OLD_BINARY) $(md5sum dump.so) 'deployed multisig does not match old multisig after rollback'

    clean_up
}


SOLANA_LOG_PID=

enable-logging() {
    solana -ul logs &
    SOLANA_LOG_PID=$!
}

disable-logging() {
    kill SOLANA_LOG_PID
}

replace-program-id() { local old=$1; local new=$1
    # sed "s/$old/$new/g" Anchor.toml
    sed "s/$old/$new/g" programs/multisig/src/lib.rs
}

keygen() { local path=$1
    solana-keygen new -so $path --no-bip39-passphrase >/dev/null
    solana -ul -k $path address
    solana -ul -k $path airdrop 100 >/dev/null
}

new-multisig() {
    target/debug/multisig-cli -c test/config.toml $@
}

old-multisig() {
    test/old-multisig-cli -c test/config.toml $@
}

start-localnet() {
    solana-test-validator -r >/dev/null &
    trap "clean_up && trap - SIGTERM && kill -- -$$" SIGINT SIGTERM EXIT
    sleep 5
}

build-and-propose() { local deployer=$1
    sed "s/$DEFAULT_PROGRAM_ID/$TEST_PROGRAM_ID/g" programs/multisig/src/lib.rs
    anchor build --verifiable
    sed "s/$TEST_PROGRAM_ID/$DEFAULT_PROGRAM_ID/g" programs/multisig/src/lib.rs
    disable-logging
    solana -ul program write-buffer target/verifiable/multisig.so
    enable-logging
    multisig -k $deployer propose program upgrade $TEST_PROGRAM_ID target/verifiable/multisig.so
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
    ~# cleaning up test artifacts
    rm dump.so
    rm owner1.json
    rm owner2.json
    rm owner3.json
    rm delegate1.json
    rm unauthorized.json
}

~#() { # recognized as a comment by vscode but a command by bash -- perfect!
    set +x
    echo
    echo ================================================================================
    echo $@
    echo ================================================================================
    set -x
}

main