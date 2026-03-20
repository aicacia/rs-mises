# Contributing

Thanks for contributing to MISES. This short guide helps contributors and automation agents perform common tasks (build, test, run) and follow repository conventions.

## Developer Quickstart

Build the whole workspace (no std features):

```bash
cargo build --workspace --no-default-features --all-targets
```

Build with `std` enabled (common for running servers and examples):

```bash
cargo build --workspace --features std --all-targets
```

Run tests for a specific crate (example: core) without default features:

```bash
cargo test -p mises-core --no-default-features
```

Run the Unix server example (from the workspace root):

```bash
cargo run -p mises-unix-server --release -- --help
# or to run the server and create the socket in the working directory
cargo run -p mises-unix-server --release -- --socket mises.sock
```

You can then exercise the server with the README's `grpcurl` examples:

```bash
grpcurl -plaintext -authority dummy unix://${PWD}/mises.sock list
grpcurl -plaintext -authority dummy unix://${PWD}/mises.sock mises.BootstrapService/Bootstrap
```

## Feature & CI Guidance

This repository defaults to `no_std`. Most crates expose a `std` feature for host builds. CI SHOULD validate at least the following combinations:

- `--no-default-features` (no_std build/test)
- `--features std` (host std-enabled build/test)
- crate-specific feature combinations used in dev (for example `in-memory` for tests)

See the repository workflow for the canonical CI configuration: [.github/workflows/ci.yml](.github/workflows/ci.yml).

## Current CI

This repository runs a CI workflow at [.github/workflows/ci.yml](.github/workflows/ci.yml) on pushes and pull requests to `main`. The workflow executes two jobs: a matrixed Rust job (`cargo-tests`) that runs workspace builds/tests for `--no-default-features` and `--features std`, and a JavaScript job (`pnpm-tests`) that installs workspace dependencies and runs `pnpm -w test`.

Locally smoke-test the same steps:

```bash
# Rust
cargo build --workspace --no-default-features --all-targets
cargo build --workspace --features std --all-targets

# JS
pnpm -w install --frozen-lockfile
pnpm -w test
```

This repository enforces formatting and linting in CI: the `lint` job runs `cargo fmt --all -- --check` and `cargo clippy --all -- -D warnings`.
A separate lint workflow is only needed for additional or stricter checks.

## Formatting, Linting, and Tests

- Run `cargo fmt --all -- --check` before opening PRs.
- Run `cargo clippy --all -- -D warnings` in CI (fail on warnings).
- Tests should be added for behavior changes and new public APIs. Prefer unit tests and small integration tests that run under `--no-default-features` where possible.

## Adding Dependencies

- Avoid adding deps with default features. Prefer `default-features = false` and explicitly enable only required features. See `.github/copilot-instructions.md` for dependency grouping policy.
- New dependencies must be grouped in `Cargo.toml` with a short comment header.

## Proto / Generated Code

If you change `.proto` files under `crates/proto/proto/`, provide regeneration instructions or helper scripts. CI should validate generated code matches committed artifacts.

## Pull Requests

- Include a short description of the change and the reasoning.
- Reference related issues or design docs from `docs/` when appropriate.
- Ensure formatting and clippy linting pass locally.

## Where to Start

- Core types and services: `crates/core/src` and `crates/core/src/service`.
- Graph model: `crates/graph/src`.
- Server runtime examples: `crates/unix-server` and `apps/mises/src-tauri`.

If you need help, open an issue or ask in the repository discussion. Thank you for contributing!
