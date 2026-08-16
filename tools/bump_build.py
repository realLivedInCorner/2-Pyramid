#!/usr/bin/env python3
"""
bump_build.py — Manually set / inspect the repo-root `BUILD` file.

> NOTE: As of the "compile-time auto-bump" change, release builds
> automatically increment the build number (starting from BUILD_START
> in `src-tauri/build.rs`, currently 20000). You usually do NOT need
> to run this script — the Rust build script handles it. This tool
> remains useful for inspection, manual override, or seeding the value.

Usage:
    python tools/bump_build.py --show       # print current value
    python tools/bump_build.py --set 42     # set to a specific value
    python tools/bump_build.py --reset      # reset to BUILD_START (20000)
    python tools/bump_build.py --bump       # increment by 1 (manual)
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
BUILD_FILE = REPO_ROOT / "BUILD"

# Must stay in sync with `BUILD_START` in `src-tauri/build.rs`.
BUILD_START = 20000


def read_current() -> int:
    if not BUILD_FILE.exists():
        print(f"[bump_build] BUILD file not found at {BUILD_FILE}; creating with {BUILD_START}.",
              file=sys.stderr)
        BUILD_FILE.write_text(f"{BUILD_START}\n", encoding="utf-8")
        return BUILD_START
    raw = BUILD_FILE.read_text(encoding="utf-8").strip()
    if not raw.isdigit():
        print(f"[bump_build] BUILD contents {raw!r} is not an integer; resetting to {BUILD_START}.",
              file=sys.stderr)
        return BUILD_START
    return int(raw)


def write_value(value: int) -> None:
    BUILD_FILE.write_text(f"{value}\n", encoding="utf-8")
    print(f"[bump_build] BUILD file updated -> {value}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Manage the build number in BUILD.")
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--set", type=int, metavar="N",
                       help="Set BUILD to N explicitly.")
    group.add_argument("--show", action="store_true",
                       help="Print the current value and exit.")
    group.add_argument("--reset", action="store_true",
                       help=f"Reset BUILD to BUILD_START ({BUILD_START}).")
    group.add_argument("--bump", action="store_true",
                       help="Increment BUILD by 1 (manual override).")
    args = parser.parse_args()

    current = read_current()

    if args.show:
        print(current)
        return 0
    if args.reset:
        write_value(BUILD_START)
        return 0
    if args.set is not None:
        write_value(args.set)
        return 0
    if args.bump:
        write_value(current + 1)
        return 0

    # No flag: default behaviour = show, with a hint.
    parser.print_help()
    print(f"\n[current BUILD = {current}]")
    return 0


if __name__ == "__main__":
    sys.exit(main())