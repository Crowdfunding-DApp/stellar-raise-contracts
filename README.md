# Stellar Raise Contracts

A pnpm monorepo containing the Soroban smart contracts and React frontend for the Stellar Raise crowdfunding platform.

---

## Project Overview

Stellar Raise is a decentralized crowdfunding application built on the Stellar network using Soroban smart contracts. This repository contains:

- **`apps/contracts`** — Rust/Soroban smart contracts (crowdfund, factory, soroban_sdk_minor)
- **`apps/frontend`** — Vite + React + TypeScript frontend application
- **`scripts/`** — Deployment and utility shell scripts
- **`docs/`** — Project documentation
- **`specs/`** — Feature specifications and design documents

---

## Prerequisites

| Tool                      | Version                                    |
| ------------------------- | ------------------------------------------ |
| Node.js                   | ≥ 20                                       |
| pnpm                      | ≥ 9                                        |
| Rust                      | stable (via rustup)                        |
| wasm32 target             | `rustup target add wasm32-unknown-unknown` |
| Soroban CLI / Stellar CLI | latest (`cargo install stellar-cli`)       |

Install pnpm if you don't have it:

```bash
npm install -g pnpm
```

---

## Installation

Clone the repo and install all JS dependencies from the root:

```bash
pnpm install
```

This installs dependencies for all workspaces (`apps/frontend`, root tooling) in one pass.

---

## Running in Development

Start all apps concurrently with labeled output:

```bash
pnpm dev
```

Or run each app individually:

```bash
# Frontend only
pnpm --filter @stellar-raise/frontend dev

# Contracts have no dev server — compile instead
pnpm --filter @stellar-raise/contracts build
```

The frontend dev server runs at `http://localhost:5173` by default.

---

## Building

Build all apps:

```bash
pnpm build
```

Build individually:

```bash
# Frontend only
pnpm build:frontend

# Contracts only (compiles to WASM)
pnpm build:contracts
```

The frontend build output lands in `apps/frontend/dist/`. The contract WASM lands in `target/wasm32-unknown-unknown/release/`.

---

## Running Tests

Run all tests across the workspace:

```bash
pnpm test
```

Run per app:

```bash
# Frontend (Vitest)
pnpm --filter @stellar-raise/frontend test

# Contracts (cargo test)
pnpm --filter @stellar-raise/contracts test
```

The root also has a Jest-based test suite for legacy tests:

```bash
npx jest
```

---

## Linting and Lint Fix

Check for lint issues across all workspaces:

```bash
pnpm lint
```

**Fix lint issues** (writes changes to disk):

```bash
pnpm lint:fix
```

`lint:fix` runs `eslint . --fix` for the frontend and `cargo clippy --fix` for contracts — it actually writes the fixes, not just reports them. Run it before committing if you see lint errors.

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

See `scripts/deployment_shell_script.md` and `scripts/wasm_build_pipeline.md` for detailed deployment documentation.

---

## Workspace Structure

```
stellar-raise-contracts/
├── apps/
│   ├── frontend/          # Vite + React + TypeScript app (@stellar-raise/frontend)
│   └── contracts/         # Rust/Soroban contracts (@stellar-raise/contracts)
│       ├── crowdfund/     # Main crowdfunding contract
│       ├── factory/       # Factory contract
│       └── soroban_sdk_minor/
├── packages/              # Reserved for shared packages
├── scripts/               # Deployment and utility scripts
├── docs/                  # Project documentation
├── specs/                 # Feature specifications
├── Cargo.toml             # Rust workspace root
├── pnpm-workspace.yaml    # pnpm workspace config
├── package.json           # Root workspace orchestrator
└── tsconfig.json          # Root TypeScript config (references apps/frontend)
```

Each app is independently runnable from its own directory using the same script names (`dev`, `build`, `test`, `lint`, `lint:fix`).

---

## Environment Variables

Copy `.env.example` to `.env` at the repo root and fill in:

| Variable          | Description                               |
| ----------------- | ----------------------------------------- |
| `STELLAR_RPC_URL` | Soroban RPC endpoint (testnet or mainnet) |
| `CONTRACT_ID`     | Deployed crowdfund contract address       |

The frontend may have its own `.env` — see `apps/frontend/` for any Vite-specific env vars (prefixed `VITE_`).

---

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for commit conventions, branch strategy, and PR guidelines.

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/) and are enforced by commitlint on push.
