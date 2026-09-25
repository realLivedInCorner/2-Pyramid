# 分发 / 商店清单（MSI 路线）

> 修订：优先 **MSI**，不做 MSIX。

## 已具备（2.4.2）

| 产物 | 说明 |
|------|------|
| `2-Pyramid-Installer.exe` | 自研 GUI 安装器（便携 payload + 注册表卸载） |
| `2-Pyramid-Installer.exe` 固定名 | 供脚本 / 固定 URL 抓取 |
| **`2-Pyramid-Installer-{ver}.msi`** | WiX x64 MSI（`tools/msi/` + `build_release.py`） |
| `--silent` 等 | exe：`--silent\|/S\|--quiet` + `--dir/--relaunch/--shortcuts` |
| MSI 静默 | `msiexec /i 2-Pyramid-Installer-2.4.2.msi /qn /l*v msi.log` |
| 退出码 | exe：0/1603…；MSI：msiexec 标准（0/1603/1618/3010…） |

## MSI 比 MSIX 省掉的

- 不用 Partner Center 那套 AppxManifest / 证书 Publisher 严格对齐
- 不用强制 Store 保留名 + 分级问卷（除非上 Store）
- 静默参数靠 **msiexec /qn**，企业/Intune/winget 都熟

## 仍建议准备（若要上架或 winget）

1. **签名**：EV/OV 代码签名证书（msi + exe），SmartScreen 会友好很多  
2. **winget manifest**（可选）：`winget create` 用固定 URL 的 msi/exe  
3. 若要进 **Microsoft Store**：商店新提报仍偏 MSIX；MSI 更适合官网/企业/Intune。  
4. 卸载干净：MSI 已带 MajorUpgrade；exe 自研卸载保留 `~/.2pyr`

## 流水线

```text
python tools/build_release.py [--beta] [--no-bump]
  → release/2-Pyramid-Installer-{ver}[-beta.{BUILD}].exe
  → release/2-Pyramid-Installer.exe          # 固定名
  → release/2-Pyramid-Installer-{ver}.msi    # WiX（WiX v3 在 PATH/固定路径）
  → 同名 .sha256
```

WiX 路径：`C:\Program Files (x86)\WiX Toolset v3.14\bin`（heat/candle/light）。
