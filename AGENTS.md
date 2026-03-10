# AGENTS.md

# What is this?

Mesh identity and security enforcement system for managing identities, keys, resources, and access policies using a graph-based model.

## Monorepo Layout

- apps/: SvelteKit web apps
- crates/: Rust libraries
- examples/: Minimal usage demos
- packages/: Shared Svelte components/utilities
- docs/: Design, spec, and examples

## What We Use

- Rust (no_std by when possible)
- SvelteKit, TailwindCSS, Vite, ESLint, Paraglide
- Tauri for desktop integration
- GitHub Actions for CI

## High-Level Patterns

- Keep modules minimal: only declarations/re-exports in mod.rs
- Group Rust imports by namespace, order: std/core/alloc → external → internal
- Prefer explicit imports, minimal dependencies
- Pass paths/config via env/adapters, never hard-code
- Public APIs require docs/examples
- Add/update tests for behavior changes

## What NOT To Do

- Do not add non-essential comments (prefer refactor)
- Do not place implementation logic in mod.rs
- Do not use glob imports or hard-coded absolute paths
- Do not enable std unless required
- Do not commit without passing CI: fmt, clippy, test (no-default-features)
- Do not add dependencies with default features enabled
- Do not mix dependency groups in Cargo.toml
- Do not skip tests/examples for new/changed behavior
- Do not modify git refs/history

---

See .github/copilot-instructions.md for detailed contributor rules.
