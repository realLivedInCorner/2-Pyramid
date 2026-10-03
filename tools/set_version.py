#!/usr/bin/env python3
"""2-Pyramid 版本号一站式设置 / 校验。

用法
----
    python tools/set_version.py --check             # 只校验各处是否一致（不一致 → 退出码 1）
    python tools/set_version.py 2.8.0               # 干跑：显示每处将如何变化
    python tools/set_version.py 2.8.0 --write       # 实际写入
    python tools/set_version.py 2.8.0 --write --changelog
                                                    # 额外把 CHANGELOG 的 [Unreleased]
                                                    # 改成 [2.8.0] - 日期（BUILD 下一个值）

为什么需要这个脚本
------------------
版本号散落在 **7 处**，其中 `installer-app` 的三处极易漏掉——更新程序界面的
「更新程序 · vX」、覆盖更新的「更新到 X」，以及**控制面板登记的已安装版本**都取自
installer-app 的 `CARGO_PKG_VERSION`。2026-10-03 发布 2.7.0 时就因为漏改它，
出现了「2.7.0 的安装包显示成 2.6.0」。`tools/build_release.py` 已内置 `--check`
守门：任一处不一致就拒绝构建。
"""

from __future__ import annotations

import argparse
import datetime as _dt
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

SEMVER_RE = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$")

# ── 版本号所在位置 ────────────────────────────────────────────────
JSON_VERSION_FILES = [
    "package.json",
    "src-tauri/tauri.conf.json",
    "installer-app/package.json",
    "installer-app/src-tauri/tauri.conf.json",
]
TOML_VERSION_FILES = [
    "src-tauri/Cargo.toml",
    "installer-app/src-tauri/Cargo.toml",
]
MSIX_IDENTITY = "tools/msix/package-identity.json"
README = "README.md"
CHANGELOG = "CHANGELOG.md"
BUILD_FILE = "BUILD"
# Cargo.lock 里需要跟着改的两个 crate
LOCK_CRATES = {
    "src-tauri/Cargo.lock": "two-pyramid",
    "installer-app/src-tauri/Cargo.lock": "two-pyr-installer-app",
}


class Target:
    """一个版本号位置。"""

    def __init__(self, path: str, label: str, read, write, expect=None):
        self.path = ROOT / path
        self.label = label
        self._read = read
        self._write = write
        # expect: 校验时的期望值（默认与主 package.json 相同，MSIX 用 "x.y.z.0"）
        self.expect = expect

    def read(self):
        if not self.path.exists():
            return None
        return self._read(self.path.read_text(encoding="utf-8"))

    def write(self, old: str, new: str) -> bool:
        text = self.path.read_text(encoding="utf-8")
        updated = self._write(text, old, new)
        if updated is None or updated == text:
            return False
        self.path.write_text(updated, encoding="utf-8")
        return True


def _json_read(key: str):
    def read(text: str):
        try:
            return json.loads(text).get(key)
        except json.JSONDecodeError:
            return None

    return read


def _json_write(key: str):
    def write(text: str, old: str, new: str):
        pattern = re.compile(r'("%s"\s*:\s*")%s(")' % (re.escape(key), re.escape(old)))
        return pattern.sub(lambda m: m.group(1) + new + m.group(2), text, count=1)

    return write


def _toml_read(text: str):
    m = re.search(r'(?m)^version\s*=\s*"([^"]+)"', text)
    return m.group(1) if m else None


def _toml_write(text: str, old: str, new: str):
    pattern = re.compile(r'(?m)^(version\s*=\s*")%s(")' % re.escape(old))
    return pattern.sub(lambda m: m.group(1) + new + m.group(2), text, count=1)


def _msix_read(text: str):
    try:
        ver = json.loads(text).get("package_version")
    except json.JSONDecodeError:
        return None
    # 展示成四段，便于和 semver 对照
    return ver


def _msix_write(text: str, old: str, new: str):
    pattern = re.compile(r'("package_version"\s*:\s*")%s(")' % re.escape(old))
    return pattern.sub(lambda m: m.group(1) + new + m.group(2), text, count=1)


def _readme_read(text: str):
    m = re.search(r"badge/version-(\d+\.\d+\.\d+)-", text)
    return m.group(1) if m else None


def _readme_write(text: str, old: str, new: str):
    return text.replace(f"badge/version-{old}-", f"badge/version-{new}-", 1)


def _lock_read(crate: str):
    def read(text: str):
        m = re.search(
            r'\[\[package\]\]\s*\nname = "%s"\s*\nversion = "([^"]+)"' % re.escape(crate), text
        )
        return m.group(1) if m else None

    return read


def _lock_write(crate: str):
    def write(text: str, old: str, new: str):
        pattern = re.compile(
            r'(\[\[package\]\]\s*\nname = "%s"\s*\nversion = ")%s(")' % (re.escape(crate), re.escape(old))
        )
        return pattern.sub(lambda m: m.group(1) + new + m.group(2), text, count=1)

    return write


def targets() -> list[Target]:
    out: list[Target] = []
    for rel in JSON_VERSION_FILES:
        out.append(Target(rel, rel, _json_read("version"), _json_write("version")))
    for rel in TOML_VERSION_FILES:
        out.append(Target(rel, rel, _toml_read, _toml_write))
    out.append(Target(MSIX_IDENTITY, "tools/msix/package-identity.json", _msix_read, _msix_write))
    out.append(Target(README, "README.md（版本徽章）", _readme_read, _readme_write))
    for rel, crate in LOCK_CRATES.items():
        out.append(Target(rel, f"{rel}（{crate}）", _lock_read(crate), _lock_write(crate)))
    return out


def canonical_version() -> str | None:
    path = ROOT / "package.json"
    try:
        return json.loads(path.read_text(encoding="utf-8"))["version"]
    except Exception:
        return None


def _expected_for(target: Target, version: str):
    if target.path.name == "package-identity.json":
        return f"{version}.0"
    return version


def cmd_check(quiet: bool = False) -> int:
    """校验所有位置与 package.json 一致；返回进程退出码。"""
    version = canonical_version()
    if not version:
        print("✗ 无法读取 package.json 的 version")
        return 1

    problems = []
    rows = []
    for t in targets():
        actual = t.read()
        expected = _expected_for(t, version)
        ok = actual == expected
        rows.append((t.label, actual, expected, ok))
        if not ok:
            problems.append((t.label, actual, expected))

    if not quiet:
        print(f"版本基准（package.json）: {version}\n")
        width = max(len(r[0]) for r in rows)
        for label, actual, expected, ok in rows:
            mark = "✓" if ok else "✗"
            print(f"  {mark} {label:<{width}}  {actual!s:<10} （期望 {expected}）")

    # CHANGELOG / BUILD 的软性提醒（不参与退出码）
    cl = ROOT / CHANGELOG
    if cl.exists():
        text = cl.read_text(encoding="utf-8")
        if not re.search(rf"(?m)^## \[{re.escape(version)}\]", text):
            print(f"\n! CHANGELOG 里没有 `## [{version}]` 段落（发布前需把 [Unreleased] 整理为正式段落）")

    if problems:
        print(f"\n✗ 有 {len(problems)} 处版本号不一致：")
        for label, actual, expected in problems:
            print(f"    {label}: {actual} ≠ {expected}")
        print("\n  修复：python tools/set_version.py " + version + " --write")
        return 1

    print(f"\n✓ 全部 {len(rows)} 处版本号一致：{version}")
    return 0


def cmd_set(new_version: str, write: bool, changelog: bool) -> int:
    if not SEMVER_RE.match(new_version):
        print(f"✗ '{new_version}' 不是合法 semver（MAJOR.MINOR.PATCH）")
        return 2

    version = canonical_version()
    if version is None:
        print("✗ 无法读取 package.json 的 version")
        return 2

    print(f"版本：{version} → {new_version}（{'写入' if write else '干跑'}）\n")

    changed = 0
    for t in targets():
        actual = t.read()
        expected = _expected_for(t, new_version)
        if actual == expected:
            print(f"  = {t.label}（已是 {actual}）")
            continue
        if actual is None:
            print(f"  ✗ {t.label}: 找不到版本号字段（文件缺失或格式变了）")
            continue
        if write:
            if t.write(actual, expected):
                print(f"  ✓ {t.label}: {actual} → {expected}")
                changed += 1
            else:
                print(f"  ✗ {t.label}: 未找到可替换的旧值 {actual}")
        else:
            print(f"  ○ {t.label}: {actual} → {expected}")

    if changelog:
        cl = ROOT / CHANGELOG
        next_build = read_build() + 1
        today = _dt.date.today().isoformat()
        text = cl.read_text(encoding="utf-8")
        if re.search(rf"(?m)^## \[{re.escape(new_version)}\]", text):
            print(f"  = CHANGELOG 已有 [{new_version}] 段落")
        elif re.search(r"(?m)^## \[Unreleased\]", text):
            if write:
                text = re.sub(
                    r"(?m)^## \[Unreleased\]",
                    f"## [{new_version}] - {today}（BUILD {next_build}）",
                    text,
                    count=1,
                )
                cl.write_text(text, encoding="utf-8")
                print(f"  ✓ CHANGELOG: [Unreleased] → [{new_version}] - {today}（BUILD {next_build}）")
                changed += 1
            else:
                print(f"  ○ CHANGELOG: [Unreleased] → [{new_version}] - {today}（BUILD {next_build}）")
        else:
            print("  ! CHANGELOG 里既没有目标段落也没有 [Unreleased]")

    if write:
        print(f"\n已更新 {changed} 处。建议接着运行：python tools/set_version.py --check")
    else:
        print("\n（干跑模式；加 --write 才会写入）")
    return 0


def read_build() -> int:
    try:
        return int((ROOT / BUILD_FILE).read_text(encoding="utf-8").strip())
    except Exception:
        return 0


def main() -> int:
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

    parser = argparse.ArgumentParser(
        description="2-Pyramid 版本号一站式设置 / 校验（七处 + 两个 Cargo.lock + README 徽章）",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument("version", nargs="?", help="目标版本，如 2.8.0")
    parser.add_argument("--write", action="store_true", help="实际写入（默认干跑）")
    parser.add_argument("--check", action="store_true", help="只校验一致性，不修改")
    parser.add_argument("--changelog", action="store_true", help="同时把 [Unreleased] 整理为正式段落")
    args = parser.parse_args()

    if args.check:
        return cmd_check()
    if not args.version:
        parser.print_help()
        return 2
    return cmd_set(args.version, args.write, args.changelog)


if __name__ == "__main__":
    sys.exit(main())
