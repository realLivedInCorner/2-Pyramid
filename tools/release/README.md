# 发布流程（正式版 / 测试版）

> 面向维护者的发布清单。工具链：`tools/build_release.py`（`npm run buildrelease` / `npm run betabuild`）。

## 一、构建前必做核对（缺一不可）

1. **legal 复核** —— `legal/*.md` 与 `legal/en/*.md` 是否仍与实现一致：
   依赖变动（`THIRD-PARTY-NOTICES.md`）、联网/采集行为（`PRIVACY.md`）、生效日期。
   *本版若无新依赖、无新增联网行为，也要在提交信息或 CHANGELOG 里写明"已复核，无变化"。*
2. **CHANGELOG** —— `[Unreleased]` 整理为 `[x.y.z] - 日期（BUILD nnnnn）`；BUILD 号 = 当前 `BUILD` 文件值 + 1。
3. **README** —— 版本徽章、功能章节、已知限制是否与现状一致。
4. **版本号四处统一** —— `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`tools/msix/package-identity.json`（`x.y.z.0`）。
5. 先提交上述改动，**再**构建（构建会自增 `BUILD` 并修改 `Cargo.lock`，随后单独提交）。

## 二、构建与校验

```powershell
npm run buildrelease      # EXE 安装器 + .sha256 + MSIX（Store 用）
```

- `BUILD` 必须**只 +1**（`2PYR_NO_BUMP=1` 已用于 MSIX 的第二次编译；若 +2 说明该机制回归）。
- 发布版二进制冒烟：`release\staging\2-pyramid.exe --analyze <包>` 与 `--convert <小包>`。
- 哈希一致性：本地 `*.sha256` 必须与 GitHub 计算的 digest 相同（`gh release view --json assets`）。
- MSIX：`Identity Version` 正确、`NotSigned`（商店会自行签名）。

## 三、发布 GitHub Release

```powershell
gh release create Stable-x.y.z `
  "release\2-Pyramid-Installer-x.y.z.exe" `
  "release\2-Pyramid-Installer-x.y.z.exe.sha256" `
  --title "2-Pyramid Stable x.y.z-BUILDnnnnn" `
  --notes-file <notes.md> --verify-tag --latest
```

- 资产**只发 EXE + `.sha256`**（应用内更新器强制校验 `.sha256`）；**MSIX 不进 Releases**。
- 标题格式：`2-Pyramid Stable x.y.z-BUILDnnnnn`；tag：`Stable-x.y.z`。
- 发版后必须用**匿名**接口确认更新器能看到它：

```powershell
(Invoke-WebRequest "https://api.github.com/repos/realLivedInCorner/2-Pyramid/releases" -UseBasicParsing `
  -Headers @{ 'User-Agent'='2pyr' }).Content | ConvertFrom-Json |
  Select-Object -First 3 tag_name, draft, prerelease
```

  新版本必须出现在**列表第一条**，且 `/releases/latest` 指向它。

## ⚠️ 血泪教训：不要删除已发布 Release 的 tag

**GitHub 会在 tag 被删除时连带删除其 Release。** 2026-10-03 发布 2.7.0 时，为了把 tag 从旧提交移到发布提交执行了
`git push origin :refs/tags/Stable-2.7.0`，结果 Release 页直接 404、更新器列表里也消失（表现为"2.7.0 找不到"）。

正确做法：

- **发布前**确保 `master` 已推送、HEAD 就是发布提交，再执行 `gh release create --verify-tag`；
- 若 tag 指错了提交，**不要删 tag**：用 API 改 Release 的 tag，或在 Release 页重新指定，或者发布一个补丁版本；
- 万一已经误删，tag 仍在时可直接在原 tag 上重建：
  `gh release create <tag> <资产…> --verify-tag --latest`（本次即以此恢复）。

## 四、Microsoft Store（Partner Center）

1. 新提交 → 使用 `release\2-Pyramid-x.y.z.0.msix`（不需本地签名）。
2. 版本号 `x.y.z.0`；Identity `2-PyramidStudio.2-Pyramid`；Publisher `CN=7BC328FB-A6A3-42E9-A750-326D3BDC3F5F`。
3. 「此版本的新增功能」按 `tools/store/whats-new-<版本>.md` 粘贴（中英各一份）；字段规则与归档方式见 `tools/store/README.md`。

## 五、仓库目录约定（新增文件放哪）

| 内容 | 位置 | 是否入库 |
|---|---|---|
| 维护者流程文档 | `tools/<领域>/README.md`（如 `tools/msi/`、`tools/release/`、`tools/store/`） | ✅ |
| 项目级文档 | 仓库根（`README.md` / `CHANGELOG.md`） | ✅ |
| 打包输入（清单模板、身份配置） | `tools/msix/`、`tools/msi/` | ✅ |
| 脚本 | `tools/*.py`、`tools/*.mjs`、`tools/convert-report.*` | ✅ |
| 商店提交文案 | `tools/store/whats-new-<版本>.md` | ✅ |
| 基准页与其说明 | `tools/benchmark/index.html`、`README.md` | ✅ |
| 截图 / 导出报告 / 渲染产物 | 与来源同目录，靠 `.gitignore` 隔离（如 `tools/benchmark/*.png`） | ❌ |
| 本地专用材料（测试签名包、证书） | 根 `store/` | ❌（已忽略） |
| 临时笔记与草稿 | 根 `docs/` | ❌（已忽略） |
| 构建产物 | `release/`、`dist/` | ❌（已忽略） |

> 新增任何文件前先对照本表；**生成物一律加 `.gitignore` 规则**，不要把截图、报告、导出文件提交进仓库。
