# Internal audit of near-governance-test
The review has been performed based on the contract code in the following repository:<br>
`https://github.com/valory-xyz/registries-near` <br>
commit: `62b77e3a7486f84c95dbf1889a8907eb8d00a2b2` or `0.1.0-pre-internal-audit`<br> 

## Objectives
The audit focused on contracts in this repo.

### Problems found instrumentally
Several checks are obtained automatically. They are commented. Some issues found need to be fixed. <br>
List of rust tools:
##### cargo tree
```
cargo tree > audits/internal/analysis/cargo_tree.txt
```
##### cargo-audit
https://docs.rs/cargo-audit/latest/cargo_audit/
```
cargo install cargo-audit
cargo-audit audit > audits/internal/analysis/cargo-audit.txt
```

##### cargo clippy 
https://github.com/rust-lang/rust-clippy
```
cargo clippy 2> audits/internal/analysis/cargo-clippy.txt
```

All automatic warnings are listed in the following file, concerns of which we address in more detail below: <br>
[cargo-tree.txt](https://github.com/valory-xyz/registries-near/blob/main/lockbox/audits/internal/analysis/cargo-tree.txt) <br>
[cargo-audit.txt](https://github.com/valory-xyz/registries-near/blob/main/lockbox/audits/internal/analysis/cargo-audit.txt) <br>
[cargo-clippy.txt](https://github.com/valory-xyz/registries-near/blob/main/lockbox/audits/internal/analysis/cargo-clippy.txt) <br>

### Issue by BlockSec list
#### find Promises that are not handled - Issue
yes. see `Critical issue. Incorrect logic ft_transfer`

#### missing macro #[private] for callback functions
no
#### find functions that are vulnerable to reentrancy attack - Double checks
Look at: https://github.com/blocksecteam/rustle/blob/main/docs/detectors/reentrancy.md

#### lack of overflow check for arithmetic operation - Issue
```            
*b += amount.0;
```

#### missing check of sender != receiver
no
#### incorrect type used in parameters or return values
no
#### changes to collections are not saved
no
#### find nft_transfer without check of approval id
no
#### find approve or revoke functions without owner check
no
#### precision loss due to incorrect operation order
no
#### rounding without specifying ceil or floor
no
#### panic in callback function may lock contract - Issue
```
pub fn create_multisig_callback
```

#### no assert_one_yocto in privileged function - Issue
```
Details: https://github.com/blocksecteam/rustle/blob/main/docs/detectors/yocto-attach.md
Details: https://docs.near.org/build/smart-contracts/security/one-yocto
Example: https://github.com/ref-finance/ref-contracts/blob/536a60c842e018a535b478c874c747bde82390dd/ref-exchange/src/owner.rs#L16
This can be implemented in the contract by adding assert_one_yocto, which is recommended for all privileged functions.
1. pub fn set_paused
2. pub fn change_upgrade_hash or rewrite condition owner_or_self to only_self
3. pub fn update
4. pub fn activate_registration
5. pub fn register_agents
6. pub fn slash
7. pub fn terminate
8. pub fn drain
9. pub fn withdraw
10. pub fn storage_withdraw
11. pub fn set_operators_check
12. pub fn change_upgrade_hash
```

#### duplicate id uses in collections
no, StorageKey
#### no panic on unregistered transfer receivers
N/A
#### find all unimplemented NEP interface
no
#### missing check of prepaid gas in ft_transfer_call
no
#### macro #[private] used in non-callback function
no
#### function result not used or checked
no
#### no upgrade function in contract
no
#### tautology used in conditional branch
no
#### missing balance check for storage expansion
no
#### missing balance check before storage unregister
no

## Other critical issues
### Critical issue 1. Incorrect logic ft_transfer
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

### Critical issue 2. whitelisting setupped, but not used 
```
    /// @param setCheck True if the whitelisting check is needed, and false otherwise.
    // Call by the service owner
    pub fn set_operators_check(&mut self, service_id: u32, set_check: bool) {
```

### Other medium issue
#### We need to clearly decide the logic of who has the right to change the contract as the owner (account in near or self via ???)
```
pub fn change_owner
require!(self.owner == env::predecessor_account_id());

vs

pub fn owner_or_self(&self)
 let caller = env::predecessor_account_id();
        caller == self.tokens.owner_id || caller == env::current_account_id()
pub fn set_paused(&mut self, paused: bool) {
    require!(self.owner_or_self());

The presence of alternatives is confusing. It is better not to make functions like owner_or_self() - because it makes it unclear how it will actually work set_paused via
require!(self.owner == env::predecessor_account_id()) OR env::predecessor_account_id() == env::current_account_id()
Please, use [#private] for self-calls   
```

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

#### return vs panic in ft_on_transfer?
```
fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let token = env::predecessor_account_id();

        // Get token balance the sender
        if let Some(b) = self
            .all_token_balances
            .get_mut(&token)
            .unwrap_or_else(|| env::panic_str("Token not registered"))
            .get_mut(&sender_id)
        {
            // TODO saturated
            // Increase for the provided amount
            *b += amount.0;
            log!("Increased the token amount! {}", amount.0);

            // No tokens will be returned
            PromiseOrValue::Value(U128::from(0))
        } else {
            // otherwise return
            PromiseOrValue::Value(U128::from(amount.0))
        }
    }
```

### Low issue (code)
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

### Low issue (doc)
1. Fixing README.md - `Build the code:` - incorrect. 

2. Fixing README.md - remove sandbox part as outdated. 

3. Fixing setup-env.sh to actual versions if needed

4. Ref FungibleToken in README. 

5. Group all private functions in one place. 




