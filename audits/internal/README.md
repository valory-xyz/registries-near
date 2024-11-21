# Internal audit of near-governance-test
The review has been performed based on the contract code in the following repository:<br>
`https://github.com/valory-xyz/registries-near` <br>
commit: `62b77e3a7486f84c95dbf1889a8907eb8d00a2b2` or `0.1.0-pre-internal-audit`<br> 

## Objectives
The audit focused on contracts in this repo.

### Issue by BlockSec list
#### find Promises that are not handled
yes. see `Critical issue. Incorrect logic ft_transfer`
#### missing macro #[private] for callback functions
no
#### find functions that are vulnerable to reentrancy attack
Look at: https://github.com/blocksecteam/rustle/blob/main/docs/detectors/reentrancy.md
#### lack of overflow check for arithmetic operation
```            
*b += amount.0;
```



### Critical issue. Incorrect logic ft_transfer
```
https://docs.near.org/build/primitives/ft#transferring-tokens
#[near]
impl Contract {
  #[payable]
  pub fn send_tokens(&mut self, receiver_id: AccountId, amount: U128) -> Promise {
    assert_eq!(env::attached_deposit(), 1, "Requires attached deposit of exactly 1 yoctoNEAR");

    let promise = ext(self.ft_contract.clone())
      .with_attached_deposit(YOCTO_NEAR)
      .ft_transfer(receiver_id, amount, None);

    return promise.then( // Create a promise to callback query_greeting_callback
      Self::ext(env::current_account_id())
      .with_static_gas(Gas(30*TGAS))
      .external_call_callback()
    )
  }

  #[private] // Public - but only callable by env::current_account_id()
  pub fn external_call_callback(&self, #[callback_result] call_result: Result<(), PromiseError>) {
    // Check if the promise succeeded
    if call_result.is_err() {
      log!("There was an error contacting external contract");
    }
  }
}
```

### Medium issue
#### fixing, please, TODO (a lot of event)
```
pub fn change_owner(&mut self, new_owner: AccountId) 
        // TODO: event
... etc
```

#### not panic, refund attached deposit + tests
```
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
```

#### not refund by logic
```
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
```

#### set_operators_statuses check service_id
```
require!(self.services.contains_key(&service_id), "Service not found");
```

### Low issue
#### not private pub fn refund_deposit_to_account
```
#[private]
    pub fn refund_deposit_to_account
```
better "private pub fn" vs "fn". To discussing

#### better code update_multisig_callback?
```
let matching = agent_instances.iter().zip(multisig_members.iter()).all(|(ai, mm)| ai == mm);
```

#### better code drain?
```
const NATIVE_TOKEN: &str = "near";
```





