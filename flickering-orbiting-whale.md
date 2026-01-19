# 地景排序功能完整代码分析文档

## 概述

XFastInstall 的地景排序功能用于自动管理 X-Plane 的 `scenery_packs.ini` 文件，确保地景按正确的优先级加载，获得最佳渲染效果。

---

## 1. 文件结构总览

| 文件 | 路径 | 用途 |
|------|------|------|
| scenery_packs_manager.rs | src-tauri/src/ | 解析、排序、写入 scenery_packs.ini |
| scenery_index.rs | src-tauri/src/ | 地景分类索引管理 |
| scenery_classifier.rs | src-tauri/src/ | 地景分类算法核心 |
| models.rs | src-tauri/src/ | 数据结构定义 |
| lib.rs | src-tauri/src/ | Tauri 命令导出 |
| installer.rs | src-tauri/src/ | 安装时集成排序 |
| app.ts | src/stores/ | 前端状态管理 |
| Settings.vue | src/views/ | 地景排序 UI |
| Home.vue | src/views/ | 安装参数传递 |
| en.ts, zh.ts | src/i18n/ | 国际化文本 |

---

## 2. Rust 后端详解

### 2.1 scenery_packs_manager.rs (593行)

**核心结构:**
```rust
pub struct SceneryPacksManager {
    xplane_path: PathBuf,
    ini_path: PathBuf,  // XPlane_Root/Custom Scenery/scenery_packs.ini
}
```

**关键方法:**

| 方法 | 行号 | 功能 |
|------|------|------|
| parse_ini() | 120-149 | 解析 scenery_packs.ini 文件 |
| sort_entries() | 152-178 | 按优先级排序条目 |
| get_entry_priority() | 205-237 | 计算单个条目的优先级 |
| get_actual_folder_name() | 181-200 | 解析 .lnk 快捷方式的实际文件夹名 |
| write_ini() | 240-278 | 写入排序结果（原子化写入） |
| auto_sort() | 297-381 | 自动排序主函数 |
| backup_ini() | 281-294 | 创建备份文件 |
| add_entry() | 384-417 | 安装后在正确位置添加条目 |
| sync_with_folder() | 436-482 | 同步文件夹中所有地景到 .ini |
| get_category_counts() | 485-506 | 获取各类别地景数量统计 |

**parse_ini() 解析逻辑:**
- 跳过 header 行: "I", "1000 Version", "SCENERY"
- 解析 `SCENERY_PACK` 行 → enabled = true
- 解析 `SCENERY_PACK_DISABLED` 行 → enabled = false
- 检测 `*GLOBAL_AIRPORTS*` 特殊标记

**sort_entries() 排序逻辑:**
1. 首先按 SceneryCategory 优先级排序
2. 同优先级下按文件夹名称字母排序（不区分大小写）
3. 处理快捷方式 (.lnk) 名称解析

**write_ini() 写入格式:**
```
I
1000 Version
SCENERY

SCENERY_PACK Custom Scenery/FolderName/
SCENERY_PACK_DISABLED Custom Scenery/DisabledFolder/
```

---

### 2.2 scenery_index.rs (575行)

**核心结构:**
```rust
pub struct SceneryIndexManager {
    xplane_path: PathBuf,
    index_path: PathBuf,
    // 索引存储位置:
    // Windows: %LOCALAPPDATA%/XFastInstall/scenery_index.json
    // macOS: ~/Library/Application Support/XFastInstall/scenery_index.json
    // Linux: ~/.config/xfastinstall/scenery_index.json
}
```

**关键方法:**

| 方法 | 行号 | 功能 |
|------|------|------|
| rebuild_index() | 228-316 | 完全重建索引（扫描所有地景） |
| update_index() | 319-421 | 增量更新索引（只更新修改过的） |
| load_index() | 175-190 | 从 JSON 加载索引 |
| save_index() | 193-206 | 原子化保存索引 |
| resolve_shortcut() | 16-148 | Windows 快捷方式解析（COM API） |

**索引 JSON 结构:**
```json
{
  "version": 1,
  "packages": {
    "FolderName": {
      "folder_name": "FolderName",
      "category": "Airport",
      "sub_priority": 0,
      "last_modified": 1234567890,
      "has_apt_dat": true,
      "has_dsf": true,
      "has_library_txt": false,
      "has_textures": true,
      "has_objects": true,
      "texture_count": 50,
      "indexed_at": 1234567890,
      "required_libraries": ["opensceneryx"],
      "missing_libraries": []
    }
  },
  "last_updated": 1234567890
}
```

---

### 2.3 scenery_classifier.rs (1050行)

**主函数:** `classify_scenery()` (行 47-367)

**分类决策树:**
```
┌─ Has apt.dat OR WorldEditor creation_agent?
│  └→ Airport
│
├─ Has sim/overlay = 1 (but no apt.dat)?
│  └→ Overlay
│
├─ Has library.txt BUT NO Earth nav data?
│  ├─ Is SAM library?
│  │  └→ FixedHighPriority
│  └─ Else
│     └→ Library
│
├─ Has Earth nav data BUT NO apt.dat?
│  ├─ Is Ortho4XP?
│  │  └→ Orthophotos
│  └─ Else
│     └→ Mesh
│
├─ Has TERRAIN_DEF?
│  └→ Mesh
│
└─ Otherwise
   └→ Other
```

**关键检测方法:**
- `has_apt_dat`: 在 Earth nav data 目录递归搜索 apt.dat
- `has_dsf`: 搜索 .dsf 文件
- `has_library_txt`: 检测 library.txt
- `has_plugins`: 检测 plugins 文件夹中的 .xpl 文件

**DSF 头解析提取属性:**
- `sim/overlay` - 叠加地景标记
- `sim/creation_agent` - 创建工具（WorldEditor, Ortho4XP）
- `sim/require_object` - 必需对象库
- 对象和地形引用

**SAM 库检测逻辑 (行 232-280):**
- 分割文件夹名称（使用非字母数字字符）
- 搜索 "sam" 作为独立单词
- 检测模式: "openSAM", "SAM_Library" 等
- 排除误匹配: "sample", "zsam" 等

**子优先级计算 (行 978-1000):**
```rust
match category {
    SceneryCategory::Orthophotos => {
        if folder_name_lower.contains("xpme") { 2 } else { 0 }
    }
    SceneryCategory::Mesh => {
        if folder_name_lower.contains("xpme") { 2 } else { 1 }
    }
    _ => 0
}
```

---

### 2.4 models.rs - 数据结构定义

**SceneryCategory 枚举（优先级从高到低）:**
```rust
pub enum SceneryCategory {
    FixedHighPriority = 0,  // SAM 库等必须最先加载
    Airport = 1,             // 机场地景（apt.dat）
    DefaultAirport = 2,      // *GLOBAL_AIRPORTS*
    Library = 3,             // 库地景（library.txt）
    Other = 4,               // 其他
    Overlay = 5,             // 叠加地景（sim/overlay=1）
    Orthophotos = 6,         // 正射影像
    Mesh = 6,                // 地形网格（与 Orthophotos 相同）
}
```

**SceneryPackageInfo 结构:**
```rust
pub struct SceneryPackageInfo {
    pub folder_name: String,
    pub category: SceneryCategory,
    pub sub_priority: u8,
    pub last_modified: SystemTime,
    pub has_apt_dat: bool,
    pub has_dsf: bool,
    pub has_library_txt: bool,
    pub has_textures: bool,
    pub has_objects: bool,
    pub texture_count: usize,
    pub indexed_at: SystemTime,
    pub required_libraries: Vec<String>,
    pub missing_libraries: Vec<String>,
}
```

**SceneryPackEntry 结构:**
```rust
pub struct SceneryPackEntry {
    pub enabled: bool,              // SCENERY_PACK vs SCENERY_PACK_DISABLED
    pub path: String,               // "Custom Scenery/FolderName/"
    pub is_global_airports: bool,   // "*GLOBAL_AIRPORTS*" 标记
}
```

---

### 2.5 lib.rs - Tauri 命令导出

| 命令 | 参数 | 返回值 | 功能 |
|------|------|--------|------|
| get_scenery_classification | xplane_path, folder_name | SceneryPackageInfo | 获取单个地景分类 |
| sort_scenery_packs | xplane_path | () | 自动排序 .ini |
| rebuild_scenery_index | xplane_path | SceneryIndexStats | 完全重建索引 |
| get_scenery_index_stats | xplane_path | SceneryIndexStats | 获取索引统计 |
| sync_scenery_packs_with_folder | xplane_path | usize | 同步文件夹到 .ini |

---

### 2.6 installer.rs - 安装时自动排序

**集成点 (行 510-540):**
```rust
if auto_sort_scenery && (task.addon_type == AddonType::Scenery ||
                          task.addon_type == AddonType::SceneryLibrary) {
    // 分类新安装的地景
    match classify_scenery(target_path, &xplane_path_buf) {
        Ok(scenery_info) => {
            // 添加条目到 scenery_packs.ini（在正确位置）
            manager.add_entry(folder_name, &scenery_info.category)
        }
        Err(e) => log_error(...)
    }
}
```

---

## 3. 前端实现

### 3.1 app.ts - 状态管理

```typescript
// 自动排序地景开关（持久化到 localStorage）
const autoSortScenery = ref(false)

// 从 localStorage 加载
const savedAutoSortScenery = localStorage.getItem('autoSortScenery')
if (savedAutoSortScenery !== null) {
  autoSortScenery.value = JSON.parse(savedAutoSortScenery)
}

// 切换方法
function toggleAutoSortScenery() {
  autoSortScenery.value = !autoSortScenery.value
  localStorage.setItem('autoSortScenery', JSON.stringify(autoSortScenery.value))
}
```

### 3.2 Settings.vue - UI (行 759-884)

**UI 组件:**
1. 标题: "Scenery Auto-Sorting"
2. 描述: "Automatically sort scenery_packs.ini after scenery installation"
3. 标签: "Experimental"
4. 切换开关

**展开内容:**
- 功能说明（3点）
- "Rebuild" 按钮 → 调用 `rebuild_scenery_index`
- "Sort All Scenery Now" 按钮 → 调用 `sort_scenery_packs`

**脚本逻辑 (行 1411-1414):**
```typescript
const handleSortSceneryNow = async () => {
  isSortingScenery.value = true
  await invoke('sort_scenery_packs', { xplanePath: store.xplanePath })
  // 显示成功/失败提示
  isSortingScenery.value = false
}
```

### 3.3 Home.vue - 安装参数传递 (行 783)

```typescript
const result = await invoke('install_addons', {
  tasks: enabledTasks,
  xplanePath: store.xplanePath,
  autoSortScenery: store.autoSortScenery,  // 传递用户设置
})
```

---

## 4. 完整优先级排序规则

```
优先级（数值越低，加载优先级越高）:

0. FixedHighPriority
   - SAM 库 (openSAM, SAM_*, etc.)
   - 必须最先加载

1. Airport
   - 有 apt.dat 的机场地景
   - 或 WorldEditor 创建的地景

2. DefaultAirport
   - *GLOBAL_AIRPORTS* 特殊标记
   - X-Plane 默认机场

3. Library
   - 有 library.txt 但无 Earth nav data 的库

4. Other
   - 无明显特征的其他地景
   - 包括仅有插件的文件夹

5. Overlay
   - sim/overlay = 1 的叠加地景
   - 修改默认地形/对象

6. Orthophotos / Mesh
   - 正射影像和地形网格（优先级相同）
   - 通过 sub_priority 区分:
     * Orthophotos: 0 (普通), 2 (XPME)
     * Mesh: 1 (普通), 2 (XPME)
     * XPME 版本加载最后

同优先级内排序: 按文件夹名称字母排序（不区分大小写）
```

---

## 5. 数据流程图

### 5.1 安装地景时的自动排序流程

```
用户拖放地景文件
    ↓
Home.vue: analyze_addons
    ↓
显示地景列表确认
    ↓
用户点击"Install" (如果 autoSortScenery = true)
    ↓
Home.vue 调用 install_addons (传递 autoSortScenery: true)
    ↓
Installer::install()
    ↓
对每个地景任务:
    ├─ 提取文件到目标路径
    ├─ classify_scenery() 分类地景
    ├─ SceneryPacksManager::add_entry() 添加到 .ini
    └─ (按优先级自动插入)
    ↓
安装完成
```

### 5.2 手动排序流程

```
用户在 Settings 中点击 "Sort All Scenery Now"
    ↓
Settings.vue: handleSortSceneryNow()
    ↓
调用 sort_scenery_packs Tauri 命令
    ↓
SceneryPacksManager::auto_sort()
    ├─ 加载/重建索引
    ├─ 解析现有 scenery_packs.ini
    ├─ 保留 enabled/disabled 状态
    ├─ 保留 *GLOBAL_AIRPORTS* 条目
    ├─ 从索引构建条目列表
    ├─ 排序条目（按优先级）
    ├─ 创建备份 (scenery_packs.ini.backup.YYYYMMDD_HHMMSS)
    └─ 写入排序后的 .ini
    ↓
显示成功/失败提示
```

### 5.3 重建索引流程

```
用户点击 "Rebuild" 按钮
    ↓
Settings.vue: handleRebuildIndex()
    ↓
调用 rebuild_scenery_index Tauri 命令
    ↓
SceneryIndexManager::rebuild_index()
    ├─ 扫描 Custom Scenery 目录
    ├─ 处理 .lnk 快捷方式
    ├─ 并行分类每个地景包（rayon）
    ├─ 保存为 JSON 索引
    └─ 返回统计信息
    ↓
显示重建结果
```

---

## 6. 配置和存储位置

| 文件 | 路径 |
|------|------|
| scenery_packs.ini | X-Plane/Custom Scenery/scenery_packs.ini |
| 索引文件 (Windows) | %LOCALAPPDATA%/XFastInstall/scenery_index.json |
| 索引文件 (macOS) | ~/Library/Application Support/XFastInstall/scenery_index.json |
| 索引文件 (Linux) | ~/.config/xfastinstall/scenery_index.json |
| 备份文件 | Custom Scenery/scenery_packs.ini.backup.YYYYMMDD_HHMMSS |
| 用户设置 | localStorage ('autoSortScenery' key) |

---

## 7. 已实现功能清单

- ✅ 自动地景分类（8个类别）
- ✅ 优先级排序
- ✅ 备份保护
- ✅ .lnk 快捷方式支持
- ✅ SAM 库特殊处理
- ✅ DSF 解析（含 7z 压缩）
- ✅ 库依赖检测
- ✅ 增量索引更新
- ✅ 安装时自动排序集成
- ✅ 手动排序功能
- ✅ enabled/disabled 状态保留
- ✅ *GLOBAL_AIRPORTS* 特殊处理
