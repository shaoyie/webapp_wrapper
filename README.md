# 创作者桌面封装

Tauri 桌面壳，用于在 Linux / Windows / macOS 上“浅封装”既有的 Web App。默认将 `http://192.168.1.92:3333` 嵌入为主界面，并通过命令行参数或环境变量覆盖 URL 与应用名称。

## 快速开始

```bash
pnpm install
# 准备默认 app（creator）并启动桌面壳
pnpm run prepare-app
pnpm tauri dev
```

仅需提供 `appId` 即可切换不同封装：

```bash
# 先复制图标等资产
pnpm run prepare-app -- --app-id=storyteller
# 运行或构建时只需要提供 APP_ID
APP_ID=storyteller pnpm tauri dev
APP_ID=storyteller pnpm tauri build
```

> 不要使用 `pnpm tauri dev -- --app-id=...` 或 `pnpm tauri build -- --app-id=...`。该参数会被传给 `cargo`，导致 `unexpected argument '--app-id'`。
>
> `beforeDevCommand` / `beforeBuildCommand` 会自动执行 `pnpm run prepare-app`，因此在 CI 或 `pnpm tauri dev/build` 场景只需提供 `APP_ID` 环境变量即可。

## 构建

```bash
# 默认 creator 配置
pnpm tauri build

# 选择其他 appId
APP_ID=storyteller pnpm tauri build
```

构建结果位于 `src-tauri/target/release/bundle/`，包含 AppImage、MSI/EXE、DMG/ZIP 等平台产物。

## appId 配置

- 所有配置集中在 `app-profiles/apps.json`，结构如下：

```json
{
  "defaultAppId": "creator",
  "profiles": {
    "creator": {
      "productName": "创作者",
      "appName": "创作者",
      "appUrl": "http://192.168.1.92:3333",
      "iconDir": "app-profiles/creator/icons"
    },
    "storyteller": {
      "productName": "创作者",
      "appName": "创作者",
      "appUrl": "https://storyteller.example.com",
      "iconDir": "app-profiles/storyteller/icons"
    }
  }
}
```

- 每个 profile 拥有独立的图标目录（例如 `app-profiles/storyteller/icons/`）。
- `scripts/select-app.mjs` 会根据 `APP_ID`/`--app-id` 复制对应图标到 `src-tauri/generated-icons/` 并生成 `.profile.json`，Tauri 打包使用该结果。
- Rust 侧同时引用 `apps.json` 以设置窗口标题、URL 等运行时信息；额外的 `APP_URL` / `APP_NAME` 环境变量仍可覆盖默认值。

## GitHub Actions

`.github/workflows/desktop-builds.yml` 会：

1. 接收 `app-id`（默认 `creator`），在 ubuntu / windows / macOS 三平台矩阵内运行。
2. 调用 `pnpm run prepare-app -- --app-id=...`，从 `app-profiles/` 复制对应图标。
3. 通过 `APP_ID` 环境变量驱动 `pnpm tauri build`，并上传各平台 artifact。

### CI 注意事项（重要）

- **修改 `package.json` 后，必须同步并提交 `pnpm-lock.yaml`**。
  - 推荐做法：本地执行 `pnpm install`，确认只出现预期依赖变更，然后提交 lockfile。
  - 否则 CI 在 `pnpm install --frozen-lockfile` 会失败，常见报错为 `ERR_PNPM_OUTDATED_LOCKFILE`。
- Workflow 中不要在 `pnpm/action-setup` 里再手动指定一个与 `package.json#packageManager` 不一致的 pnpm 版本。
  - 否则会出现 `Multiple versions of pnpm specified` 并直接失败。
  - 当前项目以 `package.json` 的 `packageManager` 为唯一 pnpm 版本来源。
- `--frozen-lockfile` 是 CI 的推荐模式，可保证构建可复现。
  - 仅在紧急排障时，才临时改用 `pnpm install --no-frozen-lockfile`；之后应尽快补齐并提交 lockfile，同步回 `--frozen-lockfile`。

如需扩展更多应用：

1. 在 `app-profiles/<your-app-id>/icons` 中放入 Tauri 需要的图标文件。
2. 在 `app-profiles/apps.json` 中新增 profile。
3. 使用 `APP_ID=<your-app-id>` / `--app-id=<your-app-id>` 即可生成对应桌面包。

## 开发环境建议

- VS Code + [Tauri 扩展](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- Rust toolchain（stable）
- Node.js 20+ 与 pnpm 10+
