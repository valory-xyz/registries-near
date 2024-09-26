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

const configHash = Array(32).fill(5);
const agentIds = [1];
const agentNumInstances = [1];
const agentBonds = [1];
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
    {initialBalance: NEAR.parse("10 N").toJSON()},
  );
  const deployer = await root.createSubAccount("deployer", {initialBalance: NEAR.parse("3 N").toJSON()});

  // Save state for test runs, it is unique for each test
  t.context.worker = worker;
  t.context.accounts = {root, contract, deployer};
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
  let result = await contract.view("total_supply", {});
  console.log(result);

  result = await contract.view("get_service_state", {service_id: 1});
  console.log(result);
});