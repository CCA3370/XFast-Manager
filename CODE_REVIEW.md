# 代码审查报告

说明：
- 审查方式：静态阅读与搜索，未运行测试或构建。
- 范围：当前仓库（Rust/Tauri + 前端）。

## 高严重度
1. 7z 解压存在路径穿越风险：`entry.name()` 直接 `join` 到临时目录，未做路径净化，可能写出临时目录之外（与 ZIP 解压使用 `sanitize_path` 的做法不一致）。涉及：
   - src-tauri/src/installer.rs:1216
   - src-tauri/src/installer.rs:1232
   - src-tauri/src/installer.rs:3006
   建议：对 7z 入口名统一应用 `sanitize_path` 或拒绝包含 `..`/绝对路径。

## 中等严重度
1. Windows 磁盘空间检查对非 UTF-8 路径会 panic：`to_str().unwrap()` 在非 UTF-8 路径上直接崩溃。
   - src-tauri/src/atomic_installer.rs:615
   建议：使用 `OsStrExt::encode_wide` 直接处理 `OsStr` 或 `to_string_lossy()` 并记录警告。

2. COM 初始化处理不一致，可能发生非对称 `CoUninitialize`：
   - src-tauri/src/scenery_packs_manager.rs:41
   - src-tauri/src/scenery_packs_manager.rs:91
   - 对比 src-tauri/src/scenery_index.rs:41
   - 对比 src-tauri/src/scenery_index.rs:44
   - 对比 src-tauri/src/scenery_index.rs:144
   当返回 `RPC_E_CHANGED_MODE` 时不应 `CoUninitialize`，否则可能影响其它 COM 用户。

3. `scenery_packs.ini` 路径规范化不一致，可能导致重复项或删除失败：
   - src-tauri/src/scenery_packs_manager.rs:384
   - src-tauri/src/scenery_packs_manager.rs:420
   - src-tauri/src/scenery_packs_manager.rs:436
   - src-tauri/src/scenery_packs_manager.rs:582
   入口使用大小写/斜杠/尾斜杠混用，建议集中规范化或统一比对规则。

4. 增量索引更新未重新计算缺失库，状态可能长期陈旧：
   - src-tauri/src/scenery_index.rs:418
   - 对比全量重建时的缺失库更新：src-tauri/src/scenery_index.rs:371
   建议在 `update_index` 结束后增量更新缺失库或触发一次补算。

## 低严重度/优化
1. `auto_sort_from_index` 强制插入 `*GLOBAL_AIRPORTS*` 且 `enabled: true`，可能覆盖用户禁用状态：
   - src-tauri/src/scenery_packs_manager.rs:534
   - src-tauri/src/scenery_packs_manager.rs:539
   - src-tauri/src/scenery_packs_manager.rs:552

2. 更新检查缓存未过期被当作错误返回，前端记录为失败日志：
   - src-tauri/src/updater.rs:62
   - src/stores/update.ts:83
   建议用可区分的状态或返回 `Ok` + 标记，避免误报。

3. `getGlobalIndex` 每次调用都重新 flatten，列表大时退化为 O(n^2)：
   - src/views/SceneryManager.vue:187
   - src/views/SceneryManager.vue:663
   - src/views/SceneryManager.vue:669
   - src/views/SceneryManager.vue:681
   建议缓存全局索引或在渲染前构建 map。

4. 日志输出使用 `file_name().unwrap()`，极端路径可触发 panic：
   - src-tauri/src/scenery_index.rs:266
   - src-tauri/src/scenery_index.rs:273

## 待确认问题/假设
1. 删除源文件逻辑的父子路径判断可能方向相反（注释与条件不一致）：
   - src-tauri/src/installer.rs:3344
   - 调用处：src-tauri/src/installer.rs:499
2. `.lnk` 解析后以目标目录名入库，但启用状态读取基于 ini 中的文件名，可能导致状态丢失：
   - src-tauri/src/scenery_index.rs:255
   - src-tauri/src/scenery_index.rs:610

## 测试与稳定性缺口
- 前端缺少单元/组件测试；搜索/分组/拖拽等关键交互需手测。
- Rust 侧缺少端到端安装流程与 `scenery_packs.ini` 读写/排序的集成测试。
- 更新检查/网络失败/缓存命中路径缺少显式测试用例。
