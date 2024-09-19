use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, TokenMetadata, NonFungibleTokenMetadataProvider, NFT_METADATA_SPEC,
};
use near_contract_standards::non_fungible_token::enumeration::NonFungibleTokenEnumeration;
use near_contract_standards::non_fungible_token::{NonFungibleToken, Token, TokenId};
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

#[derive(BorshDeserialize, BorshSerialize, PartialEq)]
pub enum ServiceState {
    NonExistent,
    PreRegistration,
    ActiveRegistration,
    FinishedRegistration,
    Deployed,
    TerminatedBonded
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault, PartialEq)]
pub struct AgentParams {
    pub num_agent_instances: u32,
    pub bond: u64
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Service {
    // Service token
    pub token: Option<AccountId>,
    // Service security deposit
    pub security_deposit: u64,
    // Service multisig address
    pub multisig: Option<AccountId>,
    // IPFS hashes pointing to the config metadata
    pub config_hashes: Vec<[u8; 32]>,
    // Agent instance signers threshold: must no less than ceil((n * 2 + 1) / 3) of all the agent instances combined
    // This number will be enough to have ((2^32 - 1) * 3 - 1) / 2, which is bigger than 6.44b
    pub threshold: u32,
    // Total number of agent instances. We assume that the number of instances is bounded by 2^32 - 1
    pub max_num_agent_instances: u32,
    // Actual number of agent instances. This number is less or equal to maxNumAgentInstances
    pub num_agent_instances: u32,
    // Service state
    pub state: ServiceState,
    // Set of canonical agent Ids for the service, each agent corresponding number of agent instances,
    // and a bond corresponding to each agent Id
    pub agent_params: LookupMap<TokenId, Option<AgentParams>>,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct ServiceRegistry {
    services: LookupMap<TokenId, Service>,
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
    Service,
    AgentParams
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
            services: LookupMap::new(StorageKey::Service),
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

    fn refund_deposit_to_account(storage_used: u64, deposit_used: u64, account_id: AccountId, deposit_in: bool) {
        let mut refund: u128 = 0;
        let mut required_cost = env::storage_byte_cost().saturating_mul(storage_used.into());
        if deposit_in {
            required_cost = required_cost.saturating_add(deposit_used.into());
        } else {
            refund = refund.saturating_add(deposit_used.into());;
        }
        let attached_deposit = env::attached_deposit();

        require!(required_cost <= attached_deposit);
        // TODO: figure this out with near_token::NearToken
//         require!(
//             required_cost <= attached_deposit,
//             format!("Must attach {} to cover storage", required_cost.exact_amount_display())
//         );

        refund += attached_deposit.saturating_sub(required_cost);
        // TODO: figure this out with near_token::NearToken
        //if refund.as_yoctonear() > 1 {
        if refund > 1 {
            Promise::new(account_id).transfer(refund);
        }
    }

    fn check_service_params(
        &self,
        metadata: TokenMetadata,
        config_hash: [u8; 32],
        agent_ids: Vec<u32>,
        agent_num_instances: Vec<u32>,
        agent_bonds: Vec<u64>
    ) {
        // TODO Check other fields?
        // Number of copies must be equal to one
        require!(metadata.copies.unwrap() == 1);

        // Check config hash
        require!(!config_hash.into_iter().all(|h| h == 0));

        // Check array lengths
        require!(agent_ids.len() == agent_bonds.len());
        require!(agent_ids.len() == agent_num_instances.len());

        // Check non-zero agent Ids
        require!(agent_ids.into_iter().all(|id| id > 0));
    }

    fn fill_service_params(
        &mut self,
        service_id: TokenId,
        config_hash: [u8; 32],
        agent_ids: Vec<u32>,
        agent_num_instances: Vec<u32>,
        agent_bonds: Vec<u64>,
        threshold: u32
    ) {
        // Get the service
        let mut service = self.services.get(&service_id).unwrap();

        // Check the service state
        require!(service.state == ServiceState::PreRegistration);

        let mut security_deposit = 0;
        let mut max_num_agent_instances = 0;

        // Process agent ids and corresponding agent params
        for i in 0..agent_ids.len() {
            require!(agent_ids[i] > 0);

            // Ignore zero agent params, as it could be the case for the service update
            if agent_num_instances[i] > 0 && agent_bonds[i] > 0 {
                let res = service.agent_params.insert(
                    &agent_ids[i].to_string(),
                    &Some(AgentParams{num_agent_instances: agent_num_instances[i], bond: agent_bonds[i]})
                );

                // Check for the agent id uniqueness
                require!(res == None);

                // Adjust security deposit value
                if security_deposit < agent_bonds[i] {
                    security_deposit = agent_bonds[i];
                }

                // Add to the maximum number of agent instances
                max_num_agent_instances += agent_num_instances[i];
            } else {
                // Otherwise remove agent id and params
                service.agent_params.remove(&agent_ids[i].to_string());
            }
        }

        service.security_deposit = security_deposit;
        service.max_num_agent_instances = max_num_agent_instances;

        // Check for the correct threshold: no less than ceil((n * 2 + 1) / 3) of all the agent instances combined
        let mut check_threshold = max_num_agent_instances * 2 + 1;
        check_threshold = check_threshold.div_ceil(3);
        require!(threshold >= check_threshold && threshold <= max_num_agent_instances);

        service.threshold = threshold;

        // Record the first config hash if the service is created, or update it, if necessary
        // Check if the config hash is equal to the previous one
        let mut equal = false;
        // Get the last config hash
        let last = service.config_hashes.last();
        // If there is no config hash, the service is being created
        if last != None {
            // Compare last and current config hashes
            equal = last.unwrap().iter().zip(config_hash.iter()).all(|(a,b)| a == b);
        }

        // If the config hash is different, push it to the list of configs
        if !equal {
            service.config_hashes.push(config_hash);
        }
    }

    #[payable]
    pub fn create(
        &mut self,
        service_owner: AccountId,
        metadata: TokenMetadata,
        token: Option<AccountId>,
        config_hash: [u8; 32],
        agent_ids: Vec<u32>,
        agent_num_instances: Vec<u32>,
        agent_bonds: Vec<u64>,
        threshold: u32
    ) -> bool {
        // Record current storage usage
        let initial_storage_usage = env::storage_usage();

        self.check_service_params(
            metadata.clone(),
            config_hash,
            agent_ids.clone(),
            agent_num_instances.clone(),
            agent_bonds.clone()
        );

        // Get the current total supply
        let supply = self.tokens.owner_by_id.len();
        // To be consistent with EVM where Ids start from 1, each new token Id is equal to supply + 1
        let service_id = (supply + 1).to_string();

        // Mint new service
        // This function is used such that the storage calculation is not engaged and deposit is not refunded
        self.tokens.internal_mint_with_refund(service_id.clone(), service_owner, Some(metadata), None);

        // Allocate the service
        self.services.insert(
            &service_id,
            &Service {
                // TODO: change with just token when other tokens are enabled
                token: None,
                security_deposit: 0,
                multisig: None,
                config_hashes: vec![],
                threshold: 0,
                max_num_agent_instances: 0,
                num_agent_instances: 0,
                state: ServiceState::PreRegistration,
                agent_params: LookupMap::new(StorageKey::AgentParams),
            }
        );

        // Fill in the service parameters
        self.fill_service_params(
            service_id,
            config_hash,
            agent_ids.clone(),
            agent_num_instances.clone(),
            agent_bonds.clone(),
            threshold
        );

        // Increased storage
        let storage = env::storage_usage() - initial_storage_usage;
        ServiceRegistry::refund_deposit_to_account(storage, 0, env::predecessor_account_id(), true);

        // TODO: event

        // TODO: If this return if needed, propagate to other functions
        true
    }

    #[payable]
    pub fn update(
        &mut self,
        service_id: TokenId,
        metadata: TokenMetadata,
        token: Option<AccountId>,
        config_hash: [u8; 32],
        agent_ids: Vec<u32>,
        agent_num_instances: Vec<u32>,
        agent_bonds: Vec<u64>,
        threshold: u32
    ) {
        // Record current storage usage
        let initial_storage_usage = env::storage_usage();

        // TODO: Check for service owner

        self.check_service_params(
            metadata.clone(),
            config_hash,
            agent_ids.clone(),
            agent_num_instances.clone(),
            agent_bonds.clone()
        );

        // Fill in the service parameters
        self.fill_service_params(
            service_id,
            config_hash,
            agent_ids.clone(),
            agent_num_instances.clone(),
            agent_bonds.clone(),
            threshold
        );

        // Increased storage
        let storage = env::storage_usage() - initial_storage_usage;
        ServiceRegistry::refund_deposit_to_account(storage, 0, env::predecessor_account_id(), true);

        // TODO: event
    }

    pub fn activate_registration(
        &mut self,
        service_id: TokenId
    ) {
        // TODO: Check for service owner

        // Record current storage usage
        let initial_storage_usage = env::storage_usage();

        // Get the service
        let mut service = self.services.get(&service_id).unwrap();

        // Check the service state
        require!(service.state == ServiceState::PreRegistration);

        service.state = ServiceState::ActiveRegistration;

        // Increased storage
        // TODO: check if this is zero, as no storage is supposedly increased
        let storage = env::storage_usage() - initial_storage_usage;
        ServiceRegistry::refund_deposit_to_account(storage, service.security_deposit, env::predecessor_account_id(), true);

        // TODO: event
    }

    pub fn terminate(
        &mut self,
        service_id: TokenId
    ) {
        // TODO: Check for service owner

        // Record current storage usage
        let initial_storage_usage = env::storage_usage();

        // Get the service
        let mut service = self.services.get(&service_id).unwrap();

        // Check if the service is already terminated
        require!(service.state != ServiceState::PreRegistration && service.state != ServiceState::TerminatedBonded);

        // Define the state of the service depending on the number of bonded agent instances
        if service.num_agent_instances > 0 {
            service.state = ServiceState::TerminatedBonded;
        } else {
            service.state = ServiceState::PreRegistration;
        }

        // TODO: remove agent instances data?

        // Increased storage
        // TODO: check if this is zero, as no storage is supposedly increased
        let storage = env::storage_usage() - initial_storage_usage;
        // Send the deposit back to the service owner
        ServiceRegistry::refund_deposit_to_account(storage, service.security_deposit, env::predecessor_account_id(), false);

        // TODO: event
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
