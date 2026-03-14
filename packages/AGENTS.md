# AGENTS.md

## Layout

```
packages/
  db/              — @aicacia/db: offline-first reactive local database
  oidc-client/     — @aicacia/oidc-client: lightweight OIDC/OAuth2 client
  svelte-headless/ — @aicacia/svelte-headless: Svelte 5 headless state utilities
```

---

## Build & Tooling

- Package manager: **pnpm** — use `pnpm install`, `pnpm build`, `pnpm dev`
- Lint: `pnpm lint` (ESLint); auto-fix with `pnpm lint:fix`
- Test: `pnpm test`
- `db` and `oidc-client` produce `esm/`, `cjs/`, `browser/` (rollup bundle), and `types/` via rollup; see each package's `rollup.config.mjs`
- `svelte-headless` uses `@sveltejs/package` — library output goes to `dist/`; `src/routes/` is a local demo app only, not published

---

## Conventions

- TypeScript strict mode; no `any` without justification
- ESM-first; CJS builds are secondary outputs for compatibility
- `src/index.ts` is the public entrypoint for every package — only re-exports, no implementation
- Each package's `exports` field in `package.json` is the source of truth for what is public API
- Prefer early returns, small focused functions, shallow control flow
- Group imports: stdlib → external → internal, alphabetically within groups
- No non-essential comments; prefer refactoring over comments
- Add or update tests and examples when behaviour changes
- Justify new dependencies; prefer zero/minimal-dependency approaches
- Do not commit without passing CI: `pnpm lint` + `pnpm test`
