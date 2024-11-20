near account delete-account contract_000.sub_olas.olas_000.testnet beneficiary sub_olas.olas_000.testnet network-config testnet sign-with-keychain send

rm -rf ../../.near-credentials/testnet/contract*

near account create-account fund-myself contract_000.sub_olas.olas_000.testnet '10 NEAR' autogenerate-new-keypair save-to-legacy-keychain sign-as sub_olas.olas_000.testnet network-config testnet sign-with-keychain send

#near send-near sub_olas.olas_000.testnet contract_000.sub_olas.olas_000.testnet 1 --networkId testnet

#near send-near olas_000.testnet sub_olas.olas_000.testnet 8 --networkId testnet

near deploy contract_000.sub_olas.olas_000.testnet target/wasm32-unknown-unknown/release/registries_near.wasm --initFunction new --initArgs '{"multisig_factory": "multisignature2.testnet", "metadata": {"spec": "nft-1.0.0", "name": "Service Registry NFT", "symbol": "SR", "icon": "data:image", base_uri: "https://gateway.autonolas.tech/ipfs/"}}' --networkId testnet

cp ../../.near-credentials/testnet/contract_000.sub_olas.olas_000.testnet.json .near-credentials/workspaces/testnet/.