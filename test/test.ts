import {Worker, NEAR, NearAccount} from "near-workspaces";
import anyTest, {TestFn} from "ava";

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
    {initialBalance: NEAR.parse('3 N').toJSON()},
  );
  const deployer = await root.createSubAccount("deployer", {initialBalance: NEAR.parse('3 N').toJSON()});

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
  const attachedDeposit = '2 N';
//  console.log("root:", root);
//  console.log("contract:", contract);
  await root.call(contract, "new_default_meta", {owner_id: deployer});
//  await contract.view("is_paused");
  const result = await contract.view("is_paused", {});
  console.log(result);
  t.is(result, false);
});