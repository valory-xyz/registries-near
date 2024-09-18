use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, TokenMetadata, NonFungibleTokenMetadataProvider, NFT_METADATA_SPEC,
};
use near_contract_standards::non_fungible_token::enumeration::NonFungibleTokenEnumeration;
use near_contract_standards::non_fungible_token::{NonFungibleToken, Token, TokenId, refund_deposit_to_account};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base64VecU8, U128};
use near_sdk::collections::LazyOption;
use near_sdk::{
    env, near_bindgen, require, AccountId, BorshStorageKey, PanicOnDefault, Promise, PromiseOrValue, StorageUsage,
};
use near_sdk::collections::{LookupMap, TreeMap, UnorderedSet};
//use near_gas::NearGas;

// pub mod external;
// pub use crate::external::*;

// const NO_DEPOSIT: Balance = 0;
// const CURRENT_STATE_VERSION: u32 = 1;
// const TGAS: u64 = 1_000_000_000_000;

#[near_bindgen]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentParams {
    pub agent_id: u32,
    pub num_agent_instances: u32,
    pub bond: u64
}

#[near_bindgen]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Service {
    // Service token
    pub token: Option<AccountId>,
    // Service security deposit
    pub security_deposit: u64,
    // Service multisig address
    pub multisig: Option<AccountId>,
    // IPFS hashes pointing to the config metadata
    pub config_hash: [u8; 32],
    // Agent instance signers threshold: must no less than ceil((n * 2 + 1) / 3) of all the agent instances combined
    // This number will be enough to have ((2^32 - 1) * 3 - 1) / 2, which is bigger than 6.44b
    pub threshold: u32,
    // Total number of agent instances. We assume that the number of instances is bounded by 2^32 - 1
    pub max_num_agent_instances: u32,
    // Actual number of agent instances. This number is less or equal to maxNumAgentInstances
    pub num_agent_instances: u32,
    // Service state
    pub state: u8,
    // Set of canonical agent Ids for the service, each agent corresponding number of agent instances,
    // and a bond corresponding to each agent Id
    pub agent_params: Vec<AgentParams>,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct ServiceRegistry {
    services: Option<LookupMap<TokenId, Service>>,
    tokens: NonFungibleToken,
    metadata: LazyOption<NFTContractMetadata>,
    paused: bool,
}

const DATA_IMAGE_SVG_NEAR_ICON: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 288 288'%3E%3Cg id='l' data-name='l'%3E%3Cpath d='M187.58,79.81l-30.1,44.69a3.2,3.2,0,0,0,4.75,4.2L191.86,103a1.2,1.2,0,0,1,2,.91v80.46a1.2,1.2,0,0,1-2.12.77L102.18,77.93A15.35,15.35,0,0,0,90.47,72.5H87.34A15.34,15.34,0,0,0,72,87.84V201.16A15.34,15.34,0,0,0,87.34,216.5h0a15.35,15.35,0,0,0,13.08-7.31l30.1-44.69a3.2,3.2,0,0,0-4.75-4.2L96.14,186a1.2,1.2,0,0,1-2-.91V104.61a1.2,1.2,0,0,1,2.12-.77l89.55,107.23a15.35,15.35,0,0,0,11.71,5.43h3.13A15.34,15.34,0,0,0,216,201.16V87.84A15.34,15.34,0,0,0,200.66,72.5h0A15.35,15.35,0,0,0,187.58,79.81Z'/%3E%3C/g%3E%3C/svg%3E";

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    NonFungibleToken,
    Metadata,
    TokenMetadata,
    Enumeration,
    Approval,
}

#[near_bindgen]
impl ServiceRegistry {
    /// Initializes the contract owned by `owner_id` with
    /// default metadata (for example purposes only).
    #[init]
    pub fn new_default_meta(owner_id: AccountId) -> Self {
        Self::new(
            owner_id,
            NFTContractMetadata {
                spec: NFT_METADATA_SPEC.to_string(),
                name: "Example NEAR non-fungible token".to_string(),
                symbol: "EXAMPLE".to_string(),
                icon: Some(DATA_IMAGE_SVG_NEAR_ICON.to_string()),
                base_uri: None,
                reference: None,
                reference_hash: None,
            },
        )
    }

    #[init]
    pub fn new(owner_id: AccountId, metadata: NFTContractMetadata) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        metadata.assert_valid();
        Self {
            services: Some(LookupMap::new()),
            tokens: NonFungibleToken::new(
                StorageKey::NonFungibleToken,
                owner_id,
                Some(StorageKey::TokenMetadata),
                Some(StorageKey::Enumeration),
                Some(StorageKey::Approval),
            ),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
            paused: false,
        }
    }

    #[payable]
    pub fn create(&mut self, service_owner: AccountId, metadata: TokenMetadata) {
        // Record current storage usage
        let initial_storage_usage = env::storage_usage();

        // TODO Check other fields?
        // Number of copies must be equal to one
        require!(metadata.copies.unwrap() == 1);

        // Get the current total supply
        let supply = self.tokens.owner_by_id.len();
        // To be consistent with EVM where Ids start from 1, each new token Id is equal to supply + 1
        let token_id = (supply + 1).to_string();

        // Mint new service
        // This function is used such that the storage calculation is not engaged and deposit is not refunded
        self.tokens.internal_mint_with_refund(token_id.clone(), service_owner, Some(metadata), None);

        // Allocate the service


        let storage = env::storage_usage() - initial_storage_usage;
        refund_deposit_to_account(storage, env::predecessor_account_id());
    }

//     pub fn set_metadata(
//         &mut self,
//         name: Option<String>,
//         symbol: Option<String>,
//         reference: Option<String>,
//         reference_hash: Option<Base64VecU8>,
//         decimals: Option<u8>,
//         icon: Option<String>,
//     ) {
//         // Only owner can change the metadata
//         require!(self.owner_or_self());
//
//         name.map(|name| self.name = name);
//         symbol.map(|symbol| self.symbol = symbol);
//         reference.map(|reference| self.reference = reference);
//         reference_hash.map(|reference_hash| self.reference_hash = reference_hash);
//         decimals.map(|decimals| self.decimals = decimals);
//         icon.map(|icon| self.icon = Some(icon));
//     }
//
//     #[payable]
//     pub fn mint(&mut self, account_id: AccountId, amount: U128) {
//         assert_eq!(
//             env::predecessor_account_id(),
//             self.controller,
//             "Only controller can call mint"
//         );
//
//         self.storage_deposit(Some(account_id.clone()), None);
//         self.token.internal_deposit(&account_id, amount.into());
//     }

    pub fn account_storage_usage(&self) -> StorageUsage {
        self.tokens.extra_storage_in_bytes_per_token
    }

    /// Return true if the caller is either controller or self
    pub fn owner_or_self(&self) -> bool {
        let caller = env::predecessor_account_id();
        caller == self.tokens.owner_id || caller == env::current_account_id()
    }

    pub fn is_paused(&self) -> bool {
        self.paused //&& !self.owner_or_self()
    }

    pub fn set_paused(&mut self, paused: bool) {
        require!(self.owner_or_self());
        self.paused = if paused { true } else { false };
    }

    pub fn total_supply(&self) -> U128 {
        self.tokens.nft_total_supply()
    }

    pub fn get_token_metadata(&self, token_id: TokenId) -> Option<TokenMetadata> {
        self.tokens.token_metadata_by_id.as_ref().and_then(|by_id| by_id.get(&token_id))
    }

    pub fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_owned()
    }
}
