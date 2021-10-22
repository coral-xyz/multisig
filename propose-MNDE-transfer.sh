#!/bin/bash
set -ex
#MULTISIG_3_ADMIN=7mSA2bgzmUCi4wh16NQEfT76XMqJULni6sheZRCjcyx7
MULTISIG_4_TREASURY=9aN4drMhmd8AX3eRdYvH1gbZiPmwgGJfjvneCECF97HD
./target/debug/multisig --cluster https://marinade.rpcpool.com \
   propose-spl-token-transfer \
   --multisig-address $MULTISIG_4_TREASURY \
   --amount $1 \
   --from GR1LBT4cU89cJWE74CP6BsJTf2kriQ9TX59tbDsfxgSi \
   --to $2 \
   --auth 9cBg3Ankxf4ijde8tjRKAGix5EGRbASnvigUA3JW8WSu -o
