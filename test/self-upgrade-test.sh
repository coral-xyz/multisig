set -euxo pipefail

# Instructions:
# You're likely testing a build from an old build that isn't part of this repo,
# so this script can't easily automate switching between branches and getting the right builds etc.
# You need to provide the old binaries and the key for the old binaries:
# - save build to test the upgrade *from* to old-multisig.so
# - save build of multisig-cli that is compatible with old-multisig.so to old-multisig-cli
#   (maybe the current build is backwards compatible?)
# - create program.json containing the same key that old-multisig.so expects
# - make sure TEST_PROGRAM_ID is set to the same address as program.json

DEFAULT_PROGRAM_ID=JPEngBKGXmLUWAXrqZ66zTUzXNBirh5Lkjpjh7dfbXV
TEST_PROGRAM_ID=JPEngBKGXmLUWAXrqZ66zTUzXNBirh5Lkjpjh7dfbXV
OLD_BINARY=test/old_multisig-mainnet-copy.so
MULTISIG=null
SIGNER=null
export RUST_BACKTRACE=1

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
    verify-program $OLD_BINARY init

    ~# generate owners
    local owner1=$(keygen owner1.json)
    local owner2=$(keygen owner2.json)
    local owner3=$(keygen owner3.json)
    local delegate1=$(keygen delegate1.json)
    local unauthorized=$(keygen unauthorized.json)

    ~# create a multisig with two owners and threshold = 2
    eval $(awk 'END{print \
        "MULTISIG=" $1 ";",\
        "SIGNER=" $2
    }'<<<$(test/old-multisig-cli -c test/config.toml admin new 3 $owner1 $owner2 $owner3))

    ~# give upgrade authority for the multisig program to the multisig
    solana -ul program set-upgrade-authority $TEST_PROGRAM_ID --new-upgrade-authority $SIGNER

    ~# add a delegate for owner 1
    old-multisig -k owner1.json admin add-delegates $delegate1

    ~# create proposal 1 to upgrade to the new multisig
    local proposal1=$(build-and-propose owner1.json | tee /dev/tty | awk 'END{print $1}')

    ~# approve proposal 1 with owners 2 and 3 and execute
    old-multisig -k owner2.json admin approve $proposal1
    old-multisig -k owner3.json admin approve $proposal1
    old-multisig -k owner1.json admin execute $proposal1

    ~# verify the upgrade
    verify-program target/deploy/serum_multisig.so upgrade

    ~# create proposal 2 to rollback the multisig program to the old version
    local proposal2=$(propose-build new-multisig owner2.json $OLD_BINARY)
    enable-logging

    ~# approve with owner 3
    new-multisig -k owner3.json admin approve $proposal2

    ~# execution is not allowed
    new-multisig -k owner1.json admin execute $proposal2 && exit 33 || true

    ~# fail to vote on the proposal using invalid wallet
    new-multisig -k unauthorized.json admin approve $proposal2 && exit 33 || true

    ~# execution is not allowed
    new-multisig -k owner1.json admin execute $proposal2 && exit 33 || true

    ~# approve with delegate for owner 1
    new-multisig -k delegate1.json admin approve $proposal2
    
    ~# execute the proposal
    new-multisig -k owner1.json admin execute $proposal2

    ~# verify the upgrade
    verify-program $OLD_BINARY rollback

    clean_up
}


SOLANA_LOG_PID=null

enable-logging() {
    solana -ul logs &
    SOLANA_LOG_PID=$!
}

disable-logging() {
    kill $SOLANA_LOG_PID
}

keygen() { local path=$1
    solana-keygen new -so $path --no-bip39-passphrase >/dev/null
    solana -ul -k $path address
    solana -ul -k $path airdrop 100 >/dev/null
}

new-multisig() {
    target/debug/multisig-cli -m $MULTISIG -c test/config.toml $@
}

old-multisig() {
    test/old-multisig-cli -m $MULTISIG -c test/config.toml $@
}

start-localnet() {
    solana-test-validator -r >/dev/null &
    trap "clean_up && trap - SIGTERM && kill -9 -- -$$" SIGINT SIGTERM EXIT
    sleep 5
}

build-and-propose() { local deployer=$1
    sed -i "s/$DEFAULT_PROGRAM_ID/$TEST_PROGRAM_ID/g" programs/multisig/src/lib.rs
    anchor build # --verifiable
    sed -i "s/$TEST_PROGRAM_ID/$DEFAULT_PROGRAM_ID/g" programs/multisig/src/lib.rs
    propose-build old-multisig $deployer target/deploy/serum_multisig.so
}

propose-build() { local cli=$1; local deployer=$2; local binary=$3
    disable-logging
    local buffer="$(solana -ul program write-buffer $binary | tee /dev/tty | awk '{print $2}')"
    solana -ul program set-buffer-authority $buffer --new-buffer-authority $SIGNER 1>&2
    $cli -k $deployer propose program upgrade $TEST_PROGRAM_ID $buffer
}

verify-program() { local expected_binary_path=$1; local last_event_name=$2
    solana -ul program dump $TEST_PROGRAM_ID dump.so
    head -c $(stat -c %s $expected_binary_path) dump.so > dump-verifiable.so
    assert_eq $(hash < $expected_binary_path) $(hash < dump-verifiable.so) \
        "deployed multisig does not match expected multisig after $last_event_name"
}

hash() {
    md5sum | awk '{print $1}'
}

assert_eq() { local expected=$1; local actual=$2; local message=$3
    if [[ "$expected" != "$actual" ]]; then
        set +x
        echo "assertion failed: $message"
        echo "expected: $expected"
        echo "actual: $actual"
        set -x
        exit 42
    fi
}

clean_up() {
    ~# cleaning up test artifacts
    rm dump.so
    rm dump-verifiable.so
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
