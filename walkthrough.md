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
multisig, and generate two additional owners here for demonstration purposes.
For a real setup, other owners would already have their keys.

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

| Address                                        | Note                                 |
|------------------------------------------------|--------------------------------------|
| `G1mys98VJUjxvnaeZU1XYM5KSpC6DVAGEF2zStWbF1Bv` | Owner 1 (`~/.config/solana/id.json`) |
| `ATsi5EkETN8jMzsJ84cgUwVhjrf1KDcU4uABZ7EQbkUB` | Owner 2 (`k2.json`)                  |
| `EHkTBThp6SxifEKJnwuFiWJFnb2iStc7u1WJpQ2Zg91Q` | Owner 3 (`k3.json`)                  |
| `9upUTzo5v4voWarUtMiBbs8XCFpEBM1t34RwGch55CMA` | Multisig program id                  |

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

Program derived address: 4D3FRYq2kJSkyKcgW7Jju4nZXup9sQjHXqDbfPMTJpQr
Threshold: 2 out of 3
Owners:
  G1mys98VJUjxvnaeZU1XYM5KSpC6DVAGEF2zStWbF1Bv
  ATsi5EkETN8jMzsJ84cgUwVhjrf1KDcU4uABZ7EQbkUB
  EHkTBThp6SxifEKJnwuFiWJFnb2iStc7u1WJpQ2Zg91Q
```

The program derived address is the address that the multisig can sign with.
Anything to be approved by the multisig should use this address as the
authority.

Let’s recap the addresses involved so far:

| Address                                        | Note                                 |
|------------------------------------------------|--------------------------------------|
| `G1mys98VJUjxvnaeZU1XYM5KSpC6DVAGEF2zStWbF1Bv` | Owner 1 (`~/.config/solana/id.json`) |
| `ATsi5EkETN8jMzsJ84cgUwVhjrf1KDcU4uABZ7EQbkUB` | Owner 2 (`k2.json`)                  |
| `EHkTBThp6SxifEKJnwuFiWJFnb2iStc7u1WJpQ2Zg91Q` | Owner 3 (`k3.json`)                  |
| `9upUTzo5v4voWarUtMiBbs8XCFpEBM1t34RwGch55CMA` | Multisig program id                  |
| `CJT35QVW8tx6uR4dnKv1LqNJ7yMfErMPtNje7wnFBJTe` | Multisig account with 3 owners       |
| `4D3FRYq2kJSkyKcgW7Jju4nZXup9sQjHXqDbfPMTJpQr` | Program account for above multisig   |

## Deploying an upgradeable program

We will deploy the [example-helloworld][hello] program, and upgrade it with the
multisig later. With the example built, deploy it:

```console
$ solana program deploy target/deploy/helloworld.so
Program Id: ZkopsoMNCqrZhh6XBG1KgadtdH61ggXxyPdjLASpDUC
```

Currently we (owner 1, `~./config/solana/id.json`) are the upgrade authority.
The program was padded with zeros to a length of 123712 bytes:

```console
$ solana program show ZkopsoMNCqrZhh6XBG1KgadtdH61ggXxyPdjLASpDUC

Program Id: ZkopsoMNCqrZhh6XBG1KgadtdH61ggXxyPdjLASpDUC
Owner: BPFLoaderUpgradeab1e11111111111111111111111
ProgramData Address: H3E6BVr7EoZMp3phw9ioMCW9etsav8MF3ZPqf9zQURgT
Authority: G1mys98VJUjxvnaeZU1XYM5KSpC6DVAGEF2zStWbF1Bv
Last Deployed In Slot: 103219
Data Length: 123712 (0x1e340) bytes
```

Confirm that the program is what we intended to deploy. We pad the local program
with zeros too, to be able to compare the files:

```console
$ solana program dump ZkopsoMNCqrZhh6XBG1KgadtdH61ggXxyPdjLASpDUC onchain-1.so
$ truncate --size 123712 target/deploy/helloworld.so
$ sha256sum onchain-1.so target/deploy/helloworld.so
306400cfd76b581f93e1dcf8953edb63867dbda6a775c7d831b9562f3ab91fa3  onchain-1.so
306400cfd76b581f93e1dcf8953edb63867dbda6a775c7d831b9562f3ab91fa3  target/deploy/helloworld.so
```

Change the upgrade authority to the multisig program derived account:

```console
solana program \
  set-upgrade-authority ZkopsoMNCqrZhh6XBG1KgadtdH61ggXxyPdjLASpDUC \
  --new-upgrade-authority 4D3FRYq2kJSkyKcgW7Jju4nZXup9sQjHXqDbfPMTJpQr
```

[hello]: https://github.com/solana-labs/example-helloworld

Let’s recap the addresses so far:

| Address                                        | Note                                 |
|------------------------------------------------|--------------------------------------|
| `G1mys98VJUjxvnaeZU1XYM5KSpC6DVAGEF2zStWbF1Bv` | Owner 1 (`~/.config/solana/id.json`) |
| `ATsi5EkETN8jMzsJ84cgUwVhjrf1KDcU4uABZ7EQbkUB` | Owner 2 (`k2.json`)                  |
| `EHkTBThp6SxifEKJnwuFiWJFnb2iStc7u1WJpQ2Zg91Q` | Owner 3 (`k3.json`)                  |
| `9upUTzo5v4voWarUtMiBbs8XCFpEBM1t34RwGch55CMA` | Multisig program id                  |
| `CJT35QVW8tx6uR4dnKv1LqNJ7yMfErMPtNje7wnFBJTe` | Multisig account with 3 owners       |
| `4D3FRYq2kJSkyKcgW7Jju4nZXup9sQjHXqDbfPMTJpQr` | Program account for above multisig   |
| `ZkopsoMNCqrZhh6XBG1KgadtdH61ggXxyPdjLASpDUC`  | Hello program id                     |

## Proposing an upgrade

Suppose we have a new version of the program (for example, by changing the
counter increment in hello-world to a decrement). To upgrade the program,
normally we first upload it to a buffer account, and then call `upgrade` on the
BPF upgradable loader. Let’s try that now:

```console
$ solana program write-buffer target/deploy/helloworld2.so
Buffer: 8SysU2FxZTqcBjvXJBqpsBJNVC1KASBP8na6eQqZnaZd

# According to the Solana docs, the buffer authority of the buffer should match
# the upgrade authority of the program.
$ solana program set-buffer-authority \
  8SysU2FxZTqcBjvXJBqpsBJNVC1KASBP8na6eQqZnaZd \
  --new-buffer-authority 4D3FRYq2kJSkyKcgW7Jju4nZXup9sQjHXqDbfPMTJpQr

$ solana program deploy \
  --buffer 8SysU2FxZTqcBjvXJBqpsBJNVC1KASBP8na6eQqZnaZd \
  --program-id ZkopsoMNCqrZhh6XBG1KgadtdH61ggXxyPdjLASpDUC

Error: Program's authority Some(4D3FRYq2kJSkyKcgW7Jju4nZXup9sQjHXqDbfPMTJpQr)
does not match authority provided G1mys98VJUjxvnaeZU1XYM5KSpC6DVAGEF2zStWbF1Bv
```

As expected, we can no longer upgrade the program; only the multisig program
account can do that now. So propose this upgrade to the multisig instead. We set
the spill address to the account that created the buffer, so it can recover its
funds.

```console
$ multisig
  --multisig-program-id 9upUTzo5v4voWarUtMiBbs8XCFpEBM1t34RwGch55CMA \
  propose-upgrade \
  --multisig-address CJT35QVW8tx6uR4dnKv1LqNJ7yMfErMPtNje7wnFBJTe \
  --buffer-address 8SysU2FxZTqcBjvXJBqpsBJNVC1KASBP8na6eQqZnaZd \
  --program-address ZkopsoMNCqrZhh6XBG1KgadtdH61ggXxyPdjLASpDUC \
  --spill-address G1mys98VJUjxvnaeZU1XYM5KSpC6DVAGEF2zStWbF1Bv

Transaction account: 4XuKxb9pVUcyzBHorXXgVQDBv7JCMEXHnmmJNqGnhouk
```

This created a new multisig transaction, which is automatically signed by its
creator:

```console
$ multisig
  --multisig-program-id 9upUTzo5v4voWarUtMiBbs8XCFpEBM1t34RwGch55CMA \
  show-transaction \
  --transaction-address 4XuKxb9pVUcyzBHorXXgVQDBv7JCMEXHnmmJNqGnhouk

Multisig: CJT35QVW8tx6uR4dnKv1LqNJ7yMfErMPtNje7wnFBJTe
Did execute: false

Signers:
  [x] G1mys98VJUjxvnaeZU1XYM5KSpC6DVAGEF2zStWbF1Bv
  [ ] ATsi5EkETN8jMzsJ84cgUwVhjrf1KDcU4uABZ7EQbkUB
  [ ] EHkTBThp6SxifEKJnwuFiWJFnb2iStc7u1WJpQ2Zg91Q

Instruction:
  Program to call:         BPFLoaderUpgradeab1e11111111111111111111111
  This is a bpf_loader_upgradeable upgrade instruction.
  Program to upgrade:      ZkopsoMNCqrZhh6XBG1KgadtdH61ggXxyPdjLASpDUC
  Buffer with new program: 8SysU2FxZTqcBjvXJBqpsBJNVC1KASBP8na6eQqZnaZd
  Spill address:           G1mys98VJUjxvnaeZU1XYM5KSpC6DVAGEF2zStWbF1Bv
```

Addresses so far:

| Address                                        | Note                                          |
|------------------------------------------------|-----------------------------------------------|
| `G1mys98VJUjxvnaeZU1XYM5KSpC6DVAGEF2zStWbF1Bv` | Owner 1 (`~/.config/solana/id.json`)          |
| `ATsi5EkETN8jMzsJ84cgUwVhjrf1KDcU4uABZ7EQbkUB` | Owner 2 (`k2.json`)                           |
| `EHkTBThp6SxifEKJnwuFiWJFnb2iStc7u1WJpQ2Zg91Q` | Owner 3 (`k3.json`)                           |
| `9upUTzo5v4voWarUtMiBbs8XCFpEBM1t34RwGch55CMA` | Multisig program id                           |
| `CJT35QVW8tx6uR4dnKv1LqNJ7yMfErMPtNje7wnFBJTe` | Multisig account with 3 owners                |
| `4D3FRYq2kJSkyKcgW7Jju4nZXup9sQjHXqDbfPMTJpQr` | Program account for above multisig            |
| `ZkopsoMNCqrZhh6XBG1KgadtdH61ggXxyPdjLASpDUC`  | Hello program id                              |
| `8SysU2FxZTqcBjvXJBqpsBJNVC1KASBP8na6eQqZnaZd` | Buffer that holds the new hello program       |
| `4XuKxb9pVUcyzBHorXXgVQDBv7JCMEXHnmmJNqGnhouk` | Multisig transaction to upgrade hello program |

## Signing a transaction

Now that transaction `4XuKxb9pVUcyzBHorXXgVQDBv7JCMEXHnmmJNqGnhouk` exists, the
other owners can approve it with their keypairs:

```console
$ multisig \
  --multisig-program-id 9upUTzo5v4voWarUtMiBbs8XCFpEBM1t34RwGch55CMA \
  --keypair-path k2.json \
  approve \
  --multisig-address CJT35QVW8tx6uR4dnKv1LqNJ7yMfErMPtNje7wnFBJTe \
  --transaction-address 4XuKxb9pVUcyzBHorXXgVQDBv7JCMEXHnmmJNqGnhouk

$ multisig \
  --multisig-program-id 9upUTzo5v4voWarUtMiBbs8XCFpEBM1t34RwGch55CMA \
  show-transaction \
  --transaction-address 4XuKxb9pVUcyzBHorXXgVQDBv7JCMEXHnmmJNqGnhouk

Multisig: CJT35QVW8tx6uR4dnKv1LqNJ7yMfErMPtNje7wnFBJTe
Did execute: false

Signers:
  [x] G1mys98VJUjxvnaeZU1XYM5KSpC6DVAGEF2zStWbF1Bv
  [x] ATsi5EkETN8jMzsJ84cgUwVhjrf1KDcU4uABZ7EQbkUB
  [ ] EHkTBThp6SxifEKJnwuFiWJFnb2iStc7u1WJpQ2Zg91Q

...
```

Two out of three signatures is enough to execute the transaction with our
configuration, so let’s do that next.

## Executing a transaction

Once enough approvals are present, anybody can execute the transaction:

```console
$ multisig \
  --multisig-program-id 9upUTzo5v4voWarUtMiBbs8XCFpEBM1t34RwGch55CMA \
  execute-transaction \
  --multisig-address CJT35QVW8tx6uR4dnKv1LqNJ7yMfErMPtNje7wnFBJTe \
  --transaction-address 4XuKxb9pVUcyzBHorXXgVQDBv7JCMEXHnmmJNqGnhouk
```

WUT H3E6BVr7EoZMp3phw9ioMCW9etsav8MF3ZPqf9zQURgT
