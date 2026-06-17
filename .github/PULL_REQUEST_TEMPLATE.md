## Description

<!-- Please provide a concise description of your changes. -->

## Related Issues

<!-- List related issues. Use "Closes #<issue-number>" to auto-close issues on merge. -->

Closes #

## Type of Change

- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update
- [ ] CI / Infrastructure

## Required CI Checks

The following status checks must pass before this PR can be merged:

**Frontend CI** (`frontend_ci.yml` — `ci-frontend` job):

- TypeScript type-check (`pnpm --filter @stellar-raise/frontend typecheck`)
- Vitest test suite (`pnpm --filter @stellar-raise/frontend test`)
- ESLint across all workspaces (`pnpm lint`)
- Prettier format check (`pnpm format:check`)

**Rust CI** (`rust_ci.yml` — `check` job):

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --release --target wasm32-unknown-unknown`
- `cargo test --workspace`

## Checklist

- [ ] My branch is based off `develop`, not `main`
- [ ] I have run `cargo fmt --all` and the code is properly formatted
- [ ] I have run `cargo clippy --all-targets -- -D warnings` with no warnings
- [ ] I have run `cargo test` and all tests pass
- [ ] I have run `pnpm typecheck` with no type errors
- [ ] I have run `pnpm test` and all frontend tests pass
- [ ] I have run `pnpm lint` with no ESLint errors
- [ ] I have run `pnpm format:check` with no formatting violations
- [ ] I have added tests for any new functionality
- [ ] All public functions have `///` doc comments
- [ ] I have updated `CHANGELOG.md` if applicable
- [ ] My commit messages follow the [conventional commits](https://www.conventionalcommits.org/) format

## Screenshots / Logs (if applicable)

<!-- Add screenshots or logs to help explain your changes. -->

## Additional Notes

<!-- Add any other context or information here. -->
