# Security Policy

**Effective date:** 2026-10-17  
**Scope:** 2-Pyramid main app, first-party installer, and installers from official release channels.

---

## 1. Supported versions

| Line | Support |
|------|---------|
| Current line (**2.203.x**, latest stable / Stable tag) | Security fixes prioritized |
| Latest Beta | Best effort; no SLA |
| Older stables (**2.202.x and earlier**) | Upgrade recommended; usually no backports |

## 2. How to report a vulnerability

**Do not** post publicly exploitable details in a public Issue.

Preferred path:

1. Open the repository [Security Advisories](https://github.com/realLivedInCorner/2-Pyramid/security/advisories) and file a **Private vulnerability report**; if that entry is unavailable, open an Issue titled [SECURITY] and maintainers will move it to a private channel
2. Or open an Issue titled `[SECURITY]` with **only impact and a high-level repro summary**; maintainers will move the discussion to a private channel  
3. Include: affected version / BUILD, repro steps, expected vs actual, PoC if you can share privately

We aim to respond within **7 business days**. Valid issues get coordinated fix and disclosure timing.

## 3. Security design notes (for your risk review)

- **Local-first:** conversions do not upload files by default; see `PRIVACY.md`
- **Update integrity:** releases **must** ship `.sha256`; the updater verifies the hash after download and **refuses and deletes on mismatch**. If the checksum is missing or cannot be fetched, the update is refused by default as well; the user may **explicitly opt** to continue without verification for that single download. The choice is **not remembered** and is logged, and skipping is **not permitted** when the installer filename does not match the release version (otherwise both the naming check and the integrity check would fail at once)
- **Download allowlist:** installers and hashes only from `github.com` and its subdomains, plus hosts **exactly equal to** `objects.githubusercontent.com` (subdomains of that host are not allowlisted). See `is_allowed_download_url` in `src-tauri/src/updater.rs`. The third-party mirror `cdn.5eggpack.top` is no longer maintained and has been removed from the update path.
- **Loopback only:** the app binds loopback addresses only — the single-instance lock on `127.0.0.1:24157` (same-port delivery), plus an action-stream port `127.0.0.1:24159` in debug builds. **It never listens on an external interface**
- **CSP:** the main app's WebView restricts script and resource origins via CSP; the installer loads only bundled content and does not enable CSP
- **No admin rights:** install uses HKCU; no UAC elevation required

## 4. Known boundaries (not vulnerabilities, but know them)

- Converting **arbitrary** user zip/packs parses many PNG / JSON / text files; bad input may be slow or fail; pack code is generally not executed
- Shader work rewrites GLSL sources and **does not** compile untrusted binaries on your GPU
- Installers from unofficial channels may bypass the SHA chain; trust only this repository's GitHub Releases

## 5. Credit

Valid security reports may be credited in Release Notes or this file (anonymous on request).
