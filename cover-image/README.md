# XFast Manager Cover Images

封面图模板文件，用于 X-Plane.org 论坛发布。

## 文件说明

- `cover.html` - 标准版封面（深蓝渐变背景，展示拖拽流程）
- `cover-dark.html` - 暗黑版封面（纯黑背景，大标题+UI预览）

## 如何生成图片

### 方法一：浏览器截图（推荐）

1. 用 Chrome/Edge 打开 HTML 文件
2. 按 `F12` 打开开发者工具
3. 按 `Ctrl+Shift+P` 打开命令面板
4. 输入 `screenshot` 并选择 "Capture node screenshot"
5. 点击封面区域（.cover 元素）
6. 自动保存 600×600 的 PNG 图片

### 方法二：使用截图工具

1. 打开 HTML 文件
2. 使用 Snipaste / ShareX 等工具截取封面区域
3. 确保截图尺寸为 600×600 像素

### 方法三：使用 Puppeteer（高质量）

```bash
npm install puppeteer

node -e "
const puppeteer = require('puppeteer');
(async () => {
  const browser = await puppeteer.launch();
  const page = await browser.newPage();
  await page.setViewport({ width: 600, height: 600 });
  await page.goto('file://' + process.cwd() + '/cover.html');
  await page.screenshot({ path: 'cover.png' });
  await browser.close();
})();
"
```

## 设计规格

- **尺寸**: 600 × 600 像素
- **字体**: Inter（Google Fonts）
- **配色**:
  - 主蓝: #3b82f6
  - 翠绿: #10b981
  - 紫色: #8b5cf6
  - 背景: #0f172a / #000

## 自定义修改

如需修改内容，直接编辑 HTML 文件中的文字：

- 修改标题：搜索 `XFast Manager`
- 修改副标题：搜索 `tagline` 或 `subtitle`
- 修改功能点：搜索 `feature-tag` 或 `feature`

## 使用建议

- **论坛首帖封面**：使用 `cover.html`（信息更丰富）
- **Twitter/社交媒体**：使用 `cover-dark.html`（视觉冲击力更强）
- **GitHub README**：两个都可以，选择与 README 背景对比度高的版本
