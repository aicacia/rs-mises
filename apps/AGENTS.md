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

## Translation Tooling (inlang + Paraglide)

- Translation source files: `apps/mises/messages/{locale}.json` (for example `en.json`, `es.json`)
- inlang project config: `apps/mises/project.inlang/settings.json`
- Generated translation runtime: `apps/mises/src/lib/paraglide/` (auto-generated, do not edit by hand)
- Paraglide compilation is wired through `paraglideVitePlugin` in `apps/mises/vite.config.ts`
- To pick up translation changes during development, run `pnpm -C ./apps/mises dev`
- To verify translated output in a production bundle, run `pnpm -C ./apps/mises build`
- When adding a locale, update `locales` (and `baseLocale` when needed) in `apps/mises/project.inlang/settings.json` and add `apps/mises/messages/<locale>.json`
- Keep translation keys aligned across locale files; missing keys fall back to `baseLocale`

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
- Do not edit `project.inlang/cache/` or `src/lib/paraglide/` directly; edit `messages/*.json` and `project.inlang/settings.json` instead
