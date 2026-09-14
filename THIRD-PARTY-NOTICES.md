# 第三方组件声明 / Third-Party Notices

**生效日期：** 2026-09-14

2-Pyramid 包含或链接若干第三方开源组件。这些组件按其**各自许可证**提供；本声明仅作汇总，完整文本以各组件仓库 / `Cargo.lock` / `package-lock.json` 为准。

主程序许可证见根目录 `LICENSE`（MIT）。第三方许可证条款与 MIT 冲突时，以第三方自身许可证为准。

---

## 1. Rust / Tauri 后端（主程序）

| 组件 | 许可证（常见） | 用途 |
|------|----------------|------|
| [Tauri](https://tauri.app) | MIT / Apache-2.0 | 桌面壳与系统能力 |
| tauri-plugin-dialog / opener / notification | MIT / Apache-2.0 | 对话框、打开链接、系统通知 |
| [serde](https://serde.rs) / serde_json | MIT OR Apache-2.0 | 序列化 |
| [image](https://crates.io/crates/image) | MIT OR Apache-2.0 | 贴图读写 |
| [zip](https://crates.io/crates/zip) | MIT | 资源包打包 |
| [flate2](https://crates.io/crates/flate2) | MIT OR Apache-2.0 | 分享码压缩 |
| [base64](https://crates.io/crates/base64) | MIT OR Apache-2.0 | 分享码编码 |
| [walkdir](https://crates.io/crates/walkdir) | MIT OR Apache-2.0 | 目录遍历 |
| [rayon](https://crates.io/crates/rayon) | MIT OR Apache-2.0 | 并行处理 |
| [tokio](https://tokio.rs) | MIT | 异步运行时 |
| [reqwest](https://crates.io/crates/reqwest) | MIT OR Apache-2.0 | 更新检查与下载 |
| [rustls](https://github.com/rustls/rustls) | MIT OR Apache-2.0 OR ISC | TLS（reqwest 依赖） |
| [chrono](https://crates.io/crates/chrono) | MIT OR Apache-2.0 | 时间 |
| [uuid](https://crates.io/crates/uuid) | MIT OR Apache-2.0 | 项目 ID |
| [regex](https://crates.io/crates/regex) | MIT OR Apache-2.0 | 文本处理 |
| [dirs](https://crates.io/crates/dirs) | MIT OR Apache-2.0 | 系统目录 |
| [tempfile](https://crates.io/crates/tempfile) | MIT OR Apache-2.0 | 临时目录 |
| [fs_extra](https://crates.io/crates/fs_extra) | MIT | 文件操作 |
| [lazy_static](https://crates.io/crates/lazy_static) | MIT OR Apache-2.0 | 静态初始化 |
| [sha2](https://crates.io/crates/sha2) | MIT OR Apache-2.0 | 哈希校验 |
| [winreg](https://crates.io/crates/winreg) | MIT | Windows 注册表 |
| [windows-sys](https://crates.io/crates/windows-sys) | MIT OR Apache-2.0 | Windows API 绑定 |

> 权威清单以仓库内 `src-tauri/Cargo.lock`、`installer-app/src-tauri/Cargo.lock` 为准。

## 2. 前端

| 组件 | 许可证（常见） | 用途 |
|------|----------------|------|
| [Vue.js](https://vuejs.org) | MIT | UI 框架 |
| [vue-i18n](https://kazupon.github.io/vue-i18n/) | MIT | 多语言 |
| [Vite](https://vitejs.dev) | MIT | 构建 |
| [TypeScript](https://www.typescriptlang.org) | Apache-2.0 | 类型 |
| [@tauri-apps/api](https://github.com/tauri-apps/tauri) | MIT OR Apache-2.0 | 前端桥接 |
| [Remix Icon](https://remixicon.com) | Apache-2.0 | 图标字体 |
| [@vitejs/plugin-vue](https://github.com/vitejs/vite-plugin-vue) | MIT | Vue 插件 |

> 以 `package.json` 与 lockfile 为准。

## 3. 安装器（installer-app）

与主程序同栈（Tauri 2 + Rust），依赖见 `installer-app/src-tauri/Cargo.lock`。

## 4. 平台与游戏资产

- **Minecraft** 相关名称、商标与游戏内行为归 Mojang Studios / Microsoft 所有。本软件不包含 Minecraft 客户端或官方资源；转换对象为你自己合法持有的资源包。
- 转换过程中生成的贴图衍生自**你导入的**资源包，著作权仍归原权利人。

## 5. 如何获取完整许可证文本

- Rust：`cargo license` 或各 crate 在 [crates.io](https://crates.io) 的 LICENSE 文件  
- 前端：`node_modules/<pkg>/LICENSE`  
- 本仓库根目录：`LICENSE`（MIT）

若你认为本声明遗漏了应列组件，请开 Issue，我们会尽快补全。

---

**English summary:** 2-Pyramid ships with open-source libraries under MIT / Apache-2.0 and similar licenses. See `Cargo.lock` and `package-lock.json` for the authoritative dependency list. Minecraft branding belongs to Mojang/Microsoft; we ship no official game assets.
