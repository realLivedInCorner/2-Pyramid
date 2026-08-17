//! 2-Pyramid 自制安装器（单文件释放式）。
//!
//! 内嵌 `payload.zip`（构建流水线生成，含 2-pyramid.exe /
//! two_pyramid_shell.dll / UImage / overlay），双击运行后：
//!   1. 释放文件到安装目录（默认 %LOCALAPPDATA%\2-Pyramid，每用户免管理员）
//!   2. 写入注册表（安装信息 HKCU\Software\2-Pyramid —— 右键菜单已移除）
//!   3. 输出安装结果
//!
//! 参数：
//!   --dir <path>   自定义安装目录
//!   --silent       静默安装（自动更新器使用）
//!   --uninstall    卸载（删除文件 + 注册表；用户数据 ~/.2pyr 保留）

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

const PAYLOAD: &[u8] = include_bytes!("../payload.zip");
const APP_NAME: &str = "2-Pyramid";
const EXE_NAME: &str = "2-pyramid.exe";

fn log(silent: bool, msg: &str) {
    if !silent {
        println!("[2pyr-installer] {}", msg);
    }
}

fn default_install_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_NAME)
}

/// 解压 payload.zip 到目标目录。拒绝路径穿越（entry 名含 .. 直接跳过）。
fn extract_payload(dest: &Path) -> Result<usize, String> {
    let reader = Cursor::new(PAYLOAD);
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| format!("payload.zip 损坏: {}", e))?;
    let mut count = 0usize;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| format!("读取条目失败: {}", e))?;
        let name = entry.name().to_string();

        // 防路径穿越
        if name.contains("..") || name.starts_with('/') || name.starts_with('\\') {
            continue;
        }
        if name.is_empty() {
            continue;
        }

        let out_path = dest.join(&name);
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path).map_err(|e| format!("创建目录失败 {}: {}", out_path.display(), e))?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败 {}: {}", parent.display(), e))?;
        }
        let mut file = std::fs::File::create(&out_path)
            .map_err(|e| format!("创建文件失败 {}: {}", out_path.display(), e))?;
        std::io::copy(&mut entry, &mut file).map_err(|e| format!("写入失败 {}: {}", out_path.display(), e))?;
        count += 1;
    }
    Ok(count)
}

/// 写安装信息注册表（HKCU\Software\2-Pyramid：安装目录、版本信息）。
/// 右键菜单已移除，不再注册 .zip shell 扩展。
fn write_registry(dir: &Path) -> Result<(), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(r"Software\2-Pyramid")
        .map_err(|e| format!("写注册表失败: {}", e))?;
    key.set_value("InstallDir", &dir.to_string_lossy().to_string())
        .map_err(|e| format!("写注册表失败: {}", e))?;
    key.set_value("Version", env!("CARGO_PKG_VERSION"))
        .map_err(|e| format!("写注册表失败: {}", e))?;
    Ok(())
}

fn delete_registry() {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let _ = hkcu.delete_subkey_all(r"Software\2-Pyramid");
}

fn install(dir: &Path, silent: bool) -> Result<(), String> {
    log(silent, &format!("安装到 {}", dir.display()));
    std::fs::create_dir_all(dir).map_err(|e| format!("创建安装目录失败: {}", e))?;

    let count = extract_payload(dir)?;
    log(silent, &format!("已释放 {} 个文件", count));

    let exe = dir.join(EXE_NAME);
    if !exe.exists() {
        return Err(format!("payload 中缺少 {}", EXE_NAME));
    }
    write_registry(dir)?;
    log(silent, "安装信息已写入注册表");

    log(silent, &format!("✅ 安装完成: {}", exe.display()));
    Ok(())
}

fn uninstall(dir: &Path, silent: bool) -> Result<(), String> {
    log(silent, &format!("卸载 {}", dir.display()));
    delete_registry();
    log(silent, "安装信息注册表已移除");

    if dir.exists() {
        std::fs::remove_dir_all(dir).map_err(|e| format!("删除安装目录失败: {}", e))?;
    }
    log(silent, "✅ 卸载完成（用户数据 ~/.2pyr 已保留）");
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let silent = args.iter().any(|a| a == "--silent");
    let uninstall = args.iter().any(|a| a == "--uninstall");
    let dir = args
        .iter()
        .position(|a| a == "--dir")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(default_install_dir);

    let result = if uninstall {
        uninstall(&dir, silent)
    } else {
        install(&dir, silent)
    };

    if let Err(e) = result {
        if !silent {
            eprintln!("[2pyr-installer] ❌ {}", e);
        }
        std::process::exit(1);
    }
}
