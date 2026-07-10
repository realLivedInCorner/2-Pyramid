#!/usr/bin/env python3
"""Bump version across package.json, tauri.conf.json, and Cargo.toml in one shot.

Usage:  python set_version.py 2.0.2           # dry-run (shows what would change)
        python set_version.py 2.0.2 --write   # actually apply changes
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent

FILES = {
    "package.json": ROOT / "package.json",
    "tauri.conf.json": ROOT / "src-tauri" / "tauri.conf.json",
    "Cargo.toml": ROOT / "src-tauri" / "Cargo.toml",
    "HomePage.vue": ROOT / "src" / "components" / "HomePage.vue",
}

SEMVER_RE = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$")


def validate_version(v: str) -> None:
    if not SEMVER_RE.match(v):
        print(f"错误: '{v}' 不是合法的 semver 版本号 (MAJOR.MINOR.PATCH)")
        sys.exit(1)


def current_version() -> str:
    with open(FILES["package.json"], encoding="utf-8") as f:
        pkg = json.load(f)
    return pkg["version"]


def replace_in_file(path: Path, old: str, new: str) -> bool:
    """Replace first occurrence of old with new. Returns True if changed."""
    text = path.read_text(encoding="utf-8")
    if old not in text:
        return False
    updated = text.replace(old, new, 1)
    if updated == text:
        return False
    path.write_text(updated, encoding="utf-8")
    return True


def replace_dev_version(path: Path, new: str) -> bool:
    """Replace Dev-X.Y.Z pattern in HomePage.vue. Returns True if changed."""
    text = path.read_text(encoding="utf-8")
    pattern = re.compile(r"Dev-\d+\.\d+\.\d+")
    if not pattern.search(text):
        return False
    updated = pattern.sub(f"Dev-{new}", text, count=1)
    if updated == text:
        return False
    path.write_text(updated, encoding="utf-8")
    return True


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    new_ver = sys.argv[1]
    write = "--write" in sys.argv

    validate_version(new_ver)
    old_ver = current_version()

    if old_ver == new_ver:
        print(f"当前版本已经是 {old_ver}，无需修改。")
        return

    print(f"当前版本: {old_ver}  →  目标版本: {new_ver}\n")

    for name, path in FILES.items():
        if not path.exists():
            print(f"  ✗ {name}  找不到文件")
            continue
        if name == "HomePage.vue":
            # HomePage uses Dev-X.Y.Z format
            if write:
                changed = replace_dev_version(path, new_ver)
                mark = "✓" if changed else "="
            else:
                text = path.read_text(encoding="utf-8")
                changed = bool(re.search(r"Dev-\d+\.\d+\.\d+", text))
                mark = "○" if changed else "="
        else:
            if write:
                changed = replace_in_file(path, old_ver, new_ver)
                mark = "✓" if changed else "="
            else:
                text = path.read_text(encoding="utf-8")
                changed = old_ver in text
                mark = "○" if changed else "="
        print(f"  {mark} {name}  {'待修改' if changed and not write else '已修改' if changed else '无变化'}")

    if write:
        print(f"\n版本号已从 {old_ver} 更新为 {new_ver}")
    else:
        print(f"\n(干跑模式，加上 --write 才会实际写入)")


if __name__ == "__main__":
    main()
