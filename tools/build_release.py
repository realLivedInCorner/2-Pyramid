#!/usr/bin/env python3
"""2-Pyramid 自建发布流水线（自制释放式安装器）。

流程：
  1. 递增 BUILD 版本号（可 --no-bump 跳过）
  2. 前端构建（vue-tsc + vite）
  3. 编译 COM server（two_pyramid_shell.dll，release）
  4. `tauri build --no-bundle`：编译主程序并嵌入前端资源（不打包）
  5. 收集产物到 release/staging/（exe、dll、UImage、overlay）
  6. 把 staging 打成 payload.zip 交给自制安装器内嵌
  7. 编译自制安装器（2pyr-installer，release）
  8. 输出单文件安装包 release/2-Pyramid-Setup-{version}.exe

用法：
  python tools/build_release.py            # 完整构建 + 安装器
  python tools/build_release.py --no-bump  # 不递增 BUILD
  python tools/build_release.py --skip-installer  # 只出 staging 产物

需要：Node.js、Rust 工具链。不依赖任何第三方打包工具。
"""

import argparse
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TAURI_DIR = ROOT / "src-tauri"
INSTALLER_DIR = TAURI_DIR / "installer"
STAGING = ROOT / "release" / "staging"
OUTPUT = ROOT / "release"


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


def read_version() -> str:
    cargo = TAURI_DIR / "Cargo.toml"
    for line in cargo.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line.startswith("version"):
            return line.split("=", 1)[1].strip().strip('"')
    return "2.0.0"


def collect_staging() -> None:
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


def make_payload_zip() -> Path:
    """把 staging 打成 payload.zip（供安装器内嵌）。"""
    print(f"\n==> 生成 payload.zip")
    payload = INSTALLER_DIR / "payload.zip"
    if payload.exists():
        payload.unlink()
    with zipfile.ZipFile(payload, "w", zipfile.ZIP_DEFLATED) as zf:
        for path in sorted(STAGING.rglob("*")):
            if path.is_file():
                zf.write(path, path.relative_to(STAGING))
    print(f"    {payload} ({payload.stat().st_size} bytes)")
    return payload


def build_installer(version: str) -> None:
    print("\n==> 编译自制安装器 (2pyr-installer, release)")
    # payload.zip 必须在编译前就位（include_bytes! 嵌入）
    make_payload_zip()
    run(["cargo", "build", "--release", "-p", "two-pyr-installer"], TAURI_DIR, "cargo build installer")

    installer_exe = TAURI_DIR / "target" / "release" / "two-pyr-installer.exe"
    if not installer_exe.exists():
        print("[FAILED] 安装器未编译成功", file=sys.stderr)
        sys.exit(1)

    OUTPUT.mkdir(parents=True, exist_ok=True)
    final = OUTPUT / f"2-Pyramid-Setup-{version}.exe"
    shutil.copy2(installer_exe, final)
    print(f"\n✅ 安装器已生成: {final}")


def main() -> None:
    parser = argparse.ArgumentParser(description="2-Pyramid 发布流水线（自制释放式安装器）")
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
    run(
        ["npx", "tauri", "build", "--no-bundle"],
        ROOT,
        "编译主程序 (tauri build --no-bundle)",
    )

    collect_staging()

    if args.skip_installer:
        print(f"\n完成（跳过安装器打包）。产物位于 {STAGING}")
        return

    build_installer(version)


if __name__ == "__main__":
    main()
