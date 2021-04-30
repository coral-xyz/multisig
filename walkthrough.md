# Walkthrough

This guide shows how to set up a new multisig as the upgrade authority for a
program, and how to upgrade that program.

## Preparation

Build and deploy the Multisig progam:

```console
$ anchor deploy
Deploying workspace: http://127.0.0.1:8899
Upgrade authority: ~/.config/solana/id.json
Deploying target/deploy/multisig.so...
Program Id: 9upUTzo5v4voWarUtMiBbs8XCFpEBM1t34RwGch55CMA

Deploy success
```

We will be using `~/.config/solana/id.json` as one of the owners of the
multisig, and generate two additional owners here. For a real deploy, others
owners would already have a key.

```console
$ solana-keygen new --outfile k2.json
...
Wrote new keypair to k2.json
pubkey: ATsi5EkETN8jMzsJ84cgUwVhjrf1KDcU4uABZ7EQbkUB

$ solana-keygen new --outfile k3.json
...
Wrote new keypair to k3.json
pubkey: EHkTBThp6SxifEKJnwuFiWJFnb2iStc7u1WJpQ2Zg91Q
```

Give them some SOL to allow them to sign:

```console
$ solana transfer --allow-unfunded-recipient ATsi5EkETN8jMzsJ84cgUwVhjrf1KDcU4uABZ7EQbkUB 100.0
$ solana transfer --allow-unfunded-recipient EHkTBThp6SxifEKJnwuFiWJFnb2iStc7u1WJpQ2Zg91Q 100.0
```

Check our own address (the public part of `~/.config/solana/id.json`):

```console
$ solana address
G1mys98VJUjxvnaeZU1XYM5KSpC6DVAGEF2zStWbF1Bv
```

Let’s recap the addresses involved so far:

| Description      | Address                                        | Keypair file               |
|------------------|------------------------------------------------|----------------------------|
| Owner 1          | `G1mys98VJUjxvnaeZU1XYM5KSpC6DVAGEF2zStWbF1Bv` | `~/.config/solana/id.json` |
| Owner 2          | `ATsi5EkETN8jMzsJ84cgUwVhjrf1KDcU4uABZ7EQbkUB` | `k2.json`                  |
| Owner 3          | `EHkTBThp6SxifEKJnwuFiWJFnb2iStc7u1WJpQ2Zg91Q` | `k3.json`                  |
| Multisig program | `9upUTzo5v4voWarUtMiBbs8XCFpEBM1t34RwGch55CMA` |                            |

## Setting up a multisig

We are going to set up a multisig for those three owners, with a threshold of 2
out of 3 signatures for it to act.

```console
$ multisig \
  --multisig-program-id 9upUTzo5v4voWarUtMiBbs8XCFpEBM1t34RwGch55CMA \
  create-multisig \
  --owner G1mys98VJUjxvnaeZU1XYM5KSpC6DVAGEF2zStWbF1Bv \
  --owner ATsi5EkETN8jMzsJ84cgUwVhjrf1KDcU4uABZ7EQbkUB \
  --owner EHkTBThp6SxifEKJnwuFiWJFnb2iStc7u1WJpQ2Zg91Q \
  --threshold 2

Multisig account: CJT35QVW8tx6uR4dnKv1LqNJ7yMfErMPtNje7wnFBJTe
Program derived address (use as upgrade authority): 4D3FRYq2kJSkyKcgW7Jju4nZXup9sQjHXqDbfPMTJpQr
```

This created multisig account `CJT35QVW8tx6uR4dnKv1LqNJ7yMfErMPtNje7wnFBJTe`. We
can double check that it was created properly:

```console
$ multisig
  --multisig-program-id 9upUTzo5v4voWarUtMiBbs8XCFpEBM1t34RwGch55CMA \
  show-multisig \
  --multisig-address CJT35QVW8tx6uR4dnKv1LqNJ7yMfErMPtNje7wnFBJTe

Threshold: 2 out of 3
Owners:
  G1mys98VJUjxvnaeZU1XYM5KSpC6DVAGEF2zStWbF1Bv
  ATsi5EkETN8jMzsJ84cgUwVhjrf1KDcU4uABZ7EQbkUB
  EHkTBThp6SxifEKJnwuFiWJFnb2iStc7u1WJpQ2Zg91Q
```
