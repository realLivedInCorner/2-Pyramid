# Privacy Policy

**Effective date:** 2026-09-14  
**Product:** 2-Pyramid (Windows desktop app and first-party installer)

2-Pyramid is designed for: **fully local processing, no account, no telemetry**. Below is what we do and do not collect.

---

## In one sentence

When you convert resource packs, your files **do not leave your machine**. We do not collect personal identity data beyond an optional display name, and we do not track usage.

---

## 1. What we do not collect

- We do not upload the packs, textures, shaders, or overlay configs you import
- We do not auto-report crash stacks, run analytics, or count DAU / feature usage
- We do not require an account and do not store passwords, emails, or phone numbers
- We do not read browser history, contacts, or other app data unrelated to features

The “user name” you enter is stored only in local config for the home greeting; you can change or clear it in Settings anytime.

## 2. Local storage

| Content | Approximate location | Notes |
|---------|----------------------|--------|
| User settings | System config dir under `2-Pyramid` | Language, theme, output mode, update channel, etc. |
| Overlay projects | Documents dir under `2-Pyramid` | Project metadata and overlay workspaces you edit |
| Conversion temp files | System temp directory | Unpacked and intermediate files; cleaned after a normal finish |
| Logs | App data / logs directory | Runtime logs; export via Settings → Developer → Export logs |

Logs may include **file paths** and error text for debugging. Review them before sharing.

## 3. Network access (only when you trigger it)

| Scenario | Destination | Data |
|----------|-------------|------|
| Check for updates | GitHub Releases API, or mirror `cdn.5eggpack.top` | Pulls release list only; no upload of your files or device fingerprint |
| Download updates | `github.com` / `objects.githubusercontent.com` / `cdn.5eggpack.top` | Official installer and `.sha256` checksum |
| Measure update sources | Same as above | Latency and download speed for “use fastest source” |

If you turn off update checks or never press update buttons, the app **does not** make those requests.

## 4. Third-party processors

Update checks/downloads talk to GitHub (and an optional mirror), under their own privacy policies. Otherwise the software does not send data to third parties.

## 5. Share codes

Overlay share codes (`2PYR-…`) are exported on your machine and transferred by you. The app **does not host or relay** them. If you share via a third-party platform, read that platform’s terms and privacy notice too.

## 6. Children

The software is not directed at children under 13 and does not knowingly collect children’s personal information.

## 7. Your rights

- Delete the `2-Pyramid` folders under Documents / config to clear local records
- Uninstall the app; the uninstaller keeps user data by default—you must delete data yourself
- Privacy questions: open a GitHub Issue

## 8. Changes

Policy updates change the date at the top and are noted in the repository. Material changes will be highlighted in Release Notes.

---

**Contact:** GitHub Issues — https://github.com/realLivedInCorner/2-Pyramid/issues
