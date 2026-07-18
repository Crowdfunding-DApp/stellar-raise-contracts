# Stellar Raise Contracts  ......

Rust/Soroban smart contracts for the Stellar Raise decentralized crowdfunding platform.

> The frontend previously in this repo (`apps/frontend`) has been split out. This repo is
> now contracts-only.

---

## Project Overview

Stellar Raise is a decentralized crowdfunding application built on the Stellar network using Soroban smart contracts. This repository contains:

- **`apps/contracts`** — Rust/Soroban smart contracts (crowdfund, factory, soroban_sdk_minor)
- **`scripts/`** — Deployment and utility shell scripts
- **`docs/`** — Project documentation
- **`.env.example`** — Environment variable template for deployment scripts

---

## Prerequisites

| Tool                      | Version                                    |
| ------------------------- | ------------------------------------------ |
| Rust                      | stable (via rustup)                        |
| wasm32 target             | `rustup target add wasm32-unknown-unknown` |
| Soroban CLI / Stellar CLI | latest (`cargo install stellar-cli`)       |

---

## Building

```bash
cargo build --workspace --target wasm32-unknown-unknown --release
```

The contract WASM lands in `target/wasm32-unknown-unknown/release/`.

---

## Running Tests

```bash
cargo test --workspace
```

---

## Linting

```bash
cargo clippy --workspace -- -D warnings
```

---

## Contract Deployment

Deployment is handled via shell scripts in `scripts/`. Copy `.env.example` to `.env` and fill in your values first:

```bash
cp .env.example .env
```

Deploy the crowdfund contract:

```bash
./scripts/deploy.sh <creator_address> <token_address> <goal> <deadline_unix> <min_contribution>
```

Interact with a deployed contract:

```bash
./scripts/interact.sh
```

Verify your environment is configured correctly:

```bash
./scripts/verify_env.sh
```

See `scripts/deployment_shell_script.md` for detailed deployment documentation.

---

## Workspace Structure

```
stellar-raise-contracts/
├── apps/
│   └── contracts/         # Rust/Soroban contracts (@stellar-raise/contracts)
│       ├── crowdfund/     # Main crowdfunding contract
│       ├── factory/       # Factory contract
│       └── soroban_sdk_minor/
├── scripts/               # Deployment and utility scripts
├── docs/                  # Project documentation
└── Cargo.toml             # Rust workspace root
```

---

## Environment Variables

Copy `.env.example` to `.env` at the repo root and fill in:

| Variable          | Description                               |
| ----------------- | ----------------------------------------- |
| `STELLAR_RPC_URL` | Soroban RPC endpoint (testnet or mainnet) |
| `CONTRACT_ID`     | Deployed crowdfund contract address       |

---

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for commit conventions, branch strategy, and PR guidelines.

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/) and are enforced by commitlint on push.
