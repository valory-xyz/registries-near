import process from "process";
import {Worker, NEAR, NearAccount} from "near-workspaces";
import {keyStores} from "near-api-js";
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

const contractName = "contract_000.sub_olas.olas_000.testnet";

const test = anyTest as TestFn<{
    worker: Worker;
}>;

test.before(async t => {
    t.context.worker = await Worker.init({
        homeDir: "/Users/kupermind/.near-credentials",
        network: "testnet",
        rootAccountId: "sub_olas"
    });
});

test.after.always(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test("Ping network", async t => {
    try {
        await t.context.worker.provider.block({finality: "final"});
    } catch (error: unknown) {
        t.fail("Failed to ping the network: ${error as string}");
        return;
    }

    t.pass("Network pinged successfully!");
});

test("Create service", async t => {
    const root = t.context.worker.rootAccount;
    const contract = root.getAccount(contractName);

    //const rootKey = await root.getKey();
    //console.log(key);

    // Create service
    const attachedDeposit = "1 N";
    await root.call(contract, "create", {
        service_owner: root,
        metadata: defaultMetadata,
        config_hash: configHash,
        agent_ids: agentIds,
        agent_num_instances: agentNumInstances,
        agent_bonds: agentBonds,
        threshold
    }, {attachedDeposit});

    const storage = await contract.view("get_storage_usage", {});
    console.log("Storage usage before activation:", storage);
});

test("Activate service registration", async t => {
    const root = t.context.worker.rootAccount;
    const contract = root.getAccount(contractName);

    // Activate service agent registration
    const attachedDeposit = "1 N";
    await root.call(contract, "activate_registration", {
        service_id: serviceId,
    }, {attachedDeposit});

    const storage = await contract.view("get_storage_usage", {});
    console.log("Storage usage before registration:", storage);
});

test("Register agent instances", async t => {
    const root = t.context.worker.rootAccount;
    const contract = root.getAccount(contractName);
    const agentInstance = root.getAccount("instance_000.sub_olas.olas_000.testnet");

    // Operator to register agent instance
    const attachedDeposit = "1 N";
    await root.call(contract, "register_agents", {
        service_id: serviceId,
        agent_instances: [agentInstance],
        agent_ids: agentIds
    }, {attachedDeposit});


    const storage = await contract.view("get_storage_usage", {});
    console.log("Storage usage before deployment:", storage);
});

test("Deploy the service", async t => {
    const root = t.context.worker.rootAccount;
    const contract = root.getAccount(contractName);

    // Deploy the service with the attached deposit on it
    const attachedDeposit = "5 N";
    await root.call(contract, "deploy", {
        service_id: serviceId,
        name_multisig: "multisig_002"
    }, {attachedDeposit, gas: "300 Tgas"});


    const storage = await contract.view("get_storage_usage", {});
    console.log("Storage usage before terminate:", storage);
});

test("Re-deploy the service", async t => {
    const root = t.context.worker.rootAccount;
    const contract = root.getAccount(contractName);

    // Re-deploy the service
    await root.call(contract, "deploy", {
        service_id: serviceId,
        name_multisig: "multisig_002.multisignature2.testnet"
    }, {gas: "300 Tgas"});

    const storage = await contract.view("get_storage_usage", {});
    console.log("Storage usage before terminate:", storage);
});

test("Terminate the service", async t => {
    const root = t.context.worker.rootAccount;
    const contract = root.getAccount(contractName);

    // Terminate service
    const attachedDeposit = "1 N";
    await root.call(contract, "terminate", {
        service_id: serviceId,
    }, {attachedDeposit});

    const storage = await contract.view("get_storage_usage", {});
    console.log("Storage usage before unbond:", storage);
});

test("Unbond the service", async t => {
    const root = t.context.worker.rootAccount;
    const contract = root.getAccount(contractName);

    // Terminate service
    const attachedDeposit = "1 N";
    await root.call(contract, "unbond", {
        service_id: serviceId,
    }, {attachedDeposit});

    const storage = await contract.view("get_storage_usage", {});
    console.log("Storage usage after unbond:", storage);
});

test("Create service, activate registration and register agents", async t => {
    const root = t.context.worker.rootAccount;
    const contract = root.getAccount(contractName);

    const attachedDeposit = "1 N";

    // Create service
    await root.call(contract, "create", {
        service_owner: root,
        metadata: defaultMetadata,
        config_hash: configHash,
        agent_ids: agentIds,
        agent_num_instances: agentNumInstances,
        agent_bonds: agentBonds,
        threshold
    }, {attachedDeposit});

    // Activate service agent registration
    await root.call(contract, "activate_registration", {
        service_id: serviceId,
    }, {attachedDeposit});

    const agentInstance = root.getAccount("instance_000.sub_olas.olas_000.testnet");

    // Operator to register agent instance
    await root.call(contract, "register_agents", {
        service_id: serviceId,
        agent_instances: [agentInstance],
        agent_ids: agentIds
    }, {attachedDeposit});
});

test("Activate service registration and register agents", async t => {
    const root = t.context.worker.rootAccount;
    const contract = root.getAccount(contractName);

    const attachedDeposit = "1 N";
    // Activate service agent registration
    await root.call(contract, "activate_registration", {
        service_id: serviceId,
    }, {attachedDeposit});

    const agentInstance = root.getAccount("instance_000.sub_olas.olas_000.testnet");

    // Operator to register agent instance
    await root.call(contract, "register_agents", {
        service_id: serviceId,
        agent_instances: [agentInstance],
        agent_ids: agentIds
    }, {attachedDeposit});
});

test.only("Terminate and unbond the service", async t => {
    const root = t.context.worker.rootAccount;
    const contract = root.getAccount(contractName);

    // Terminate service
    const attachedDeposit = "1 N";
    await root.call(contract, "terminate", {
        service_id: serviceId,
    }, {attachedDeposit});

    // Terminate service
    await root.call(contract, "unbond", {
        service_id: serviceId,
    }, {attachedDeposit});
});