use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, TokenMetadata, NonFungibleTokenMetadataProvider, NFT_METADATA_SPEC,
};
use near_contract_standards::non_fungible_token::enumeration::NonFungibleTokenEnumeration;
use near_contract_standards::non_fungible_token::{NonFungibleToken, Token};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Serialize, Deserialize};
use near_sdk::json_types::{Base64VecU8, U128, Base58PublicKey};
use near_sdk::serde_json::json;
use near_sdk::collections::LazyOption;
use near_sdk::{
    env, near, require, AccountId, BorshStorageKey, PanicOnDefault, Promise, PromiseOrValue, StorageUsage, Gas,
    IntoStorageKey, PromiseError, NearToken, log
};
use near_sdk::store::{LookupMap, Vector};
use near_sdk::ext_contract;
//use near_account_id::{AccountId};
//near_token::NearToken
//use near_gas::NearGas;

// pub mod external;
// pub use crate::external::*;

// const NO_DEPOSIT: Balance = 0;
// const CURRENT_STATE_VERSION: u32 = 1;
// const TGAS: u64 = 1_000_000_000_000;

// #[near(serializers=[borsh])]
// #[derive(Clone, Serialize, Deserialize)]
// #[serde(crate = "near_sdk::serde", untagged)]
// pub struct MutisigAccountMember {
//     account_id: AccountId,
// }

#[derive(Serialize, Deserialize, PartialEq)]
#[serde(crate = "near_sdk::serde", untagged)]
pub enum MultisigMember {
    AccessKey { public_key: Base58PublicKey },
    Account { account_id: AccountId },
}

// MultisigFactory interface
#[ext_contract(multisig_factory)]
trait MultisigFactory {
    #[payable]
    fn create(
        &mut self,
        name: AccountId,
        members: Vec<MultisigMember>,
        num_confirmations: u64,
    ) -> Promise;
}


// Multisig2 interface
#[ext_contract(multisig2)]
trait Multisig2 {
    fn get_members(&self) -> Vec<MultisigMember>;
}

// FungibleToken interface
#[ext_contract(fungible_token)]
trait FungibleToken {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
}


#[near(serializers=[borsh])]
#[derive(PartialEq, Clone)]
pub enum ServiceState {
    NonExistent,
    PreRegistration,
    ActiveRegistration,
    FinishedRegistration,
    Deployed,
    TerminatedBonded
}

#[near(serializers=[borsh])]
pub struct AgentParams {
    pub num_agent_instances: u32,
    pub bond: u128,
    pub instances: Vector<AccountId>
}

#[near(serializers=[borsh])]
pub struct OperatorData {
    pub balance: u128,
    pub instances: Vector<AccountId>
}

#[near(serializers=[borsh])]
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

const CALL_GAS: Gas = Gas::from_tgas(5);
const CREATE_CALL_GAS: Gas = Gas::from_tgas(100);

#[near(contract_state)]
pub struct ServiceRegistry {
    owner: AccountId,
    services: LookupMap<u32, Service>,
    tokens: NonFungibleToken,
    metadata: LazyOption<NFTContractMetadata>,
    token_balances: LookupMap<AccountId, LookupMap<AccountId, u128>>,
    agent_instance_operators: LookupMap<AccountId, AccountId>,
    paused: bool,
    multisig_factory: AccountId,
    balance: u128,
    slashed_funds: u128
}

const DATA_IMAGE_SVG_NEAR_ICON: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 288 288'%3E%3Cg id='l' data-name='l'%3E%3Cpath d='M187.58,79.81l-30.1,44.69a3.2,3.2,0,0,0,4.75,4.2L191.86,103a1.2,1.2,0,0,1,2,.91v80.46a1.2,1.2,0,0,1-2.12.77L102.18,77.93A15.35,15.35,0,0,0,90.47,72.5H87.34A15.34,15.34,0,0,0,72,87.84V201.16A15.34,15.34,0,0,0,87.34,216.5h0a15.35,15.35,0,0,0,13.08-7.31l30.1-44.69a3.2,3.2,0,0,0-4.75-4.2L96.14,186a1.2,1.2,0,0,1-2-.91V104.61a1.2,1.2,0,0,1,2.12-.77l89.55,107.23a15.35,15.35,0,0,0,11.71,5.43h3.13A15.34,15.34,0,0,0,216,201.16V87.84A15.34,15.34,0,0,0,200.66,72.5h0A15.35,15.35,0,0,0,187.58,79.81Z'/%3E%3C/g%3E%3C/svg%3E";

#[derive(BorshStorageKey, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
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
    AgentInstanceOperator,
    CustomToken,
    TokenBalances
}

#[near]
impl ServiceRegistry {
    /// Initializes the contract owned by `owner_id` with
    /// default metadata (for example purposes only).
    #[init]
    pub fn new_default_meta(owner_id: AccountId, multisig_factory: AccountId) -> Self {
        Self::new(
            owner_id,
            multisig_factory,
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
    pub fn new(owner_id: AccountId, multisig_factory: AccountId, metadata: NFTContractMetadata) -> Self {
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
            token_balances: LookupMap::new(StorageKey::CustomToken),
            paused: false,
            multisig_factory,
            balance: 0 as u128,
            slashed_funds: 0 as u128
        }
    }

    fn refund_deposit_to_account(&self, storage_used: u64, service_deposit: u128, account_id: AccountId, deposit_in: bool) {
        log!("storage used: {}", storage_used);
        let near_deposit = NearToken::from_yoctonear(service_deposit);
        let mut required_cost = env::storage_byte_cost().saturating_mul(storage_used.into());
        required_cost = required_cost.saturating_add(near_deposit);

        let mut refund = env::attached_deposit();
        // Deposit is added on a balance
        if deposit_in {
            // Required cost must not be bigger than the attached deposit
            require!(required_cost <= refund);
            refund = refund.saturating_sub(required_cost);
        } else {
            // This could be the case if the storage price went up during the lifespan of the service
            require!(required_cost <= env::account_balance());
            refund = refund.saturating_add(required_cost);
        }
        //log!("required cost: {}", required_cost.as_yoctonear());
        log!("refund: {}", refund.as_yoctonear());
        log!("balance: {}", env::account_balance().as_yoctonear());
        if refund.as_yoctonear() > 1 {
            Promise::new(account_id).transfer(refund);
        }
    }

    pub fn change_owner(&mut self, new_owner: AccountId) {
        // Check the ownership
        require!(self.owner == env::predecessor_account_id());

        // Check account validity
        require!(env::is_valid_account_id(new_owner.as_bytes()));

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
        // Check array lengths
        require!(agent_ids.len() > 0);
        require!(agent_ids.len() == agent_bonds.len());
        require!(agent_ids.len() == agent_num_instances.len());

        // Check config hash
        require!(!config_hash.into_iter().all(|h| h == 0));

        // Check uniqueness of agent ids: sorted agent ids must match its size with the original array
        let mut check_agent_ids = agent_ids.clone();
        check_agent_ids.sort_unstable();
        check_agent_ids.dedup();
        require!(check_agent_ids.len() == agent_ids.len());
        //let v: Vec<_> = agent_ids.into_iter().unique().collect();

        // Check non-zero agent Ids
        require!(agent_ids.into_iter().all(|id| id > 0));
    }

    fn fill_service_params(
        &mut self,
        service_owner: AccountId,
        service_id: u32,
        token: Option<AccountId>,
        config_hash: [u8; 32],
        agent_ids: Vec<u32>,
        agent_num_instances: Vec<u32>,
        agent_bonds: Vec<u128>,
        threshold: u32
    ) {
        // Get the service
        let service = self.services.get_mut(&service_id).unwrap();

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
                service.agent_ids.push(agent_id);

                service.agent_params.insert(
                    agent_id,
                    AgentParams{
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
        // Record the state of collections
        service.agent_ids.flush();
        service.agent_params.flush();

        service.security_deposit = security_deposit;
        service.max_num_agent_instances = max_num_agent_instances;

        // Check the token field and register the service owner, if required
        if token.is_some() && token != service.token {
            // Initialize or get registered token map
            let token_balances = self
                .token_balances
                // Get token map
                .entry(token.clone().unwrap())
                // or create a new one if not
                .or_insert(LookupMap::new(StorageKey::TokenBalances));

            // Check if the service owner is registered
            if !token_balances.contains_key(&service_owner) {
                token_balances.set(service_owner, Some(0));
                token_balances.flush();
            }
            self.token_balances.flush();
            service.token = token;
        }

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
            service.config_hashes.push(config_hash);
            service.config_hashes.flush();
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
        self.tokens.internal_mint_with_refund(service_id.to_string().clone(), service_owner.clone(), Some(metadata), None);

        // Allocate the service
        self.services.insert(
            service_id,
            Service {
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
            service_owner,
            service_id,
            token,
            config_hash,
            agent_ids.clone(),
            agent_num_instances.clone(),
            agent_bonds.clone(),
            threshold
        );
        // Record service map state
        self.services.flush();

        // Increased storage
//         log!("initial storage usage {}", initial_storage_usage);
//         log!("storage usage after {}", env::storage_usage());
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

        // Check that all current agent ids are updated / removed to correspond the CRUD way
        let service = self.services.get(&service_id).unwrap();
        require!(service.agent_ids.iter().all(|ai| agent_ids.contains(ai)), "Not all agent Ids are updated");

        self.check_service_params(
            config_hash,
            agent_ids.clone(),
            agent_num_instances.clone(),
            agent_bonds.clone()
        );

        // Fill in the service parameters
        self.fill_service_params(
            owner_id,
            service_id,
            token,
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
        service_id: u32,
        account_id: Option<AccountId>
    ) {
        let service_owner = account_id.unwrap_or_else(env::predecessor_account_id);

        // Check for service owner
        let owner_id = self.tokens
            .owner_by_id
            .get(&service_id.to_string())
            .unwrap_or_else(|| env::panic_str("Service not found"));
        require!(service_owner == owner_id, "Predecessor must be token owner.");

        // Get the service
        let service = self.services.get_mut(&service_id).unwrap();

        // Check the service state
        require!(service.state == ServiceState::PreRegistration);

        // Update service state
        service.state = ServiceState::ActiveRegistration;

        let security_deposit = service.security_deposit;

        // Increased storage
//         log!("storage usage after {}", env::storage_usage());

        if service.token.is_none() {
            // Update registry native token balance
            self.balance = self.balance.saturating_add(security_deposit.into());
            self.refund_deposit_to_account(0, security_deposit, env::predecessor_account_id(), true);
        } else {
            // Get token balance for the service owner and reduce it by a security deposit value
            if let Some(b) = self
                .token_balances
                .get_mut(&service.token.clone().unwrap())
                .unwrap_or_else(|| env::panic_str("Token not registered"))
                .get_mut(&owner_id)
            {
                // Decrease by the security deposit amount
                if *b < security_deposit {
                    env::panic_str("Not enough token deposit");
                }
                *b -= security_deposit;
            } else {
                // Fail otherwise
                env::panic_str("Sender not registered");
            }
        }

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
        let service = self.services.get_mut(&service_id).unwrap();

        // Check the service state
        require!(service.state == ServiceState::ActiveRegistration);

        // Initialize or get operator struct
        let operator_data = service
            .operators
            // Get operator struct
            .entry(operator.clone())
            // or create a new one if not
            .or_insert(OperatorData{
                balance: 0 as u128,
                instances: Vector::new(StorageKey::AgentInstance)
            });

        // Traverse agent instances and corresponding agent ids
        let mut total_bond = 0 as u128;
        for i in 0..agent_ids.len() {
            // Operator address must be different from agent instance one
            require!(operator != agent_instances[i]);

            // Check account validity
            require!(env::is_valid_account_id(agent_instances[i].as_bytes()));

            // Check if there is an empty slot for the agent instance in this specific service
            let agent_params = service.agent_params.get_mut(&agent_ids[i]).unwrap();
            require!(agent_params.num_agent_instances > agent_params.instances.len() as u32);

            // Check that the agent instance address is unique across all services
            let res = self.agent_instance_operators.insert(agent_instances[i].clone(), operator.clone());
            require!(res.is_none());

            // Add agent instance into corresponding maps
            agent_params.instances.push(agent_instances[i].clone());
            agent_params.instances.flush();
            operator_data.instances.push(agent_instances[i].clone());
            operator_data.instances.flush();
            service.agent_instances.insert(agent_instances[i].clone(), agent_ids[i]);

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

        service.agent_params.flush();
        service.operators.flush();
        service.agent_instances.flush();
        self.agent_instance_operators.flush();

        // Increased storage
//         log!("initial storage usage {}", initial_storage_usage);
//         log!("storage usage after {}", env::storage_usage());
        let storage = env::storage_usage() - initial_storage_usage;

        if service.token.is_some() {
            // Get token balance for the operator and reduce it by a total bond value
            if let Some(b) = self
                .token_balances
                .get_mut(&service.token.clone().unwrap())
                .unwrap_or_else(|| env::panic_str("Token not registered"))
                .get_mut(&operator)
            {
                // Decrease by the security deposit amount
                if *b < total_bond {
                    env::panic_str("Not enough token deposit");
                }
                *b -= total_bond;
            } else {
                // Fail otherwise
                env::panic_str("Sender not registered");
            }

            // Security deposit is set to zero since it was deposited via token transfer already
            total_bond = 0;
        } else {
            // Update native token balance
            self.balance = self.balance.saturating_add(total_bond.into());
        }

        // Consume storage and bond cost and refund the rest
        self.refund_deposit_to_account(storage, total_bond, env::predecessor_account_id(), true);

        // TODO: event
    }

    pub fn get_multisig_members(&self, name_multisig: AccountId) -> Promise {
        // Update multisig with the new owners set
        // Get multisig owners
        multisig2::ext(name_multisig)
            .with_static_gas(CALL_GAS)
            .get_members()
            // Compare multisig owners with the set of agent instances
            .then(
               // Create a promise to callback update_multisig_callback
               Self::ext(env::current_account_id())
                   .with_static_gas(CALL_GAS)
                   .check_members()
            )
    }

    #[private]
    pub fn check_members(
        &self,
        #[callback_result] call_result: Result<Vec<MultisigMember>, PromiseError>,
    ) -> u64 {
        // Check if the promise succeeded by calling the method outlined in external.rs
        if call_result.is_err() {
            env::panic_str("Multisig check failed");
        }

        call_result.unwrap().len() as u64
    }

    // TODO: needs to be payable?
    #[payable]
    pub fn deploy(
        &mut self,
        service_id: u32,
        name_multisig: AccountId
    ) -> Promise {
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

        // Check account validity
        require!(env::is_valid_account_id(name_multisig.as_bytes()));

        // Get all agent instances for the multisig
        let mut agent_instances = Vec::new();
        for ai in service.agent_ids.iter() {
            //agent_instances.extend(service.agent_params.get(ai).unwrap().instances.iter().cloned());
            let instances = &service.agent_params.get(ai).unwrap().instances;
            for inst in instances.iter() {
                agent_instances.push(MultisigMember::Account{account_id: inst.clone()});
            }
        }

        let is_sub_account = name_multisig.is_sub_account_of(&self.multisig_factory);
        // Check if the multisig name is a full account of a factory, or a short name for the factory to create it with
        // If not a factory multisig name, create a new multisig instance
        if !is_sub_account {
            // The multisig account must not have any predecessors
            require!(name_multisig.get_parent_account_id().is_none());

            // Create new multisig
            //log!("Calling external");
            multisig_factory::ext(self.multisig_factory.clone())
                .with_static_gas(CREATE_CALL_GAS)
                .with_attached_deposit(env::attached_deposit())
                .create(name_multisig.clone(), agent_instances, service.threshold as u64)
                .then(
                    // Create a promise to callback create_multisig_callback
                    Self::ext(env::current_account_id())
                        .with_static_gas(CALL_GAS)
                        .create_multisig_callback(service_id, name_multisig.clone())
                )
        } else {
            // Deposit must be zero in this scenario
            require!(env::attached_deposit() == NearToken::from_yoctonear(0), "Deposit is not required");

            // Update multisig with the new owners set
            // Get multisig owners
            multisig2::ext(name_multisig.clone())
                .with_static_gas(CALL_GAS)
                .get_members()
                // Compare multisig owners with the set of agent instances
                .then(
                   // Create a promise to callback update_multisig_callback
                   Self::ext(env::current_account_id())
                       .with_static_gas(CALL_GAS)
                       .update_multisig_callback(service_id, name_multisig.clone(), agent_instances)
                )
        }
    }

    #[private] // Public - but only callable by env::current_account_id()
    pub fn create_multisig_callback(
        &mut self,
        service_id: u32,
        name_multisig: AccountId,
        #[callback_result] call_result: Result<(), PromiseError>,
    ) {
        // Check if the promise succeeded by calling the method outlined in external.rs
        if call_result.is_err() {
            env::panic_str("Multisig creation failed");

            // TODO refund
        }

        // Get the service, record its multisig and update state
        let service = self.services.get_mut(&service_id).unwrap();
        service.multisig = Some(name_multisig);
        service.state = ServiceState::Deployed;

        // TODO: event
    }

    #[private]
    pub fn update_multisig_callback(
        &mut self,
        service_id: u32,
        name_multisig: AccountId,
        agent_instances: Vec<MultisigMember>,
        #[callback_result] call_result: Result<Vec<MultisigMember>, PromiseError>,
    ) -> bool {
        // Check if the promise succeeded by calling the method outlined in external.rs
        if call_result.is_err() {
            env::panic_str("Multisig update failed");

            // TODO refund
        }

        // Get the service, record its multisig and update state
        let service = self.services.get_mut(&service_id).unwrap();

        let mut success = false;
        // Check agent instances vs multisig members
        let multisig_members = call_result.unwrap();
        let matching = agent_instances.iter().zip(multisig_members.iter()).filter(|&(ai, mm)| ai == mm).count();
        //let matching = agent_instances.iter().zip(multisig_members.iter()).filter(|&(ai, mm)| match ai {MultisigMember::Account(value) => value == mm, _ => {false}}).count();
        if matching == agent_instances.len() && matching == multisig_members.len() {
            service.multisig = Some(name_multisig);
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

        // Get the service
        let service = self.services.get_mut(&service_id).unwrap();

        // Check if the service is already terminated
        require!(service.state == ServiceState::Deployed);

        // Only the multisig of a correspondent address can slash its agent instances
        require!(service.multisig.clone().unwrap() == env::predecessor_account_id());

        // Traverse all agent instances
        for i in 0..agent_instances.len() {
            let amount = amounts[i];
            let agent_instance = agent_instances[i].clone();

            // Get the operator and its balance
            let operator = self.agent_instance_operators.get(&agent_instance).unwrap();
            let operator_data = service.operators.get_mut(operator).unwrap();
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
        let service = self.services.get_mut(&service_id).unwrap();

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
            let instances = &mut service.agent_params.get_mut(a).unwrap().instances;
            instances.clear();
            instances.flush();
        }

        // Update registry balance
        let mut refund = service.security_deposit;
        self.balance = self.balance.saturating_sub(refund.into());

        // TODO: Calculate refund of freed storage
        //log!("initial storage usage {}", initial_storage_usage);
        //log!("storage usage after {}", env::storage_usage());

        if service.token.is_some() {
            // Send the token refund back to the service owner
            fungible_token::ext(service.token.clone().unwrap())
                .with_static_gas(CALL_GAS)
                .ft_transfer(owner_id, U128::from(refund), None);

            // Zero the refund since it has been already sent back
            refund = 0;
        }

        // Decreased storage
        let storage = initial_storage_usage - env::storage_usage();
        // Send the deposit back to the service owner
        self.refund_deposit_to_account(storage, refund, env::predecessor_account_id(), false);

        // TODO: event
    }

    #[payable]
    pub fn unbond(&mut self, service_id: u32) {
        // Get the operator account
        let operator = env::predecessor_account_id();

        // Record current storage usage
        let initial_storage_usage = env::storage_usage();

        // Get the service
        let service = self.services.get_mut(&service_id).unwrap();

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
            let agent_id = service.agent_instances.get(operator_data.instances.get(i).unwrap()).unwrap();
            // Get agent bond by agent id
            let bond = service.agent_params.get(&agent_id).unwrap().bond;
            // Add bond to the refund
            refund = refund.saturating_add(bond.into());

            // Remove the relevant data
            self.agent_instance_operators.remove(operator_data.instances.get(i).unwrap());
            service.agent_instances.remove(operator_data.instances.get(i).unwrap());
        }
        self.agent_instance_operators.flush();
        service.agent_instances.flush();

        // Check if the refund exceeds operator's balance
        // This situation is possible if the operator was slashed for the agent instance misbehavior
        if refund > operator_data.balance {
            refund = operator_data.balance;
        }

        // Remove the operator data from current service
        service.operators.remove(&operator);
        service.operators.flush();

        // Update registry balance
        self.balance = self.balance.saturating_sub(refund.into());

        if service.token.is_some() {
            // Send the token refund back to the service owner
            fungible_token::ext(service.token.clone().unwrap())
                .with_static_gas(CALL_GAS)
                .ft_transfer(operator, U128::from(refund), None);

            // Zero the refund since it has been already sent back
            refund = 0;
        }

//         log!("initial storage usage {}", initial_storage_usage);
//         log!("storage usage after {}", env::storage_usage());
        // Increased storage
        // TODO: need to correctly recalculate the storage decrease
        let storage = initial_storage_usage - env::storage_usage();
        // Refund storage, bond cost and the rest
        self.refund_deposit_to_account(storage, refund, env::predecessor_account_id(), false);

        // TODO: event
    }

    pub fn withdraw(&mut self, token: AccountId) {
        // Get the sender balance
        let sender_id = env::predecessor_account_id();
        if let Some(b) = self
            .token_balances
            .get_mut(&token)
            .unwrap_or_else(|| env::panic_str("Token not registered"))
            .get_mut(&sender_id)
        {
            // Set the balance to zero
            let refund = *b;
            *b = 0;

            // Send tokens back to the sender
            fungible_token::ext(token)
                .with_static_gas(CALL_GAS)
                .ft_transfer(sender_id.clone(), U128::from(refund), None);

            // Return excess of amount
            self.refund_deposit_to_account(0, 0, sender_id.clone(), false);
        } else {
            // Fail otherwise
            env::panic_str("Sender not registered");
        }
    }

    // TODO Shall this be payable as 1 yocto is needed for?
    pub fn drain(&mut self) {
        // Check the ownership
        require!(self.owner == env::predecessor_account_id());

        let amount = self.slashed_funds;
        // TODO: 1 or 0 here?
        if amount > 1 {
            self.slashed_funds = 0;
            Promise::new(env::predecessor_account_id()).transfer(NearToken::from_yoctonear(amount));
        }

        // TODO: event
    }

    // Call by the operator
    #[payable]
    pub fn storage_deposit(&mut self, account_id: Option<AccountId>, token: AccountId) {
        let initial_storage_usage = env::storage_usage();

        let sender_id = account_id.unwrap_or_else(env::predecessor_account_id);

        // Check the token field and register account, if required
        // Initialize or get registered token map
        let token_balances = self
            .token_balances
            // Get token map
            .entry(token)
            // or create a new one if not
            .or_insert(LookupMap::new(StorageKey::TokenBalances));

        // Check if the service owner is registered
        if !token_balances.contains_key(&sender_id) {
            token_balances.set(sender_id.clone(), Some(0));
            token_balances.flush();
        }
        self.token_balances.flush();

        let storage = env::storage_usage() - initial_storage_usage;
        // Pay for the storage and refund excessive amount
        self.refund_deposit_to_account(storage, 0, sender_id, true);
    }

    #[payable]
    pub fn storage_withdraw(&mut self, token: AccountId) {
        let initial_storage_usage = env::storage_usage();

        let account_id = env::predecessor_account_id();

        // Get the token balance
        if let Some(b) = self
            .token_balances
            .get_mut(&token)
            .unwrap_or_else(|| env::panic_str("Token not registered"))
            .get_mut(&account_id)
        {
            // The balance must be zero
            require!(*b == 0);

        } else {
            // Fail otherwise
            env::panic_str("Sender not registered");
        }

        // Remove account storage associated with the token
        self.token_balances.get_mut(&token).unwrap().remove(&account_id);
        self.token_balances.flush();

        let storage = initial_storage_usage - env::storage_usage();
        // Send the storage released cost back to the sender
        self.refund_deposit_to_account(storage, 0, account_id, false);
    }

    pub fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let token = env::predecessor_account_id();

        // Get token balance the sender
        if let Some(b) = self
            .token_balances
            .get_mut(&token)
            .unwrap_or_else(|| env::panic_str("Token not registered"))
            .get_mut(&sender_id)
        {
            // Increase for the provided amount
            *b += amount.0;
        } else {
            // Fail otherwise
            env::panic_str("Sender not registered");
        }

//         log!("Increased the token amount! {}", amount.0);

        // No tokens will be returned
        PromiseOrValue::Value(U128::from(0))
    }

    // TODO: unwrap or else or default panic message is ok?
    pub fn get_service_state(&self, service_id: u32) -> u8 {
        self.services.get(&service_id).unwrap().state.clone() as u8
    }

    pub fn get_service_multisig(&self, service_id: u32) -> AccountId {
        self.services.get(&service_id).unwrap().multisig.clone().unwrap()
    }

    pub fn get_service_config_hash(&self, service_id: u32) -> [u8; 32] {
        *self.services.get(&service_id).unwrap().config_hashes.iter().last().unwrap()
    }

    pub fn get_service_previous_config_hashes(&self, service_id: u32) -> Vec<[u8; 32]> {
        // Get config_hashes vector in reverse order without the first element, which is the current config hash
        self.services.get(&service_id).unwrap().config_hashes.iter().rev().skip(1).cloned().collect()
    }

    pub fn get_agent_ids(&self, service_id: u32) -> Vec<u32> {
        self.services.get(&service_id).unwrap().agent_ids.iter().cloned().collect()
    }

    pub fn get_service_agent_params_num_instances(&self, service_id: u32) -> Vec<u32> {
        let mut agent_params_num_agent_instances = Vec::new();

        // Get the service
        // TODO: unwrap or else or leave just unwrap
        let service = self.services.get(&service_id).unwrap_or_else(|| env::panic_str("Service not found"));
        for ai in service.agent_ids.iter() {
            agent_params_num_agent_instances.push(service.agent_params.get(&ai).unwrap().num_agent_instances);
        }
        agent_params_num_agent_instances
    }

    pub fn get_service_agent_params_bonds(&self, service_id: u32) -> Vec<u128> {
        let mut agent_params_bonds = Vec::new();

        // Get the service
        let service = self.services.get(&service_id).unwrap_or_else(|| env::panic_str("Service not found"));
        for ai in service.agent_ids.iter() {
            agent_params_bonds.push(service.agent_params.get(&ai).unwrap().bond);
        }
        agent_params_bonds
    }

    // Get all agent instances of the service
    pub fn get_service_agent_instances(&self, service_id: u32) -> Vec<AccountId> {
        let mut agent_instances = Vec::new();
        // Get the service
        let service = self.services.get(&service_id).unwrap_or_else(|| env::panic_str("Service not found"));
        for ai in service.agent_ids.iter() {
            agent_instances.extend(service.agent_params.get(ai).unwrap().instances.iter().cloned());
        }
        agent_instances
    }

    pub fn get_instances_for_agent_id(&self, service_id: u32, agent_id: u32) -> Vec<AccountId> {
        // TODO: concatenate
        // Get the service
        let service = self.services.get(&service_id).unwrap_or_else(|| env::panic_str("Service not found"));
        // Get agent instances for a specified agent Id
        service.agent_params.get(&agent_id).unwrap_or_else(|| env::panic_str("Agent not found")).instances.iter().cloned().collect()
    }

    pub fn get_operator_balance(&self, operator: AccountId, service_id: u32) -> u128 {
        // TODO: concatenate
        // Get the service
        let service = self.services.get(&service_id).unwrap_or_else(|| env::panic_str("Service not found"));
        // Get operator balance for a specified service
        service.operators.get(&operator).unwrap_or_else(|| env::panic_str("Operator not found")).balance
    }

    pub fn get_operator_service_agent_instances(&self, operator: AccountId, service_id: u32) -> Vec<AccountId> {
        // TODO: concatenate
        // Get the service
        let service = self.services.get(&service_id).unwrap_or_else(|| env::panic_str("Service not found"));
        // Get agent instances for a specified agent Id
        service.operators.get(&operator).unwrap_or_else(|| env::panic_str("Operator not found")).instances.iter().cloned().collect()
    }

    pub fn get_registry_balance(&self) -> u128 {
        self.balance
    }

    pub fn get_registry_slashed_funds(&self) -> u128 {
        self.slashed_funds
    }

    pub fn get_storage_usage(&self) -> u64 {
        env::storage_usage()
    }

    pub fn get_storage_price(&self) -> u128 {
        env::storage_byte_cost().saturating_mul(env::storage_usage().into()).as_yoctonear()
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

    // TODO convert to u32?
    pub fn total_supply(&self) -> U128 {
        self.tokens.nft_total_supply()
    }

    pub fn get_token_metadata(&self, service_id: u32) -> Option<TokenMetadata> {
        self.tokens.token_metadata_by_id.as_ref().and_then(|by_id| by_id.get(&service_id.to_string()))
    }

    pub fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_owned()
    }
}

// impl Default for AccountId {
//     fn default() -> Self {
//         Self {
//             account_id: "aaa";
//         }
//     }
// }
//
impl Default for ServiceRegistry {
    fn default() -> Self {
        Self {
            owner: "".parse().unwrap(),
            services: LookupMap::new(StorageKey::Service),
            tokens: NonFungibleToken::new(
                StorageKey::NonFungibleToken,
                "".parse().unwrap(),
                Some(StorageKey::TokenMetadata),
                Some(StorageKey::Enumeration),
                Some(StorageKey::Approval),
            ),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&NFTContractMetadata {
                spec: Default::default(),
                name: Default::default(),
                symbol: Default::default(),
                icon: None,
                base_uri: None,
                reference: None,
                reference_hash: None,
            })),
            agent_instance_operators: LookupMap::new(StorageKey::AgentInstanceOperator),
            token_balances: LookupMap::new(StorageKey::CustomToken),
            paused: Default::default(),
            multisig_factory: "".parse().unwrap(),
            balance: Default::default(),
            slashed_funds: Default::default()
        }
    }

//     /// This function can only be called from the factory or from the contract itself.
//     #[init(ignore_state)]
//     pub fn migrate(from_version: u32) -> Self {
//         if from_version == 0 {
//             // Adding icon as suggested here: https://nomicon.io/Standards/FungibleToken/Metadata.html
//             let old_state: BridgeTokenV0 = env::state_read().expect("Contract isn't initialized");
//             let new_state: BridgeToken = old_state.into();
//             assert!(new_state.controller_or_self());
//             new_state
//         } else {
//             env::state_read().unwrap()
//         }
//     }
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
