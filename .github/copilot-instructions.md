## 快速定位（给 AI 编码代理）

- 目标：维护一个 Tauri + Vue 前端 + Rust 后端的桌面安装器（XFastInstall）。
- 主要入口：前端在 `src/`（Vue + TypeScript），后端在 `src-tauri/src/`（Rust crate `xfastinstall_lib`）。

## 大体架构与数据流

- 前端（Vue 3 + Pinia）负责 UI、文件拖拽与用户交互，代码在 `src/`（例如 `src/main.ts`, `src/views/*`）。
- 后端（Rust + Tauri）以 library 形式实现命令，定义在 `src-tauri/src/lib.rs` 并由 `src-tauri/src/main.rs` 调用。
- 通信：前端使用 Tauri 的 invoke 调用后台命令（例：`analyze_addons`, `install_addons`, `register_context_menu`）。示例：
  - `await invoke<AnalysisResult>('analyze_addons', { paths, xplanePath })` （查看 `src/views/Home.vue`）
  - 后端在 `lib.rs` 用 `#[tauri::command]` 暴露函数并在 builder 中通过 `tauri::generate_handler!` 注册。
- 事件：后端在启动时会将 CLI 参数通过 `app.emit("cli-args", args)` 发到前端（见 `src-tauri/src/lib.rs` 的 setup）。

## 关键文件与模式（示例引用）

- 前端路由与状态：`src/main.ts`（router）、`src/stores/app.ts`（Pinia store，持有 xplanePath/currentTasks 状态）。
- 类型同步：Rust 的 `src-tauri/src/models.rs` 使用 serde，并通过命名策略（PascalCase / camelCase）与前端 `src/types/index.ts` 对应 —— 注意字段命名差异（`InstallTask` 中 Rust 的 `source_path` 对应 TS 的 `sourcePath`）。
- 安装逻辑：`src-tauri/src/installer.rs` 包含目录复制与 zip/7z 解压实现（使用 `zip` 与 `sevenz-rust`）。
- Windows 集成：右键菜单注册在 `src-tauri/src/registry.rs`（使用 `winreg`，仅在 Windows target 编译时生效）。

## 构建 / 开发工作流（明确可复现命令）

- 安装依赖：`npm install`（前端），并确保已安装 Rust toolchain。
- 本地开发（热重载 UI + Tauri）：
  1. `npm run tauri:dev` — 会先运行 Vite（port 1420，参见 `vite.config.ts`），然后启动 Tauri dev。`tauri.conf.json` 的 `devUrl` 指向 `http://localhost:1420`。
  2. 如果只调试前端：`npm run dev` （Vite）
- 打包发布：`npm run tauri:build`（调用 `tauri build`，`beforeBuildCommand` 会先运行 `npm run build`）
- 生成 Windows ico：`npm run generate:icon`（脚本在 `package.json`，源为 `src-tauri/icons/icon.png`）。

## 项目约定与注意事项（不要猜测，直接按现有模式）

- 命名/序列化：Rust 侧使用 serde 的 `rename_all` 属性（见 `models.rs`）——在修改模型时同时检查 `src/types/index.ts` 是否需要同步。
- 后端命令必须用 `#[tauri::command]` 标注并加入 `tauri::generate_handler!`，否则前端 `invoke` 会找不到。
- Vite 的 dev 端口被固定为 1420（`vite.config.ts`），改变时要同步更新 `src-tauri/tauri.conf.json` 的 `devUrl`。
- Windows 专用代码用 cfg(target_os = "windows") 条件编译；不要在非 Windows 环境尝试运行注册上下文菜单函数。

## 常见修改点与示例 PR 方向

- 添加新的后端命令：编辑 `src-tauri/src/lib.rs`，在同一文件声明函数并导出到 handler，然后在前端 `src/views/*` 用 `invoke` 调用。
- 变更数据契约：修改 `models.rs` -> 更新 `src/types/index.ts` -> 运行并手动验证前端/后端通信（invoke 返回值与 TS 类型匹配）。

## 可参考的文件清单（首要）

- `src/` — Vue 前端（主要：`src/main.ts`, `src/views/Home.vue`, `src/views/Settings.vue`, `src/stores/`）
- `src-tauri/src/lib.rs` — Tauri 命令暴露与 app setup
- `src-tauri/src/*.rs` — 业务逻辑（`installer.rs`, `analyzer.rs`, `registry.rs`, `models.rs`）
- `vite.config.ts`, `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`

如果这些要点有遗漏或你希望我在某些区域（例如：analyzer 实现细节、打包产物签名或 CI 流水线）补充更具体的示例，请告诉我我将迭代更新此文件。
