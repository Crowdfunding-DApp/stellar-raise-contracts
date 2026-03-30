# security_policy_enforcement

`scripts/security_policy_enforcement.sh` enforces baseline CI/CD security policy for the repository's GitHub Actions workflows.

## What it enforces

1. Every workflow must declare explicit `permissions`.
2. No workflow may use `permissions: write-all`.
3. Every workflow must define at least one `timeout-minutes` bound.
4. Every `actions/checkout` step must set `persist-credentials: false`.
5. Workflows must not interpolate `${{ secrets.* }}` directly in `run:` commands.
6. Workflow actions must not use mutable refs such as `@main`, `@master`, or `@HEAD`.
7. `.github/workflows/security.yml` must execute both the policy script and `scripts/security_policy_enforcement.test.sh`.

## Usage

```bash
bash scripts/security_policy_enforcement.sh
bash scripts/security_policy_enforcement.test.sh
```

The script writes a plain-text report to `.security-policy-reports/security_policy_report_<timestamp>.txt`.

## CI integration

`security.yml` should run the following steps:

```yaml
- name: Run security policy enforcement
  run: ./scripts/security_policy_enforcement.sh

- name: Run security policy enforcement tests
  run: ./scripts/security_policy_enforcement.test.sh
```

## Security assumptions

- Repository workflow files are the source of truth for CI/CD behavior.
- Secrets are injected through `env:` bindings or action inputs, not echoed inline in `run:` commands.
- Explicit `permissions` and timeout bounds are required on all checked-in workflows so review does not depend on GitHub defaults.
- Disabling persisted checkout credentials is sufficient hardening for workflows that only need read access to repository contents.

## Test coverage notes

The companion test suite exercises:

- a passing fixture repository
- report generation
- missing permissions
- forbidden `write-all`
- missing timeout bounds
- missing checkout hardening
- direct secret interpolation
- mutable action refs
- missing security workflow hooks
- the real repository configuration

This gives branch coverage across each policy rule and the summary exit paths.
