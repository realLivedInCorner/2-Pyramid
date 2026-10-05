# Privacy Policy

**Effective date:** 2026-10-06  
**Product:** 2-Pyramid (Windows desktop app and first-party installer)

2-Pyramid is designed as **fully local: no account, no telemetry**. This document describes what we do and do not collect.

---

## In one sentence

When converting resource packs, your files **never leave your device**. We do not upload personal identity data, and we do not track usage.

---

## 1. What we do not collect

- We do not upload your imported packs, textures, shaders, or overlay configs (**except Foray AI analysis**, see §4)
- No automatic crash reporting, no analytics, no DAU / feature metrics
- No account registration; we do not store passwords, emails, or phone numbers
- We do not read browser history, contacts, or unrelated app data

The optional **display name** you enter is stored only in local config for the home greeting. You can change or clear it in Settings at any time; it is **not uploaded**.

## 2. Local storage (multiple locations)

| Content | Actual location | Notes |
|---------|-----------------|-------|
| Settings, history, background, backups, Foray AI config | `.2pyr` under your user home (e.g. `~/.2pyr` / `%USERPROFILE%\.2pyr`) | `configs/settings.json`, `history.json`, `background/`, `backups/`, `foray-ai.json` |
| Overlay projects | `2-Pyramid` under system Documents | Project metadata and overlay workspaces |
| Logs | `2-Pyramid\logs` under Local AppData | Rolling daily logs; export via Settings → Developer mode |
| Update marker | `2-Pyramid` under system config dir | In-app update state |
| Conversion temp files | System temp (`.2pyr-work-*`) | Intermediate extract/build files; removed after a normal run by a background thread or a detached child process, and stale leftovers are swept on the next conversion |

Logs may include **file paths** and error messages for troubleshooting. **Both export paths (session log and on-disk logs) redact by default**: the exported file hides the Windows user name, machine name, IPs, e-mails and API keys/tokens, and collapses pack paths to `<path>\file.ext`. Redaction applies to the exported file only — on-disk and in-app logs stay verbatim so you can debug locally. It can be turned off in Settings → Developer options; developer mode additionally allows exporting the unredacted text (with a confirmation step — never share that publicly).

## 3. Network access (when you trigger it, or while the automatic update check is on)

| Scenario | Destination | Data |
|----------|-------------|------|
| Update check | GitHub Releases API (`api.github.com`) | Release list only; your files and device fingerprint are never uploaded |
| Update download | `github.com` / `objects.githubusercontent.com` | Official installer and its `.sha256` checksum |
| **Foray AI analysis (optional, off by default)** | **Your configured OpenAI-compatible `baseURL`** | **Depends on data tier; see §4** |

> The update source is fixed to official GitHub: Settings no longer offers an update-source option, speed test or switch. The China mirror `cdn.5eggpack.top` used to be an optional update source and was **fully removed in 2026-10** (its maintainer stopped maintaining it). No request is sent to that domain any more.

**The automatic update check is on by default**: the app fetches the release list once at startup (first row above). If you turn off “Automatically check for updates” in Settings → Version & Updates, the app only goes online when you explicitly check or download. Beyond that, and with Foray AI disabled, the app makes no network requests.

## 4. Third parties

Beyond update check/download (GitHub only), the software **does not send data to third parties by default**.

**Sole exception: Foray AI analysis** (only if you enable it and configure a service): requests go to your **OpenAI-compatible** `baseURL`. They may include: **directory tree and extension stats (included from data tier 1, which is the default tier)**, `pack.mcmeta`, **copies** of the JSON (≤64 KB) / shaders (≤128 KB) you select, and texture **summaries** (size / average color / histogram, **not pixels**), depending on the data tier you choose; **tier 0 sends nothing and calls no external service**. The API key is stored only in the local config file (`~/.2pyr/foray-ai.json`) to authenticate to that service; **on Windows that file is stored as plain text readable by the current user, so do not keep a key there on a shared account**. That service’s privacy policy is set by its provider and is unrelated to the 2-Pyramid authors.

## 5. Share codes

Overlay share codes (`2PYR-…`) are exported on your device and transferred through your own channels. The app does **not** relay or host them. If you use a third-party platform, read that platform’s terms and privacy policy as well.

## 6. Minors

This software is not designed for children under 13 and does not knowingly collect children’s personal information.

## 7. Your rights

- Delete the local data listed in §2 at any time (mainly `~/.2pyr` and the `2-Pyramid` folders under Documents / Local AppData)
- Uninstall the app; the uninstaller keeps user data by default—you must delete data yourself
- For privacy questions, open a GitHub Issue

## 8. Changes

Policy updates change the date at the top and are noted in the repository. Significant changes are announced in Release Notes.

## 8.1 Repository-level data flow (not the app)

This section covers the **GitHub repository**, not the installed application:

- **Issue / PR / commit metadata → Feishu (Lark)**: a GitHub Actions workflow in this repository (`.github/workflows/feishu-notify.yml`) sends **issue and pull-request titles, author logins, URLs and commit metadata** to a third-party Feishu (Lark) webhook maintained by the project, so maintainers get notified. This metadata is **public on GitHub** and contains nothing from your local machine or resource packs; if you would rather not have it forwarded, do not open issues/PRs, or ask the maintainers to remove the webhook.
- The application itself never talks to Feishu and never sends your pack contents anywhere except the optional Foray AI endpoint you configure.

## 9. Install / uninstall (EXE / MSIX / silent)

Silent install (`--silent`) uploads nothing extra. Uninstall removes program files; `~/.2pyr` and other user data are kept by default. The GitHub Releases exe installer and the Microsoft Store MSIX package write to the **same logical paths**, but under MSIX (a packaged app) Windows **redirects writes** to `%APPDATA%` / `%LOCALAPPDATA%` / `%TEMP%` into a **package-specific virtualised location**, so the files land somewhere different than with the exe build (and uninstall behaviour is governed by the Store terms).

---

**Contact:** GitHub Issues — https://github.com/realLivedInCorner/2-Pyramid/issues
