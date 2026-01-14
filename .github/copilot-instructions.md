# Copilot 指南（XFastInstall-Tauri）

## 大图景
- 本项目是 **Tauri 2 + Rust** 后端（`src-tauri/src/`）+ **Vue 3 + TS** 前端（`src/`，Pinia + vue-i18n + Tailwind）的跨平台 X-Plane 插件安装器。
- 典型数据流：`Home.vue` 收到拖拽/CLI 文件 → `invoke('analyze_addons')` → Rust `scanner.rs`/`analyzer.rs` 生成 `InstallTask[]` → 用户确认 → `invoke('install_addons')` → Rust `installer.rs` 安装并 `emit('install-progress')`。

## 开发/构建命令
- 安装依赖：`npm install`
- 开发（Tauri + 热更新）：`npm run tauri:dev`
- 生产构建：`npm run tauri:build`
- 仅前端：`npm run dev`
- Rust（在 `src-tauri/`）：`cargo test` / `cargo fmt` / `cargo clippy --all-targets --all-features`

## 前端约定（Vue/TS）
- 路径别名：`@` → `src`（见 `vite.config.ts`）。
- 与 Rust 通讯只用 Tauri `invoke()`/`listen()`（`@tauri-apps/api/core`、`@tauri-apps/api/event`）：
  - commands：`analyze_addons({ paths, xplanePath, passwords, verificationPreferences })`、`install_addons({ tasks })`、`validate_xplane_path`、`check_path_exists`、`register_context_menu`/`unregister_context_menu`、日志相关 commands。
  - events：`cli-args`（启动参数/单实例转发）、`install-progress`（安装进度）。
- 启动时不要再用 `invoke('get_cli_args')` 做同步拉取：当前实现依赖 `App.vue`/`Home.vue` 的 `listen('cli-args')`，并通过 `stores/app.ts` 的 `addCliArgsToBatch()` 做 500ms 合并去重。
- 进度渲染：`Home.vue` 监听 `install-progress`，交给 `stores/progress.ts`（用 `requestAnimationFrame` 节流以平滑动画）。
- 日志：优先使用 `src/services/logger.ts` 的 `logBasic`/`logOperation`/`logDebug`/`logError`（生产构建会通过 `vite.config.ts` 的 esbuild `drop: ['console','debugger']` 移除 `console.*`）。
- i18n：`src/i18n/index.ts` 只在 `zh-*` 显示中文，其余默认英文；语言切换请用 `setLocale()` 并同步后端 `set_log_locale`（`App.vue` 会在 `onMounted` 调用 `syncLocaleToBackend()`）。

## Rust 后端约定（Tauri commands + 管线）
- Tauri commands 定义在 `src-tauri/src/lib.rs`，耗时/CPU/IO 操作放进 `tokio::task::spawn_blocking`，避免阻塞 UI。
- 模块管线：`scanner.rs`（扫描/识别/密码需求）→ `analyzer.rs`（去重/生成任务/目标路径）→ `installer.rs`（解压/复制/覆盖/进度/校验）。
- 类型与序列化：核心数据结构在 `src-tauri/src/models.rs`，字段命名使用 serde rename 规则（`AddonType` 为 `PascalCase`，大部分结构为 `camelCase`）。前端 `src/types/index.ts` 与其保持一一对应（字段名/可选性/枚举值一致）。
- 安全/健壮性：`installer.rs` 内有 `sanitize_path()` 防路径穿越；并使用 `MAX_EXTRACTION_SIZE`/`MAX_COMPRESSION_RATIO` 做可疑压缩包防护。

## CI（GitHub Actions）
- 提交信息包含 `dbuild` 会触发“极速构建（仅测试）”配置；不包含则走生产优化构建（详见 `.github/workflows/README.md`）。
