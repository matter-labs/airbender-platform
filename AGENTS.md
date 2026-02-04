# AGENTS.md

This file defines project-specific guidance for coding agents working in `airbender-platform`.

## Project Snapshot

- Rust workspace for Airbender SDK/host/CLI and examples.
- Toolchain is pinned in `rust-toolchain.toml`.

## Scope and Boundaries

- Prefer changes inside this repository (`airbender-platform`) unless explicitly asked to modify sibling repositories.
- Treat upstream `zksync-airbender` crates as external dependencies for routine tasks.
- Keep changes focused; avoid unrelated refactors.

## Build and Lint Commands

- Do not require `RUST_MIN_STACK` for normal development flows.
- `Cargo.toml` already contains package-specific profile overrides to avoid stack-overflow compilation issues:
  - `[profile.dev.package.keccak_special5]`
  - `[profile.dev.package.setups]`
  - `[profile.release.package.keccak_special5]`
  - `[profile.release.package.setups]`
- If a build or lint only works with `RUST_MIN_STACK`, treat that as a project issue and report it.
- Full workspace clippy can emit large warning streams from upstream dependency crates. For task-focused linting, prefer targeted crate runs.

## Clippy Policy (Important)

- Outside `crates/airbender-crypto`: fix warnings directly when straightforward.
- Inside `crates/airbender-crypto`:
  - Be careful with `dead_code` warnings, especially when code is feature/platform dependent.
  - If a `dead_code` item may be conditionally used, annotate with:
    - `#[allow(dead_code)]`
    - and this exact comment on the line above:
      - `// TODO: Investigate the correct approach to avoid warning here`
- If a crypto warning is ambiguous or risky to resolve, stop and ask for clarification.

## Testing Guidance

- Run targeted tests for touched crates/files before broad test sweeps.
- Prefer behavior-focused tests; avoid fragile implementation-coupled assertions.

## Dependencies and Tooling

- Do not add new dependencies without explicit approval.
- Keep existing workspace conventions for dependency placement and crate organization.
