# deployment_shell_script.md

Documents `scripts/deploy.sh`, `scripts/interact.sh`, and `scripts/verify_env.sh` — the three deployment helper scripts for the Stellar Raise crowdfund contract.

---

## deploy.sh

Builds (via WASM check), deploys, and initialises the crowdfund contract, then runs a post-deploy smoke check to confirm the contract is live.

### Why this script exists

The previous `deploy.sh` used `set -e` but swallowed error context — a failed `stellar contract deploy` would exit silently. This version adds:

- `set -euo pipefail` for fail-fast, unset-variable-safe execution.
- A `--dry-run` flag that prints resolved parameters and the deploy command without submitting any transaction.
- A `--help` flag with full usage documentation.
- A WASM artifact existence check before any network call, with a hint to run `pnpm build:contracts` when the file is missing.
- Caught non-zero exit codes from `stellar` CLI calls, printed alongside the resolved command for quick debugging.
- A post-deploy smoke check (`goal` invocation) that confirms the contract is live and properly initialised.

### Usage

```bash
./scripts/deploy.sh [--dry-run] [--help] <creator> <token> <goal> <deadline> [min_contribution]
```

| Argument           | Type    | Description                                         |
| :----------------- | :------ | :-------------------------------------------------- |
| `creator`          | string  | Stellar address or identity of the campaign creator |
| `token`            | string  | Stellar address of the token contract               |
| `goal`             | integer | Funding goal in stroops                             |
| `deadline`         | integer | Unix timestamp — must be in the future              |
| `min_contribution` | integer | Minimum pledge amount (default: `1`)                |

### Flags

| Flag        | Description                                                                  |
| :---------- | :--------------------------------------------------------------------------- |
| `--dry-run` | Print resolved parameters and the deploy command; exit 0 without submitting. |
| `--help`    | Print usage documentation and exit.                                          |

### Environment variables

| Variable                     | Default   | Description                              |
| :--------------------------- | :-------- | :--------------------------------------- |
| `NETWORK`                    | `testnet` | Stellar network to target                |
| `STELLAR_RPC_URL`            | —         | Optional custom RPC endpoint             |
| `STELLAR_NETWORK_PASSPHRASE` | —         | Optional network passphrase override     |
| `SOURCE_ACCOUNT`             | —         | Optional override for the source account |

### WASM artifact check

The script resolves `target/wasm32-unknown-unknown/release/*.wasm` before attempting to deploy. If no WASM file is found, it exits immediately with:

```
ERROR: No WASM artifact found at target/wasm32-unknown-unknown/release/*.wasm
Hint:  Run 'pnpm build:contracts' or
       'cargo build --target wasm32-unknown-unknown --release -p crowdfund'
```

### Post-deploy smoke check

After a successful `initialize`, the script calls the read-only `goal` function:

```bash
stellar contract invoke --id "$CONTRACT_ID" --network "$NETWORK" -- goal
```

A non-zero exit from this call indicates the contract did not deploy correctly and the script exits with code `6`.

### Exit codes

| Code | Meaning                                      |
| :--- | :------------------------------------------- |
| 0    | Success (or `--dry-run` / `--help`)          |
| 1    | Missing WASM artifact                        |
| 4    | `stellar contract deploy` failure            |
| 5    | `stellar contract invoke initialize` failure |
| 6    | Post-deploy smoke check (`goal`) failed      |

### Example

```bash
DEADLINE=$(date -d "+30 days" +%s)
./scripts/deploy.sh GCREATOR... GTOKEN... 1000 "$DEADLINE" 10

# Preview without submitting:
./scripts/deploy.sh --dry-run GCREATOR... GTOKEN... 1000 "$DEADLINE" 10
```

---

## interact.sh

Invokes contract actions (contribute, withdraw, refund) after deployment.

### Usage

```bash
./scripts/interact.sh [--help] <contract_id> <action> [args...]
```

### Actions

| Action       | Additional args          | Description                                     |
| :----------- | :----------------------- | :---------------------------------------------- |
| `contribute` | `<contributor> <amount>` | Contribute tokens to the campaign               |
| `withdraw`   | `<creator>`              | Withdraw funds (goal met and deadline passed)   |
| `refund`     | `<caller>`               | Refund a single contributor via `refund_single` |

### Flags

| Flag     | Description                         |
| :------- | :---------------------------------- |
| `--help` | Print usage documentation and exit. |

### Environment variables

| Variable  | Default   | Description               |
| :-------- | :-------- | :------------------------ |
| `NETWORK` | `testnet` | Stellar network to target |

### Error output

All `stellar contract invoke` failures print the resolved command alongside the error, for example:

```
ERROR: contribute failed (exit 1).
Command: stellar contract invoke --id "C..." --network "testnet" ...
```

---

## verify_env.sh

Validates the local toolchain and required environment variables before deployment.

### Usage

```bash
./scripts/verify_env.sh
```

### Checks performed

1. `rustc` is installed and on PATH.
2. `cargo` is installed and on PATH.
3. `stellar` CLI is installed and on PATH.
4. `wasm32-unknown-unknown` Rust target is installed.
5. `cargo build --dry-run` succeeds for the `crowdfund` package.
6. Required environment variables are set: `STELLAR_RPC_URL`, `STELLAR_NETWORK_PASSPHRASE`, `SOURCE_ACCOUNT`, `CONTRACT_ID`.
7. `.soroban/` is listed in `.gitignore` if the directory exists.

All checks run before exiting — a single failure does not short-circuit the rest. Every missing variable is listed together so the operator can fix all gaps at once.

### Exit codes

| Code | Meaning                   |
| :--- | :------------------------ |
| 0    | All checks passed         |
| 1    | One or more checks failed |

---

## Security assumptions

- Pass a **named Stellar CLI identity** (e.g., `stellar keys generate --global alice`) as the `creator` argument — never a raw secret key.
- `set -euo pipefail` is set in all three scripts; unhandled errors abort execution immediately.
- `.soroban/` and `~/.config/stellar/` contain plaintext secret keys — never commit them.
