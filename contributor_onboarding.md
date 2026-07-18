# Contributor Onboarding Plan

## Documentation-First Approach

- **README & CONTRIBUTING.md** — Clearly documented setup steps, prerequisites, and conventions guide new developers from zero to running the project locally.
- **Environment setup scripts** — `scripts/verify_env.sh` checks tooling requirements and provides actionable error messages, reducing friction.

## Low-Barrier Entry Points

- **Good first issues** — Labeled issues for small, well-scoped tasks (e.g., test coverage, documentation fixes, UI tweaks).
- **Specs directory** — Feature specifications in `specs/` describe design intent, making it easy to understand what needs building before diving into code.

## Community & Communication

- **PR template** — `.github/PULL_REQUEST_TEMPLATE.md` standardizes submissions and reminds contributors of checklist items (tests, linting, changelog).
- **Commit convention enforcement** — Commitlint and husky pre-push hooks guide contributors toward conventional commits, keeping history clean without requiring manual discipline.

## CI Guardrails

- **Automated CI** — Rust CI, spellcheck, and stale issue workflows run on every PR, catching regressions early and reducing reviewer burden so maintainers can focus on substantive feedback.

This layered approach ensures contributors can get started quickly, submit quality work, and receive timely reviews.
