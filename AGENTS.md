# Repository Guidelines

## 核心要点
- **每次对话一定要用中文回答**

## Project Structure & Module Organization
- `src/`: Vue 3 + TypeScript frontend (views in `src/views`, shared UI in `src/components`, Pinia stores in `src/stores`, i18n in `src/i18n`, types in `src/types`).
- `src-tauri/`: Rust + Tauri backend (commands and core logic in `src-tauri/src`).
- `public/`: static assets served by Vite.
- `scripts/`: project helper scripts (e.g., install rollup native).
- Build outputs: `dist/` (frontend), `src-tauri/target/` (Rust).
- Key config: `vite.config.ts`, `tailwind.config.js`, `src-tauri/tauri.conf.json`.

## Build, Test, and Development Commands
- `npm install`: install dependencies (runs `scripts/install-rollup-native.js`).
- `npm run dev`: Vite dev server for frontend only.
- `npm run tauri:dev`: run the full desktop app in dev mode.
- `npm run build`: build frontend assets into `dist/`.
- `npm run tauri:build`: build the production desktop app.
- `npm run generate:icon`: regenerate `src-tauri/icons/icon.ico` from PNG.
- Rust (from `src-tauri/`): `cargo test`, `cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo check`.

## Coding Style & Naming Conventions
- Vue SFC + TypeScript; follow existing 2-space indentation in `.vue` and `.ts`.
- Components use PascalCase filenames (e.g., `SceneryEntryCard.vue`); composables/stores use camelCase (`theme.ts`, `useThemeStore`).
- Prefer Tailwind utility classes for styling; keep custom CSS scoped and minimal.
- Rust code should be formatted with `cargo fmt`; follow standard Rust naming (snake_case functions, PascalCase types).

## Testing Guidelines
- Rust unit tests live in `src-tauri/src` (run with `cargo test`).
- No frontend test framework is configured; validate UI changes manually.
- Use descriptive test names (snake_case) for new Rust tests.

## Commit & Pull Request Guidelines
- Git history shows mixed conventions: many `feat:`/`docs:` prefixes plus plain English. Prefer conventional prefixes when possible (e.g., `feat:`, `fix:`, `docs:`).
- PRs should include a concise summary, testing steps, and screenshots/GIFs for UI changes.
- Update `CHANGELOG.md` for user-facing changes using Keep a Changelog under `[Unreleased]` (don’t list fixes separately for unreleased features).
