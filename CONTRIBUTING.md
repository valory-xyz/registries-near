# Contributing Guide

Thank you for your interest in contributing to this project!  
Contributions of all kinds are welcome: bug reports, feature suggestions, documentation improvements, and code changes.

Please read this guide carefully before submitting a contribution.

---

## Repository Overview

This repository contains NEAR smart contracts and supporting scripts.

Main technologies:
- Rust (smart contracts)
- NEAR Protocol
- near-cli
- Bash scripts for deployment and setup

---

## Development Setup

### Prerequisites

Make sure you have the following installed:

- Rust (stable)
- cargo
- near-cli
- Node.js (required by near-cli)
- Bash (for helper scripts)

Install near-cli:
```bash
npm install -g near-cli
```

Login to NEAR:
```bash
near login
```

---

## Building the Contracts

To build the contracts:

```bash
cargo build --release
```

Or use the provided helper script:

```bash
./scripts/build.sh
```

---

## Testing

If unit or integration tests are present, run:

```bash
cargo test
```

Some tests may require a local NEAR sandbox or testnet configuration.

---

## Deployment (Testnet)

This repository includes helper scripts for testnet deployment.

Example:
```bash
./scripts/setup_contract_account_testnet.sh
```

Make sure you understand what the script does before running it.

---

## Code Style

- Follow standard Rust formatting:
```bash
cargo fmt
```

- Ensure there are no clippy warnings when possible:
```bash
cargo clippy
```

Readable, well-structured code is strongly preferred.

---

## Pull Request Guidelines

When submitting a Pull Request:

- Clearly describe **what** was changed and **why**
- Keep PRs focused and minimal
- Avoid unrelated refactoring
- Update documentation if behavior changes
- Ensure the code builds and tests pass

---

## Commit Messages

Use clear and descriptive commit messages.

Recommended format:
```
<type>: short description

(optional longer explanation)
```

Examples:
- `fix: correct registry validation logic`
- `feat: add new registry entry type`
- `docs: update README`

---

## Reporting Issues

If you find a bug or have a suggestion:
- Open an issue
- Provide clear reproduction steps if applicable
- Include logs or error messages when relevant

---

## Commit Messages & Branching

- Use **Conventional Commits**:
    - `feat: ...`, `fix: ...`, `docs: ...`, `refactor: ...`, `test: ...`, `chore: ...`, `perf: ...`
- Branch names:
    - `feat/<short-topic>`, `fix/<short-topic>`, `docs/<short-topic>`
- Reference issues/PRs in the body (e.g., `Closes #123`).

> Optionally enforce **DCO** (`Signed-off-by`) or a **CLA** as part of CI.

---

## License & CLA/DCO

- This project is licensed under **MIT**. See `LICENSE`.
- If required, contributors must sign a **CLA** or use **DCO** sign-offs. Document the process in this section or link to your CLA portal.
- By contributing, you agree that your contributions will be licensed under the same license as the project.

---

## Contact

- General questions: **info@valory.xyz**
- Security: **security@valory.xyz**
