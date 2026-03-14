# AGENTS.md

## Layout

```
apps/
  mises/           — SvelteKit + Tauri desktop/web app (main Mises client)
```

---

## Build & Tooling

- Package manager: **pnpm** — use `pnpm install`, `pnpm build`, `pnpm dev`
- Web dev server: `pnpm dev` (Vite)
- Desktop dev: `pnpm desktop:dev` (Tauri + Vite)
- Production build: `pnpm build`
- Type-check: `pnpm check` (`svelte-kit sync && svelte-check`)
- Lint: `pnpm lint` (ESLint + Prettier check); auto-fix with `pnpm lint:fix`

---

## Shared Conventions

- TypeScript strict mode; no `any` without justification
- Svelte 5 runes (`$state`, `$derived`, `$effect`) — no legacy stores
- Prefer early returns, small focused functions, shallow control flow
- Group imports: stdlib → external → internal, alphabetically within groups
- No non-essential comments; prefer refactoring over comments
- Add or update tests/examples when behaviour changes
- Do not commit without passing CI: `pnpm lint` + `pnpm check`
- Do not hard-code URLs or secrets — use `.env` / environment variables
- Generated files (proto bindings, paraglide runtime) must not be edited by hand
