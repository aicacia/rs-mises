# AGENTS.md

## Build & Tooling

- Format: `cargo fmt --all -- --check`
- Lint: `cargo clippy -D warnings` (with and without default features)
- Build: `cargo build -p <crate> --no-default-features --all-targets`
- Build with alloc: `cargo build -p <crate> --no-default-features --features alloc --all-targets`
- Test: `cargo test -p <crate> --no-default-features`

## Conventions

- Default to `no_std`; only enable `std` when required (IO, threading, async runtimes).
- Keep `mod.rs` files minimal: only module declarations and re-exports, never implementation logic.
- Group imports: `std`/`core`/`alloc` → external crates → internal (`crate`, `super`).
- All new dependencies must set `default-features = false` and enable only required features.
- Group dependencies in `Cargo.toml` by logical category (e.g., `# Crypto`, `# Serialization`) with a comment header above each group, alphabetically within each group, blank line between groups.
- Public APIs require doc comments or examples.
- Add or update tests/examples when behavior changes.
- No non-essential comments; prefer refactoring over comments.
- Do not place implementation logic in `mod.rs`.
- Do not use glob imports or hard-coded absolute paths.
