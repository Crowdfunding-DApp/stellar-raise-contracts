# Contributing Guide

Thank you for your interest in contributing to this project!
Please read this guide carefully before opening a Pull Request.

## Prerequisites

- Rust (stable): https://rustup.rs
- Soroban CLI: `cargo install soroban-cli`
- wasm32 target: `rustup target add wasm32-unknown-unknown`

## Setting Up Locally

1. Fork and clone the repository:

```bash
git clone https://github.com/<your-username>/stellar-raise-contracts.git
cd stellar-raise-contracts
```

2. Build the contract:

```bash
cargo build --target wasm32-unknown-unknown --release
```

3. Run the test suite:

```bash
cargo test
```

## Branching Convention

Always branch off `develop`, never `main`:

```bash
git checkout develop
git pull origin develop
git checkout -b feature/your-feature-name
```

## Submitting a Pull Request

1. Ensure all tests pass:

```bash
cargo test
```

2. Ensure code is formatted:

```bash
cargo fmt --all
```

3. Ensure no Clippy warnings:

```bash
cargo clippy --all-targets -- -D warnings
```

4. Push your branch and open a PR targeting `develop`.
5. Reference the issue number with `Closes #<issue-number>` in your PR description.

## Code Style

- Follow standard Rust formatting enforced by `cargo fmt`.
- All public functions must have `///` doc comments.
- All new features must include tests.
- Commit messages must follow conventional commits format. Accepted types:
  - `feat:`
  - `fix:`
  - `docs:`
  - `style:`
  - `refactor:`
  - `perf:`
  - `test:`
  - `ci:`
  - `chore:`
  - `revert:`
- Two additional rules apply, both enforced by `.commitlintrc.json`:
  - The subject must not be written in all capitals.
  - The full header (type, optional scope, and subject) must be 100 characters or fewer.
- CI runs commitlint on every pull request and a violation fails the build. Only
  the commits introduced by the PR are checked: history that predates this
  enforcement is exempt, and merge commits are ignored.
- To check your branch before opening a PR:

```bash
npx commitlint --config .commitlintrc.json --from origin/develop --to HEAD
```

## Need Help?

Open a Discussion or comment on the relevant issue. We are happy to help.
