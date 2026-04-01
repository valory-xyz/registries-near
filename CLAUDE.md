# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

NEAR smart contract implementing an NFT-based Service Registry for Autonolas. Manages the full service lifecycle: creation → registration → deployment → termination. Services are represented as NFTs, operators post bonds (in NEAR or custom tokens), and deployed services are managed through multisig contracts.

## Build & Test Commands

```bash
# Build the WASM contract
./scripts/build.sh
# or: npm run build
# or: cargo build --target wasm32-unknown-unknown --release

# Run sandbox tests (local in-memory NEAR network)
npx ava test/ServiceRegistry.ts

# Run a single test by title match
npx ava test/ServiceRegistry.ts -m "test title pattern"

# Run testnet tests
npm run test-testnet

# Debug mode (verbose NEAR output)
NEAR_WORKSPACES_DEBUG=true npx ava test/ServiceRegistry.ts

# Install JS dependencies
yarn

# Check/clean Rust dependencies
cargo tree
cargo clean
```

**Toolchain requirement:** `rustc 1.79.0+`, target `wasm32-unknown-unknown`.

Build output goes to `artifacts/registries_near.wasm`.

## Architecture

### Contract (src/lib.rs)

Single-file contract (~1450 lines). Key components:

- **ServiceRegistry** — main contract struct holding all state: services (`LookupMap<u32, Service>`), NFT tokens, token balances, slashed funds, pause flag, multisig factory reference
- **Service** — core data structure with lifecycle state, agent configs, operator tracking, multisig address, config hashes, and bond amounts
- **ServiceState** — enum governing transitions: `NonExistent → PreRegistration → ActiveRegistration → FinishedRegistration → Deployed → TerminatedBonded`

### State-changing flow

1. `create()` — mints NFT, initializes service in `PreRegistration`
2. `update()` — modifies service config (only in `PreRegistration`)
3. `activate_registration()` — opens service for agent registration
4. `register_agents()` — operators register agent instances with bonds
5. `deploy()` — creates/updates multisig, transitions to `Deployed`
6. `terminate()` — returns to `TerminatedBonded`, begins unbonding
7. `unbond()` — operators reclaim bonds after termination

### Cross-contract interactions

- **MultisigFactory** — creates multisig accounts for deployed services
- **Multisig2** — queries multisig members for validation
- **FT standard** — `ft_on_transfer` handles incoming custom token bonds

### Storage

Uses NEAR SDK `BorshStorageKey` enum for namespaced storage. All collections use `LookupMap`/`Vector` with unique prefixes per service (format: `"{prefix}{service_id}"`). Storage deposits are tracked and refunded.

### Testing

Tests are in `test/` using AVA + near-workspaces (TypeScript). Sandbox tests deploy the contract and a test token (`artifacts/test_token.wasm`) to a local NEAR sandbox, then exercise the full service lifecycle. Test accounts are created with 100 NEAR each.

## Key Patterns

- Owner-only functions check `env::predecessor_account_id() == self.owner`
- Service owner is verified via NFT ownership (`self.tokens.owner_by_id`)
- Storage cost tracking: functions measure `env::storage_usage()` before/after and refund the difference
- All payable functions require `#[payable]` and attached deposits
- Contract uses `#[near(contract_state)]` and `#[near]` attribute macros (NEAR SDK v5.5)
