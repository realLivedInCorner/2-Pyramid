# Contributing

Thanks for your interest in 2-Pyramid! Issues and pull requests are welcome.

---

## 1. Before you start

- Search [Issues](https://github.com/realLivedInCorner/2-Pyramid/issues) and Discussions to avoid duplicates
- For large features, open an Issue to align on design first
- Code of conduct: stay respectful and technical; no personal attacks or unrelated political arguments

## 2. Development environment

```bash
git clone <repo>
cd 2-Pyramid          # or your local path (repo lives under code/)
npm install
npm run 2pyr          # Tauri dev (Rust + Vite)
```

Common commands:

| Command | Purpose |
|---------|---------|
| `npm test` | Rust unit tests (`cargo test --lib`) |
| `npm run build` | Frontend build only |
| `npm run buildrelease` | Full stable release build (bumps BUILD) |

Requires: Node.js, Rust stable, Windows (Windows desktop is the only officially supported target today).

## 3. Code standards

### Rust

- Run `cargo fmt` (or match nearby style if no rustfmt config is enforced)
- Prefer passing `cargo clippy`; avoid new `unwrap()` on non-test paths
- Put conversion logic under `src-tauri/src/converters/` domain modules (`ui/` / `textures/` / `reverse/`, etc.) and register it on the Scheduler—**do not** put business logic in `invoke_conversion.rs`
- Add `#[cfg(test)]` tests when changing image or path logic

### Frontend / Vue

- TypeScript + `<script setup>`
- Copy goes in both `src/locales/zh-CN.json` and `en-US.json`
- Match existing glass-card visuals and theme tokens

### Commit messages

- Imperative intent, e.g. `fix(overlay): correct share code prefix` / `feat(ui): …`
- One concern per PR; unrelated formatting in separate commits

## 4. Pull requests

1. Fork and branch from `master` (e.g. `feat/xxx`)
2. Ensure `npm test` passes; for UI, say how you verified
3. PR description: motivation, changes, testing, related Issues
4. Keep the diff readable; explain large or generated files

Maintainers may ask for renames, split commits, or extra tests.

## 5. License of contributions

Unless otherwise agreed in writing, you contribute under the **MIT License** to the public, allowing 2-Pyramid Studio **and all downstream recipients** to use, modify, and redistribute. By submitting you confirm you have the right to grant that (e.g. it does not violate employment or third-party copyright).

**Do not submit:**

- Unauthorized Minecraft assets, other people’s packs, or cracked content
- Oversized binaries (compress screenshots; invent test textures)
- Paths, logs, or account data containing personal information

## 6. Reporting bugs

Include when possible:

- 2-Pyramid version and BUILD (Settings → Version Info)
- Source / target pack versions (Java pack_format or Bedrock)
- Repro steps and expected result
- Logs from Settings → Developer → Export logs

A **small public sample pack** speeds up conversion bug fixes a lot.

## 7. Feature ideas

Explain the use case, why current flow is not enough, and the interaction you expect. Screenshots or sketches help.

---

**Summary:** Discuss large changes in an issue first. Put converters in domain modules, keep zh/en locales in sync, run `npm test`, and write clear commits. Contributions are MIT-licensed to the public (the Studio is only the maintainer); do not commit unauthorized Minecraft assets or personal data.
