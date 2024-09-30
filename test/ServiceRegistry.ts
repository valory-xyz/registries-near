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

    // Prepare sandbox for tests, create accounts, deploy contracts, etx.
    const root = worker.rootAccount;
    const contract = await root.devDeploy(
        "target/wasm32-unknown-unknown/release/registries_near.wasm",
        {initialBalance: NEAR.parse("5.21832 N").toJSON()},
    );
    const deployer = await root.createSubAccount("deployer", {initialBalance: NEAR.parse("100 N").toJSON()});
    const operator = await root.createSubAccount("operator", {initialBalance: NEAR.parse("100 N").toJSON()});
    const agentInstance = await root.createSubAccount("agent_instance1", {initialBalance: NEAR.parse("100 N").toJSON()});
    const agentInstance2 = await root.createSubAccount("agent_instance2", {initialBalance: NEAR.parse("100 N").toJSON()});

    // Save state for test runs, it is unique for each test
    t.context.worker = worker;
    t.context.accounts = {root, contract, deployer, operator, agentInstance, agentInstance2};
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
        token: deployer,
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
    result = await contract.view("get_service_state", {service_id: 1});
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
        token: deployer,
        config_hash: configHash,
        agent_ids: agentIds,
        agent_num_instances: agentNumInstances,
        agent_bonds: agentBonds,
        threshold
    }, {attachedDeposit});

    // Update service
    await deployer.call(contract, "update", {
        service_id: serviceId,
        token: deployer,
        config_hash: configHash,
        agent_ids: agentIds,
        agent_num_instances: agentNumInstances,
        agent_bonds: agentBonds,
        threshold
    }, {attachedDeposit});

    // Check that the service is in the PreRegistration state
    let result = await contract.view("get_service_state", {service_id: 1});
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
        token: deployer,
        config_hash: configHash,
        agent_ids: agentIds,
        agent_num_instances: agentNumInstances,
        agent_bonds: agentBonds,
        threshold
    }, {attachedDeposit});

    // Update service
    await deployer.call(contract, "update", {
        service_id: serviceId,
        token: deployer,
        config_hash: configHash2,
        agent_ids: [1, 2],
        agent_num_instances: [0, 1],
        agent_bonds: [0, 1],
        threshold
    }, {attachedDeposit});

    // Check that the service is in the PreRegistration state
    let result = await contract.view("get_service_state", {service_id: 1});
    t.is(result, 1);

    // Check the updated config
    result = await contract.view("get_service_config_hash", {service_id: 1});
    t.deepEqual(result, configHash2);

    // Check previous configs
    result = await contract.view("get_service_previous_config_hashes", {service_id: 1});
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
        token: deployer,
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
    let result = await contract.view("get_service_state", {service_id: 1});
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
        token: deployer,
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
    let result = await contract.view("get_service_state", {service_id: 1});
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
        token: deployer,
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
    let result = await contract.view("get_service_state", {service_id: 1});
    t.is(result, 3);

    // Check operator balance
    result = await contract.view("get_operator_balance", {operator: operator, service_id: 1});
    t.is(result, agentBonds[0]);

    // Check operator agent instances
    result = await contract.view("get_operator_service_agent_instances", {operator: operator, service_id: 1});
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
        token: deployer,
        config_hash: configHash,
        agent_ids: agentIds,
        agent_num_instances: agentNumInstances,
        agent_bonds: agentBonds,
        threshold
    }, {attachedDeposit, gas: "300 Tgas"});

    storage = await contract.view("get_storage_usage", {});
    console.log("Storage usage before registration:", storage);

    accountBalance = await contract.availableBalance();
    console.log("Account balance before registration", accountBalance.toString());
    return;

    // Activate service agent registration
    await deployer.call(contract, "activate_registration", {
        service_id: serviceId,
    }, {attachedDeposit});

    // Operator to register agent instance
    await operator.call(contract, "register_agents", {
        service_id: serviceId,
        agent_instances: [agentInstance],
        agent_ids: agentIds
    }, {attachedDeposit});

    // Terminate service
    await deployer.call(contract, "terminate", {
        service_id: serviceId,
    }, {attachedDeposit});

    // Check registry balance after registration activation
    let balance = await contract.view("get_registry_balance", {});
    t.is(balance, agentBonds[0]);

    // Check that the service is in the TerminatedBonded state
    let result = await contract.view("get_service_state", {service_id: 1});
    t.is(result, 5);

    // Unbond operator
    await operator.call(contract, "unbond", {
        service_id: serviceId,
    }, {attachedDeposit});

    // Check operator balance - operator is not found
//	const error = t.throwsAsync(async () => {
//		await contract.view("get_operator_balance", {operator: operator, service_id: 1});
//	}, {instanceOf: TypedError});

//	const error = t.throwsAsync(async () => {
//		contract.view("get_operator_balance", {operator: operator, service_id: 1});
//	});
//	console.log(error);

//    const error = await t.throws(
//        await contract.view("get_operator_balance", {operator: operator, service_id: 1})
//    ).then;
    //console.log(error);
    //t.like(error.message, "Operator not found");

    // Check contract balance after registration
    balance = await contract.view("get_registry_balance", {});
    t.is(balance, 0);
});