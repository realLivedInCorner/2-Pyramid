# Privacy Policy

**Effective date:** 2026-09-14  
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
| Conversion temp files | System temp | Intermediate extract/build files; cleaned after a normal run |

Logs may include **file paths** and error messages for troubleshooting. Review before exporting if paths may be sensitive.

## 3. Network access (only when you trigger it)

| Scenario | Destination | Data |
|----------|-------------|------|
| Check for updates | GitHub Releases API or mirror `cdn.5eggpack.top` | Release list only; no file or device fingerprint upload |
| Download updates | `github.com` / `objects.githubusercontent.com` / `cdn.5eggpack.top` | Official installer and `.sha256` checksum |
| Update source speed test | Same as above | Latency and download rate for “fastest source” |
| **Foray AI analysis (optional, off by default)** | **Your configured OpenAI-compatible `baseURL`** | **Depends on data tier; see §4** |

If update checks are off and you never tap update actions, and Foray AI is disabled, the app makes **no** network requests.
(Note: the automatic update check is **on by default** at startup; turn it off in Settings → Version & Updates.)

## 4. Third parties

Beyond update check/download (GitHub and optional mirror, under their own privacy policies), the software **does not send data to third parties by default**.

**Sole exception: Foray AI analysis** (only if you enable it and configure a service): requests go to your **OpenAI-compatible** `baseURL`. They may include: directory tree and extension stats, `pack.mcmeta`, copies of JSON/shaders you select, and texture **summaries** (size / average color / histogram, **not pixels**), depending on your data tier. The API key is stored only in local config for authenticating to that service. That service’s privacy policy is set by its provider and is unrelated to the 2-Pyramid authors.

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

Silent install (`--silent`) uploads nothing extra. Uninstall removes program files; `~/.2pyr` and other user data are kept by default. The GitHub Releases exe installer and the Microsoft Store MSIX package behave the same: they only write the install directory (or package location) and uninstall registry entries on this machine.

---

**Contact:** GitHub Issues — https://github.com/realLivedInCorner/2-Pyramid/issues
