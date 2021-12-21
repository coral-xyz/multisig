cargo build -p multisig-cli
multisig() { 
    target/debug/cli $@
}
