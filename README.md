# XFastInstall

**一键安装 X-Plane 插件的现代化工具 | Modern X-Plane Addon Installer**

XFastInstall 是一款专为 X-Plane 飞行模拟器设计的智能插件安装工具，支持 Windows、macOS 和 Linux 平台。告别手动解压和复制文件的繁琐操作，只需拖放即可完成安装。

XFastInstall is an intelligent addon installer designed for X-Plane flight simulator, supporting Windows, macOS, and Linux. Say goodbye to manual extraction and file copying - just drag and drop to install.

---

## ✨ 核心功能 | Key Features

### 🎯 智能识别
自动识别四大类插件类型，无需手动分类：
- **飞机 (Aircraft)**: 自动检测 `.acf` 文件，安装到 `Aircraft/` 目录
- **地景 (Scenery)**: 识别 `library.txt` 或 `.dsf` 文件，安装到 `Custom Scenery/` 目录
- **插件 (Plugins)**: 检测 `.xpl` 文件，自动处理平台特定子目录（win_x64/mac_x64/lin_x64）
- **导航数据 (Navdata)**: 识别 `cycle.json` 文件，支持 GNS430 数据包

Automatically detects four addon types without manual classification:
- **Aircraft**: Detects `.acf` files, installs to `Aircraft/` directory
- **Scenery**: Recognizes `library.txt` or `.dsf` files, installs to `Custom Scenery/`
- **Plugins**: Detects `.xpl` files, handles platform-specific subdirectories automatically
- **Navdata**: Recognizes `cycle.json` files, supports GNS430 data packages

### 📦 全格式支持
支持所有常见压缩格式，包括加密压缩包：
- **.zip** 文件（支持 ZipCrypto 和 AES 加密）
- **.7z** 文件（支持密码保护）
- **.rar** 文件（支持密码保护）
- **文件夹**直接拖放安装

Supports all common archive formats, including encrypted archives:
- **.zip** files (supports ZipCrypto and AES encryption)
- **.7z** files (password-protected)
- **.rar** files (password-protected)
- **Folders** for direct drag-and-drop installation

### 🔍 智能去重
深度扫描并自动去重嵌套插件：
- 自动识别飞机包内的插件组件
- 避免重复安装子目录
- 保持插件完整性

Deep scanning with automatic deduplication of nested addons:
- Automatically recognizes plugin components within aircraft packages
- Prevents duplicate installation of subdirectories
- Maintains addon integrity

### ⚡ 高性能安装
多项性能优化，大幅提升安装速度：
- **并行 ZIP 解压**：多线程解压大型 ZIP 文件
- **并行文件复制**：多核并发复制文件
- **元数据缓存**：5 分钟 TTL 缓存，减少重复扫描
- **4MB 缓冲区**：优化文件 I/O 性能

Multiple performance optimizations for faster installation:
- **Parallel ZIP extraction**: Multi-threaded decompression for large ZIP files
- **Parallel file copying**: Multi-core concurrent file operations
- **Metadata caching**: 5-minute TTL cache reduces repeated scanning
- **4MB buffer**: Optimized file I/O performance

### ⚠️ 安全保护
多重安全检查，保护您的 X-Plane 安装：
- **冲突检测**：安装前警告已存在的插件
- **覆盖模式**：可选择覆盖或跳过已有文件
- **大小限制**：最大解压 20GB，防止磁盘空间耗尽
- **压缩比检查**：最大 100:1 压缩比，防止 ZIP 炸弹攻击
- **路径遍历防护**：防止恶意压缩包访问系统文件

Multiple safety checks to protect your X-Plane installation:
- **Conflict detection**: Warns about existing addons before installation
- **Overwrite mode**: Choose to overwrite or skip existing files
- **Size limits**: 20GB max extraction size prevents disk space exhaustion
- **Compression ratio check**: 100:1 max ratio prevents ZIP bomb attacks
- **Path traversal protection**: Prevents malicious archives from accessing system files

### 🖱️ Windows 右键菜单集成
Windows 用户专享便捷功能：
- 在任意文件或文件夹上右键点击
- 选择"Install to X-Plane"直接安装
- 无需管理员权限（使用 HKEY_CURRENT_USER 注册表）

Windows-exclusive convenience feature:
- Right-click on any file or folder
- Select "Install to X-Plane" for direct installation
- No administrator privileges required (uses HKEY_CURRENT_USER registry)

### 🌍 双语界面
完整的中英文双语支持：
- 自动检测系统语言
- 可手动切换语言
- 所有界面和日志均支持双语

Full bilingual support (Chinese/English):
- Auto-detects system language
- Manual language switching available
- All UI and logs support both languages

### 🌙 现代化界面
航空主题的现代化设计：
- 深色/浅色主题切换
- 直观的拖放操作
- 实时安装进度显示
- 详细的日志记录

Aviation-themed modern design:
- Dark/light theme toggle
- Intuitive drag-and-drop interface
- Real-time installation progress
- Detailed logging system

---

## 🚀 快速开始 | Quick Start

### 下载安装 | Download & Install

1. 前往 [Releases](https://github.com/yourusername/XFastInstall-Tauri/releases) 页面下载最新版本
2. 运行安装程序
3. 首次启动时，在设置中配置 X-Plane 安装路径

1. Go to [Releases](https://github.com/yourusername/XFastInstall-Tauri/releases) to download the latest version
2. Run the installer
3. On first launch, configure your X-Plane installation path in Settings

### 使用方法 | Usage

**方法一：拖放安装 | Method 1: Drag & Drop**
1. 将插件文件或文件夹拖放到主界面
2. 查看自动识别的插件类型和安装位置
3. 点击"安装"按钮完成安装

1. Drag and drop addon files or folders onto the main interface
2. Review auto-detected addon types and installation locations
3. Click "Install" to complete installation

**方法二：右键菜单（仅 Windows）| Method 2: Context Menu (Windows Only)**
1. 在设置中点击"注册右键菜单"
2. 右键点击任意插件文件或文件夹
3. 选择"Install to X-Plane"即可安装

1. Click "Register Context Menu" in Settings
2. Right-click any addon file or folder
3. Select "Install to X-Plane" to install

---

## 🛠️ 开发者信息 | Developer Information

### 技术栈 | Tech Stack
- **前端 | Frontend**: Vue 3 + TypeScript + TailwindCSS
- **后端 | Backend**: Rust + Tauri 2
- **构建 | Build**: Vite + GitHub Actions

### 构建方法 | Build Instructions

```bash
# 安装依赖 | Install dependencies
npm install

# 开发模式 | Development mode
npm run tauri:dev

# 生产构建 | Production build
npm run tauri:build
```

### 许可证 | License
详见 LICENSE 文件 | See LICENSE file for details

---

## 📝 更新日志 | Changelog

查看 [CHANGELOG.md](CHANGELOG.md) 了解版本更新历史。

See [CHANGELOG.md](CHANGELOG.md) for version history.

---

## 🤝 贡献 | Contributing

欢迎提交 Issue 和 Pull Request！

Issues and Pull Requests are welcome!

---

**享受更便捷的 X-Plane 插件安装体验！| Enjoy a more convenient X-Plane addon installation experience!**