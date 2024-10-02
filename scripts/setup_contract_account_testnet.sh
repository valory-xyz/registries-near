near account delete-account contract_000.sub_olas.olas_000.testnet beneficiary sub_olas.olas_000.testnet network-config testnet sign-with-keychain send

rm -rf ../../.near-credentials/testnet/contract*

near account create-account fund-myself contract_000.sub_olas.olas_000.testnet '10 NEAR' autogenerate-new-keypair save-to-legacy-keychain sign-as sub_olas.olas_000.testnet network-config testnet sign-with-keychain send

#near send-near sub_olas.olas_000.testnet contract_000.sub_olas.olas_000.testnet 1 --networkId testnet

near deploy contract_000.sub_olas.olas_000.testnet target/wasm32-unknown-unknown/release/registries_near.wasm --initFunction new_default_meta --initArgs '{"owner_id":"sub_olas.olas_000.testnet", "multisig_factory": "multisafe.testnet"}' --networkId testnet

cp ../../.near-credentials/testnet/contract_000.sub_olas.olas_000.testnet.json .near-credentials/workspaces/testnet/.