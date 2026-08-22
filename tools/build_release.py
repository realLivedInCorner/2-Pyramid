#!/usr/bin/env python3
"""2-Pyramid 自建发布流水线（Windows 专用）。

流程：
  1. 前端构建（vue-tsc + vite）
  2. `tauri build --no-bundle`：编译主程序并嵌入前端资源（不打包）。
     BUILD 构建号由主程序 build.rs 在 release 编译时自动递增——
     这是唯一的递增点（历史版本里 Python 也会 bump 一次，导致
     每次构建号 +2；现已移除）
  3. 收集产物到 release/staging/（exe、UImage、overlay）
  4. 把 staging 打成 payload.zip，内嵌进独立安装器项目
     （installer-app —— Tauri 2 + Vue 3，自定义安装界面与注册表逻辑）
  5. 编译安装器项目（tauri build --no-bundle，便携版）
  6. 输出单文件安装包 release/2-Pyramid-Installer-{version}.exe
     （--beta 时输出 2-Pyramid-Installer-{version}-beta.{BUILD}.exe，
       安装器以 beta 渠道编译：独立注册表键、Beta 标识、可并存）

便携版不对外发布，只作为安装器内嵌 payload。
仅支持 Windows 平台。

用法：
  python tools/build_release.py              # 正式版完整构建 + 安装器
  python tools/build_release.py --beta       # beta 渠道构建 + 安装器
  python tools/build_release.py --no-bump    # 不递增 BUILD（2PYR_NO_BUMP=1）
  python tools/build_release.py --skip-installer  # 只出 staging 产物

需要：Node.js、Rust 工具链。不依赖任何第三方打包工具。
"""

import argparse
import os
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TAURI_DIR = ROOT / "src-tauri"
INSTALLER_APP = ROOT / "installer-app"
STAGING = ROOT / "release" / "staging"
OUTPUT = ROOT / "release"
BUILD_FILE = ROOT / "BUILD"


def run(cmd: list[str], cwd: Path, label: str, env: dict | None = None) -> None:
    print(f"\n==> {label}: {' '.join(cmd)}")
    # Windows 上 npm/npx 是 .cmd 包装器，必须经 shell 才能被
    # CreateProcess 找到（本项目仅支持 Windows）
    merged = dict(os.environ)
    if env:
        merged.update(env)
    result = subprocess.run(cmd, cwd=str(cwd), shell=True, env=merged)
    if result.returncode != 0:
        print(f"[FAILED] {label} (exit {result.returncode})", file=sys.stderr)
        sys.exit(1)


def read_build() -> int:
    """读取仓库根 BUILD 文件当前值。"""
    if BUILD_FILE.exists():
        try:
            return int(BUILD_FILE.read_text(encoding="utf-8").strip())
        except ValueError:
            return 0
    return 0


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
    ]
    for src, dst in files:
        if not src.exists():
            print(f"[FAILED] 缺少构建产物: {src}", file=sys.stderr)
            sys.exit(1)
        shutil.copy2(src, dst)

    # 外部依赖资源（tauri --no-bundle 不会复制到 exe 旁）
    for asset in ("UImage", "overlay"):
        src_dir = TAURI_DIR / asset
        if src_dir.is_dir():
            shutil.copytree(src_dir, STAGING / asset)

    # java_ui 模板（Bedrock 转换用）：存放于 doc/java_ui（设计文档同目录）
    java_ui_src = ROOT / "doc" / "java_ui"
    if java_ui_src.is_dir():
        shutil.copytree(java_ui_src, STAGING / "java_ui")


def make_payload_zip() -> Path:
    """把 staging 打成 payload.zip，放入安装器项目（内嵌发布）。"""
    print(f"\n==> 生成 payload.zip -> installer-app/src-tauri/")
    payload = INSTALLER_APP / "src-tauri" / "payload.zip"
    if payload.exists():
        payload.unlink()
    with zipfile.ZipFile(payload, "w", zipfile.ZIP_DEFLATED) as zf:
        for path in sorted(STAGING.rglob("*")):
            if path.is_file():
                zf.write(path, path.relative_to(STAGING))
    print(f"    {payload} ({payload.stat().st_size} bytes)")
    return payload


def build_installer(version: str, beta: bool) -> None:
    channel = "beta" if beta else "stable"
    print(f"\n==> 编译安装器项目 (installer-app, tauri build --no-bundle, 渠道: {channel})")
    make_payload_zip()
    # 渠道经环境变量注入 Rust 编译期（option_env!），beta/正式版注册表与标识隔离
    run(
        ["npx", "tauri", "build", "--no-bundle"],
        INSTALLER_APP,
        "tauri build installer-app",
        env={"2PYR_CHANNEL": channel},
    )

    installer_exe = INSTALLER_APP / "src-tauri" / "target" / "release" / "two-pyr-installer-app.exe"
    if not installer_exe.exists():
        print("[FAILED] 安装器未编译成功", file=sys.stderr)
        sys.exit(1)

    OUTPUT.mkdir(parents=True, exist_ok=True)
    if beta:
        final = OUTPUT / f"2-Pyramid-Installer-{version}-beta.{read_build()}.exe"
    else:
        final = OUTPUT / f"2-Pyramid-Installer-{version}.exe"
    shutil.copy2(installer_exe, final)
    print(f"\n✅ 安装器已生成（{channel} 渠道）: {final}")


def main() -> None:
    # Windows GBK 控制台无法输出 ✅/✓ 等字符，统一重配 stdout，避免构建完成后崩溃
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

    parser = argparse.ArgumentParser(description="2-Pyramid 发布流水线（Windows，便携版内嵌安装器）")
    parser.add_argument("--no-bump", action="store_true", help="不递增 BUILD 版本")
    parser.add_argument("--skip-installer", action="store_true", help="只构建产物，不生成安装器")
    parser.add_argument("--beta", action="store_true", help="beta 渠道构建（独立注册表、Beta 标识、可与正式版并存）")
    args = parser.parse_args()

    version = read_version()
    channel = "beta" if args.beta else "stable"
    # 主程序与前端同样渠道感知：2PYR_CHANNEL 进入 Rust 编译期
    # （option_env!，窗口标题 / AppInfo.channel），VITE_CHANNEL 进入
    # vite（index.html 的 %VITE_BETA_MARK% 替换）。
    # BUILD 递增交给主程序 build.rs（release 编译期唯一递增点）；
    # --no-bump 时经 2PYR_NO_BUMP=1 让 build.rs 只读不写。
    channel_env = {"2PYR_CHANNEL": channel, "VITE_CHANNEL": channel}
    if args.no_bump:
        channel_env["2PYR_NO_BUMP"] = "1"
    print(f"==> 2-Pyramid 发布流水线 · 版本 {version} · 渠道 {channel}（{'测试' if args.beta else '正式'}版）")

    run(["npm", "run", "build"], ROOT, "主项目前端构建", env=channel_env)
    run(
        ["npx", "tauri", "build", "--no-bundle"],
        ROOT,
        "编译主程序 (tauri build --no-bundle)",
        env=channel_env,
    )

    collect_staging()

    if args.skip_installer:
        print(f"\n完成（跳过安装器打包）。产物位于 {STAGING}")
        return

    build_installer(version, args.beta)


if __name__ == "__main__":
    main()
