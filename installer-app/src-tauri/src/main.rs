//! 2-Pyramid 独立安装器（Tauri 2 + Vue 3）。
//!
//! 内嵌 payload.zip（构建流水线生成，含主程序便携版全部文件）。
//! 图形界面负责安装目录选择与状态展示；Rust 侧命令：
//!   * install(dir)    —— 释放 payload + 写注册表（安装信息）
//!   * uninstall(dir)  —— 删除文件 + 删注册表（用户数据 ~/.2pyr 保留）
//!   * launch_app(dir) —— 启动刚安装的主程序
//!   * get_default_dir / get_version / is_installed
//!
//! 静默模式：`installer.exe --silent [--dir <path>]` 直接安装后退出
//! （自动更新器使用），不启动图形界面。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

const PAYLOAD: &[u8] = include_bytes!("../payload.zip");
const APP_NAME: &str = "2-Pyramid";
const EXE_NAME: &str = "2-pyramid.exe";

// ── 核心逻辑（silent 模式与 GUI 命令共用） ────────────────────────

fn default_install_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_NAME)
}

fn extract_payload(dest: &Path) -> Result<usize, String> {
    let reader = Cursor::new(PAYLOAD);
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| format!("payload.zip 损坏: {}", e))?;
    let mut count = 0usize;

    for i in 0..archive.len() {
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
    }
    Ok(count)
}

fn write_registry(dir: &Path) -> Result<(), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(r"Software\2-Pyramid")
        .map_err(|e| format!("写注册表失败: {}", e))?;
    key.set_value("InstallDir", &dir.to_string_lossy().to_string())
        .map_err(|e| format!("写注册表失败: {}", e))?;
    let version: &str = env!("CARGO_PKG_VERSION");
    key.set_value("Version", &version)
        .map_err(|e| format!("写注册表失败: {}", e))?;
    Ok(())
}

fn delete_registry() {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let _ = hkcu.delete_subkey_all(r"Software\2-Pyramid");
}

fn registry_install_dir() -> Option<PathBuf> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey(r"Software\2-Pyramid").ok()?;
    let dir: String = key.get_value("InstallDir").ok()?;
    Some(PathBuf::from(dir))
}

fn install_impl(dir: &Path) -> Result<String, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("创建安装目录失败: {}", e))?;
    let count = extract_payload(dir)?;
    let exe = dir.join(EXE_NAME);
    if !exe.exists() {
        return Err(format!("payload 中缺少 {}", EXE_NAME));
    }
    write_registry(dir)?;
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
fn is_installed() -> bool {
    registry_install_dir()
        .map(|d| d.join(EXE_NAME).exists())
        .unwrap_or(false)
}

#[tauri::command]
fn install(dir: String) -> Result<String, String> {
    let path = PathBuf::from(dir);
    install_impl(&path)
}

#[tauri::command]
fn uninstall(dir: String) -> Result<String, String> {
    let path = PathBuf::from(dir);
    uninstall_impl(&path)
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
        match install_impl(&dir) {
            Ok(_) => std::process::exit(0),
            Err(_) => std::process::exit(1),
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_default_dir,
            get_version,
            is_installed,
            install,
            uninstall,
            launch_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running installer app");
}
