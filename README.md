# Registries Near
Set of Autonolas registries contracts on NEAR.

## Pre-requisites
The program requires that the following environment is satisfied:
```
rustc --version
rustc 1.79.0 (129f3b996 2024-06-10)
```

Advise the script `setup-env.sh` to correctly install the required environment.

## Development
Install the dependencies:
```
yarn
```

If you need to remove / check dependencies, run:
```
cargo clean
cargo tree
```

You might also want to completely remove the `Cargo.lock` file.

Build the code with:
```
anchor build
```

### Create NEAR accounts
Documentation (subject to change): https://docs.near.org/concepts/protocol/account-id

Current version:
```bash
near account create-account fund-later `ACCOUNT_NAME` autogenerate-new-keypair save-to-legacy-keychain network-config testnet create
```

### Testing
```bash
npx ava test/test.ts
```

### Localnet
The local validator in this case is the project `near-sandbox`
https://github.com/near/near-sandbox
RPC: http://0.0.0.0:3030

Install sandbox:
```bash
npm i -g near-sandbox
```

Init sandbox:
```bash
# home of sandbox must be outside of repo, in /tmp
near-sandbox --home /tmp/near-sandbox init
# in another shell-windows
near-sandbox --home /tmp/near-sandbox run
```

### Testnet
- RPC: https://rpc.testnet.near.org
- Faucet: https://near-faucet.io/
- Explorer: https://nearblocks.io/
