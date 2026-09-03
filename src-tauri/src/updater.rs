use std::fs;
use std::io::Write;
use std::process::Command;

use reqwest::header::{ACCEPT, USER_AGENT};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::commands::{read_config_file, write_config_file};

// ── Data structures ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetInfo {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
    pub content_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub version: String,
    pub name: String,
    pub body: String,
    pub published_at: String,
    pub html_url: String,
    pub assets: Vec<AssetInfo>,
    pub priority: UpdatePriority,
    pub is_prerelease: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum UpdatePriority {
    #[serde(rename = "safe")]
    Safe,
    #[serde(rename = "optional")]
    Optional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub has_update: bool,
    pub current_version: String,
    pub latest: Option<ReleaseInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: String,
    body: String,
    prerelease: bool,
    published_at: String,
    html_url: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    content_type: String,
}

// ── Tag parsing ──────────────────────────────────────────────

/// Tags: `Safe-2.0.2` (force-install), `Stable-2.0.2` (stable),
/// `UnStable-2.0.2` / `Beta-2.0.2` (test/beta), plain `v2.0.1`.
fn parse_tag(tag: &str) -> (String, UpdatePriority) {
    let tag = tag.trim();
    if let Some(rest) = tag.strip_prefix("Safe-") {
        (rest.to_string(), UpdatePriority::Safe)
    } else if let Some(rest) = tag.strip_prefix("Stable-")
        .or_else(|| tag.strip_prefix("UnStable-"))
        .or_else(|| tag.strip_prefix("Beta-"))
    {
        (rest.to_string(), UpdatePriority::Optional)
    } else {
        (strip_v(tag), UpdatePriority::Optional)
    }
}

/// 测试版 tag：`UnStable-*` / `Beta-*`（不区分大小写）。
/// 只进入「测试版」更新通道；稳定通道忽略。
fn is_test_tag(tag: &str) -> bool {
    let lower = tag.trim().to_ascii_lowercase();
    lower.starts_with("unstable-") || lower.starts_with("beta-")
}

fn strip_v(s: &str) -> String {
    s.strip_prefix('v').unwrap_or(s).to_string()
}

// ── Version comparison ──────────────────────────────────────

fn version_greater(a: &str, b: &str) -> bool {
    let pa: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
    let pb: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();
    let len = pa.len().max(pb.len());
    for i in 0..len {
        let va = pa.get(i).copied().unwrap_or(0);
        let vb = pb.get(i).copied().unwrap_or(0);
        if va > vb { return true; }
        if va < vb { return false; }
    }
    false
}

// ── GitHub API (async) ──────────────────────────────────────

// 更新源：2-Pyramid 官方仓库的 releases（对应网页
// https://github.com/realLivedInCorner/2-Pyramid/releases）
const GITHUB_API: &str = "https://api.github.com/repos/realLivedInCorner/2-Pyramid/releases";

async fn fetch_releases() -> Result<Vec<GitHubRelease>, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(GITHUB_API)
        .query(&[("per_page", "30")])
        .header(USER_AGENT, "2-Pyramid-Updater/2.0")
        .header(ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("Network request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API returned error: {}", resp.status()));
    }

    resp.json::<Vec<GitHubRelease>>()
        .await
        .map_err(|e| format!("Failed to parse GitHub response: {}", e))
}

async fn check_github_releases(channel: &str, current_version: &str) -> Result<UpdateCheckResult, String> {
    let releases = fetch_releases().await?;

    let mut parsed: Vec<ReleaseInfo> = releases
        .iter()
        .filter_map(|r| {
            let (version, priority) = parse_tag(&r.tag_name);
            // 通道语义：
            //   master   —— 仅稳定版（排除 Beta-/UnStable- 与 prerelease）
            //   unstable —— 仅测试版（Beta-/UnStable-/prerelease）
            //   both     —— 同时接受两个通道的更新内容（全部，取最高版本）
            let include = match channel {
                "both" => true,
                "unstable" => r.prerelease || is_test_tag(&r.tag_name),
                _ => !r.prerelease && !is_test_tag(&r.tag_name),
            };
            if !include { return None; }
            Some(ReleaseInfo {
                tag_name: r.tag_name.clone(),
                version,
                name: r.name.clone(),
                body: r.body.clone(),
                published_at: r.published_at.clone(),
                html_url: r.html_url.clone(),
                assets: r.assets.iter().map(|a| AssetInfo {
                    name: a.name.clone(),
                    browser_download_url: a.browser_download_url.clone(),
                    size: a.size,
                    content_type: a.content_type.clone(),
                }).collect(),
                priority,
                is_prerelease: r.prerelease,
            })
        })
        .collect();

    parsed.sort_by(|a, b| {
        if version_greater(&a.version, &b.version) {
            std::cmp::Ordering::Less
        } else if version_greater(&b.version, &a.version) {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });

    let latest = parsed.into_iter().next();
    let has_update = latest
        .as_ref()
        .map(|r| version_greater(&r.version, current_version))
        .unwrap_or(false);

    Ok(UpdateCheckResult {
        has_update,
        current_version: current_version.to_string(),
        latest: if has_update { latest } else { None },
    })
}

// ── Download (async streaming) ──────────────────────────────

fn find_windows_asset(release: &ReleaseInfo) -> Result<&AssetInfo, String> {
    for ext in &[".msi", ".exe"] {
        for asset in &release.assets {
            if asset.name.ends_with(ext) {
                return Ok(asset);
            }
        }
    }
    release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".msi") || a.name.ends_with(".exe"))
        .ok_or_else(|| "No Windows installer found".to_string())
}

async fn download_installer(app: &AppHandle, release: &ReleaseInfo) -> Result<String, String> {
    let asset = find_windows_asset(release)?;

    let temp_dir = std::env::temp_dir().join("2_pyramid_update");
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;

    let file_path = temp_dir.join(&asset.name);
    let file_path_str = file_path.to_string_lossy().to_string();

    crate::log_info!("downloading {} ({} bytes)", asset.browser_download_url, asset.size);

    let client = reqwest::Client::new();
    let mut resp = client
        .get(&asset.browser_download_url)
        .header(USER_AGENT, "2-Pyramid-Updater/2.0")
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    let total = resp.content_length().unwrap_or(asset.size);
    let mut downloaded: u64 = 0;
    let mut last_emit: u64 = 0;
    let mut file = fs::File::create(&file_path)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    while let Some(chunk) = resp.chunk().await
        .map_err(|e| format!("Download read failed: {}", e))?
    {
        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write file: {}", e))?;
        downloaded += chunk.len() as u64;

        // Emit progress every ~512KB — avoids flooding the JS event loop
        if downloaded.saturating_sub(last_emit) >= 512 * 1024 || downloaded >= total {
            let _ = app.emit(
                "update-download-progress",
                DownloadProgress { downloaded, total },
            );
            last_emit = downloaded;
        }
    }

    file.flush().map_err(|e| format!("Failed to flush file: {}", e))?;

    // Final progress event
    let _ = app.emit(
        "update-download-progress",
        DownloadProgress { downloaded: total, total },
    );

    crate::log_info!("downloaded installer to {}", file_path_str);
    Ok(file_path_str)
}

// ── Install ──────────────────────────────────────────────────

fn update_marker_path() -> Result<std::path::PathBuf, String> {
    let base = dirs::config_dir()
        .ok_or_else(|| "Failed to get config directory".to_string())?;
    Ok(base
        .join("2-Pyramid")
        .join("KyanitePackTool")
        .join(".update_done"))
}

fn write_update_marker(new_version: &str) -> Result<(), String> {
    let path = update_marker_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    fs::write(&path, new_version)
        .map_err(|e| format!("Failed to write update marker: {}", e))?;
    Ok(())
}

fn launch_installer(path: &str) -> Result<(), String> {
    crate::log_info!("launching installer: {}", path);

    if path.ends_with(".msi") {
        Command::new("msiexec")
            .args(["/i", path])
            .spawn()
            .map_err(|e| format!("Failed to launch MSI installer: {}", e))?;
    } else {
        // 自制释放式安装器（2pyr-installer）：以图形向导方式拉起，
        // 用户能看到并确认安装流程。
        //
        // 注意：不要用 --silent 静默安装 —— 更新场景下旧程序刚退出、
        // 安装目录里的 exe 可能仍被锁着，静默解压会无声失败；
        // 图形向导等用户点「安装」时旧进程早已退出，不存在锁竞争，
        // 用户也能看到进度反馈。
        Command::new(path)
            .spawn()
            .map_err(|e| format!("Failed to launch installer: {}", e))?;
    }

    Ok(())
}

// ── Current version ──────────────────────────────────────────

fn current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ── Tauri commands ───────────────────────────────────────────

#[tauri::command]
pub async fn check_for_update(app: AppHandle, channel: Option<String>) -> Result<UpdateCheckResult, String> {
    let channel = channel.unwrap_or_else(|| {
        read_config_file()
            .ok()
            .and_then(|c| c.update_channel)
            .unwrap_or_else(|| "master".to_string())
    });
    crate::log_info!("checking for update, channel={}", channel);
    check_github_releases(&channel, &current_version()).await
}

#[tauri::command]
pub async fn download_update(app: AppHandle, tag_name: String) -> Result<String, String> {
    crate::log_info!("download_update requested for tag={}", tag_name);
    let releases = fetch_releases().await?;

    let (version_str, _priority) = parse_tag(&tag_name);
    let release = releases
        .iter()
        .find(|r| {
            let (v, _) = parse_tag(&r.tag_name);
            v == version_str
        })
        .ok_or_else(|| format!("No release found for tag {}", tag_name))?;

    let ri = ReleaseInfo {
        tag_name: release.tag_name.clone(),
        version: version_str,
        name: release.name.clone(),
        body: release.body.clone(),
        published_at: release.published_at.clone(),
        html_url: release.html_url.clone(),
        assets: release
            .assets
            .iter()
            .map(|a| AssetInfo {
                name: a.name.clone(),
                browser_download_url: a.browser_download_url.clone(),
                size: a.size,
                content_type: a.content_type.clone(),
            })
            .collect(),
        priority: _priority,
        is_prerelease: release.prerelease,
    };

    download_installer(&app, &ri).await
}

#[tauri::command]
pub fn install_update(app: AppHandle, installer_path: String, new_version: String) -> Result<(), String> {
    let _ = write_update_marker(&new_version);
    launch_installer(&installer_path)?;
    // 给安装向导一点启动时间再退出旧程序（子进程独立运行，不随父进程消失）
    std::thread::sleep(std::time::Duration::from_millis(1500));
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn get_update_channel() -> Result<String, String> {
    let cfg = read_config_file()?;
    Ok(cfg.update_channel.unwrap_or_else(|| "master".to_string()))
}

#[tauri::command]
pub fn set_update_channel(channel: String) -> Result<(), String> {
    // master = 仅稳定；unstable = 仅测试；both = 同时接受两个通道
    if channel != "master" && channel != "unstable" && channel != "both" {
        return Err(format!("Invalid update channel: {}", channel));
    }
    let mut cfg = read_config_file()?;
    cfg.update_channel = Some(channel);
    write_config_file(&cfg)?;
    Ok(())
}

#[tauri::command]
pub fn check_update_marker() -> Result<Option<String>, String> {
    let path = update_marker_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let version = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read update marker: {}", e))?;
    let _ = fs::remove_file(&path);
    Ok(Some(version.trim().to_string()))
}
