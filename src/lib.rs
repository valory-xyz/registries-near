use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, TokenMetadata, NonFungibleTokenMetadataProvider, NFT_METADATA_SPEC,
};
use near_contract_standards::non_fungible_token::enumeration::NonFungibleTokenEnumeration;
use near_contract_standards::non_fungible_token::{NonFungibleToken, Token};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base64VecU8, U128};
use near_sdk::serde_json::json;
use near_sdk::collections::LazyOption;
use near_sdk::{
    env, near_bindgen, require, AccountId, BorshStorageKey, PanicOnDefault, Promise, PromiseOrValue, StorageUsage, Gas,
    PromiseError
};
use near_sdk::collections::{LookupMap, Vector};
use near_sdk::ext_contract;
//use near_account_id::{AccountId};
//near_token::NearToken
//use near_gas::NearGas;

// pub mod external;
// pub use crate::external::*;

// const NO_DEPOSIT: Balance = 0;
// const CURRENT_STATE_VERSION: u32 = 1;
// const TGAS: u64 = 1_000_000_000_000;

// MultisigFactory interface
#[ext_contract(multisig_factory)]
trait MultisigFactory {
    #[payable]
    fn create(
        &mut self,
        name: AccountId,
        members: Vec<AccountId>,
        num_confirmations: u64,
    ) -> Promise;
}

// Multisig2 interface
#[ext_contract(multisig2)]
trait Multisig2 {
    fn get_members(&self) -> Vec<AccountId>;
}


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
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct AgentParams {
    pub num_agent_instances: u32,
    pub bond: u128,
    pub instances: Vector<AccountId>
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct OperatorData {
    pub balance: u128,
    pub instances: Vector<AccountId>
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Service {
    // Service token
    pub token: Option<AccountId>,
    // Service security deposit
    pub security_deposit: u128,
    // Service multisig address
    pub multisig: Option<AccountId>,
    // IPFS hashes pointing to the config metadata
    pub config_hashes: Vector<[u8; 32]>,
    // Agent instance signers threshold: must no less than ceil((n * 2 + 1) / 3) of all the agent instances combined
    // This number will be enough to have ((2^32 - 1) * 3 - 1) / 2, which is bigger than 6.44b
    pub threshold: u32,
    // Total number of agent instances. We assume that the number of instances is bounded by 2^32 - 1
    pub max_num_agent_instances: u32,
    // Actual number of agent instances. This number is less or equal to maxNumAgentInstances
    pub num_agent_instances: u32,
    // Service state
    pub state: ServiceState,
    // Iterable set of canonical agent Ids for the service
    pub agent_ids: Vector<u32>,
    // Map of canonical agent Ids for the service, each agent corresponding number of agent instances,
    // and a bond corresponding to each agent Id
    // TODO: consider changing to UnorderedMap with the iterator instead of separate agent_ids
    pub agent_params: LookupMap<u32, AgentParams>,
    // Map of agent instances in the service and their corresponding agent ids
    pub agent_instances: LookupMap<AccountId, u32>,
    // Map of operators in the service and their corresponding OperatorData struct
    pub operators: LookupMap<AccountId, OperatorData>
}

const TGAS: u64 = 1_000_000_000_000;
const CREATE_CALL_GAS: u64 = 50_000_000_000_000;
const MULTISIG_FACTORY: &str = "multisafe.near";

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct ServiceRegistry {
    owner: AccountId,
    services: LookupMap<u32, Service>,
    tokens: NonFungibleToken,
    metadata: LazyOption<NFTContractMetadata>,
    agent_instance_operators: LookupMap<AccountId, AccountId>,
    slashed_funds: u128,
    paused: bool
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
    AgentId,
    AgentParam,
    AgentInstance,
    AgentInstancePerAgentId,
    ConfigHash,
    OperatorData,
    AgentInstanceOperator
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
            owner: env::predecessor_account_id(),
            services: LookupMap::new(StorageKey::Service),
            tokens: NonFungibleToken::new(
                StorageKey::NonFungibleToken,
                owner_id,
                Some(StorageKey::TokenMetadata),
                Some(StorageKey::Enumeration),
                Some(StorageKey::Approval),
            ),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
            agent_instance_operators: LookupMap::new(StorageKey::AgentInstanceOperator),
            slashed_funds: 0 as u128,
            paused: false
        }
    }

    fn refund_deposit_to_account(&self, storage_used: u64, deposit_used: u128, account_id: AccountId, deposit_in: bool) {
        let mut refund: u128 = 0;
        let mut required_cost = env::storage_byte_cost().saturating_mul(storage_used.into());
        if deposit_in {
            required_cost = required_cost.saturating_add(deposit_used.into());
        } else {
            refund = refund.saturating_add(deposit_used.into());
        }
        let attached_deposit = env::attached_deposit();

        require!(required_cost <= attached_deposit);

        refund += attached_deposit.saturating_sub(required_cost);
        if refund > 1 {
            Promise::new(account_id).transfer(refund);
        }
    }

    pub fn change_owner(&mut self, new_owner: AccountId) {
        // Check the ownership
        require!(self.owner == env::predecessor_account_id());

        // Check account validity
        require!(env::is_valid_account_id(new_owner.as_ref().as_bytes()));

        self.owner = new_owner;

        // TODO: event
    }

    fn check_service_params(
        &self,
        config_hash: [u8; 32],
        agent_ids: Vec<u32>,
        agent_num_instances: Vec<u32>,
        agent_bonds: Vec<u128>
    ) {
        // Check config hash
        require!(!config_hash.into_iter().all(|h| h == 0));

        // Check uniqueness of agent ids: sorted agent ids must match its size with the original array
        let mut check_agent_ids = agent_ids.clone();
        check_agent_ids.sort_unstable();
        check_agent_ids.dedup();
        require!(check_agent_ids.len() == agent_ids.len());
        //let v: Vec<_> = agent_ids.into_iter().unique().collect();

        // Check array lengths
        require!(agent_ids.len() == agent_bonds.len());
        require!(agent_ids.len() == agent_num_instances.len());

        // Check non-zero agent Ids
        require!(agent_ids.into_iter().all(|id| id > 0));
    }

    fn fill_service_params(
        &mut self,
        service_id: u32,
        config_hash: [u8; 32],
        agent_ids: Vec<u32>,
        agent_num_instances: Vec<u32>,
        agent_bonds: Vec<u128>,
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
            let agent_id = agent_ids[i];
            require!(agent_id > 0);

            // Ignore zero agent params, as it is the case for the service update
            if agent_num_instances[i] > 0 && agent_bonds[i] > 0 {
                service.agent_ids.push(&agent_id);

                service.agent_params.insert(
                    &agent_id,
                    &AgentParams{
                        num_agent_instances: agent_num_instances[i],
                        bond: agent_bonds[i],
                        instances: Vector::new(StorageKey::AgentInstancePerAgentId)
                    }
                );

                // Adjust security deposit value
                if security_deposit < agent_bonds[i] {
                    security_deposit = agent_bonds[i];
                }

                // Add to the maximum number of agent instances
                max_num_agent_instances += agent_num_instances[i];
            } else {
                // Otherwise remove agent id and params
                service.agent_params.remove(&agent_id);
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
        let last = service.config_hashes.iter().last();
        // If there is no config hash, the service is being created
        // Otherwise there must be at least one config hash
        if last.is_some() {
            // Compare last and current config hashes if the service is updated
            equal = last.unwrap().iter().zip(config_hash.iter()).all(|(a, b)| a == b);
        }

        // If the config hash is different, push it to the list of configs
        if !equal {
            service.config_hashes.push(&config_hash);
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
        agent_bonds: Vec<u128>,
        threshold: u32
    ) -> bool {
        // Record current storage usage
        let initial_storage_usage = env::storage_usage();

        // TODO Check other fields?
        // Number of copies must be equal to one
        require!(metadata.copies.unwrap() == 1);

        self.check_service_params(
            config_hash,
            agent_ids.clone(),
            agent_num_instances.clone(),
            agent_bonds.clone()
        );

        // Get the current total supply
        let supply = self.tokens.owner_by_id.len() as u32;
        // To be consistent with EVM where Ids start from 1, each new token Id is equal to supply + 1
        let service_id = supply + 1;

        // Mint new service
        // This function is used such that the storage calculation is not engaged and deposit is not refunded
        self.tokens.internal_mint_with_refund(service_id.to_string().clone(), service_owner, Some(metadata), None);

        // Allocate the service
        self.services.insert(
            &service_id,
            &Service {
                // TODO: change with just token when other tokens are enabled
                token: None,
                security_deposit: 0,
                multisig: None,
                config_hashes: Vector::new(StorageKey::ConfigHash),
                threshold: 0,
                max_num_agent_instances: 0,
                num_agent_instances: 0,
                state: ServiceState::PreRegistration,
                agent_ids: Vector::new(StorageKey::AgentId),
                agent_params: LookupMap::new(StorageKey::AgentParam),
                agent_instances: LookupMap::new(StorageKey::AgentInstance),
                operators: LookupMap::new(StorageKey::OperatorData)
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
        self.refund_deposit_to_account(storage, 0, env::predecessor_account_id(), true);

        // TODO: event

        // TODO: If this return if needed, propagate to other functions
        true
    }

    #[payable]
    pub fn update(
        &mut self,
        service_id: u32,
        token: Option<AccountId>,
        config_hash: [u8; 32],
        agent_ids: Vec<u32>,
        agent_num_instances: Vec<u32>,
        agent_bonds: Vec<u128>,
        threshold: u32
    ) {
        // Record current storage usage
        let initial_storage_usage = env::storage_usage();

        // Check for service owner
        let owner_id = self.tokens
            .owner_by_id
            .get(&service_id.to_string())
            .unwrap_or_else(|| env::panic_str("Service not found"));
        require!(env::predecessor_account_id() == owner_id, "Predecessor must be token owner.");

        // TODO: Check that all current agent ids are updated / removes

        self.check_service_params(
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
        self.refund_deposit_to_account(storage, 0, env::predecessor_account_id(), true);

        // TODO: event
    }

    #[payable]
    pub fn activate_registration(
        &mut self,
        service_id: u32
    ) {
        // Check for service owner
        let owner_id = self.tokens
            .owner_by_id
            .get(&service_id.to_string())
            .unwrap_or_else(|| env::panic_str("Service not found"));
        require!(env::predecessor_account_id() == owner_id, "Predecessor must be token owner.");

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
        self.refund_deposit_to_account(storage, service.security_deposit, env::predecessor_account_id(), true);

        // TODO: event
    }

    #[payable]
    pub fn register_agents(
        &mut self,
        service_id: u32,
        agent_instances: Vec<AccountId>,
        agent_ids: Vec<u32>
    ) {
        // Check array lengths
        require!(agent_ids.len() == agent_instances.len());

        let operator = env::predecessor_account_id();

        // Record current storage usage
        let initial_storage_usage = env::storage_usage();

        // Get the service
        // TODO Check if service id exists?
        let mut service = self.services.get(&service_id).unwrap();

        // Check the service state
        require!(service.state == ServiceState::ActiveRegistration);

        // Initialize or get operator struct
        let mut operator_data = OperatorData{
            balance: 0 as u128,
            instances: Vector::new(StorageKey::AgentInstance)
        };
        match service.operators.get(&operator) {
            Some(v) => operator_data = v,
            None => {},
        }

        // Traverse agent instances and corresponding agent ids
        let mut total_bond = 0 as u128;
        for i in 0..agent_ids.len() {
            let agent_id = agent_ids[i];
            let agent_instance = agent_instances[i].clone();

            // Operator address must be different from agent instance one
            require!(operator != agent_instance);

            // Check account validity
            require!(env::is_valid_account_id(agent_instance.as_ref().as_bytes()));

            // Check if there is an empty slot for the agent instance in this specific service
            let mut agent_params = service.agent_params.get(&agent_id).unwrap();
            require!(agent_params.num_agent_instances > agent_params.instances.len() as u32);

            // Check that the agent instance address is unique across all services
            let res = self.agent_instance_operators.insert(&agent_instance, &operator);
            require!(res.is_none());

            // Add agent instance into corresponding maps
            agent_params.instances.push(&agent_instance);
            operator_data.instances.push(&agent_instance);
            service.agent_instances.insert(&agent_instance, &agent_id);

            // Increase the total number of agent instances in a service
            service.num_agent_instances += 1;

            // Increase the total bond
            total_bond = total_bond.saturating_add(agent_params.bond.into());
        }

        // If the service agent instance capacity is reached, the service registration is finished
        if service.num_agent_instances == service.max_num_agent_instances {
            service.state = ServiceState::FinishedRegistration;
        }

        // Update operator struct
        operator_data.balance = operator_data.balance.saturating_add(total_bond.into());

        // Increased storage
        let storage = env::storage_usage() - initial_storage_usage;
        // Consume storage and bond cost and refund the rest
        self.refund_deposit_to_account(storage, total_bond, env::predecessor_account_id(), true);

        // TODO: event
    }

    // TODO: needs to be payable?
    #[payable]
    pub fn deploy(
        &mut self,
        service_id: u32,
        name_multisig: AccountId
    ) {
        // Check for service owner
        let owner_id = self.tokens
            .owner_by_id
            .get(&service_id.to_string())
            .unwrap_or_else(|| env::panic_str("Service not found"));
        require!(env::predecessor_account_id() == owner_id, "Predecessor must be token owner.");

        // Get the service
        let service = self.services.get(&service_id).unwrap();

        // Check if the service is already terminated
        require!(service.state == ServiceState::FinishedRegistration);

        // Get all agent instances for the multisig
        let mut agent_instances = Vec::new();
        for a in service.agent_ids.iter() {
            agent_instances.extend(service.agent_params.get(&a).unwrap().instances.iter());
        }

        // Check if the multisig is not set => the service was never deployed
        // or if the multisig account does not the provided one => override current multisig with a new one
        let multisig = service.multisig;
        if multisig.is_none() || multisig.clone().unwrap() != name_multisig {
            // Create new multisig
            //log!("Calling external");
            // Create a promise to call TestToken.is_paused()
            let promise = multisig_factory::ext(MULTISIG_FACTORY.parse().unwrap())
                .with_static_gas(Gas(5 * TGAS))
                .with_attached_deposit(env::attached_deposit())
                .create(name_multisig.clone(), agent_instances.clone(), service.threshold as u64)
                .then(
                   // Create a promise to callback create_multisig_callback
                   Self::ext(env::current_account_id())
                       .with_static_gas(Gas(5 * TGAS))
                       .create_multisig_callback(service_id, name_multisig.clone())
                );
        } else {
            // Update multisig with the new owners set
            // Get multisig owners
            multisig2::ext(multisig.unwrap().clone())
                .with_static_gas(Gas(5 * TGAS))
                .get_members()
                // Compare multisig owners with the set of agent instances
                .then(
                   // Create a promise to callback update_multisig_callback
                   Self::ext(env::current_account_id())
                       .with_static_gas(Gas(5 * TGAS))
                       .update_multisig_callback(service_id, agent_instances.clone())
                );
        }
    }

    #[private] // Public - but only callable by env::current_account_id()
    pub fn create_multisig_callback(
        &self,
        service_id: u32,
        name_multisig: AccountId,
        #[callback_result] call_result: Result<(), PromiseError>,
    ) {
        // Check if the promise succeeded by calling the method outlined in external.rs
        if call_result.is_err() {
            //log!("There was an error contacting Hello NEAR");
        }

        // Get the service, record its multisig and update state
        let mut service = self.services.get(&service_id).unwrap();
        service.multisig = Some(name_multisig);
        service.state = ServiceState::Deployed;

        // TODO: event
    }

    #[private]
    pub fn update_multisig_callback(
        &self,
        service_id: u32,
        agent_instances: Vec<AccountId>,
        #[callback_result] call_result: Result<Vec<AccountId>, PromiseError>,
    ) -> bool {
        // Check if the promise succeeded by calling the method outlined in external.rs
        if call_result.is_err() {
            //log!("There was an error contacting Hello NEAR");
        }

        // Get the service, record its multisig and update state
        let mut service = self.services.get(&service_id).unwrap();

        // Check agent instances vs multisig members
        let multisig_members = call_result.unwrap();
        let matching = agent_instances.iter().zip(multisig_members.iter()).filter(|&(ai, mm)| ai == mm).count();
        let mut success = false;
        if matching == agent_instances.len() && matching == multisig_members.len() {
            // Update service state
            service.state = ServiceState::Deployed;
            success = true;
        }

        // Revert if the multisig members comparison fails
        require!(success);

        // TODO: event

        success
    }

    // TODO: needs to be payable?
    #[payable]
    pub fn slash(
        &mut self,
        agent_instances: Vec<AccountId>,
        amounts: Vec<u128>,
        service_id: u32
    ) {
        // Check array lengths
        require!(amounts.len() == agent_instances.len());

        // Record current storage usage
        let initial_storage_usage = env::storage_usage();

        // Get the service
        let service = self.services.get(&service_id).unwrap();

        // Check if the service is already terminated
        require!(service.state == ServiceState::Deployed);

        // Only the multisig of a correspondent address can slash its agent instances
        require!(service.multisig.unwrap() == env::predecessor_account_id());

        // Traverse all agent instances
        for i in 0..agent_instances.len() {
            let amount = amounts[i];
            let agent_instance = agent_instances[i].clone();

            // Get the operator and its balance
            let operator = self.agent_instance_operators.get(&agent_instance).unwrap();
            let mut operator_data = service.operators.get(&operator).unwrap();
            let mut balance = operator_data.balance;

            // Slash the balance of the operator, make sure it does not go below zero
            if amount >= balance {
                // We cannot add to the slashed amount more than the balance of the operator
                self.slashed_funds = self.slashed_funds.saturating_add(balance.into());
                balance = 0;
            } else {
                self.slashed_funds = self.slashed_funds.saturating_add(amount.into());
                balance = balance.saturating_sub(amount.into());
            }

            // Update the operator balance value
            operator_data.balance = balance;

            // TODO event
        }

        // Increased storage
        let storage = env::storage_usage() - initial_storage_usage;
        // TODO this is probably not needed as no storage is affected
        self.refund_deposit_to_account(storage, 0, env::predecessor_account_id(), true);
    }

    // TODO: needs to be payable?
    #[payable]
    pub fn terminate(
        &mut self,
        service_id: u32
    ) {
        // Check for service owner
        let owner_id = self.tokens
            .owner_by_id
            .get(&service_id.to_string())
            .unwrap_or_else(|| env::panic_str("Service not found"));
        require!(env::predecessor_account_id() == owner_id, "Predecessor must be token owner.");

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

        // Remove agent instances data from agent params
        for a in service.agent_ids.iter() {
            service.agent_params.get(&a).unwrap().instances.clear();
        }
        
        // TODO: Calculate refund of freed storage

        // Increased storage
        // TODO: check if this is zero, as no storage is supposedly increased
        // TODO: This will mostly likely fail as the storage must decrease
        let storage = env::storage_usage() - initial_storage_usage;
        // Send the deposit back to the service owner
        self.refund_deposit_to_account(storage, service.security_deposit, env::predecessor_account_id(), false);

        // TODO: event
    }

    #[payable]
    pub fn unbond(&mut self, service_id: u32) {
        // Get the operator account
        let operator = env::predecessor_account_id();

        // Record current storage usage
        let initial_storage_usage = env::storage_usage();

        // Get the service
        let mut service = self.services.get(&service_id).unwrap();

        // Check the service state
        require!(service.state == ServiceState::TerminatedBonded);

        // Get the operator struct
        let operator_data = service.operators.get(&operator).unwrap_or_else(|| env::panic_str("Operator has no instances"));

        // Decrease the total number of agent instances in a service
        service.num_agent_instances -= operator_data.instances.len() as u32;

        // When number of instances is equal to zero, all the operators have unbonded and the service is moved into
        // the PreRegistration state, from where it can be updated / initiate registration / get deployed again
        if service.num_agent_instances == 0 {
            service.state = ServiceState::PreRegistration;
        }

        // Calculate registration refund and clear all operator agent instances in thi service
        let mut refund = 0 as u128;
        for i in 0..operator_data.instances.len() {
            // Get agent id by the agent instance
            let agent_id = service.agent_instances.get(&operator_data.instances.get(i).unwrap()).unwrap();
            // Get agent bond by agent id
            let bond = service.agent_params.get(&agent_id).unwrap().bond;
            // Add bond to the refund
            refund = refund.saturating_add(bond.into());

            // Remove the relevant data
            self.agent_instance_operators.remove(&operator_data.instances.get(i).unwrap());
            service.agent_instances.remove(&operator_data.instances.get(i).unwrap());
        }
        // Check if the refund exceeds operator's balance
        // This situation is possible if the operator was slashed for the agent instance misbehavior
        if refund > operator_data.balance {
            refund = operator_data.balance;
        }

        // Remove the operator data from current service
        service.operators.remove(&operator);

        // Increased storage
        // TODO: need to correctly recalculate the storage decrease
        let storage = env::storage_usage() - initial_storage_usage;
        // Refund storage, bond cost and the rest
        self.refund_deposit_to_account(storage, refund, env::predecessor_account_id(), false);

        // TODO: event
    }

    // TODO Shall this be payable as 1 yocto is needed for?
    pub fn drain(&mut self) {
        // Check the ownership
        require!(self.owner == env::predecessor_account_id());

        let amount = self.slashed_funds;
        // TODO: 1 or 0 here?
        if amount > 1 {
            self.slashed_funds = 0;
            Promise::new(env::predecessor_account_id()).transfer(amount);
        }

        // TODO: event
    }

//     pub fn get_service(&self, service_id: u32) -> Service {
//         self.services.get(&service_id).unwrap_or_default()
//     }
//
//     pub fn get_agent_params(&self, service_id: u32) -> Vec<AgentParams> {
//         let mut agent_params = Vec::new();
//
//         // Get the service
//         let service = self.services.get(&service_id).unwrap_or_default();//unwrap_or_else(|| env::panic_str("Service not found"));
//         agent_params
//     }

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
//     pub fn mint(&mut self, account_id: AccountId, amount: u128) {
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

    pub fn get_token_metadata(&self, token_id: u32) -> Option<TokenMetadata> {
        self.tokens.token_metadata_by_id.as_ref().and_then(|by_id| by_id.get(&token_id.to_string()))
    }

    pub fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_owned()
    }
}

// near_contract_standards::impl_non_fungible_token_core!(ServiceRegistry, tokens);
// near_contract_standards::impl_non_fungible_token_approval!(ServiceRegistry, tokens);
// near_contract_standards::impl_non_fungible_token_enumeration!(ServiceRegistry, tokens);
//
// #[near_bindgen]
// impl NonFungibleTokenMetadataProvider for ServiceRegistry {
//     fn nft_metadata(&self) -> NFTContractMetadata {
//         self.metadata.get().unwrap()
//     }
// }
