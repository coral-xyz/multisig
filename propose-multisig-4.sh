#!/bin/bash
set -e
#MULTISIG_3_ADMIN=7mSA2bgzmUCi4wh16NQEfT76XMqJULni6sheZRCjcyx7
MULTISIG_4_TREASURY=9aN4drMhmd8AX3eRdYvH1gbZiPmwgGJfjvneCECF97HD
./target/debug/multisig --cluster mainnet \
   propose-binary-transaction \
   --multisig-address $MULTISIG_4_TREASURY \
   --data $1
