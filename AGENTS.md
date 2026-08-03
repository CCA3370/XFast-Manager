# Repository Guidelines

- Do not touch the existing Map implementation; this part will be removed soon.
- Each time a task is executed, a Commit message should be written and commited once a small stage is completed. Once complete all stage, push them.
- After all stage is completed, please inform me in detail which tests I need to perform manually, and what things you did.
- There's no need to pursue minimal changes; always ensure the current implementation is the optimal solution, stop treating compatibility as a constraint, and focus on ensuring overall optimality.
- Update CHANGELOG.md regularly, focusing on user-centric content. Avoid describing technical information; only describe user-perceptible feature additions/changes/fixes. Pay attention to the timeline and do not include changes/fixes to features that have not yet been released (not merged into the main branch).

## Project Structure & Module Organization
`src/` contains the Vue 3 + TypeScript frontend: page-level views in `src/views/`, reusable UI in `src/components/`, Pinia stores in `src/stores/`, and shared services/utilities in `src/services/` and `src/utils/`. `src/generated/` is generated code; do not edit `src/generated/changelog.ts` by hand. `src-tauri/` contains the Rust/Tauri backend, organized by domain (`analysis/`, `install/`, `management/`, `scenery/`, `services/`, `data/`). Repository-level `api/` holds serverless endpoints, `data/` stores bundled JSON datasets, `public/` and `src-tauri/icons/` hold static assets, and `.github/workflows/` defines CI/release automation.

## Build, Test, and Development Commands
Use Node 22 locally when possible to match GitHub Actions.

- `npm install`: install frontend and Tauri CLI dependencies.
- `npm run dev`: start the Vite frontend only.
- `npm run tauri:dev`: run the desktop app with the frontend and Rust backend together.
- `npm run build`: build the frontend into `dist/`.
- `npm run tauri:build`: create platform bundles through Tauri.
- `npm run lint` / `npm run lint:fix`: check or fix frontend lint issues.
- `npm run format:check` / `npm run format`: verify or apply Prettier formatting in `src/`.
- `cargo test`: run Rust unit tests from the workspace root.

## Coding Style & Naming Conventions
Frontend formatting follows Prettier: 2-space indentation, single quotes, no semicolons, trailing commas, 100-column width, LF endings. ESLint enforces Vue + TypeScript rules; `any` is disallowed. Vue SFCs and major UI components use PascalCase filenames such as `SceneryEntryCard.vue`; composables use `useX.ts`; stores and services use descriptive lower-case filenames. Rust code follows `src-tauri/rustfmt.toml`: 4-space indentation, 100-column width, snake_case modules, and inline unit tests near the implementation.

## Testing Guidelines
Rust tests live inline under `#[cfg(test)]` modules across `src-tauri/src/**`; add new unit tests next to the code they cover. There is no configured frontend unit-test runner yet, so frontend changes should at minimum pass `npm run lint` and a manual `npm run tauri:dev` smoke test. If you touch release notes or update UX, verify the generated changelog bundle still builds.

## Commit & Pull Request Guidelines
Recent history follows Conventional Commits, often with scopes: `feat: ...`, `feat(csl): ...`, `chore(data): ...`. Keep commits focused and imperative. PRs should link related issues, summarize user-visible behavior, list verification steps run, and include screenshots for UI changes. For bug fixes, include reproduction context similar to the issue template: OS, app version, logs, and screenshots when relevant. Update `CHANGELOG.md` and versioned release metadata when preparing release-facing changes.
