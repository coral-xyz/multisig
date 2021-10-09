#!/bin/bash
set -e
MULTISIG_3_ADMIN=7mSA2bgzmUCi4wh16NQEfT76XMqJULni6sheZRCjcyx7
./target/debug/multisig --cluster mainnet \
   propose-binary-transaction \
   --multisig-address $MULTISIG_3_ADMIN \
   --data $1
