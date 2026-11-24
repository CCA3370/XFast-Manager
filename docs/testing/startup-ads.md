# XFast Manager 启动广告投放与验收

## 在 GitHub 投放

1. 打开 XFast Manager 仓库，切换到 `ad-campaigns` 分支。打开 `ad-campaigns/images/`，用 **Add file → Upload files** 上传素材并提交到该分支。
2. 确认图片已经出现在这个分支，再编辑同目录的 `manifest.json`。把总开关和要投放的广告开关都设为 `true`，提交到 `ad-campaigns`。上线、调序和换素材都在这个分支完成。
3. 等待约一分钟，让 API 缓存更新。打开 `/api/ad-campaigns` 核对返回的广告 ID、顺序和素材，再正常重启支持启动广告的客户端检查展示。 [线上投放清单](https://x-fast-manager.vercel.app/api/ad-campaigns)
4. 停投一条时将该条的 `enabled` 设为 `false`。全部停投时关闭清单最外层的 `enabled`。配置在后续启动时生效，已经打开的弹窗可直接关闭。

下面是带时间和中文图片的示例。示例中的图片、日期和链接需要换成实际投放内容。

```json
{
  "schemaVersion": 1,
  "enabled": true,
  "campaigns": [
    {
      "id": "winter-2026",
      "enabled": true,
      "startsAt": "2026-12-01T00:00:00Z",
      "endsAt": "2026-12-15T00:00:00Z",
      "creative": { "image": "winter-en.png", "alt": "Winter promotion" },
      "localizations": {
        "zh": { "image": "winter-zh.png", "alt": "冬季推广活动" }
      },
      "url": "https://example.com/"
    }
  ]
}
```

- 沿用 `id` 更新同一条广告，已屏蔽的用户会继续跳过它。新广告使用新的 ID。启动时按清单顺序查找第一条符合条件、素材能加载且未被本机屏蔽的广告。
- `startsAt` 和 `endsAt` 使用 UTC。到达开始时间即可展示，到达结束时间即停止。省略某一项时，该方向没有时间限制。
- 图片使用 PNG、JPEG 或 WebP，单张最多 2 MiB。文件名以英文字母或数字开头，扩展名之前可使用英文字母、数字、短横线和下划线。`alt` 简述图上的必要内容，供屏幕阅读器使用。跳转链接使用 `https://`。
- `creative` 是所有界面语言的默认素材，有相应 `localizations` 的语言采用对应素材。可用的界面语言代码为 `zh`、`en`、`es`、`fr`、`de`、`ja`、`pt`、`hi`、`ar`、`ru`、`ko`。

## 本地桌面验收

在仓库根目录启动本地广告接口。它使用仓库内已有图片，点击会打开 `https://example.com/`。

```powershell
node docs/testing/startup-ads-fixture.mjs active
```

在另一终端启动桌面开发版。请先退出已运行的 XFast Manager，避免新实例把启动请求转给旧窗口。首次运行时先完成设置并关闭更新说明，再启动一次测试。

```powershell
$env:VITE_XFAST_VERCEL_API_BASE_URL = 'http://localhost:1432/api'
npm run tauri:dev
```

| 操作 | 预期结果 |
| --- | --- |
| 保持窗口空闲直至看到广告，关闭后重启 | 两次都展示第一条广告，每次只弹一次 |
| 勾选「不再展示此广告」后关闭并重启 | 第一条在本机持续跳过，接着展示第二条 |
| 点击广告图 | 默认浏览器打开示例地址，应用中的弹窗关闭 |
| 在中文和英文之间切换后重启 | 第一条分别展示安装界面图和默认海报。测试语言时先完成这一项，再屏蔽第一条 |
| 调为 650 × 500 的最小窗口，切换浅色、深色和阿拉伯语 | 整张图可见，控件可操作。Tab 停留在弹窗内，Esc 关闭后恢复焦点 |
| 首次设置、打开更新说明、带文件启动，或在广告出现前操作界面 | 相应任务优先，应用可继续使用 |
| 弹窗出现后向窗口拖入文件 | 广告让出窗口，文件照常进入处理流程 |
| 分别用 `empty`、`slow` 替换 `active` 重启测试接口，或关闭测试接口再重启客户端 | 应用正常打开。超时后的广告响应不会打断已经在使用的应用 |
| 用 `missing-image` 重启测试接口后启动客户端 | 首条素材加载失败，仍有时间时展示第二条 |

完成后退出两个进程，并在桌面开发终端执行 `Remove-Item Env:VITE_XFAST_VERCEL_API_BASE_URL` 恢复线上接口。测试广告 ID 保存在本机，反复测屏蔽可以使用新的测试配置或给 fixture 中的 ID 加一个新后缀。
