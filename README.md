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
./scripts/build.sh
```

### Manage NEAR accounts
Create / delete accounts, transfer funds and deploy contracts on testnet:
```bash
./scripts/setup_contract_account_testnet.sh
```

Current version:
```bash
near account create-account fund-later `ACCOUNT_NAME` autogenerate-new-keypair save-to-legacy-keychain network-config testnet create
```

### Testing
Sandbox:
```bash
npx ava test/ServiceRegistry.ts
```

Testnet:
```bash
npx ava --config ava.testnet.config.cjs test/testnet_ServiceRegistry.ts
```

Testing with debug:
```bash
NEAR_WORKSPACES_DEBUG=true npx ava test/ServiceRegistry.ts
```

Deploy the contract in the testnet:
```bash
near deploy contract_000.sub_olas.olas_000.testnet target/wasm32-unknown-unknown/release/registries_near.wasm --initFunction new --initArgs '{"multisig_factory": "multisignature2.testnet", "metadata": {"spec": "nft-1.0.0", "name": "Service Registry NFT", "symbol": "SR", "icon": "data:image", base_uri: "https://gateway.autonolas.tech/ipfs/"}}' --networkId testnet
```

### Testnet
- RPC: https://rpc.testnet.near.org
- Faucet: https://near-faucet.io/
- Explorer: https://nearblocks.io/
