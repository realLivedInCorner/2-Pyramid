# Disclaimer

**Effective date:** 2026-09-14

Please read this carefully before using 2-Pyramid. Installing or using the software means you have read, understood, and agree to the following.

---

## 1. Provided as is

The software is provided **AS IS**, without warranty of any kind, express or implied, including merchantability, fitness for a particular purpose, and non-infringement. The authors do not warrant that features are complete, conversion results are accurate, or that the software will never fail or interrupt.

## 2. Conversion risk is yours

Resource-pack version conversion involves many heuristics for textures, models, shaders, and UI. There is **no guarantee** of usable results for every pack and every target version. You may see:

- Misaligned textures, color shifts, or UI glitches
- Shaders that fail or crash the client on the target version
- Output structure that does not match your expectations

**Back up important packs first and verify in a test environment.** The authors are not responsible for file corruption, data loss, broken saves, or other losses from using the software.

## 3. Not affiliated with Mojang / Microsoft

2-Pyramid is a **third-party community tool**, not affiliated with, partnered with, authorized by, or endorsed by Mojang Studios, Microsoft, or official Minecraft. Minecraft names and related trademarks belong to their owners and are used only for reference.

When you convert or distribute packs with this software, you must comply with the Minecraft EULA / terms of use and the original pack authors’ licenses. Legal compliance is your responsibility.

## 4. Open-source components

The software includes many third-party open-source libraries. See [`THIRD-PARTY-NOTICES.md`](./THIRD-PARTY-NOTICES.md). Those components are under their own licenses; the authors are not liable for defects in third-party components.

## 4a. External AI / API services (Foray)

If you enable **Foray** AI analysis:

1. The feature is **optional** and **sends nothing to third parties by default** (tier 0 is fully local).
2. Requests go only to **your** configured OpenAI-compatible `baseURL`. The authors do not operate, proxy, or store your API key or analysis content.
3. **Availability, accuracy, compliance, retention, and security of any model, relay, or cloud service are between you and that provider.** They are not the responsibility of 2-Pyramid authors or contributors.
4. Prompts and tier descriptions are viewable and editable locally; review them before sending file copies that may be sensitive.

## 5. Updates and security

**Silent deploy**: if you install via a silent script (--silent and related flags), you are treated as having read this disclaimer and [EULA.md](./EULA.md). Custom install dir / shortcuts / uninstall behavior follow the installer arguments. The fixed-name package (2-Pyramid-Installer.exe) is for deploy scripts and update channels only; it does not change license terms. Microsoft Store MSIX installs are governed by store terms together with this disclaimer.


In-app updates download installers from GitHub Releases or a configured mirror and verify SHA-256. If you obtain installers from unofficial channels, security and integrity are your responsibility.

## 6. Limitation of liability

To the maximum extent permitted by law, 2-Pyramid Studio and contributors are not liable for any indirect, incidental, special, or consequential damages (including lost profits, data loss, or business interruption), whether in contract, tort, or otherwise, even if advised of the possibility of such damages.

## 7. Credits

The project has drawn on ideas from other tools and authors in the community. Credits are in-app under Settings → Version Info and in `README.md`. Credits express thanks only and **do not** create joint copyright, employment, or agency.

---

**Contact:** https://github.com/realLivedInCorner/2-Pyramid/issues
