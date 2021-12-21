cargo build -p multisig-cli
multisig() { 
    target/debug/multisig-cli $@
}
export RUST_BACKTRACE=1