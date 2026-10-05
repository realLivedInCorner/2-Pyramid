# Security Policy

**Effective date:** 2026-10-06  
**Scope:** 2-Pyramid main app, first-party installer, and installers from official release channels.

---

## 1. Supported versions

| Line | Support |
|------|---------|
| Latest stable (Stable tag) | Security fixes prioritized |
| Latest Beta | Best effort; no SLA |
| Older stables | Upgrade recommended; usually no backports |

## 2. How to report a vulnerability

**Do not** post publicly exploitable details in a public Issue.

Preferred path:

1. Open the repository [Security Advisories](https://github.com/realLivedInCorner/2-Pyramid/security/advisories) and file a **Private vulnerability report**; if that entry is unavailable, open an Issue titled [SECURITY] and maintainers will move it to a private channel
2. Or open an Issue titled `[SECURITY]` with **only impact and a high-level repro summary**; maintainers will move the discussion to a private channel  
3. Include: affected version / BUILD, repro steps, expected vs actual, PoC if you can share privately

We aim to respond within **7 business days**. Valid issues get coordinated fix and disclosure timing.

## 3. Security design notes (for your risk review)

- **Local-first:** conversions do not upload files by default; see `PRIVACY.md`
- **Update integrity (mandatory):** releases **must** ship `.sha256`; the updater verifies the hash after download — **mismatches are refused and deleted, and any release without a `.sha256` asset (or whose checksum cannot be fetched) is refused outright** (updates that cannot be proven intact are never installed)
- **Download allowlist:** installers and hashes only from `github.com` / `objects.githubusercontent.com` (the China mirror `cdn.5eggpack.top` is retired and has been removed from the update path)
- **CSP:** the WebView restricts script and resource origins
- **No admin rights:** install uses HKCU; no UAC elevation required

## 4. Known boundaries (not vulnerabilities, but know them)

- Converting **arbitrary** user zip/packs parses many PNG / JSON / text files; bad input may be slow or fail; pack code is generally not executed
- Shader work rewrites GLSL sources and **does not** compile untrusted binaries on your GPU
- Installers from unofficial channels may bypass the SHA chain; trust only this repository's GitHub Releases

## 5. Credit

Valid security reports may be credited in Release Notes or this file (anonymous on request).
