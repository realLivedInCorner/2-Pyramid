# 分发 / 商店清单（MSIX 路线）

> 修订：双通道分发。
>
> | 通道 | 产物 | 去向 |
> |------|------|------|
> | GitHub Releases | `2-Pyramid-Installer-{ver}.exe` | 官网 / 更新器 / 用户手动安装 |
> | Microsoft Store | `2-Pyramid-{ver}.msix` | Partner Center 提交 |
>
> **不再构建 MSI。**

## 流水线

```text
python tools/build_release.py [--beta] [--no-bump] [--skip-msix] [--sign-msix]
  ├─ release/staging/                         # exe + UImage + overlay + legal
  ├─ release/2-Pyramid-Installer-{ver}.exe    # GitHub Releases（+ .sha256）
  └─ release/2-Pyramid-{ver}.msix             # Microsoft Store（+ .sha256）
```

## MSIX（Microsoft Store）

| 项 | 位置 / 说明 |
|----|-------------|
| 包身份 | `tools/msix/package-identity.json`（**上架前必须换成 Partner Center 真实值**） |
| Manifest | `tools/msix/AppxManifest.template.xml`（流水线渲染） |
| 打包工具 | Windows SDK `makeappx pack` |
| 资产 | `src-tauri/icons/`：StoreLogo、Square*、Wide310x150、SplashScreen |
| 能力 | `runFullTrust`（Tauri 桌面全信任） |
| 版本 | `major.minor.patch.0`（**第 4 段 revision 必须为 0**；Store 自行递增，BUILD 不进包版本） |

### 上架前检查

1. **Partner Center 包身份**写入 `package-identity.json`：
   - `identity_name`：保留名（Package/Identity Name）
   - `publisher`：`CN=…`，**必须与签名证书 / 商店配置一致**
   - `publisher_display_name` / `display_name`
2. 占位符（含 `REPLACE_`）替换完毕；流水线在未替换时会 WARN。
3. 通过 Partner Center 提交 MSIX（商店侧签名分发；本地**不必**为上架而签）。
4. 若需 sideload 自测：`python tools/build_release.py --sign-msix`，并设置
   - `2PYR_MSIX_PFX` / `2PYR_MSIX_PFX_PASS`
5. 分级问卷、商店文案、截图按 Partner Center 要求准备。
6. **受限功能 
unFullTrust**：桌面 Win32（Tauri）必需。Partner Center 会警告，需在提交时**申请批准**并说明用途（本地资源包转换，无沙箱逃逸）；未批准前包无法通过认证。

### 本地试装（开发者模式）

```powershell
# 开发者模式开启后
Add-AppxPackage -Path .\release\2-Pyramid-2.5.0.msix
```

## EXE（GitHub Releases）

| 项 | 说明 |
|----|------|
| 产物 | `2-Pyramid-Installer-{ver}.exe`（beta 带 `-beta.{BUILD}`） |
| 脚本 | `installer-app/` 自研安装器，内嵌 `payload.zip` |
| 静默 | `--silent\|/S\|--quiet` + `--dir/--relaunch/--shortcuts` |
| 完整性 | 同名 `.sha256` 旁路文件；更新器下载后校验 |
| 渠道 | 更新源白名单：`github.com` / `objects.githubusercontent.com` / `cdn.5eggpack.top` |

## 对比（旧 MSI 路线 → 现状）

| | 旧（MSI） | 新（MSIX + EXE） |
|--|-----------|------------------|
| 商店 | 不支持 | MSIX 上架 MS Store |
| 企业静默 | `msiexec /qn` | EXE `--silent`（MSI 已移除） |
| 签名 | 可选 | 商店由商店签；sideload 可本地签 |
| WiX 依赖 | WiX v3 heat/candle/light | 不需要（makeappx） |

## 工具探测

| 工具 | 探测 | 回退 |
|------|------|------|
| `makeappx` | `MAKEAPPX` 环境变量 → Windows Kits 10 最新 x64 | 跳过 MSIX 并 WARN |
| `signtool` | `SIGNTOOL` → Windows Kits | 跳过签名 |
