//! 2-Pyramid 独立安装器（Tauri 2 + Vue 3，Windows 专用）。
//!
//! 内嵌 payload.zip（构建流水线生成，含主程序便携版全部文件）。
//! 图形界面提供 OOBE 式分步安装流程（欢迎/GitHub 介绍 → 安装地址 →
//! 实时进度 → 完成），并注册控制面板卸载入口。
//!
//! Rust 侧命令：
//!   * install(dir, shortcuts) —— 释放 payload（emit 实时进度）、
//!     释放 uninstaller.exe、按需创建桌面/开始菜单/任务栏快捷方式、
//!     写安装信息与卸载注册表
//!   * uninstall(dir) —— 删除文件与全部注册表（用户数据 ~/.2pyr 保留）；
//!     卸载器自身在安装目录内时派出独立清理进程，于退出后删除自身
//!   * launch_app / get_default_dir / get_version / get_channel / is_installed
//!   * is_uninstall_mode —— 以 --uninstall 启动时前端展示卸载流程
//!
//! 静默模式：`installer.exe --silent [--dir <path>]` 直接安装后退出
//! （自动更新器使用），不启动图形界面。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::Cursor;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

const PAYLOAD: &[u8] = include_bytes!("../payload.zip");
const APP_NAME: &str = "2-Pyramid";
const EXE_NAME: &str = "2-pyramid.exe";
const UNINSTALLER_NAME: &str = "uninstall.exe";
const GITHUB_URL: &str = "https://github.com/realLivedInCorner/2-Pyramid";

// 构建渠道：stable（正式版）/ beta（测试版）。
// 发布流水线在编译安装器时通过环境变量 2PYR_CHANNEL 注入。
const CHANNEL: &str = match option_env!("2PYR_CHANNEL") {
    Some(v) => v,
    None => "stable",
};

fn is_beta() -> bool {
    CHANNEL == "beta"
}

/// 安装信息注册表键（beta 与正式版隔离，可并存）。
fn app_reg_path() -> &'static str {
    if is_beta() { r"Software\2-Pyramid-Beta" } else { r"Software\2-Pyramid" }
}

/// 控制面板「卸载程序」注册表键。
fn uninstall_reg_path() -> &'static str {
    if is_beta() {
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall\2-Pyramid Beta"
    } else {
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall\2-Pyramid"
    }
}

fn display_name() -> &'static str {
    if is_beta() { "2-Pyramid Beta" } else { "2-Pyramid" }
}

/// 快捷方式文件名（含扩展名）。
fn shortcut_file_name() -> &'static str {
    if is_beta() { "2-Pyramid Beta.lnk" } else { "2-Pyramid.lnk" }
}

// ── 进度事件载荷 ─────────────────────────────────────────────────

#[derive(Serialize, Clone)]
struct InstallProgress {
    current: usize,
    total: usize,
    name: String,
}

// ── 快捷方式选项 ─────────────────────────────────────────────────

/// 安装界面三个快捷方式勾选项：桌面 / 开始菜单 / 任务栏（快捷栏）。
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShortcutOptions {
    desktop: bool,
    start_menu: bool,
    taskbar: bool,
}

impl ShortcutOptions {
    /// 静默更新模式：不创建任何快捷方式（避免覆盖用户已删除的入口）。
    fn none() -> Self {
        Self { desktop: false, start_menu: false, taskbar: false }
    }
}

// ── 核心逻辑（silent 模式与 GUI 命令共用） ────────────────────────

fn default_install_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    if is_beta() {
        base.join("2-Pyramid-Beta")
    } else {
        base.join(APP_NAME)
    }
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
    // 带 --uninstall 参数：卸载器双击 / 控制面板卸载时直接进入卸载流程
    let uninstall_cmd = format!("\"{}\" --uninstall", uninstaller);
    // 控制面板显示版本：正式版保持原样，beta 附加渠道标记
    let version: &str = env!("CARGO_PKG_VERSION");
    let display_version = if is_beta() {
        format!("{} (beta)", version)
    } else {
        version.to_string()
    };

    // 安装信息
    let (key, _) = hkcu
        .create_subkey(app_reg_path())
        .map_err(|e| format!("写注册表失败: {}", e))?;
    key.set_value("InstallDir", &dir_str).map_err(|e| format!("写注册表失败: {}", e))?;
    key.set_value("Version", &version).map_err(|e| format!("写注册表失败: {}", e))?;
    key.set_value("Channel", &CHANNEL).map_err(|e| format!("写注册表失败: {}", e))?;
    key.set_value("UninstallString", &uninstall_cmd).map_err(|e| format!("写注册表失败: {}", e))?;

    // 控制面板「卸载程序」入口
    let (ukey, _) = hkcu
        .create_subkey(uninstall_reg_path())
        .map_err(|e| format!("写卸载注册表失败: {}", e))?;
    ukey.set_value("DisplayName", &display_name()).map_err(|e| format!("写卸载注册表失败: {}", e))?;
    ukey.set_value("DisplayVersion", &display_version).map_err(|e| format!("写卸载注册表失败: {}", e))?;
    ukey.set_value("Publisher", &"2-Pyramid Studio").map_err(|e| format!("写卸载注册表失败: {}", e))?;
    ukey.set_value("UninstallString", &uninstall_cmd).map_err(|e| format!("写卸载注册表失败: {}", e))?;
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
    let _ = hkcu.delete_subkey_all(app_reg_path());
    let _ = hkcu.delete_subkey_all(uninstall_reg_path());
}

fn registry_install_dir() -> Option<PathBuf> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey(app_reg_path()).ok()?;
    let dir: String = key.get_value("InstallDir").ok()?;
    Some(PathBuf::from(dir))
}

// ── 快捷方式创建（PowerShell WScript.Shell / Shell.Application） ─

/// UTF-16LE 编码（PowerShell -EncodedCommand 要求）。
fn utf16le(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect()
}

/// 标准 Base64（不引入额外 crate，离线构建友好）。
fn b64_encode(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

/// 运行一段 PowerShell 脚本（无窗口、无交互，失败静默）。
fn run_powershell(script: &str) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let encoded = b64_encode(&utf16le(script));
    let _ = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-EncodedCommand",
            &encoded,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .status();
}

/// 按选项创建快捷方式。返回描述性摘要（用于安装完成文案）。
fn create_shortcuts(dir: &Path, opts: &ShortcutOptions) -> String {
    let mut created: Vec<&str> = Vec::new();
    let exe = dir.join(EXE_NAME);

    // 桌面 / 开始菜单：WScript.Shell 生成 .lnk，稳定可靠
    if opts.desktop || opts.start_menu {
        let mut script = String::from(
            "$ErrorActionPreference='SilentlyContinue'\n$ws=New-Object -ComObject WScript.Shell\n",
        );
        let exe_s = ps_quote(&exe.to_string_lossy());
        let dir_s = ps_quote(&dir.to_string_lossy());
        let lnk_s = ps_quote(shortcut_file_name());
        if opts.desktop {
            script.push_str(&format!(
                "$p=Join-Path ([Environment]::GetFolderPath('Desktop')) {lnk}\n\
                 $s=$ws.CreateShortcut($p)\n$s.TargetPath={exe}\n$s.WorkingDirectory={work}\n\
                 $s.Description='2-Pyramid'\n$s.Save()\n",
                lnk = lnk_s,
                exe = exe_s,
                work = dir_s,
            ));
            created.push("桌面");
        }
        if opts.start_menu {
            script.push_str(&format!(
                "$p=Join-Path ([Environment]::GetFolderPath('Programs')) {lnk}\n\
                 $s=$ws.CreateShortcut($p)\n$s.TargetPath={exe}\n$s.WorkingDirectory={work}\n\
                 $s.Description='2-Pyramid'\n$s.Save()\n",
                lnk = lnk_s,
                exe = exe_s,
                work = dir_s,
            ));
            created.push("开始菜单");
        }
        run_powershell(&script);
    }

    // 任务栏固定：通过 Shell.Application InvokeVerb('taskbarpin') 尽力而为。
    // Windows 10/11 对程序化固定限制较多，失败时静默跳过。
    if opts.taskbar {
        let script = format!(
            "$ErrorActionPreference='SilentlyContinue'\n\
             $shell=New-Object -ComObject Shell.Application\n\
             $folder=$shell.Namespace({})\n\
             if($folder -ne $null){{$item=$folder.ParseName('{}');if($item -ne $null){{$item.InvokeVerb('taskbarpin')}}}}",
            ps_quote(&dir.to_string_lossy()),
            EXE_NAME,
        );
        run_powershell(&script);
        created.push("任务栏(尽力而为)");
    }

    if created.is_empty() {
        "未创建".to_string()
    } else {
        created.join("、")
    }
}

/// 卸载时清理快捷方式（.lnk 直接删除；任务栏固定尽力解除）。
fn remove_shortcuts(dir: &Path) {
    if let Some(desktop) = dirs::desktop_dir() {
        let _ = std::fs::remove_file(desktop.join(shortcut_file_name()));
    }
    if let Some(data_dir) = dirs::data_dir() {
        let _ = std::fs::remove_file(
            data_dir
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
                .join(shortcut_file_name()),
        );
    }
    if dir.join(EXE_NAME).exists() {
        let script = format!(
            "$ErrorActionPreference='SilentlyContinue'\n\
             $shell=New-Object -ComObject Shell.Application\n\
             $folder=$shell.Namespace({})\n\
             if($folder -ne $null){{$item=$folder.ParseName('{}');if($item -ne $null){{$item.InvokeVerb('taskbarunpin')}}}}",
            ps_quote(&dir.to_string_lossy()),
            EXE_NAME,
        );
        run_powershell(&script);
    }
}

/// PowerShell 单引号字符串字面量转义。
fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

fn install_impl<F>(dir: &Path, shortcuts: ShortcutOptions, mut on_progress: F) -> Result<String, String>
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
    let shortcut_note = create_shortcuts(dir, &shortcuts);
    Ok(format!(
        "安装完成，共释放 {} 个文件到 {}（快捷方式：{}）",
        count,
        dir.display(),
        shortcut_note
    ))
}

/// 无窗口、不等待地启动一段 PowerShell 脚本（DETACHED，
/// 父进程退出后脚本继续运行）。
fn run_powershell_detached(script: &str) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    let encoded = b64_encode(&utf16le(script));
    let _ = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-EncodedCommand",
            &encoded,
        ])
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
        .spawn();
}

/// 派生独立的清理进程：用进程句柄 WaitForExit 阻塞等待本卸载器
/// 退出（事件驱动，不做任何轮询，也不会产生 ping 之类的网络
/// 探测），随后删除卸载器自身与安装目录。
fn spawn_self_delete_helper(self_path: &Path, dir: &Path) {
    let pid = std::process::id();
    let self_s = ps_quote(&self_path.to_string_lossy());
    let dir_s = ps_quote(&dir.to_string_lossy());
    let script = format!(
        "$ErrorActionPreference='SilentlyContinue'\n\
         $p=Get-Process -Id {pid} -ErrorAction SilentlyContinue\n\
         if($p){{$p.WaitForExit()}}\n\
         Start-Sleep -Milliseconds 400\n\
         Remove-Item -LiteralPath {self} -Force\n\
         Start-Sleep -Milliseconds 300\n\
         Remove-Item -LiteralPath {self} -Force\n\
         Remove-Item -LiteralPath {dir} -Recurse -Force",
        pid = pid,
        self = self_s,
        dir = dir_s,
    );
    run_powershell_detached(&script);
}

fn uninstall_impl(dir: &Path) -> Result<String, String> {
    delete_registry();
    remove_shortcuts(dir);

    let self_path = std::env::current_exe().ok();
    // Windows 路径不区分大小写；自身位于安装目录内则走“退出时清理自身”分支
    let self_inside = self_path
        .as_ref()
        .and_then(|p| p.parent())
        .map(|d| d.to_string_lossy().eq_ignore_ascii_case(&dir.to_string_lossy()))
        .unwrap_or(false);

    // 逐项删除安装目录内容：跳过自身（正在运行的 exe 被系统锁定），
    // 其余文件/目录尽力删除，锁定失败不中断卸载流程。
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if self_path.as_ref() == Some(&path) {
                continue;
            }
            if path.is_dir() {
                let _ = std::fs::remove_dir_all(&path);
            } else {
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    if self_inside {
        // 自身在安装目录内：派出清理进程，等待本进程退出后删除自身与目录
        if let Some(sp) = &self_path {
            spawn_self_delete_helper(sp, dir);
        }
        Ok("卸载完成：程序文件已移除，用户数据已保留。\n窗口即将自动关闭，卸载程序随后自行清理残留。".to_string())
    } else {
        // 自身不在安装目录内（如 --uninstall 启动的原始安装包）：直接删除目录
        if dir.exists() {
            std::fs::remove_dir_all(dir).map_err(|e| format!("删除安装目录失败: {}", e))?;
        }
        Ok("卸载完成（用户数据 ~/.2pyr 已保留）".to_string())
    }
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

/// 构建渠道：stable（正式版）/ beta（测试版）。
#[tauri::command]
fn get_channel() -> String {
    CHANNEL.to_string()
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

/// 以 uninstall.exe 文件名运行（安装时释放的自身副本）时视为卸载器。
fn is_uninstaller_binary() -> bool {
    std::env::current_exe()
        .map(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy() == UNINSTALLER_NAME)
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

#[tauri::command]
fn is_uninstall_mode() -> bool {
    std::env::args().any(|a| a == "--uninstall") || is_uninstaller_binary()
}

#[tauri::command]
fn install(app: AppHandle, dir: String, shortcuts: ShortcutOptions) -> Result<String, String> {
    let path = PathBuf::from(dir);
    install_impl(&path, shortcuts, |current, total, name| {
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
        match install_impl(&dir, ShortcutOptions::none(), |_, _, _| {}) {
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
            get_channel,
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
