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

//test.only("Deploy contract", async t => {
//    const root = t.context.worker.rootAccount;
//    console.log(root);
//    const contract = await root.devDeploy(
//        "target/wasm32-unknown-unknown/release/registries_near.wasm",
//        {initialBalance: NEAR.parse("6 N").toJSON()},
//    );
//    console.log(contract);
//});


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
        token: root,
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
    console.log("Storage usage before activation:", storage);
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
    console.log("Storage usage before activation:", storage);
});

test("Create service, activate registration and register agents", async t => {
    const root = t.context.worker.rootAccount;
    const contract = root.getAccount(contractName);

    const attachedDeposit = "1 N";

    // Create service
    await root.call(contract, "create", {
        service_owner: root,
        metadata: defaultMetadata,
        token: root,
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

    const storage = await contract.view("get_storage_usage", {});
    console.log("Storage usage before activation:", storage);
});

test.only("Deploy the service", async t => {
    const root = t.context.worker.rootAccount;
    const contract = root.getAccount(contractName);

    // Operator to register agent instance
    const attachedDeposit = "5 N";
    await root.call(contract, "deploy", {
        service_id: serviceId,
        name_multisig: "multisig_000"
    }, {attachedDeposit, gas: "300 Tgas"});


    const storage = await contract.view("get_storage_usage", {});
    console.log("Storage usage before activation:", storage);
});


//
//await Promise.all([
//    async() => {
//        // Initialize the contract
//        await root.call(contract, "new_default_meta", {
//            owner_id: root,
//            multisig_factory: "multisafe.testnet"
//        });
//
//        // Create service
//        const attachedDeposit = "2 N";
//        await root.call(contract, "create", {
//            service_owner: deployer,
//            metadata: defaultMetadata,
//            token: deployer,
//            config_hash: configHash,
//            agent_ids: agentIds,
//            agent_num_instances: agentNumInstances,
//            agent_bonds: agentBonds,
//            threshold
//        }, {attachedDeposit});
////
////        storage = await contract.view("get_storage_usage", {});
////        console.log("Storage usage before activation:", storage);
////
////        accountBalance = await contract.availableBalance();
////        console.log("Account balance before activation", accountBalance.toString());
////
////        // Activate service agent registration
////        await deployer.call(contract, "activate_registration", {
////            service_id: serviceId,
////        }, {attachedDeposit});
////
////        storage = await contract.view("get_storage_usage", {});
////        console.log("Storage usage before registration:", storage);
////
////        accountBalance = await contract.availableBalance();
////        console.log("Account balance before registration", accountBalance.toString());
////
////        // Operator to register agent instance
////        await operator.call(contract, "register_agents", {
////            service_id: serviceId,
////            agent_instances: [agentInstance],
////            agent_ids: agentIds
////        }, {attachedDeposit});
////
////        storage = await contract.view("get_storage_usage", {});
////        console.log("Storage usage before terminate:", storage);
////
////        accountBalance = await contract.availableBalance();
////        console.log("Account balance before terminate", accountBalance.toString());
////
////        // Terminate service
////        await deployer.call(contract, "terminate", {
////            service_id: serviceId,
////        }, {attachedDeposit});
////
////        storage = await contract.view("get_storage_usage", {});
////        console.log("Storage usage before unbond:", storage);
////
////        accountBalance = await contract.availableBalance();
////        console.log("Account balance before unbond", accountBalance.toString());
////
////        // Check registry balance after registration activation
////        let balance = await contract.view("get_registry_balance", {});
////        t.is(balance, agentBonds[0]);
////
////        // Check that the service is in the TerminatedBonded state
////        let result = await contract.view("get_service_state", {service_id: 1});
////        t.is(result, 5);
////
////        // Unbond operator
////        await operator.call(contract, "unbond", {
////            service_id: serviceId,
////        }, {attachedDeposit});
////
////        storage = await contract.view("get_storage_usage", {});
////        console.log("Storage usage after unbond:", storage);
////
////        accountBalance = await contract.availableBalance();
////        console.log("Account balance after unbond", accountBalance.toString());
////
////        // Check contract balance after registration
////        balance = await contract.view("get_registry_balance", {});
////        t.is(balance, 0);
//    }
//]);