# GitHub Agent Instructions

## Purpose

Short, exact rules for the GitHub agent and contributors.

---

## Comment policy

- Allow comments only in `tests`, `examples`, or for tiny, unavoidable clarifications.
- Prefer refactoring over comments. Do not add comments for style or noise.

---

## Git policy

- Agents must not modify anything using git (no commits, pushes, rebases, merges, tag changes, or any ref manipulation).
- Git may be used for read-only operations only, such as `git status`, `git log`, `git show`, `git branch`, `git ls-files`, and reading files from the repo.
- When in doubt, prefer reading repository state and ask for human confirmation before taking actions that would change repository history or refs.

---

## no_std policy

- Default to `no_std`; enable `std` as optional features. Rely on `core`.

Example crate header:

```rust
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
```

- Prefer `&[u8]`, iterators, and `core` types. Gate heap-dependent APIs behind `alloc`/`std`.
- Move OS, file I/O, and threading code into adapter crates that enable `std`.

---

## Rust imports

- **Group under a single `use` namespace when possible.** Prefer grouped imports instead of many separate `use` lines:

```rust
use std::{
    fs::{File, read_to_string},
    io::{self, Read},
};
```

- **List multiple items from the same module on separate lines** inside the braces for readability:

```rust
use crate::{
    a::Alpha,
    b::Beta,
    c::Gamma,
};
```

- **Order import groups consistently:**
  1. built-in libraries (`std`, `core`, `alloc`) ✅
  2. external crates ✅
  3. internal modules (`crate`, `super`, project modules) ✅

- Prefer specific imports over glob (`*`) imports and keep imports minimal.
- **Do not use hard-coded absolute filesystem paths** in source files (for example `/home/...`). Pass paths via configuration, environment variables, or adapters so code is portable and testable.
- **Place all `use` imports at the top of the file.** Avoid inline fully-qualified paths; import symbols at the top and reference them by short names in code.

---

## CI checks

- CI uses the MSRV in `rust-toolchain.toml`.
- Format: `cargo fmt --all -- --check`
- Lint (fail on warnings): run `cargo clippy` with and without default features and use `-D warnings`.
- Builds:
  - `cargo build -p <crate> --no-default-features --all-targets`
  - `cargo build -p <crate> --no-default-features --features alloc --all-targets`
- Tests: `cargo test -p <crate> --no-default-features` (CI must run this per crate)

---

## Changes checklist

1. No non-essential comments.
2. Public API has docs or examples.
3. Crate builds with `--no-default-features` (and `--features alloc` if applicable).
4. Prefer refactor over comments.
5. Add or update tests/examples when behavior changes.
6. New dependencies must set `default-features = false` and enable only required features.
7. New dependencies must be grouped in the crate's `Cargo.toml` by logical category with a comment header above each group (for example, `# Crypto`, `# Serialization`, `# Logging`). Within each group, list dependencies alphabetically and leave a blank line between groups.
8. Ensure `cargo test --no-default-features` passes for affected crates; CI enforces `fmt`/`clippy`.
