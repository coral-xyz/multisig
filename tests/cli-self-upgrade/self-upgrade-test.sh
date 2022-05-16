set -euxo pipefail

# This test can be executed before using multisig to upgrade itself, so the upgrader
# can be confident that the upgrade will not break multisig.
# Steps in this test:
# 1. Deploy an old version of multisig and give upgrade authority to a multisig
# 2. Use the multisig to upgrade the program to the current version
# 3. Use the upgraded multisig to rollback the program to the previous version

# You're likely testing an upgrade from an old build that isn't part of this commit,
# so the test script can't automatically generate old builds, keys, etc.
# You need to provide the old data:

# build to test the upgrade *from*
# example: solana -um program dump JPEngBKGXmLUWAXrqZ66zTUzXNBirh5Lkjpjh7dfbXV test/old-multisig.so
OLD_BINARY=test/old-multisig.so

# build of cli that is compatible with old-multisig.so
# maybe the current build is backwards compatible?
OLD_CLI=test/old-multisig-cli

# the private key for the program-id that old-multisig.so was built with
# for mainnet, check 1password
PROGRAM_PRIVATE_KEY=test/program.json

# the pubkey for program.json
TEST_PROGRAM_ID=JPEngBKGXmLUWAXrqZ66zTUzXNBirh5Lkjpjh7dfbXV

# The program ID used in the current source code from this commit
DEFAULT_PROGRAM_ID=JPEngBKGXmLUWAXrqZ66zTUzXNBirh5Lkjpjh7dfbXV

# Don't worry about these nulls, they're set by the test:
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
    solana -ul program deploy "$OLD_BINARY" --program-id "$PROGRAM_PRIVATE_KEY"
    enable-logging
    verify-program "$OLD_BINARY" init

    ~# generate owners
    local proposer=$(keygen proposer.json)
    local simple_owner=$(keygen simple_owner.json)
    local owner_w_delegate=$(keygen owner_w_delegate.json)
    local delegate=$(keygen delegate.json)
    local unauthorized=$(keygen unauthorized.json)

    ~# create a multisig with two owners and threshold = 2
    eval $(awk 'END{print \
        "MULTISIG=" $1 ";",\
        "SIGNER=" $2
    }'<<<$("$OLD_CLI" -c test/config.toml admin new 3 $owner_w_delegate $proposer $simple_owner))

    ~# give upgrade authority for the multisig program to the multisig
    solana -ul program --commitment processed set-upgrade-authority $TEST_PROGRAM_ID --new-upgrade-authority $SIGNER

    ~# add a delegate for owner
    old-multisig -k owner_w_delegate.json admin add-delegates $delegate

    ~# create proposal 1 to upgrade to the new multisig
    local proposal1=$(build-and-propose proposer.json | tee /dev/tty | awk 'END{print $1}')

    ~# approve proposal 1 with owners and execute
    old-multisig -k owner_w_delegate.json admin approve $proposal1
    old-multisig -k simple_owner.json admin approve $proposal1
    old-multisig admin execute $proposal1

    ~# verify the upgrade
    verify-program target/deploy/serum_multisig.so upgrade

    ~# create proposal 2 to rollback the multisig program to the old version
    local proposal2=$(propose-build new-multisig proposer.json "$OLD_BINARY")
    enable-logging

    ~# approve with one owner
    new-multisig -k simple_owner.json admin approve $proposal2

    ~# execution is not allowed
    new-multisig admin execute $proposal2 && exit 33 ||:

    ~# fail to vote on the proposal using invalid wallet
    new-multisig -k unauthorized.json admin approve $proposal2 && exit 33 ||:

    ~# execution is not allowed
    new-multisig admin execute $proposal2 && exit 33 ||:

    ~# approve with delegate
    new-multisig -k delegate.json --delegated-owner $owner_w_delegate admin approve $proposal2
    
    ~# execute the proposal
    new-multisig admin execute $proposal2

    ~# verify the upgrade
    verify-program "$OLD_BINARY" rollback

    ~# test passed
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
    "$OLD_CLI" -m $MULTISIG -c test/config.toml $@
}

start-localnet() {
    solana-test-validator -r >/dev/null &
    trap "(clean_up ||:); trap - SIGTERM && kill -- -$$" SIGINT SIGTERM EXIT
    set +x
    echo 'waiting for local validator to be connectable...'
    while ! solana -ul ping -c1 --commitment processed 2>/dev/null; do sleep 0.1; done
    set -x
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
    rm dump.so ||:
    rm dump-verifiable.so ||:
    rm owner_w_delegate.json ||:
    rm proposer.json ||:
    rm simple_owner.json ||:
    rm delegate.json ||:
    rm unauthorized.json ||:
}

~#() { # recognized as a comment by vscode but a command by bash -- perfect
    set +x
    echo
    echo ================================================================================
    echo $@
    echo ================================================================================
    set -x
}

main
