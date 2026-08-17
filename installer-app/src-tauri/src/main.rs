//! 2-Pyramid 独立安装器（Tauri 2 + Vue 3，Windows 专用）。
//!
//! 内嵌 payload.zip（构建流水线生成，含主程序便携版全部文件）。
//! 图形界面提供 OOBE 式分步安装流程（欢迎/GitHub 介绍 → 安装地址 →
//! 实时进度 → 完成），并注册控制面板卸载入口。
//!
//! Rust 侧命令：
//!   * install(dir, create_shortcut) —— 释放 payload（emit 实时进度）、
//!     释放 uninstaller.exe、写安装信息与卸载注册表
//!   * uninstall(dir) —— 删除文件与全部注册表（用户数据 ~/.2pyr 保留）
//!   * launch_app / get_default_dir / get_version / is_installed
//!   * is_uninstall_mode —— 以 --uninstall 启动时前端展示卸载流程
//!
//! 静默模式：`installer.exe --silent [--dir <path>]` 直接安装后退出
//! （自动更新器使用），不启动图形界面。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

const PAYLOAD: &[u8] = include_bytes!("../payload.zip");
const APP_NAME: &str = "2-Pyramid";
const EXE_NAME: &str = "2-pyramid.exe";
const UNINSTALLER_NAME: &str = "uninstall.exe";
const GITHUB_URL: &str = "https://github.com/realLivedInCorner/2-Pyramid";

// ── 进度事件载荷 ─────────────────────────────────────────────────

#[derive(Serialize, Clone)]
struct InstallProgress {
    current: usize,
    total: usize,
    name: String,
}

// ── 核心逻辑（silent 模式与 GUI 命令共用） ────────────────────────

fn default_install_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_NAME)
}

/// 解压 payload.zip。`on_file` 在每释放一个文件后回调（用于进度上报）。
fn extract_payload<F>(dest: &Path, mut on_file: F) -> Result<usize, String>
where
    F: FnMut(usize, usize, &str),
{
    let reader = Cursor::new(PAYLOAD);
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| format!("payload.zip 损坏: {}", e))?;
    let total = archive.len();
    let mut count = 0usize;

    for i in 0..total {
        let mut entry = archive.by_index(i).map_err(|e| format!("读取条目失败: {}", e))?;
        let name = entry.name().to_string();
        if name.contains("..") || name.starts_with('/') || name.starts_with('\\') || name.is_empty() {
            continue;
        }
        let out_path = dest.join(&name);
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path).map_err(|e| format!("创建目录失败: {}", e))?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
        }
        let mut file = std::fs::File::create(&out_path).map_err(|e| format!("创建文件失败: {}", e))?;
        std::io::copy(&mut entry, &mut file).map_err(|e| format!("写入失败: {}", e))?;
        count += 1;
        on_file(count, total, &name);
    }
    Ok(count)
}

/// 复制安装器自身到安装目录作为卸载器（uninstall.exe）。
fn deploy_uninstaller(dir: &Path) -> Result<(), String> {
    let current = std::env::current_exe().map_err(|e| format!("获取自身路径失败: {}", e))?;
    let dest = dir.join(UNINSTALLER_NAME);
    if current != dest {
        std::fs::copy(&current, &dest).map_err(|e| format!("复制卸载器失败: {}", e))?;
    }
    Ok(())
}

/// 写安装信息 + 控制面板卸载入口注册表。
fn write_registry(dir: &Path) -> Result<(), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let dir_str = dir.to_string_lossy().to_string();
    let exe = format!("{}\\{}", dir_str, EXE_NAME);
    let uninstaller = format!("{}\\{}", dir_str, UNINSTALLER_NAME);

    // 安装信息
    let (key, _) = hkcu
        .create_subkey(r"Software\2-Pyramid")
        .map_err(|e| format!("写注册表失败: {}", e))?;
    key.set_value("InstallDir", &dir_str).map_err(|e| format!("写注册表失败: {}", e))?;
    let version: &str = env!("CARGO_PKG_VERSION");
    key.set_value("Version", &version).map_err(|e| format!("写注册表失败: {}", e))?;
    key.set_value("UninstallString", &uninstaller).map_err(|e| format!("写注册表失败: {}", e))?;

    // 控制面板「卸载程序」入口
    let (ukey, _) = hkcu
        .create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\2-Pyramid")
        .map_err(|e| format!("写卸载注册表失败: {}", e))?;
    ukey.set_value("DisplayName", &"2-Pyramid").map_err(|e| format!("写卸载注册表失败: {}", e))?;
    ukey.set_value("DisplayVersion", &version).map_err(|e| format!("写卸载注册表失败: {}", e))?;
    ukey.set_value("Publisher", &"2-Pyramid Studio").map_err(|e| format!("写卸载注册表失败: {}", e))?;
    ukey.set_value("UninstallString", &uninstaller).map_err(|e| format!("写卸载注册表失败: {}", e))?;
    ukey.set_value("InstallLocation", &dir_str).map_err(|e| format!("写卸载注册表失败: {}", e))?;
    ukey.set_value("DisplayIcon", &exe).map_err(|e| format!("写卸载注册表失败: {}", e))?;
    ukey.set_value("NoModify", &1u32).map_err(|e| format!("写卸载注册表失败: {}", e))?;
    ukey.set_value("NoRepair", &1u32).map_err(|e| format!("写卸载注册表失败: {}", e))?;
    Ok(())
}

fn delete_registry() {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let _ = hkcu.delete_subkey_all(r"Software\2-Pyramid");
    let _ = hkcu.delete_subkey_all(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\2-Pyramid");
}

fn registry_install_dir() -> Option<PathBuf> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey(r"Software\2-Pyramid").ok()?;
    let dir: String = key.get_value("InstallDir").ok()?;
    Some(PathBuf::from(dir))
}

fn create_desktop_shortcut(_dir: &Path) {
    // 快捷方式创建（预留）：当前版本暂不实现，静默跳过。
    // 将来可通过 WScript.Shell COM 或 IShellLink 创建 .lnk。
}

fn install_impl<F>(dir: &Path, create_shortcut: bool, mut on_progress: F) -> Result<String, String>
where
    F: FnMut(usize, usize, &str),
{
    std::fs::create_dir_all(dir).map_err(|e| format!("创建安装目录失败: {}", e))?;
    let count = extract_payload(dir, |c, t, n| on_progress(c, t, n))?;
    let exe = dir.join(EXE_NAME);
    if !exe.exists() {
        return Err(format!("payload 中缺少 {}", EXE_NAME));
    }
    deploy_uninstaller(dir)?;
    write_registry(dir)?;
    if create_shortcut {
        create_desktop_shortcut(dir);
    }
    Ok(format!("安装完成，共释放 {} 个文件到 {}", count, dir.display()))
}

fn uninstall_impl(dir: &Path) -> Result<String, String> {
    delete_registry();
    if dir.exists() {
        std::fs::remove_dir_all(dir).map_err(|e| format!("删除安装目录失败: {}", e))?;
    }
    Ok("卸载完成（用户数据 ~/.2pyr 已保留）".to_string())
}

// ── Tauri 命令 ───────────────────────────────────────────────────

#[tauri::command]
fn get_default_dir() -> String {
    default_install_dir().to_string_lossy().to_string()
}

#[tauri::command]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn get_github_url() -> String {
    GITHUB_URL.to_string()
}

#[tauri::command]
fn is_installed() -> bool {
    registry_install_dir()
        .map(|d| d.join(EXE_NAME).exists())
        .unwrap_or(false)
}

#[tauri::command]
fn get_installed_dir() -> Option<String> {
    registry_install_dir().map(|d| d.to_string_lossy().to_string())
}

#[tauri::command]
fn is_uninstall_mode() -> bool {
    std::env::args().any(|a| a == "--uninstall")
}

#[tauri::command]
fn install(app: AppHandle, dir: String, create_shortcut: bool) -> Result<String, String> {
    let path = PathBuf::from(dir);
    install_impl(&path, create_shortcut, |current, total, name| {
        let _ = app.emit(
            "install-progress",
            InstallProgress {
                current,
                total,
                name: name.to_string(),
            },
        );
    })
}

#[tauri::command]
fn uninstall(dir: String) -> Result<String, String> {
    uninstall_impl(&PathBuf::from(dir))
}

#[tauri::command]
fn launch_app(dir: String) -> Result<(), String> {
    let exe = PathBuf::from(dir).join(EXE_NAME);
    if !exe.exists() {
        return Err(format!("未找到 {}", exe.display()));
    }
    std::process::Command::new(&exe)
        .spawn()
        .map_err(|e| format!("启动失败: {}", e))?;
    Ok(())
}

// ── 入口 ─────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // 静默安装（自动更新器）：--silent [--dir <path>]
    if args.iter().any(|a| a == "--silent") {
        let dir = args
            .iter()
            .position(|a| a == "--dir")
            .and_then(|i| args.get(i + 1))
            .map(PathBuf::from)
            .unwrap_or_else(default_install_dir);
        match install_impl(&dir, false, |_, _, _| {}) {
            Ok(_) => std::process::exit(0),
            Err(_) => std::process::exit(1),
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_default_dir,
            get_version,
            get_github_url,
            is_installed,
            is_uninstall_mode,
            get_installed_dir,
            install,
            uninstall,
            launch_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running installer app");
}
