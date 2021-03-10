# Multisig

An example of a multisig to execute arbitrary Solana transactions.

This program can be used to allow a multisig to govern anything a regular
Pubkey can govern. One can use the multisig as a BPF program upgrade
authority, a mint authority, etc.

To use, one must first create a `Multisig` account, specifying two important
parameters:

1. Owners - the set of addresses that sign transactions for the multisig.
2. Threshold - the number of signers required to execute a transaction.

Once the `Multisig` account is created, one can create a `Transaction`
account, specifying the parameters for a normal solana transaction.

To sign, owners should invoke the `approve` instruction, and finally,
the `execute_transaction`, once enough (i.e. `threhsold`) of the owners have
signed.
