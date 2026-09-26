use std::fs;
use std::io::Write;
#[cfg(not(feature = "store"))]
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
    /// major / minor / patch / none —— 前端用来标注「值得更新」的程度
    pub bump_kind: String,
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

// ── Version comparison (major.minor.patch) ─────────────────

/// 语义化版本核心三元组。预发布后缀（`-beta.1`）与构建元数据（`+meta`）
/// 不参与比较主次序——渠道由 tag 前缀/ prerelease 标记决定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SemVer {
    major: u32,
    minor: u32,
    patch: u32,
}

fn parse_semver(s: &str) -> Option<SemVer> {
    let s = s.trim();
    let s = s.strip_prefix('v').or_else(|| s.strip_prefix('V')).unwrap_or(s);
    let core = s.split(['-', '+']).next().unwrap_or(s).trim();
    if core.is_empty() {
        return None;
    }
    let mut parts = core.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let patch: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    Some(SemVer { major, minor, patch })
}

/// 更新级别：按 major / minor / patch 首个升高位判定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VersionBump {
    None,
    Patch,
    Minor,
    Major,
}

impl VersionBump {
    fn as_str(self) -> &'static str {
        match self {
            VersionBump::None => "none",
            VersionBump::Patch => "patch",
            VersionBump::Minor => "minor",
            VersionBump::Major => "major",
        }
    }
}

/// 比较 `a` 是否严格大于 `b`（按 major.minor.patch）。
/// 解析失败时回退到宽松逐段数字比较，避免脏 tag 直接卡死更新。
fn version_greater(a: &str, b: &str) -> bool {
    match (parse_semver(a), parse_semver(b)) {
        (Some(pa), Some(pb)) => pa > pb,
        _ => {
            let pa: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
            let pb: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();
            let len = pa.len().max(pb.len());
            for i in 0..len {
                let va = pa.get(i).copied().unwrap_or(0);
                let vb = pb.get(i).copied().unwrap_or(0);
                if va > vb {
                    return true;
                }
                if va < vb {
                    return false;
                }
            }
            false
        }
    }
}

/// 判断 `latest` 相对 `current` 的更新级别。
/// 无法解析时：若 `latest > current` 则按 Minor 保守估计。
fn version_bump(current: &str, latest: &str) -> VersionBump {
    match (parse_semver(current), parse_semver(latest)) {
        (Some(c), Some(l)) => {
            if l.major > c.major {
                VersionBump::Major
            } else if l.major < c.major {
                VersionBump::None
            } else if l.minor > c.minor {
                VersionBump::Minor
            } else if l.minor < c.minor {
                VersionBump::None
            } else if l.patch > c.patch {
                VersionBump::Patch
            } else {
                VersionBump::None
            }
        }
        _ => {
            if version_greater(latest, current) {
                VersionBump::Minor
            } else {
                VersionBump::None
            }
        }
    }
}

// ── GitHub API (async) ──────────────────────────────────────

// 可用的更新源：
//   * mirror（默认）— 国内镜像，由 cdn.5eggpack.top 提供，schema 与 GitHub Releases
//     API 一致，便于国内用户快速检测与下载更新。
//   * github — 官方 GitHub Releases 源。
const MIRROR_API: &str = "https://cdn.5eggpack.top/api/github/releases";
const GITHUB_API: &str = "https://api.github.com/repos/realLivedInCorner/2-Pyramid/releases";

/// 读取当前更新源配置（"github" 或默认 "mirror"）。
fn effective_update_source() -> String {
    read_config_file()
        .ok()
        .and_then(|c| c.update_source)
        .unwrap_or_else(|| "mirror".to_string())
}

/// 根据配置返回本次要请求的 releases API 端点。
fn releases_api_endpoint() -> &'static str {
    match effective_update_source().as_str() {
        "github" => GITHUB_API,
        _ => MIRROR_API,
    }
}

async fn fetch_releases() -> Result<Vec<GitHubRelease>, String> {
    let endpoint = releases_api_endpoint();
    let client = reqwest::Client::new();
    let resp = client
        .get(endpoint)
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

/// 最终优先级：
/// * `Safe-*` tag —— 强制更新（安全/紧急修复）
/// * major 升位 —— 强制更新（大版本不兼容风险，默认要求升级）
/// * minor / patch —— 可选更新，用户自愿
fn effective_priority(tag_priority: UpdatePriority, bump: VersionBump) -> UpdatePriority {
    match tag_priority {
        UpdatePriority::Safe => UpdatePriority::Safe,
        _ if bump == VersionBump::Major => UpdatePriority::Safe,
        _ => UpdatePriority::Optional,
    }
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

    let mut latest = parsed.into_iter().next();
    let bump = latest
        .as_ref()
        .map(|r| version_bump(current_version, &r.version))
        .unwrap_or(VersionBump::None);
    // 仅 major.minor.patch 任一升高才视为有更新
    let has_update = bump != VersionBump::None;

    // 按 bump 覆盖 tag 优先级：Safe 永远强制；major 也强制；其余可选
    if let Some(ref mut r) = latest {
        r.priority = effective_priority(r.priority.clone(), bump);
    }

    Ok(UpdateCheckResult {
        has_update,
        current_version: current_version.to_string(),
        bump_kind: bump.as_str().to_string(),
        latest: if has_update { latest } else { None },
    })
}

// ── Download (async streaming) ──────────────────────────────

/// 下载域名白名单：安装包与哈希文件只允许从这些主机下载。
/// 镜像/API 返回的 browser_download_url 一律先过这里，防止被
/// 篡改的数据把用户引向任意域名。
fn is_allowed_download_url(url: &str) -> bool {
    match reqwest::Url::parse(url) {
        Ok(u) => u.host_str().map(|h| {
            h == "github.com"
                || h.ends_with(".github.com")
                || h == "objects.githubusercontent.com"
                || h == "cdn.5eggpack.top"
        }).unwrap_or(false),
        Err(_) => false,
    }
}

/// 安装包文件名校验：必须是我们流水线的命名（2-Pyramid-Installer-…
/// 且包含所选版本号），防止镜像/响应里塞入别的文件。
fn is_expected_installer_name(name: &str, version: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.starts_with("2-pyramid-installer-")
        && lower.contains(&version.to_ascii_lowercase())
}

fn find_windows_asset(release: &ReleaseInfo) -> Result<&AssetInfo, String> {
    // 只认符合命名规范的安装包（含版本号）
    let candidates: Vec<&AssetInfo> = release
        .assets
        .iter()
        .filter(|a| {
            (a.name.ends_with(".exe") || a.name.ends_with(".msi"))
                && is_expected_installer_name(&a.name, &release.version)
        })
        .collect();
    for ext in &[".exe", ".msi"] {
        if let Some(asset) = candidates.iter().find(|a| a.name.ends_with(ext)) {
            return Ok(asset);
        }
    }
    release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".exe") || a.name.ends_with(".msi"))
        .ok_or_else(|| "No Windows installer found".to_string())
}

/// 在 release 资产里找 `<安装包名>.sha256` 校验文件。
fn find_sha256_asset<'a>(
    release: &'a ReleaseInfo,
    installer_name: &str,
) -> Option<&'a AssetInfo> {
    let needle = format!("{}.sha256", installer_name.to_ascii_lowercase());
    release
        .assets
        .iter()
        .find(|a| a.name.to_ascii_lowercase() == needle)
}

/// 下载并解析 .sha256 文件（小文件，上限 4KB）。
async fn fetch_expected_sha256(url: &str) -> Result<String, String> {
    if !is_allowed_download_url(url) {
        return Err(format!("哈希文件域名不在白名单: {}", url));
    }
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .header(USER_AGENT, "2-Pyramid-Updater/2.0")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch checksum: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Checksum fetch returned {}", resp.status()));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read checksum: {}", e))?;
    if bytes.len() > 4096 {
        return Err("Checksum file too large".to_string());
    }
    let text = String::from_utf8_lossy(&bytes);
    let hex = text.split_whitespace().next().unwrap_or_default().trim();
    if hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(hex.to_ascii_lowercase())
    } else {
        Err("Checksum 格式不正确".to_string())
    }
}

async fn download_installer(app: &AppHandle, release: &ReleaseInfo) -> Result<String, String> {
    let asset = find_windows_asset(release)?;

    // 1. 下载域名白名单校验（镜像响应可能伪造下载地址）
    if !is_allowed_download_url(&asset.browser_download_url) {
        return Err(format!(
            "安装包域名不在白名单，已拒绝下载: {}",
            asset.browser_download_url
        ));
    }

    // 2. 预取期望的 SHA-256（若有同名 .sha256 资产）；没有则跳过校验
    let expected_sha256 = match find_sha256_asset(release, &asset.name) {
        Some(sha_asset) => match fetch_expected_sha256(&sha_asset.browser_download_url).await {
            Ok(hex) => Some(hex),
            Err(e) => {
                crate::log_warn!("sha256 资产获取失败（跳过校验）: {}", e);
                None
            }
        },
        None => {
            crate::log_warn!("release 未附带 .sha256 校验文件，跳过完整性校验");
            None
        }
    };

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
    // sha2 的 new()/update()/finalize() 都来自 Digest trait，需先引入
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();

    while let Some(chunk) = resp.chunk().await
        .map_err(|e| format!("Download read failed: {}", e))?
    {
        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write file: {}", e))?;
        // 边下载边算哈希
        hasher.update(&chunk);
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

    // 3. SHA-256 校验
    if let Some(expected) = expected_sha256 {
        let actual = format!("{:x}", hasher.finalize());
        if !actual.eq_ignore_ascii_case(&expected) {
            let _ = fs::remove_file(&file_path);
            return Err(format!(
                "安装包 SHA-256 校验失败：期望 {}，实际 {}。已删除下载文件",
                expected, actual
            ));
        }
        crate::log_info!("sha256 verified: {}", expected);
    }

    // Final progress event
    let _ = app.emit(
        "update-download-progress",
        DownloadProgress { downloaded: total, total },
    );

    crate::log_info!("downloaded installer to {}", file_path_str);
    Ok(file_path_str)
}

// ── 更新源测速 ──────────────────────────────────────────────

/// 单个更新源的测速结果（序列化给前端展示）。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSpeed {
    pub source: String,
    pub reachable: bool,
    /// 从发起到收到响应头的耗时（毫秒）
    pub latency_ms: u64,
    /// 平均下载速率（KB/s，保留 1 位小数）
    pub speed_kbps: f64,
    pub error: Option<String>,
}

/// 测一个更新源：GET releases API（per_page=1），最多读 512KB，
/// 统计响应头耗时与平均速率。超时 10 秒。
async fn measure_source(endpoint: &str, source: &str) -> SourceSpeed {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return SourceSpeed {
                source: source.to_string(),
                reachable: false,
                latency_ms: 0,
                speed_kbps: 0.0,
                error: Some(e.to_string()),
            }
        }
    };
    let started = std::time::Instant::now();
    match client
        .get(endpoint)
        .query(&[("per_page", "1")])
        .header(USER_AGENT, "2-Pyramid-Updater/2.0")
        .header(ACCEPT, "application/vnd.github+json")
        .send()
        .await
    {
        Ok(mut resp) => {
            let latency_ms = started.elapsed().as_millis() as u64;
            let ok = resp.status().is_success();
            let mut bytes: usize = 0;
            while let Some(chunk) = resp.chunk().await.ok().flatten() {
                bytes += chunk.len();
                if bytes >= 512 * 1024 {
                    break;
                }
            }
            let elapsed_s = started.elapsed().as_millis().max(1) as f64 / 1000.0;
            let speed_kbps = (bytes as f64 / 1024.0) / elapsed_s;
            SourceSpeed {
                source: source.to_string(),
                reachable: ok,
                latency_ms,
                speed_kbps: (speed_kbps * 10.0).round() / 10.0,
                error: if ok { None } else { Some(format!("HTTP {}", resp.status())) },
            }
        }
        Err(e) => SourceSpeed {
            source: source.to_string(),
            reachable: false,
            latency_ms: 0,
            speed_kbps: 0.0,
            error: Some(e.to_string()),
        },
    }
}

/// 并发测速两个更新源（镜像 / GitHub 官方），供设置页展示。
/// 注：离线依赖受限（无 tokio-macros），此处顺序执行；单源超时 8 秒。
#[tauri::command]
pub async fn measure_update_sources() -> Result<Vec<SourceSpeed>, String> {
    let mirror = measure_source(MIRROR_API, "mirror").await;
    let github = measure_source(GITHUB_API, "github").await;
    Ok(vec![mirror, github])
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
    // Microsoft Store（feature = "store"）禁止拉起外部安装器 / msiexec，
    // 以通过 WACK「已阻止的可执行文件」。商店渠道请走 Store 更新。
    #[cfg(feature = "store")]
    {
        let _ = path;
        Err(
            "Microsoft Store 版请通过商店更新，不支持拉起本地安装器。\
             Store builds update via the Microsoft Store only."
                .to_string(),
        )
    }

    #[cfg(not(feature = "store"))]
    {
        crate::log_info!("launching installer: {}", path);

        if path.ends_with(".msi") {
            Command::new("msiexec")
                .args(["/i", path])
                .spawn()
                .map_err(|e| format!("Failed to launch MSI installer: {}", e))?;
            return Ok(());
        }

        // 自制安装器：更新时打开图形向导（覆盖更新页），由用户确认后覆盖安装。
        // 传 --from-app：检测到已安装时进入更新模式，而不是全新安装流程。
        // 不要用 --silent：更新场景需要可见反馈，且旧 exe 退出时机由向导控制，
        // 避免静默解压撞上文件锁。
        Command::new(path)
            .arg("--from-app")
            .spawn()
            .map_err(|e| format!("Failed to launch installer: {}", e))?;

        Ok(())
    }
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
pub fn get_update_source() -> Result<String, String> {
    let cfg = read_config_file()?;
    Ok(cfg.update_source.unwrap_or_else(|| "mirror".to_string()))
}

#[tauri::command]
pub fn set_update_source(source: String) -> Result<(), String> {
    // mirror = 国内镜像（默认）；github = 官方 GitHub Releases
    if source != "mirror" && source != "github" {
        return Err(format!("Invalid update source: {}", source));
    }
    let mut cfg = read_config_file()?;
    cfg.update_source = Some(source);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_semver_core() {
        assert_eq!(
            parse_semver("2.2.0"),
            Some(SemVer { major: 2, minor: 2, patch: 0 })
        );
        assert_eq!(
            parse_semver("v2.1.5"),
            Some(SemVer { major: 2, minor: 1, patch: 5 })
        );
        assert_eq!(
            parse_semver("2.2.0-beta.1"),
            Some(SemVer { major: 2, minor: 2, patch: 0 })
        );
        assert_eq!(
            parse_semver("2.2.0+build.7"),
            Some(SemVer { major: 2, minor: 2, patch: 0 })
        );
        assert_eq!(
            parse_semver("2.2"),
            Some(SemVer { major: 2, minor: 2, patch: 0 })
        );
        assert!(parse_semver("").is_none());
        assert!(parse_semver("not-a-version").is_none());
    }

    #[test]
    fn version_greater_semver() {
        assert!(version_greater("2.2.0", "2.1.9"));
        assert!(version_greater("3.0.0", "2.99.99"));
        assert!(version_greater("2.1.1", "2.1.0"));
        assert!(!version_greater("2.1.0", "2.1.0"));
        assert!(!version_greater("2.0.9", "2.1.0"));
        // 预发布与正式版同 core：视为相等，不提示更新
        assert!(!version_greater("2.2.0-beta", "2.2.0"));
    }

    #[test]
    fn version_bump_levels() {
        assert_eq!(version_bump("2.1.5", "2.2.0"), VersionBump::Minor);
        assert_eq!(version_bump("2.2.0", "3.0.0"), VersionBump::Major);
        assert_eq!(version_bump("2.2.0", "2.2.1"), VersionBump::Patch);
        assert_eq!(version_bump("2.2.0", "2.2.0"), VersionBump::None);
        assert_eq!(version_bump("2.3.0", "2.2.0"), VersionBump::None);
        assert_eq!(version_bump("3.0.0", "2.9.9"), VersionBump::None);
    }

    #[test]
    fn effective_priority_rules() {
        // Safe tag 强制，无论 bump
        assert_eq!(
            effective_priority(UpdatePriority::Safe, VersionBump::Patch),
            UpdatePriority::Safe
        );
        // major 强制
        assert_eq!(
            effective_priority(UpdatePriority::Optional, VersionBump::Major),
            UpdatePriority::Safe
        );
        // minor / patch 可选
        assert_eq!(
            effective_priority(UpdatePriority::Optional, VersionBump::Minor),
            UpdatePriority::Optional
        );
        assert_eq!(
            effective_priority(UpdatePriority::Optional, VersionBump::Patch),
            UpdatePriority::Optional
        );
    }
}
