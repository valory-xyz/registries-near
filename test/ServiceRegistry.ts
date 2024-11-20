import {Worker, NEAR, NearAccount} from "near-workspaces";
import anyTest, {TestFn} from "ava";

const serviceId = 1;
const configHash = Array(32).fill(5);
const configHash2 = Array(32).fill(9);
const agentIds = [1];
const agentNumInstances = [1];
const agentBonds = [1000];
const threshold = 1;

const defaultContractMetadata = {
    spec: "nft-1.0.0", // NFT_METADATA_SPEC from near_contract_standards::non_fungible_token::metadata
    name: "Service Registry NFT",
    symbol: "SR",
    icon: "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 288 288'%3E%3Cg id='l' data-name='l'%3E%3Cpath d='M187.58,79.81l-30.1,44.69a3.2,3.2,0,0,0,4.75,4.2L191.86,103a1.2,1.2,0,0,1,2,.91v80.46a1.2,1.2,0,0,1-2.12.77L102.18,77.93A15.35,15.35,0,0,0,90.47,72.5H87.34A15.34,15.34,0,0,0,72,87.84V201.16A15.34,15.34,0,0,0,87.34,216.5h0a15.35,15.35,0,0,0,13.08-7.31l30.1-44.69a3.2,3.2,0,0,0-4.75-4.2L96.14,186a1.2,1.2,0,0,1-2-.91V104.61a1.2,1.2,0,0,1,2.12-.77l89.55,107.23a15.35,15.35,0,0,0,11.71,5.43h3.13A15.34,15.34,0,0,0,216,201.16V87.84A15.34,15.34,0,0,0,200.66,72.5h0A15.35,15.35,0,0,0,187.58,79.81Z'/%3E%3C/g%3E%3C/svg%3E",
    base_uri: "https://gateway.autonolas.tech/ipfs/"
}

const defaultServiceMetadata = {
    title: "Service Name",
    description: "Service Description",
    media: "",
    media_hash: "",
    copies: 1,
    issued_at: "",
    expires_at: "",
    starts_at: "",
    updated_at: "",
    extra: "",
    reference: "",
    reference_hash: "",
}


const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
}>;

test.beforeEach(async t => {
    // Init the worker and start a Sandbox server
    const worker = await Worker.init();


    // Prepare sandbox for tests, create accounts, deploy contracts, etx.
    const root = worker.rootAccount;
    // Deploy the main registry contract
    const contract = await root.devDeploy(
        "target/wasm32-unknown-unknown/release/registries_near.wasm",
        {initialBalance: NEAR.parse("20 N").toJSON()},
    );
    // Deploy the test token contract
    const token = await root.devDeploy(
        "artifacts/test_token.wasm",
        {initialBalance: NEAR.parse("10 N").toJSON()},
    );

    // Allocate accounts
    const deployer = await root.createSubAccount("deployer", {initialBalance: NEAR.parse("100 N").toJSON()});
    const operator = await root.createSubAccount("operator", {initialBalance: NEAR.parse("100 N").toJSON()});
    const agentInstance = await root.createSubAccount("agent_instance1", {initialBalance: NEAR.parse("100 N").toJSON()});
    const agentInstance2 = await root.createSubAccount("agent_instance2", {initialBalance: NEAR.parse("100 N").toJSON()});

    // Initialize token contract
    await root.call(token, "new", {attachedDeposit: NEAR.parse("1 N")});

    // Mint tokens
    await root.call(token, "mint", {
        account_id: root.accountId,
        amount: NEAR.parse("100 N")
    }, {attachedDeposit: NEAR.parse("1 N")});

    await root.call(token, "mint", {
        account_id: deployer.accountId,
        amount: NEAR.parse("100 N")
    }, {attachedDeposit: NEAR.parse("1 N")});

    await root.call(token, "mint", {
        account_id: operator.accountId,
        amount: NEAR.parse("100 N")
    }, {attachedDeposit: NEAR.parse("1 N")});

    // Register contract account
    await root.call(token, "storage_deposit", {
        account_id: contract.accountId,
        registration_only: true
    }, {attachedDeposit: NEAR.parse("1 N")})

    // Save state for test runs, it is unique for each test
    t.context.worker = worker;
    t.context.accounts = {root, contract, token, deployer, operator, agentInstance, agentInstance2};
});

test.afterEach.always(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test("Create service and check its state", async t => {
    const {root, contract, deployer} = t.context.accounts;

    // Initialize the contract
    await root.call(contract, "new", {
        multisig_factory: deployer,
        metadata: defaultContractMetadata
    });

    // Create service
    const attachedDeposit = "5 N";
    await root.call(contract, "create", {
        service_owner: deployer,
        metadata: defaultServiceMetadata,
        config_hash: configHash,
        agent_ids: agentIds,
        agent_num_instances: agentNumInstances,
        agent_bonds: agentBonds,
        threshold
    }, {attachedDeposit});
    // Check that the total supply is 1
    let result = await contract.view("total_supply", {});
    t.is(result, "1");

    // Check that the service is in the PreRegistration state
    result = await contract.view("get_service_state", {service_id: serviceId});
    t.is(result, 1);

    const metadata = await contract.view("nft_metadata", {});
    console.log(metadata);
});

test("Update service with the same setup and check its state", async t => {
    const {root, contract, deployer} = t.context.accounts;

    // Initialize the contract
    await root.call(contract, "new", {
        multisig_factory: deployer,
        metadata: defaultContractMetadata
    });

    // Create service
    const attachedDeposit = "5 N";
    await root.call(contract, "create", {
        service_owner: deployer,
        metadata: defaultServiceMetadata,
        config_hash: configHash,
        agent_ids: agentIds,
        agent_num_instances: agentNumInstances,
        agent_bonds: agentBonds,
        threshold
    }, {attachedDeposit});

    // Update service
    await deployer.call(contract, "update", {
        service_id: serviceId,
        config_hash: configHash,
        agent_ids: agentIds,
        agent_num_instances: agentNumInstances,
        agent_bonds: agentBonds,
        threshold
    }, {attachedDeposit});

    // Check that the service is in the PreRegistration state
    let result = await contract.view("get_service_state", {service_id: serviceId});
    t.is(result, 1);
});

test("Update service with different agent ids and check its state", async t => {
    const {root, contract, deployer} = t.context.accounts;

    // Initialize the contract
    await root.call(contract, "new", {
        multisig_factory: deployer,
        metadata: defaultContractMetadata
    });

    // Create service
    const attachedDeposit = "5 N";
    await root.call(contract, "create", {
        service_owner: deployer,
        metadata: defaultServiceMetadata,
        config_hash: configHash,
        agent_ids: agentIds,
        agent_num_instances: agentNumInstances,
        agent_bonds: agentBonds,
        threshold
    }, {attachedDeposit});

    // Update service
    await deployer.call(contract, "update", {
        service_id: serviceId,
        config_hash: configHash2,
        agent_ids: [1, 2],
        agent_num_instances: [0, 1],
        agent_bonds: [0, 1],
        threshold
    }, {attachedDeposit});

    // Check that the service is in the PreRegistration state
    let result = await contract.view("get_service_state", {service_id: serviceId});
    t.is(result, 1);

    // Check the updated config
    result = await contract.view("get_service_config_hash", {service_id: serviceId});
    t.deepEqual(result, configHash2);

    // Check previous configs
    result = await contract.view("get_service_previous_config_hashes", {service_id: serviceId});
    t.deepEqual(result, [configHash]);
});

test("Activate service agent registration and check service state", async t => {
    const {root, contract, deployer} = t.context.accounts;

    // Initialize the contract
    await root.call(contract, "new", {
        multisig_factory: deployer,
        metadata: defaultContractMetadata
    });

    // Create service
    const attachedDeposit = "5 N";
    await root.call(contract, "create", {
        service_owner: deployer,
        metadata: defaultServiceMetadata,
        config_hash: configHash,
        agent_ids: agentIds,
        agent_num_instances: agentNumInstances,
        agent_bonds: agentBonds,
        threshold
    }, {attachedDeposit});

    // Activate service agent registration
    await deployer.call(contract, "activate_registration", {
        service_id: serviceId,
    }, {attachedDeposit});

    // Check that the service is in the ActiveRegistration state
    let result = await contract.view("get_service_state", {service_id: serviceId});
    t.is(result, 2);
});

test("Terminate service after its registration activation and check its state", async t => {
    const {root, contract, deployer} = t.context.accounts;

    // Initialize the contract
    await root.call(contract, "new", {
        multisig_factory: deployer,
        metadata: defaultContractMetadata
    });

    // Create service
    const attachedDeposit = "5 N";
    await root.call(contract, "create", {
        service_owner: deployer,
        metadata: defaultServiceMetadata,
        config_hash: configHash,
        agent_ids: agentIds,
        agent_num_instances: agentNumInstances,
        agent_bonds: agentBonds,
        threshold
    }, {attachedDeposit});

    // Activate service agent registration
    await deployer.call(contract, "activate_registration", {
        service_id: serviceId,
    }, {attachedDeposit});

    // Terminate service
    await deployer.call(contract, "terminate", {
        service_id: serviceId,
    }, {attachedDeposit});

    // Check that the service is in the PreRegistration state
    let result = await contract.view("get_service_state", {service_id: serviceId});
    t.is(result, 1);
});

test("Register agent instances by the operator and check service state and values", async t => {
    const {root, contract, deployer, operator, agentInstance} = t.context.accounts;

    // Initialize the contract
    await root.call(contract, "new", {
        multisig_factory: deployer,
        metadata: defaultContractMetadata
    });

    // Create service
    const attachedDeposit = "5 N";
    await root.call(contract, "create", {
        service_owner: deployer,
        metadata: defaultServiceMetadata,
        config_hash: configHash,
        agent_ids: agentIds,
        agent_num_instances: agentNumInstances,
        agent_bonds: agentBonds,
        threshold
    }, {attachedDeposit});

    // Activate service agent registration
    await deployer.call(contract, "activate_registration", {
        service_id: serviceId,
    }, {attachedDeposit});

    // Record contract balance before agent instance registration
    //const balanceBefore = await contract.availableBalance();
    //t.log(balanceBefore.toHuman());

    // Check registry balance after registration activation
    let balance = await contract.view("get_registry_balance", {});
    t.is(balance, agentBonds[0]);

    // Operator to register agent instance
    await operator.call(contract, "register_agents", {
        service_id: serviceId,
        agent_instances: [agentInstance],
        agent_ids: agentIds
    }, {attachedDeposit});

    // Check that the service is in the FinishedRegistration state
    let result = await contract.view("get_service_state", {service_id: serviceId});
    t.is(result, 3);

    // Check operator balance
    result = await contract.view("get_operator_balance", {operator: operator, service_id: serviceId});
    t.is(result, agentBonds[0]);

    // Check operator agent instances
    result = await contract.view("get_operator_service_agent_instances", {operator: operator, service_id: serviceId});
    t.deepEqual(result, [agentInstance.accountId]);

    // Check contract balance after registration
    balance = await contract.view("get_registry_balance", {});
    t.is(balance, 2 * agentBonds[0]);
    //t.log(balance.toHuman());
});

test("Unbond after service termination and check service state and values", async t => {
    const {root, contract, deployer, operator, agentInstance} = t.context.accounts;

    // Initialize the contract
    await root.call(contract, "new", {
        multisig_factory: deployer,
        metadata: defaultContractMetadata
    });

    let storage = await contract.view("get_storage_usage", {});
    console.log("Storage usage before create:", storage);

    //let storagePrice = await contract.view("get_storage_price", {});
    //console.log("Storage price before create:", storagePrice);

    let accountBalance = await contract.availableBalance();
    console.log("Account balance before create:", accountBalance.toString());

    // Create service
    const attachedDeposit = "5 N";
    await root.call(contract, "create", {
        service_owner: deployer,
        metadata: defaultServiceMetadata,
        config_hash: configHash,
        agent_ids: agentIds,
        agent_num_instances: agentNumInstances,
        agent_bonds: agentBonds,
        threshold
    }, {attachedDeposit, gas: "300 Tgas"});

    storage = await contract.view("get_storage_usage", {});
    console.log("Storage usage before activation:", storage);

    accountBalance = await contract.availableBalance();
    console.log("Account balance before activation", accountBalance.toString());

    // Activate service agent registration
    await deployer.call(contract, "activate_registration", {
        service_id: serviceId,
    }, {attachedDeposit});

    storage = await contract.view("get_storage_usage", {});
    console.log("Storage usage before registration:", storage);

    accountBalance = await contract.availableBalance();
    console.log("Account balance before registration", accountBalance.toString());

    // Operator to register agent instance
    await operator.call(contract, "register_agents", {
        service_id: serviceId,
        agent_instances: [agentInstance],
        agent_ids: agentIds
    }, {attachedDeposit});

    storage = await contract.view("get_storage_usage", {});
    console.log("Storage usage before terminate:", storage);

    accountBalance = await contract.availableBalance();
    console.log("Account balance before terminate", accountBalance.toString());

    // Terminate service
    await deployer.call(contract, "terminate", {
        service_id: serviceId,
    }, {attachedDeposit});

    storage = await contract.view("get_storage_usage", {});
    console.log("Storage usage before unbond:", storage);

    accountBalance = await contract.availableBalance();
    console.log("Account balance before unbond", accountBalance.toString());

    // Check registry balance after registration activation
    let balance = await contract.view("get_registry_balance", {});
    t.is(balance, agentBonds[0]);

    // Check that the service is in the TerminatedBonded state
    let result = await contract.view("get_service_state", {service_id: serviceId});
    t.is(result, 5);

    // Unbond operator
    await operator.call(contract, "unbond", {
        service_id: serviceId,
    }, {attachedDeposit});

    storage = await contract.view("get_storage_usage", {});
    console.log("Storage usage after unbond:", storage);

    accountBalance = await contract.availableBalance();
    console.log("Account balance after unbond", accountBalance.toString());

    // Check operator balance - operator is not found
//	const error = t.throwsAsync(async () => {
//		await contract.view("get_operator_balance", {operator: operator, service_id: serviceId});
//	}, {instanceOf: TypedError});

//	const error = t.throwsAsync(async () => {
//		contract.view("get_operator_balance", {operator: operator, service_id: serviceId});
//	});
//	console.log(error);

//    const error = await t.throws(
//        await contract.view("get_operator_balance", {operator: operator, service_id: serviceId})
//    ).then;
    //console.log(error);
    //t.like(error.message, "Operator not found");

    // Check contract balance after registration
    balance = await contract.view("get_registry_balance", {});
    t.is(balance, 0);
});

//test("Deploy, then unbond after service termination and check service state and values", async t => {
//    const {root, contract, deployer, operator, agentInstance} = t.context.accounts;
//
//    // Initialize the contract
//    await root.call(contract, "new", {
//        multisig_factory: deployer,
//        metadata: defaultContractMetadata
//    });
//
//    let storage = await contract.view("get_storage_usage", {});
//    console.log("Storage usage before create:", storage);
//
//    //let storagePrice = await contract.view("get_storage_price", {});
//    //console.log("Storage price before create:", storagePrice);
//
//    let accountBalance = await contract.availableBalance();
//    console.log("Account balance before create:", accountBalance.toString());
//
//    // Create service
//    const attachedDeposit = "5 N";
//    await root.call(contract, "create", {
//        service_owner: deployer,
//        metadata: defaultServiceMetadata,
//        config_hash: configHash,
//        agent_ids: agentIds,
//        agent_num_instances: agentNumInstances,
//        agent_bonds: agentBonds,
//        threshold
//    }, {attachedDeposit, gas: "300 Tgas"});
//
//    storage = await contract.view("get_storage_usage", {});
//    console.log("Storage usage before activation:", storage);
//
//    accountBalance = await contract.availableBalance();
//    console.log("Account balance before activation", accountBalance.toString());
//
//    // Activate service agent registration
//    await deployer.call(contract, "activate_registration", {
//        service_id: serviceId,
//    }, {attachedDeposit});
//
//    storage = await contract.view("get_storage_usage", {});
//    console.log("Storage usage before registration:", storage);
//
//    accountBalance = await contract.availableBalance();
//    console.log("Account balance before registration", accountBalance.toString());
//
//    // Operator to register agent instance
//    await operator.call(contract, "register_agents", {
//        service_id: serviceId,
//        agent_instances: [agentInstance],
//        agent_ids: agentIds
//    }, {attachedDeposit});
//
//    storage = await contract.view("get_storage_usage", {});
//    console.log("Storage usage before deploy:", storage);
//
//    accountBalance = await contract.availableBalance();
//    console.log("Account balance before deploy", accountBalance.toString());
//
//    // Deploy the service
//    await deployer.call(contract, "deploy", {
//        service_id: serviceId,
//        name_multisig: "multisig_000"
//    }, {attachedDeposit});
//
//    storage = await contract.view("get_storage_usage", {});
//    console.log("Storage usage before terminate:", storage);
//
//    accountBalance = await contract.availableBalance();
//    console.log("Account balance before terminate", accountBalance.toString());
//
//    // Terminate service
//    await deployer.call(contract, "terminate", {
//        service_id: serviceId,
//    }, {attachedDeposit});
//
//    storage = await contract.view("get_storage_usage", {});
//    console.log("Storage usage before unbond:", storage);
//
//    accountBalance = await contract.availableBalance();
//    console.log("Account balance before unbond", accountBalance.toString());
//
//    // Check registry balance after registration activation
//    let balance = await contract.view("get_registry_balance", {});
//    t.is(balance, agentBonds[0]);
//
//    // Check that the service is in the TerminatedBonded state
//    let result = await contract.view("get_service_state", {service_id: serviceId});
//    t.is(result, 5);
//
//    // Unbond operator
//    await operator.call(contract, "unbond", {
//        service_id: serviceId,
//    }, {attachedDeposit});
//
//    storage = await contract.view("get_storage_usage", {});
//    console.log("Storage usage after unbond:", storage);
//
//    accountBalance = await contract.availableBalance();
//    console.log("Account balance after unbond", accountBalance.toString());
//
//    // Check contract balance after registration
//    balance = await contract.view("get_registry_balance", {});
//    t.is(balance, 0);
//});
//
//test("Unbond after service termination and check service state and values 2", async t => {
//    const {root, contract, token, deployer, operator, agentInstance} = t.context.accounts;
//
//    // Initialize the contract
//    await root.call(contract, "new", {
//        multisig_factory: deployer,
//        metadata: defaultContractMetadata
//    });
//
//    let storage = await contract.view("get_storage_usage", {});
//    console.log("Storage usage before create:", storage);
//
//    //let storagePrice = await contract.view("get_storage_price", {});
//    //console.log("Storage price before create:", storagePrice);
//
//    let accountBalance = await contract.availableBalance();
//    console.log("Account balance before create:", accountBalance.toString());
//
//    // Create service
//    const attachedDeposit = "5 N";
//    await root.call(contract, "create", {
//        service_owner: deployer,
//        metadata: defaultServiceMetadata,
//        token: token.accountId,
//        config_hash: configHash,
//        agent_ids: agentIds,
//        agent_num_instances: agentNumInstances,
//        agent_bonds: agentBonds,
//        threshold
//    }, {attachedDeposit, gas: "300 Tgas"});
//});

test("Unbond after terminating the service with a token deposit", async t => {
    const {root, contract, token, deployer, operator, agentInstance} = t.context.accounts;

    // Initialize the contract
    await root.call(contract, "new", {
        multisig_factory: deployer,
        metadata: defaultContractMetadata
    });

    // Create service
    const attachedDeposit = "5 N";
    await root.call(contract, "create", {
        service_owner: deployer,
        metadata: defaultServiceMetadata,
        token: token.accountId,
        config_hash: configHash,
        agent_ids: agentIds,
        agent_num_instances: agentNumInstances,
        agent_bonds: agentBonds,
        threshold
    }, {attachedDeposit, gas: "300 Tgas"});

    // Activate service agent registration
    await deployer.call(token, "ft_transfer_call", {
        receiver_id: contract.accountId,
        amount: agentBonds[0].toString(),
        msg: ""
    }, {attachedDeposit: "1", gas: "300 Tgas"});

    let storage = await contract.view("get_storage_usage", {});
    console.log("Storage usage before activation:", storage);

//    let accountBalance = await contract.availableBalance();
//    console.log("Account balance before activation", accountBalance.toString());

    // Activate service agent registration
    await deployer.call(contract, "activate_registration", {
        service_id: serviceId,
    }, {attachedDeposit});

    // Check that the service is in the ActiveRegistration state
    let result = await contract.view("get_service_state", {service_id: serviceId});
    t.is(result, 2);

    storage = await contract.view("get_storage_usage", {});
    console.log("Storage usage before registration:", storage);

//    accountBalance = await contract.availableBalance();
//    console.log("Account balance before registration", accountBalance.toString());

    // Check token registry balance after registration activation
    let balance = await token.view("ft_balance_of", {account_id: contract.accountId});
    t.is(Number(balance), agentBonds[0]);

    // Register operator
    await operator.call(contract, "storage_deposit", {
        token: token.accountId
    }, {attachedDeposit});

    // Send tokens to the registry contract
    await operator.call(token, "ft_transfer_call", {
        receiver_id: contract.accountId,
        amount: agentBonds[0].toString(),
        msg: ""
    }, {attachedDeposit: "1", gas: "300 Tgas"});

    // Operator to register agent instance
    await operator.call(contract, "register_agents", {
        service_id: serviceId,
        agent_instances: [agentInstance],
        agent_ids: agentIds
    }, {attachedDeposit});

    storage = await contract.view("get_storage_usage", {});
    console.log("Storage usage before terminate:", storage);

//    accountBalance = await contract.availableBalance();
//    console.log("Account balance before terminate", accountBalance.toString());

    // Check token registry balance after registering agent instances
    balance = await token.view("ft_balance_of", {account_id: contract.accountId});
    t.is(Number(balance), 2 * agentBonds[0]);

    // Terminate service
    await deployer.call(contract, "terminate", {
        service_id: serviceId,
    }, {attachedDeposit});

    storage = await contract.view("get_storage_usage", {});
    console.log("Storage usage before unbond:", storage);

//    accountBalance = await contract.availableBalance();
//    console.log("Account balance before unbond", accountBalance.toString());

    // Check that the service is in the TerminatedBonded state
    result = await contract.view("get_service_state", {service_id: serviceId});
    t.is(result, 5);

    // Unbond operator
    await operator.call(contract, "unbond", {
        service_id: serviceId,
    }, {attachedDeposit});

    storage = await contract.view("get_storage_usage", {});
    console.log("Storage usage after unbond:", storage);

//    accountBalance = await contract.availableBalance();
//    console.log("Account balance after unbond", accountBalance.toString());

    // Check contract balance after registration
    balance = await contract.view("get_registry_balance", {});
    t.is(balance, 0);
});