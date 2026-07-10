//! 共享资源路径解析器
//!
//! 为 overlay、UImage 等打包资源提供统一的多策略路径定位能力。
//! Tauri v2 资源 API 解析 → 全局缓存 → 文件系统搜索 → 用户文档回退。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

// ── 全局缓存 ──────────────────────────────────────────────────────────

/// 资源路径缓存条目：名称 → 解析后的绝对路径
static RESOURCE_PATH_CACHE: RwLock<Option<Vec<(String, PathBuf)>>> = RwLock::new(None);

// ── 公共 API ──────────────────────────────────────────────────────────

/// 从 Tauri 资源 API 解析指定名称的资源目录并写入全局缓存。
/// 在 `lib.rs::setup` 中调用一次即可。
pub fn cache_resource_from_app(app: &tauri::AppHandle, name: &str) {
    match find_resource_in_app(app, name, None) {
        Ok(p) => {
            crate::log_info!("resource '{}' cached from app: {}", name, p.display());
            if let Ok(mut cache) = RESOURCE_PATH_CACHE.write() {
                let mut entries = cache.take().unwrap_or_default();
                entries.retain(|(n, _)| n != name);
                entries.push((name.to_string(), p));
                *cache = Some(entries);
            }
        }
        Err(e) => {
            crate::log_info!("failed to cache resource '{}' from app: {}", name, e);
        }
    }
}

/// 按名称查找已缓存的资源路径
pub fn get_cached_resource(name: &str) -> Option<PathBuf> {
    if let Ok(cache) = RESOURCE_PATH_CACHE.read() {
        if let Some(ref entries) = *cache {
            for (n, p) in entries {
                if n == name && p.exists() {
                    return Some(p.clone());
                }
            }
        }
    }
    None
}

/// 多策略解析资源目录路径（不缓存，仅查找）。
///
/// `validate` 闭包用于校验候选目录确实是目标资源。
/// 对 overlay 而言需要包含 `lang/zh_cn.json`；对 UImage 而言目录存在即可。
pub fn resolve_resource_dir(
    name: &str,
    validate: impl Fn(&Path) -> bool,
) -> Result<PathBuf, String> {
    // 1. 优先从缓存获取
    if let Some(cached) = get_cached_resource(name) {
        if validate(&cached) {
            crate::log_info!("resource '{}' found in cache: {}", name, cached.display());
            return Ok(cached);
        }
    }

    // 2. 文件系统搜索
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = exe_path.parent().ok_or("无法获取可执行文件目录")?;
    let cwd = std::env::current_dir().ok();

    let mut search_bases: Vec<PathBuf> = vec![exe_dir.to_path_buf()];
    if let Some(ref cwd) = cwd {
        search_bases.push(cwd.clone());
    }
    // 向上查找项目根（开发环境）
    if let Some(cwd) = &cwd {
        let mut dir = cwd.clone();
        for _ in 0..10 {
            if dir.join("Cargo.toml").exists() {
                search_bases.push(dir.clone());
                break;
            }
            if !dir.pop() {
                break;
            }
        }
    }

    for base in &search_bases {
        // 候选路径列表（按优先级）
        let candidates: Vec<PathBuf> = vec![
            base.join(name),                               // exe 同级
            base.join("resources").join(name),             // resources/<name>/
            base.join("_up_").join(name),                  // Tauri _up_/<name>/
            base.join("_up_"),                             // _up_/ 本身（当其内容直接是资源时）
            base.join(name).join(name),                    // <name>/<name>/（嵌套情况，如 overlay/overlay/）
        ];

        for c in &candidates {
            crate::log_info!("checking resource '{}': {}", name, c.display());
            if c.exists() && validate(c) {
                crate::log_info!("resource '{}' found at: {}", name, c.display());
                return Ok(c.clone());
            }
        }
    }

    // 3. 用户文档目录回退
    if let Ok(user_dir) = crate::overlay::user_data_root_dir() {
        let fallback = user_dir.join(name);
        if fallback.exists() && validate(&fallback) {
            crate::log_info!("resource '{}' found in user data: {}", name, fallback.display());
            return Ok(fallback);
        }
    }

    Err(format!("在已知位置中找不到资源目录: {}", name))
}

// ── Tauri 资源 API 查找 ───────────────────────────────────────────────

/// 在 Tauri 资源目录中查找指定名称的资源根目录。
/// `validate_file` 为可选的具体文件校验（如 "lang/zh_cn.json"），
/// 传 None 则仅校验目录存在且名称匹配。
fn find_resource_in_app(
    app: &tauri::AppHandle,
    name: &str,
    validate_file: Option<&str>,
) -> Result<PathBuf, String> {
    use tauri::Manager as _;

    // 策略1: 直接 resolve 子目录
    if let Ok(subdir) = app.path().resolve(name, tauri::path::BaseDirectory::Resource) {
        if subdir.exists() && (validate_file.map_or(true, |f| subdir.join(f).exists())) {
            crate::log_info!("find_resource '{}': direct resolve -> {}", name, subdir.display());
            return Ok(subdir);
        }
    }

    // 策略2: 资源根目录 + 名称
    if let Ok(resource_root) = app.path().resolve("", tauri::path::BaseDirectory::Resource) {
        crate::log_info!("find_resource '{}': resource_root={}", name, resource_root.display());
        let candidate = resource_root.join(name);
        if candidate.exists() && (validate_file.map_or(true, |f| candidate.join(f).exists())) {
            crate::log_info!("find_resource '{}': root/name -> {}", name, candidate.display());
            return Ok(candidate);
        }
        // 根目录本身可能直接就是资源内容
        if validate_file.map_or(false, |f| resource_root.join(f).exists()) {
            crate::log_info!("find_resource '{}': root itself matches", name);
            return Ok(resource_root);
        }
    }

    // 策略3: 通过具体文件反推父目录
    if let Some(vf) = validate_file {
        for prefix in &[format!("{}/{}", name, vf), vf.to_string()] {
            if let Ok(file_path) = app.path().resolve(&prefix, tauri::path::BaseDirectory::Resource) {
                if file_path.exists() {
                    if let Some(parent) = file_path.parent().and_then(|p| {
                        if prefix.starts_with(name) {
                            Some(p.parent()?.to_path_buf())
                        } else {
                            Some(p.to_path_buf())
                        }
                    }) {
                        crate::log_info!("find_resource '{}': via file '{}' -> {}", name, prefix, parent.display());
                        return Ok(parent);
                    }
                }
            }
        }
    }

    // 策略4: _up_ 目录
    if let Ok(up_dir) = app.path().resolve("_up_", tauri::path::BaseDirectory::Resource) {
        crate::log_info!("find_resource '{}': _up_={}", name, up_dir.display());
        for sub in &[name.to_string(), String::new()] {
            let candidate = if sub.is_empty() { up_dir.clone() } else { up_dir.join(sub) };
            if candidate.exists() && (validate_file.map_or(true, |f| candidate.join(f).exists())) {
                crate::log_info!("find_resource '{}': via _up_ -> {}", name, candidate.display());
                return Ok(candidate);
            }
        }
    }

    Err(format!("Tauri resource API 中找不到资源: {}", name))
}

// ── 便捷构造 ──────────────────────────────────────────────────────────

/// 为 overlay 资源构造校验闭包
pub fn overlay_validator() -> impl Fn(&Path) -> bool {
    |p: &Path| p.join("lang").join("zh_cn.json").exists()
}

/// 为 UImage 资源构造校验闭包
pub fn uimage_validator() -> impl Fn(&Path) -> bool {
    |p: &Path| p.is_dir()
}

// ── 向后兼容：确保资源在 exe 同级 ────────────────────────────────────

/// 确保 overlay 和 UImage 资源在 exe 同级目录存在。
/// 用于向后兼容：如果旧版 MSI 将资源放到了 _up_/ 下，则复制到 exe 同级。
pub fn ensure_resources_at_exe_level() -> Result<(), String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = exe_path.parent().ok_or("无法获取可执行文件目录")?;

    for name in &["overlay", "UImage"] {
        let at_exe = exe_dir.join(name);
        let in_up = exe_dir.join("_up_").join(name);
        if !at_exe.exists() && in_up.exists() {
            crate::log_info!("向后兼容: 从 _up_/ 复制 {}/ 到 exe 同级", name);
            copy_dir_all(&in_up, &at_exe)
                .map_err(|e| format!("复制 {} 失败: {}", name, e))?;
        }
    }

    Ok(())
}

// ── 工具函数 ──────────────────────────────────────────────────────────

/// 递归复制目录
pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
