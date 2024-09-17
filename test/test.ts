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
    {initialBalance: NEAR.parse("3 N").toJSON()},
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

test("Check contract state", async t => {
  const {root, contract, deployer} = t.context.accounts;
//  console.log("root:", root);
//  console.log("contract:", contract);
  await root.call(contract, "new_default_meta", {owner_id: deployer});
//  await contract.view("is_paused");
  let result = await contract.view("is_paused", {});
  t.is(result, false);

  const attachedDeposit = "1 N";
  await root.call(contract, "create", {service_owner: deployer, metadata: defaultMetadata}, {attachedDeposit});
  result = await contract.view("total_supply", {});
  console.log(result);

  result = await contract.view("get_token_metadata", {token_id: "1"});
  console.log(result);
});