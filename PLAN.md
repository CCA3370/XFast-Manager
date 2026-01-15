# XFastInstall 任务规划

## 概述
本规划文档详细说明了 TODO.md 中列出的功能实现计划，包括任务取消/跳过功能和原子安装功能。

---

## 任务 1: 添加取消和跳过任务功能

### 1.1 前端 UI 实现

#### 1.1.1 按钮组件设计
**位置**: `src/views/Home.vue`

- 在安装进度显示区域下方添加按钮组
- **跳过按钮** (左侧):
  - 样式: 黄色/橙色警告色
  - 图标: 跳过图标
  - 文本: "跳过当前任务" (i18n)

- **取消按钮** (右侧):
  - 样式: 红色危险色
  - 图标: 停止/取消图标
  - 文本: "取消所有任务" (i18n)

#### 1.1.2 确认对话框
**组件**: 使用现有 Modal 系统或创建新的确认对话框

**跳过任务对话框**:
- 标题: "确认跳过任务"
- 内容:
  - 显示当前任务名称
  - 如果是干净安装/覆盖安装: 警告 "⚠️ 跳过此任务将导致原有文件丢失"
  - 提示: "已处理的文件将被删除"
- 按钮: "确认跳过" / "取消"

**取消所有任务对话框**:
- 标题: "确认取消所有任务"
- 内容:
  - 显示剩余任务数量
  - 如果包含干净安装/覆盖安装: 警告 "⚠️ 取消任务将导致部分原有文件丢失"
  - 提示: "所有已处理的文件将被删除"
- 按钮: "确认取消" / "取消"

#### 1.1.3 状态管理
**Store**: `src/stores/app.ts`

添加新状态:
```typescript
interface AppState {
  // 现有状态...
  isInstalling: boolean
  currentTaskIndex: number
  installationCancelled: boolean
  taskSkipRequested: boolean
}
```

添加新 actions:
- `requestCancelInstallation()`: 请求取消所有任务
- `requestSkipCurrentTask()`: 请求跳过当前任务
- `resetInstallationState()`: 重置安装状态

### 1.2 后端 Rust 实现

#### 1.2.1 任务控制机制
**文件**: `src-tauri/src/installer.rs`

实现方案:
- 使用 `Arc<AtomicBool>` 作为取消标志，在安装循环中定期检查
- 为每个任务记录已创建的文件/目录列表
- 实现回滚函数清理已创建的文件

新增结构:
```rust
pub struct InstallationControl {
    cancel_flag: Arc<AtomicBool>,
    skip_flag: Arc<AtomicBool>,
    processed_files: Arc<Mutex<Vec<PathBuf>>>,
}
```

#### 1.2.2 Tauri 命令
**文件**: `src-tauri/src/lib.rs`

新增命令:
```rust
#[tauri::command]
async fn cancel_installation(state: State<'_, InstallationControl>) -> Result<(), String>

#[tauri::command]
async fn skip_current_task(state: State<'_, InstallationControl>) -> Result<(), String>
```

#### 1.2.3 清理逻辑
**文件**: `src-tauri/src/installer.rs`

实现函数:
- `cleanup_partial_installation()`: 删除部分安装的文件
- 对于覆盖安装/干净安装，需要特殊处理:
  - 如果已删除原目录但未完成新安装，无法恢复（需在对话框中明确警告）
  - 记录操作日志以便用户了解发生了什么

### 1.3 国际化
**文件**: `src/i18n/zh.ts`, `src/i18n/en.ts`

添加翻译键:
- `install.skipTask`: "跳过当前任务" / "Skip Current Task"
- `install.cancelAll`: "取消所有任务" / "Cancel All Tasks"
- `install.confirmSkip`: "确认跳过" / "Confirm Skip"
- `install.confirmCancel`: "确认取消" / "Confirm Cancel"
- `install.skipWarning`: "跳过此任务将导致原有文件丢失" / "Skipping this task will cause original files to be lost"
- `install.cancelWarning`: "取消任务将导致部分原有文件丢失" / "Cancelling tasks will cause some original files to be lost"
- `install.cleanupInProgress`: "正在清理已处理的文件..." / "Cleaning up processed files..."

---

## 任务 2: 添加原子安装开关设置

### 2.1 前端 UI 实现

#### 2.1.1 设置页面更新
**文件**: `src/views/Settings.vue`

添加折叠栏组件:
- 标题: "原子安装模式" (带开关按钮)
- 默认状态: 关闭
- 展开内容:
  ```
  原子安装模式说明：

  启用后，安装过程将更加安全可靠：
  - 先将文件解压/复制到临时目录
  - 使用原子操作移动文件，确保操作完整性
  - 如果安装失败，原有文件不会损坏
  - 对于覆盖安装，会保留旧文件直到新文件完全就绪

  注意：原子安装需要更多磁盘空间（临时目录）
  ```

#### 2.1.2 状态持久化
**Store**: `src/stores/app.ts`

添加状态:
```typescript
interface AppState {
  // 现有状态...
  atomicInstallEnabled: boolean
}
```

持久化到 localStorage: `atomicInstallEnabled` 键

### 2.2 国际化
添加翻译:
- `settings.atomicInstall`: "原子安装模式" / "Atomic Installation Mode"
- `settings.atomicInstallDesc`: [详细说明文本]

---

## 任务 3: 实现原子安装功能

### 3.1 架构设计

#### 3.1.1 临时目录管理
**文件**: `src-tauri/src/installer.rs`

新增模块:
```rust
mod atomic {
    pub struct AtomicInstaller {
        temp_dir: PathBuf,
        target_dir: PathBuf,
        backup_dir: Option<PathBuf>,
    }
}
```

临时目录位置:
- 与目标目录同盘（确保原子移动可行）
- 路径: `{target_drive}/.xfastinstall_temp_{timestamp}_{random}`

#### 3.1.2 安装流程

**场景 1: 首次安装（目标目录不存在）**
```
1. 解压/复制到临时目录
2. 验证文件完整性
3. 原子移动: temp_dir -> target_dir
4. 清理临时目录
```

**场景 2: 干净安装（目标目录存在）**
```
1. 解压/复制到临时目录
2. 验证文件完整性
3. 重命名原目录: target_dir -> target_dir.origin
4. 原子移动: temp_dir -> target_dir
5. 校验新安装
6. 备份文件处理:
   - 识别需要保留的文件（涂装、配置等）
   - 从 target_dir.origin 原子移动到 target_dir
   - 对于涂装，跳过已存在的文件
7. 删除 target_dir.origin
```

**场景 3: 覆盖安装（目标目录存在）**
```
1. 解压/复制到临时目录
2. 验证文件完整性
3. 遍历临时目录中的文件:
   - 对于每个文件，原子移动覆盖目标文件
   - 保留目标目录中不存在于新文件的旧文件
4. 清理临时目录
```

### 3.2 核心功能实现

#### 3.2.1 原子移动函数
**文件**: `src-tauri/src/installer.rs`

```rust
fn atomic_move(src: &Path, dst: &Path) -> Result<(), InstallerError> {
    // 使用 std::fs::rename (同盘原子操作)
    // 如果失败，回退到复制+删除
}
```

#### 3.2.2 备份文件识别
**文件**: `src-tauri/src/models.rs` 或新建 `src-tauri/src/backup.rs`

需要保留的文件模式:
- 涂装目录: `liveries/`, `livery/`, `paint/`
- 配置文件: `*.cfg`, `*.ini`, `*.conf`
- 用户数据: `userdata/`, `settings/`

```rust
fn should_preserve_file(path: &Path, addon_type: &AddonType) -> bool {
    // 根据插件类型和文件路径判断是否需要保留
}
```

#### 3.2.3 文件校验
**文件**: `src-tauri/src/installer.rs`

```rust
fn verify_installation(dir: &Path, expected_files: &[PathBuf]) -> Result<(), InstallerError> {
    // 验证所有预期文件都存在
    // 可选: 验证文件大小/哈希
}
```

### 3.3 错误处理和回滚

#### 3.3.1 错误场景
- 临时目录创建失败
- 磁盘空间不足
- 原子移动失败
- 校验失败

#### 3.3.2 回滚策略
```rust
impl AtomicInstaller {
    fn rollback(&self) -> Result<(), InstallerError> {
        // 如果有 backup_dir，恢复原目录
        // 清理临时目录
        // 清理部分安装的文件
    }
}
```

### 3.4 性能优化

- 对于大文件，使用并行复制到临时目录
- 原子移动操作本身很快（只是元数据操作）
- 进度报告需要考虑多阶段（解压、移动、备份恢复）

### 3.5 日志记录

**文件**: `src-tauri/src/logger.rs`

记录关键操作:
- 临时目录创建/删除
- 原子移动操作
- 备份文件处理
- 回滚操作

---

## 实现顺序建议

### 阶段 1: 任务取消/跳过功能（任务 1）
1. 后端控制机制实现（2-3小时）
2. 前端 UI 和状态管理（2小时）
3. 清理逻辑实现（2小时）
4. 国际化和测试（1小时）

### 阶段 2: 原子安装开关（任务 2）
1. 设置页面 UI（1小时）
2. 状态持久化（30分钟）
3. 国际化（30分钟）

### 阶段 3: 原子安装核心功能（任务 3）
1. 临时目录管理和原子移动基础（3小时）
2. 首次安装场景实现（2小时）
3. 干净安装场景实现（4小时）
4. 覆盖安装场景实现（3小时）
5. 备份文件识别和处理（2小时）
6. 错误处理和回滚（3小时）
7. 测试和优化（4小时）

**总计预估**: 约 30 小时的开发工作

---

## 测试计划

### 单元测试
- Rust: 原子移动函数测试
- Rust: 备份文件识别测试
- Rust: 回滚逻辑测试

### 集成测试
- 首次安装流程
- 干净安装流程（带备份恢复）
- 覆盖安装流程
- 任务取消/跳过流程
- 磁盘空间不足场景
- 权限错误场景

### 手动测试清单
- [ ] 跳过任务按钮功能
- [ ] 取消所有任务按钮功能
- [ ] 干净安装时的警告提示
- [ ] 原子安装开关切换
- [ ] 原子安装 - 首次安装
- [ ] 原子安装 - 干净安装（验证涂装保留）
- [ ] 原子安装 - 覆盖安装（验证旧文件保留）
- [ ] 非原子安装仍正常工作
- [ ] 多语言界面正确显示

---

## 潜在问题和解决方案

### 问题 1: 跨盘原子移动
**问题**: `std::fs::rename` 不支持跨盘操作
**解决**: 确保临时目录与目标目录同盘

### 问题 2: 磁盘空间
**问题**: 原子安装需要额外空间（临时目录 + 可能的备份）
**解决**:
- 安装前检查可用空间
- 在 UI 中明确提示空间需求
- 提供非原子安装选项

### 问题 3: 备份文件冲突
**问题**: 涂装文件名可能与新安装冲突
**解决**:
- 对于涂装，跳过已存在的文件（保留新安装的）
- 记录跳过的文件到日志

### 问题 4: 长路径问题（Windows）
**问题**: Windows 路径长度限制
**解决**:
- 使用 `\\?\` 前缀
- 临时目录使用短路径名

### 问题 5: 权限问题
**问题**: 某些文件可能被占用或无权限
**解决**:
- 详细的错误提示
- 提供重试机制
- 记录到日志

---

## 代码审查要点

1. **安全性**:
   - 路径遍历漏洞检查
   - 符号链接处理
   - 权限验证

2. **可靠性**:
   - 错误处理完整性
   - 回滚逻辑正确性
   - 边界条件处理

3. **性能**:
   - 大文件处理效率
   - 并行操作正确性
   - 内存使用合理性

4. **用户体验**:
   - 进度报告准确性
   - 错误消息清晰度
   - 国际化完整性

---

## 文档更新

完成后需要更新:
- `CLAUDE.md`: 添加原子安装架构说明
- `README.md`: 添加新功能说明
- 用户文档: 原子安装模式使用指南
