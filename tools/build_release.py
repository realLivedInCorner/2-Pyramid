#!/usr/bin/env python3
"""2-Pyramid 自建发布流水线（Windows 专用）。

流程：
  1. 前端构建（vue-tsc + vite）
  2. `tauri build --no-bundle`：编译主程序并嵌入前端资源（不打包）。
     BUILD 构建号由主程序 build.rs 在 release 编译时自动递增——
     这是唯一的递增点
  3. 收集产物到 release/staging/（exe、UImage、overlay、legal）
  4. 把 staging 打成 payload.zip，内嵌进独立安装器项目
     （installer-app —— Tauri 2 + Vue 3，自定义安装界面与注册表逻辑）
  5. 编译安装器项目（tauri build --no-bundle，便携版）
  6. 输出单文件安装包 release/2-Pyramid-Installer-{version}.exe
     （--beta 时输出 2-Pyramid-Installer-{version}-beta.{BUILD}.exe）
     → GitHub Releases
  7. 输出 MS Store 规范 MSIX：release/2-Pyramid-{version}.msix
     （makeappx + AppxManifest；身份来自 tools/msix/package-identity.json）
     → Microsoft Store。不再产出 MSI。

便携版不对外发布，只作为安装器内嵌 payload。
仅支持 Windows 平台。

用法：
  python tools/build_release.py              # 正式版：staging + 安装器 EXE + MSIX
  python tools/build_release.py --beta       # beta 渠道
  python tools/build_release.py --no-bump    # 不递增 BUILD（2PYR_NO_BUMP=1）
  python tools/build_release.py --skip-installer  # 只出 staging
  python tools/build_release.py --skip-msix       # 不打 MSIX
  python tools/build_release.py --sign-msix       # 签名 MSIX（sideload 测试）

需要：Node.js、Rust 工具链、Windows SDK（makeappx）。
签名（可选）环境变量：
  2PYR_MSIX_PFX      .pfx 路径
  2PYR_MSIX_PFX_PASS 密码
"""

import argparse
import hashlib
import json
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
MSIX_SRC = ROOT / "tools" / "msix"
ICON_DIR = TAURI_DIR / "icons"


def run(cmd: list[str], cwd: Path, label: str, env: dict | None = None) -> None:
    print(f"\n==> {label}: {' '.join(cmd)}")
    # Windows 下 npm/npx 是 .cmd 包装器，必须经 shell 才能被
    # CreateProcess 找到（本项目仅支持 Windows）。
    merged = dict(os.environ)
    if env:
        merged.update(env)
    result = subprocess.run(cmd, cwd=str(cwd), shell=True, env=merged)
    if result.returncode != 0:
        print(f"[FAILED] {label} (exit {result.returncode})", file=sys.stderr)
        sys.exit(1)


def read_build() -> int:
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

    for asset in ("UImage", "overlay"):
        src_dir = TAURI_DIR / asset
        if src_dir.is_dir():
            shutil.copytree(src_dir, STAGING / asset)

    legal_src = ROOT / "legal"
    if legal_src.is_dir():
        shutil.copytree(legal_src, STAGING / "legal")
        print(f"    legal/ -> {STAGING / 'legal'}")
    license_src = ROOT / "LICENSE"
    if license_src.exists():
        shutil.copy2(license_src, STAGING / "legal" / "LICENSE")


def make_payload_zip() -> Path:
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
    write_sha256_sidecar(final)
    print(f"    GitHub Releases 资产: {final.name}")


def find_makeappx() -> Path | None:
    """定位 Windows SDK 的 makeappx.exe（x64 优先）。"""
    env_path = os.environ.get("MAKEAPPX")
    if env_path and Path(env_path).exists():
        return Path(env_path)

    kits = Path(r"C:\Program Files (x86)\Windows Kits\10\bin")
    candidates: list[Path] = []
    if kits.is_dir():
        for ver_dir in sorted(kits.glob("10.*"), reverse=True):
            for arch in ("x64", "x86", "arm64"):
                p = ver_dir / arch / "makeappx.exe"
                if p.exists():
                    candidates.append(p)
    return candidates[0] if candidates else None


def find_signtool() -> Path | None:
    env_path = os.environ.get("SIGNTOOL")
    if env_path and Path(env_path).exists():
        return Path(env_path)
    kits = Path(r"C:\Program Files (x86)\Windows Kits\10\bin")
    if kits.is_dir():
        for ver_dir in sorted(kits.glob("10.*"), reverse=True):
            for arch in ("x64", "x86"):
                p = ver_dir / arch / "signtool.exe"
                if p.exists():
                    return p
    return None


def load_package_identity() -> dict:
    path = MSIX_SRC / "package-identity.json"
    data = json.loads(path.read_text(encoding="utf-8"))
    # 粗校验：占位符未替换时仍可打本地包，但会警告
    for key in ("identity_name", "publisher", "display_name", "publisher_display_name"):
        val = str(data.get(key, ""))
        if (
            "REPLACE" in val
            or "Placeholder" in val
            or "00000000-0000-0000-0000-000000000000" in val
            or not val.strip()
        ):
            print(
                f"[WARN] package-identity.json 的 {key} 仍是占位/空值：{val!r}\n"
                f"       上架 MS Store 前必须换成 Partner Center 真实身份。",
                file=sys.stderr,
            )
    return data


def package_version(version: str) -> str:
    """AppxManifest Version 必须是 4 段 major.minor.patch.revision。

    MS Store 要求 revision（第 4 段）为 0；Store 侧自行递增。
    BUILD 构建号不进入包版本，避免包接受校验失败。
    """
    ver = version.split("-")[0]
    parts = ver.split(".")
    while len(parts) < 3:
        parts.append("0")
    return f"{parts[0]}.{parts[1]}.{parts[2]}.0"


def render_manifest(identity: dict, version4: str) -> str:
    template = (MSIX_SRC / "AppxManifest.template.xml").read_text(encoding="utf-8")
    mapping = {
        "IDENTITY_NAME": identity["identity_name"],
        "PUBLISHER": identity["publisher"],
        "PUBLISHER_DISPLAY_NAME": identity["publisher_display_name"],
        "DISPLAY_NAME": identity["display_name"],
        "DESCRIPTION": identity.get("description", identity["display_name"]),
        "VERSION": version4,
        "MIN_VERSION": identity.get("min_version", "10.0.17763.0"),
        "MAX_VERSION_TESTED": identity.get("max_version_tested", "10.0.22621.0"),
    }
    out = template
    for key, val in mapping.items():
        out = out.replace("{{" + key + "}}", str(val))
    return out


def build_msix(version: str, staging: Path, beta: bool) -> Path | None:
    """把 staging 打成 MS Store 规范 MSIX（makeappx）。

    产物：release/2-Pyramid-{version}.msix
    上传 Microsoft Store（Partner Center）→ 商店侧签名分发。
    """
    makeappx = find_makeappx()
    if not makeappx:
        print(
            "[WARN] 未找到 Windows SDK makeappx.exe，跳过 MSIX。\n"
            "       安装 Windows 10/11 SDK，或设置环境变量 MAKEAPPX=路径。",
            file=sys.stderr,
        )
        return None

    identity = load_package_identity()
    version4 = package_version(version)

    pkg_dir = OUTPUT / "msix-build"
    if pkg_dir.exists():
        shutil.rmtree(pkg_dir)
    pkg_dir.mkdir(parents=True, exist_ok=True)

    # payload：staging 全量（exe / UImage / overlay / legal）
    for item in staging.iterdir():
        dest = pkg_dir / item.name
        if item.is_dir():
            shutil.copytree(item, dest)
        else:
            shutil.copy2(item, dest)

    # MS Store 资产
    assets = pkg_dir / "Assets"
    assets.mkdir(parents=True, exist_ok=True)
    for name in (
        "StoreLogo.png",
        "Square150x150Logo.png",
        "Square44x44Logo.png",
        "Square71x71Logo.png",
        "Square310x310Logo.png",
        "Wide310x150Logo.png",
        "SplashScreen.png",
    ):
        src = ICON_DIR / name
        if src.exists():
            shutil.copy2(src, assets / name)
        else:
            print(f"[WARN] 缺少 MSIX 资产: {src}", file=sys.stderr)

    # Manifest
    manifest_text = render_manifest(identity, version4)
    (pkg_dir / "AppxManifest.xml").write_text(manifest_text, encoding="utf-8")
    print(f"    AppxManifest Version={version4} Identity={identity['identity_name']}")

    if beta:
        out_name = f"2-Pyramid-{version}-beta.{read_build()}.msix"
    else:
        out_name = f"2-Pyramid-{version}.msix"
    msix_path = OUTPUT / out_name

    run(
        [
            str(makeappx),
            "pack",
            "/d",
            str(pkg_dir),
            "/p",
            str(msix_path),
            "/o",
        ],
        ROOT,
        "makeappx pack MSIX",
    )

    if not msix_path.exists():
        print("[FAILED] MSIX 未生成", file=sys.stderr)
        return None

    write_sha256_sidecar(msix_path)
    print(f"    MS Store 资产: {msix_path.name}")

    if os.environ.get("2PYR_SIGN_MSIX") == "1":
        sign_msix(msix_path)

    return msix_path


def sign_msix(msix_path: Path) -> None:
    """用环境变量证书签名（仅 sideload / 本地测试；Store 提交由商店侧签名）。"""
    signtool = find_signtool()
    pfx = os.environ.get("2PYR_MSIX_PFX")
    pfx_pass = os.environ.get("2PYR_MSIX_PFX_PASS", "")
    if not signtool:
        print("[WARN] 未找到 signtool，跳过签名", file=sys.stderr)
        return
    if not pfx or not Path(pfx).exists():
        print("[WARN] 2PYR_MSIX_PFX 未设置或文件不存在，跳过签名", file=sys.stderr)
        return

    cmd = [
        str(signtool),
        "sign",
        "/fd",
        "SHA256",
        "/f",
        pfx,
    ]
    if pfx_pass:
        cmd.extend(["/p", pfx_pass])
    cmd.append(str(msix_path))
    run(cmd, ROOT, "signtool sign MSIX")


def write_sha256_sidecar(final: Path) -> None:
    digest = hashlib.sha256()
    with open(final, "rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            digest.update(chunk)
    hex_digest = digest.hexdigest()
    sidecar = Path(str(final) + ".sha256")
    sidecar.write_text(hex_digest + "\n", encoding="utf-8")
    print(f"    {sidecar.name}: {hex_digest}")


def main() -> None:
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

    parser = argparse.ArgumentParser(
        description="2-Pyramid 发布流水线（Windows：EXE → GitHub Releases，MSIX → Microsoft Store）"
    )
    parser.add_argument("--no-bump", action="store_true", help="不递增 BUILD 版本")
    parser.add_argument("--skip-installer", action="store_true", help="只构建产物，不生成安装器 EXE")
    parser.add_argument("--skip-msix", action="store_true", help="不生成 MSIX")
    parser.add_argument(
        "--sign-msix",
        action="store_true",
        help="用 2PYR_MSIX_PFX 签名 MSIX（sideload 测试；商店提交无需本地签）",
    )
    parser.add_argument("--beta", action="store_true", help="beta 渠道构建（独立注册表、Beta 标识、可与正式版并存）")
    args = parser.parse_args()

    version = read_version()
    channel = "beta" if args.beta else "stable"
    channel_env = {"2PYR_CHANNEL": channel, "VITE_CHANNEL": channel}
    if args.no_bump:
        channel_env["2PYR_NO_BUMP"] = "1"
    if args.sign_msix:
        os.environ["2PYR_SIGN_MSIX"] = "1"
    print(
        f"==> 2-Pyramid 发布流水线 · 版本 {version} · 渠道 {channel}"
        f"（{'测试' if args.beta else '正式'}版）"
    )

    run(["npm", "run", "build"], ROOT, "主项目前端构建", env=channel_env)
    run(
        ["npx", "tauri", "build", "--no-bundle"],
        ROOT,
        "编译主程序 (tauri build --no-bundle)",
        env=channel_env,
    )

    collect_staging()

    if args.skip_installer and args.skip_msix:
        print(f"\n完成（跳过打包）。产物位于 {STAGING}")
        return

    if not args.skip_installer:
        build_installer(version, args.beta)
    else:
        print("\n[skip] 安装器 EXE（GitHub Releases）")

    if not args.skip_msix:
        try:
            build_msix(version.split("-")[0], STAGING, args.beta)
        except Exception as e:
            print(f"[WARN] MSIX build failed: {e}", file=sys.stderr)
    else:
        print("\n[skip] MSIX（Microsoft Store）")

    print(
        "\n分发约定：\n"
        "  · GitHub Releases ← 2-Pyramid-Installer-*.exe（+ .sha256）\n"
        "  · Microsoft Store ← 2-Pyramid-*.msix（Partner Center 提交）\n"
        "  · 不再产出 MSI"
    )


if __name__ == "__main__":
    main()
