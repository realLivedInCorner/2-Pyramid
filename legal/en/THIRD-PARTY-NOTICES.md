# Third-Party Notices

**Effective date:** 2026-09-14

2-Pyramid includes or links third-party open-source components. Those components are provided under **their own licenses**. This notice is a summary only; the authoritative texts are in each component’s repository / `Cargo.lock` / `package-lock.json`.

The main application license is [`LICENSE`](../LICENSE) (MIT). If a third-party license conflicts with MIT, the third-party license controls for that component.

---

## 1. Rust / Tauri backend (main app)

| Component | License (common) | Use |
|-----------|------------------|-----|
| [Tauri](https://tauri.app) | MIT / Apache-2.0 | Desktop shell and system APIs |
| tauri-plugin-dialog / opener / notification | MIT / Apache-2.0 | Dialogs, open links, system notifications |
| [serde](https://serde.rs) / serde_json | MIT OR Apache-2.0 | Serialization |
| [image](https://crates.io/crates/image) | MIT OR Apache-2.0 | Texture I/O |
| [zip](https://crates.io/crates/zip) | MIT | Pack zip |
| [flate2](https://crates.io/crates/flate2) | MIT OR Apache-2.0 | Share-code compression |
| [base64](https://crates.io/crates/base64) | MIT OR Apache-2.0 | Share-code encoding |
| [walkdir](https://crates.io/crates/walkdir) | MIT OR Apache-2.0 | Directory walk |
| [rayon](https://crates.io/crates/rayon) | MIT OR Apache-2.0 | Parallelism |
| [tokio](https://tokio.rs) | MIT | Async runtime |
| [reqwest](https://crates.io/crates/reqwest) | MIT OR Apache-2.0 | Update check / download |
| [rustls](https://github.com/rustls/rustls) | MIT OR Apache-2.0 OR ISC | TLS (via reqwest) |
| [chrono](https://crates.io/crates/chrono) | MIT OR Apache-2.0 | Time |
| [uuid](https://crates.io/crates/uuid) | MIT OR Apache-2.0 | Project IDs |
| [regex](https://crates.io/crates/regex) | MIT OR Apache-2.0 | Text |
| [dirs](https://crates.io/crates/dirs) | MIT OR Apache-2.0 | System dirs |
| [tempfile](https://crates.io/crates/tempfile) | MIT OR Apache-2.0 | Temp dirs |
| [fs_extra](https://crates.io/crates/fs_extra) | MIT | File ops |
| [lazy_static](https://crates.io/crates/lazy_static) | MIT OR Apache-2.0 | Lazy statics |
| [sha2](https://crates.io/crates/sha2) | MIT OR Apache-2.0 | Checksums |
| [winreg](https://crates.io/crates/winreg) | MIT | Windows registry |
| [windows-sys](https://crates.io/crates/windows-sys) | MIT OR Apache-2.0 | Windows API |

> Authoritative list: `src-tauri/Cargo.lock`, `installer-app/src-tauri/Cargo.lock`.

## 2. Frontend

| Component | License (common) | Use |
|-----------|------------------|-----|
| [Vue.js](https://vuejs.org) | MIT | UI |
| [vue-i18n](https://kazupon.github.io/vue-i18n/) | MIT | i18n |
| [Vite](https://vitejs.dev) | MIT | Build |
| [TypeScript](https://www.typescriptlang.org) | Apache-2.0 | Types |
| [@tauri-apps/api](https://github.com/tauri-apps/tauri) | MIT OR Apache-2.0 | Bridge |
| [Remix Icon](https://remixicon.com) | Apache-2.0 | Icons |
| [@vitejs/plugin-vue](https://github.com/vitejs/vite-plugin-vue) | MIT | Vue plugin |

> Authoritative list: `package.json` and lockfiles.

## 3. Installer (installer-app)

Same stack as the main app (Tauri 2 + Rust). See `installer-app/src-tauri/Cargo.lock`.

## 4. Platform and game assets

- **Minecraft** names, trademarks, and in-game behavior belong to Mojang Studios / Microsoft. This software does not ship the Minecraft client or official game assets; conversion targets are resource packs you already own.
- Textures generated during conversion are derived from **packs you imported**; copyright remains with the original rights holders.

## 5. Getting full license texts

- Rust: `cargo license` or each crate’s LICENSE on [crates.io](https://crates.io)  
- Frontend: `node_modules/<pkg>/LICENSE`  
- This repository root: `LICENSE` (MIT)

If a required component is missing from this list, open an Issue and we will add it.

---

**Summary:** 2-Pyramid ships open-source libraries under MIT / Apache-2.0 and similar licenses. See `Cargo.lock` and `package-lock.json` for the full dependency list. Minecraft branding belongs to Mojang/Microsoft; we ship no official game assets.
