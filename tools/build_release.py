#!/usr/bin/env python3
"""2-Pyramid 自建发布流水线。

流程：
  1. 递增 BUILD 版本号（可 --no-bump 跳过）
  2. 前端构建（vue-tsc + vite）
  3. 编译 COM server（two_pyramid_shell.dll，release）
  4. `tauri build --no-bundle`：编译主程序并嵌入前端资源（不打包）
  5. 收集产物到 release/staging/（exe、dll、UImage、overlay）
  6. 调用 Inno Setup 编译器 ISCC.exe 生成安装器
     输出 release/2-Pyramid-Setup-{version}.exe

用法：
  python tools/build_release.py            # 完整构建 + 打包
  python tools/build_release.py --no-bump  # 不递增 BUILD
  python tools/build_release.py --skip-installer  # 只构建产物不打包

需要：Node.js、Rust 工具链、Inno Setup 6（https://jrsoftware.org/isinfo.php）
"""

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TAURI_DIR = ROOT / "src-tauri"
STAGING = ROOT / "release" / "staging"
OUTPUT = ROOT / "release"

# Inno Setup 编译器的常见安装位置
ISCC_CANDIDATES = [
    os.environ.get("ISCC", ""),
    r"C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
    r"C:\Program Files\Inno Setup 6\ISCC.exe",
    str(Path(os.environ.get("LOCALAPPDATA", "")) / r"Programs\Inno Setup 6\ISCC.exe"),
]


def run(cmd: list[str], cwd: Path, label: str) -> None:
    print(f"\n==> {label}: {' '.join(cmd)}")
    result = subprocess.run(cmd, cwd=str(cwd), shell=False)
    if result.returncode != 0:
        print(f"[FAILED] {label} (exit {result.returncode})", file=sys.stderr)
        sys.exit(1)


def bump_build() -> None:
    """递增仓库根 BUILD 文件（与 tools/bump_build.py 的格式一致）。"""
    build_file = ROOT / "BUILD"
    current = 0
    if build_file.exists():
        text = build_file.read_text(encoding="utf-8").strip()
        try:
            current = int(text)
        except ValueError:
            current = 0
    new = current + 1
    build_file.write_text(f"{new}\n", encoding="utf-8")
    print(f"==> BUILD {current} -> {new}")


def find_iscc() -> str:
    for candidate in ISCC_CANDIDATES:
        if candidate and Path(candidate).is_file():
            return candidate
    print(
        "[FAILED] 未找到 Inno Setup 编译器 ISCC.exe。\n"
        "请安装 Inno Setup 6: https://jrsoftware.org/isinfo.php\n"
        "或设置环境变量 ISCC 指向 ISCC.exe。",
        file=sys.stderr,
    )
    sys.exit(1)


def read_version() -> str:
    cargo = TAURI_DIR / "Cargo.toml"
    for line in cargo.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line.startswith("version"):
            return line.split("=", 1)[1].strip().strip('"')
    return "2.0.0"


def collect_staging(version: str) -> None:
    print(f"\n==> 收集产物 -> {STAGING}")
    if STAGING.exists():
        shutil.rmtree(STAGING)
    STAGING.mkdir(parents=True, exist_ok=True)

    target = TAURI_DIR / "target" / "release"
    files = [
        (target / "2-pyramid.exe", STAGING / "2-pyramid.exe"),
        (target / "two_pyramid_shell.dll", STAGING / "two_pyramid_shell.dll"),
    ]
    for src, dst in files:
        if not src.exists():
            print(f"[FAILED] 缺少构建产物: {src}", file=sys.stderr)
            sys.exit(1)
        shutil.copy2(src, dst)

    # 运行期资源（tauri --no-bundle 不会复制到 exe 旁）
    for asset in ("UImage", "overlay"):
        src_dir = TAURI_DIR / asset
        if src_dir.is_dir():
            shutil.copytree(src_dir, STAGING / asset)


def main() -> None:
    parser = argparse.ArgumentParser(description="2-Pyramid 发布流水线")
    parser.add_argument("--no-bump", action="store_true", help="不递增 BUILD 版本")
    parser.add_argument("--skip-installer", action="store_true", help="只构建产物，不生成安装器")
    args = parser.parse_args()

    version = read_version()

    if not args.no_bump:
        bump_build()

    run(["npm", "run", "build"], ROOT, "前端构建")
    run(
        ["cargo", "build", "--release", "-p", "two_pyramid_shell"],
        TAURI_DIR,
        "编译 COM server (release)",
    )
    # tauri build --no-bundle：编译主程序 + 嵌入前端，不调用打包器
    run(
        ["npx", "tauri", "build", "--no-bundle"],
        ROOT,
        "编译主程序 (tauri build --no-bundle)",
    )

    collect_staging(version)

    if args.skip_installer:
        print(f"\n完成（跳过安装器打包）。产物位于 {STAGING}")
        return

    iscc = find_iscc()
    OUTPUT.mkdir(parents=True, exist_ok=True)
    cmd = [
        iscc,
        f"/DMyAppVersion={version}",
        f"/DStagingDir={STAGING}",
        str(ROOT / "installer" / "installer.iss"),
    ]
    run(cmd, ROOT, "Inno Setup 打包")

    installer = OUTPUT / f"2-Pyramid-Setup-{version}.exe"
    if installer.is_file():
        print(f"\n✅ 安装器已生成: {installer}")
    else:
        print("[FAILED] 安装器未生成", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
