import {Worker, NEAR, NearAccount} from "near-workspaces";
import anyTest, {TestFn} from "ava";

const defaultMetadata = {
    title: "Service Name",
    description: "Service Destription",
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

const serviceId = 1;
const configHash = Array(32).fill(5);
const configHash2 = Array(32).fill(9);
const agentIds = [1];
const agentNumInstances = [1];
const agentBonds = [1000];
const threshold = 1;


const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
}>;

test.beforeEach(async t => {
    // Init the worker and start a Sandbox server
    const worker = await Worker.init();

    console.log()

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
    await root.call(contract, "new_default_meta", {
        owner_id: deployer,
        multisig_factory: deployer
    });

    // Create service
    const attachedDeposit = "5 N";
    await root.call(contract, "create", {
        service_owner: deployer,
        metadata: defaultMetadata,
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
});

test("Update service with the same setup and check its state", async t => {
    const {root, contract, deployer} = t.context.accounts;

    // Initialize the contract
    await root.call(contract, "new_default_meta", {
        owner_id: deployer,
        multisig_factory: deployer
    });

    // Create service
    const attachedDeposit = "5 N";
    await root.call(contract, "create", {
        service_owner: deployer,
        metadata: defaultMetadata,
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
    await root.call(contract, "new_default_meta", {
        owner_id: deployer,
        multisig_factory: deployer
    });

    // Create service
    const attachedDeposit = "5 N";
    await root.call(contract, "create", {
        service_owner: deployer,
        metadata: defaultMetadata,
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
    await root.call(contract, "new_default_meta", {
        owner_id: deployer,
        multisig_factory: deployer
    });

    // Create service
    const attachedDeposit = "5 N";
    await root.call(contract, "create", {
        service_owner: deployer,
        metadata: defaultMetadata,
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
    await root.call(contract, "new_default_meta", {
        owner_id: deployer,
        multisig_factory: deployer
    });

    // Create service
    const attachedDeposit = "5 N";
    await root.call(contract, "create", {
        service_owner: deployer,
        metadata: defaultMetadata,
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
    await root.call(contract, "new_default_meta", {
        owner_id: deployer,
        multisig_factory: deployer
    });

    // Create service
    const attachedDeposit = "5 N";
    await root.call(contract, "create", {
        service_owner: deployer,
        metadata: defaultMetadata,
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

test.only("Unbond after service termination and check service state and values", async t => {
    const {root, contract, deployer, operator, agentInstance} = t.context.accounts;

    // Initialize the contract
    await root.call(contract, "new_default_meta", {
        owner_id: deployer,
        multisig_factory: deployer
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
        metadata: defaultMetadata,
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
//    await root.call(contract, "new_default_meta", {
//        owner_id: deployer,
//        multisig_factory: deployer
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
//        metadata: defaultMetadata,
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
//    await root.call(contract, "new_default_meta", {
//        owner_id: deployer,
//        multisig_factory: deployer
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
//        metadata: defaultMetadata,
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
    await root.call(contract, "new_default_meta", {
        owner_id: deployer,
        multisig_factory: deployer
    });

    // Create service
    const attachedDeposit = "5 N";
    await root.call(contract, "create", {
        service_owner: deployer,
        metadata: defaultMetadata,
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