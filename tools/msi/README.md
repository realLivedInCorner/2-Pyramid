# WiX MSI（已退役）

本目录为历史 WiX 3 MSI 定义，**已不再参与发布流水线**。

分发路线（见 `docs/compose/ms-store-checklist.md`）：

| 通道 | 产物 | 构建 |
|------|------|------|
| GitHub Releases | `2-Pyramid-Installer-*.exe` | `installer-app/` |
| Microsoft Store | `2-Pyramid-*.msix` | `tools/msix/` + `build_release.py` |

请勿再用本目录产出安装包。如需企业侧 MSI，请自行从 `release/staging/` 再打包。
