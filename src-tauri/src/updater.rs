use std::fs;
use std::io::Write;
#[cfg(not(feature = "store"))]
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

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

// ── 结构化错误 ───────────────────────────────────────────────
//
// 为什么不再用 `String` 当错误类型 —— 两个理由，都不是「代码整洁」：
//
// ① 界面要把失败归到用户**能据此决定要不要重试**的原因上（断网 / 超时 /
//    服务器拒绝 / 磁盘写不进去 / 校验不通过）。原来是
//    `"Download failed: error sending request for url (…)"` 这种英文技术串
//    直接铺在界面上，用户读到的只有一个「失败」。
// ② 「拿不到 .sha256 时允许用户显式跳过校验」是一条**安全边界**，需要一个
//    机器可判的标记。哈希**确实对不上**绝不能和「校验文件缺失」共用一条
//    回退分支 —— 靠匹配错误字符串里的中文关键词来区分，等于把边界写成注释。
//
// 所以错误拆两半：`kind` 给程序分支、也给界面选文案；`detail` 保留原始错误链
// （HTTP 状态码、IO 原文、期望/实际哈希）。`detail` 里的英文原文是**故意**
// 不翻译的 —— 翻译过的技术细节没法拿去搜索，而它存在的唯一用途就是排查。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateErrorKind {
    /// 连不上：DNS 失败 / 断网 / 连接被重置 / 传输中途断流
    Network,
    /// 超时（请求或读取）
    Timeout,
    /// 服务器返回非 2xx：403 / 404 / 429 / 5xx
    Http,
    /// 本地写盘失败：临时目录不可写、磁盘满、文件被占用
    Disk,
    /// release 里根本没有 `.sha256` 资产
    ShaMissing,
    /// `.sha256` 存在，但下载/读取/解析失败
    ShaUnreachable,
    /// 哈希**确实对不上** —— 比缺失更强的信号
    ShaMismatch,
    /// 找不到可下载的安装包资产
    Asset,
    /// 白名单 / 命名规范没通过，主动拒绝（安全决定，不是故障）
    Blocked,
    /// 已有下载在跑，拒绝并发
    Busy,
    /// 用户取消
    Cancelled,
    /// 拉起安装器失败
    Install,
    /// 其它（兜底，界面用通用措辞）
    Other,
}

impl UpdateErrorKind {
    /// 机器可判的「这条失败能否被用户显式跳过校验」。
    ///
    /// 只有「拿不到校验文件」这两种放行，理由：
    ///   · 它通常是我们自己发布流水线的问题（忘了传 .sha256，或那个小文件
    ///     挂在 CDN 上取不到），安装包本身仍然过了域名白名单与文件名规范；
    ///   · 哈希**不匹配**则意味着落盘的字节和发布出来的不是同一份 —— 可能是
    ///     传输损坏，也可能被替换。此时放行等于把完整性校验整个作废，
    ///     正确的补救是**重新下载**，不是绕过去。
    ///
    /// 抽成函数而不是散在调用点，是为了能被单测钉死（见文件末尾
    /// `sha_skip_only_for_missing_or_unreachable`）：以后有人想在这里加一类，
    /// 测试会先报出来。
    fn allows_unverified_sha256(self) -> bool {
        matches!(self, UpdateErrorKind::ShaMissing | UpdateErrorKind::ShaUnreachable)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateError {
    pub kind: UpdateErrorKind,
    pub detail: String,
}

impl UpdateError {
    fn new(kind: UpdateErrorKind, detail: impl Into<String>) -> Self {
        Self { kind, detail: detail.into() }
    }

    /// reqwest 错误的归类。`stage` 说明失败发生在哪一步（拉 release / 下载
    /// 安装包 / 拉校验文件）—— 三个请求的失败信息长得几乎一样，没有这个前缀，
    /// 日志里分不清是哪一个挂了。
    fn from_reqwest(stage: &str, e: reqwest::Error) -> Self {
        let kind = if e.is_timeout() {
            UpdateErrorKind::Timeout
        } else if e.is_connect() || e.is_body() || e.is_decode() {
            // is_connect：DNS / 拒绝连接 / TLS 握手 —— 用户侧看到的都是「连不上」；
            // is_body / is_decode：连接建立后中途断流。不必再细分，detail 里有原文。
            UpdateErrorKind::Network
        } else {
            UpdateErrorKind::Other
        };
        Self::new(kind, format!("[{}] {}", stage, e))
    }

    /// HTTP 非 2xx。状态码必须进 detail：403/429 是限流、404 是资产没了、
    /// 5xx 是对端故障，三者该给的建议完全不同，不能在界面上一句带过。
    fn from_status(stage: &str, status: reqwest::StatusCode, url: &str) -> Self {
        Self::new(
            UpdateErrorKind::Http,
            format!("[{}] HTTP {} — {}", stage, status.as_u16(), url),
        )
    }
}

// ── 下载互斥与取消 ───────────────────────────────────────────
//
// 互斥为什么必须在后端：界面上的「重试」是关掉页面再点一次，两次点击之间
// 完全可能有两个下载在跑（切页后按钮就是可用的）。两路写**同一个**临时文件，
// 互相覆盖 —— 最后算出的哈希两边都对不上，而用户看到的是「校验失败」，
// 于是被怀疑的是发布者。前端禁用按钮只是体验，不是防线。
static DOWNLOADING: AtomicBool = AtomicBool::new(false);
/// 取消标记。只在一轮下载的生命周期里有效，由 `DownloadGuard` 负责复位。
static CANCEL_REQUESTED: AtomicBool = AtomicBool::new(false);

/// RAII 守卫：成功 / 失败 / 提前 return 都会复位标记。
/// 手写 `store(false)` 的每个返回点迟早漏一个，漏掉就是「以后再也下不了」。
struct DownloadGuard;

impl DownloadGuard {
    fn acquire() -> Result<Self, UpdateError> {
        match DOWNLOADING.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => {
                // 新一轮下载从「未取消」开始：上一轮的取消标记不能泄漏进来
                CANCEL_REQUESTED.store(false, Ordering::SeqCst);
                Ok(Self)
            }
            Err(_) => Err(UpdateError::new(
                UpdateErrorKind::Busy,
                "已有一个下载在进行中，拒绝并发下载",
            )),
        }
    }
}

impl Drop for DownloadGuard {
    fn drop(&mut self) {
        CANCEL_REQUESTED.store(false, Ordering::SeqCst);
        DOWNLOADING.store(false, Ordering::SeqCst);
    }
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

// 更新源：**只有官方 GitHub Releases**。
//
// 历史上的国内镜像 `cdn.5eggpack.top` 已彻底移除（2026-10：镜像作者停止维护），
// 更新源概念（含设置项、测速、切换命令）一并删除——没有任何可切换的源，
// 也不再有相关 UI。配置文件里遗留的 `update_source` 字段会被忽略。
const GITHUB_API: &str = "https://api.github.com/repos/realLivedInCorner/2-Pyramid/releases";

async fn fetch_releases() -> Result<Vec<GitHubRelease>, UpdateError> {
    let client = reqwest::Client::new();
    let resp = client
        .get(GITHUB_API)
        .query(&[("per_page", "30")])
        .header(USER_AGENT, "2-Pyramid-Updater/2.0")
        .header(ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| UpdateError::from_reqwest("fetch releases", e))?;

    if !resp.status().is_success() {
        return Err(UpdateError::from_status("fetch releases", resp.status(), GITHUB_API));
    }

    resp.json::<Vec<GitHubRelease>>()
        .await
        .map_err(|e| UpdateError::from_reqwest("parse releases", e))
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

async fn check_github_releases(channel: &str, current_version: &str) -> Result<UpdateCheckResult, UpdateError> {
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
/// 响应里返回的 browser_download_url 一律先过这里，防止被篡改的数据
/// 把用户引向任意域名。（镜像源已停用，故白名单同步收掉其域名。）
fn is_allowed_download_url(url: &str) -> bool {
    match reqwest::Url::parse(url) {
        Ok(u) => u.host_str().map(|h| {
            h == "github.com"
                || h.ends_with(".github.com")
                || h == "objects.githubusercontent.com"
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

/// 找安装包。返回 `(资产, 是否命中严格命名)`。
///
/// 严格命名 = `2-Pyramid-Installer-<版本>`；回退分支只为兼容历史上命名不规范的
/// release（那时版本号不一定在文件名里）。回退**不允许**用来走「跳过校验」：
/// 两条弱化叠加（名字不校验 + 字节不校验）就等于完全不设防，所以调用方必须
/// 拿到这个布尔值，在放行跳过之前再判一次。
fn find_windows_asset(release: &ReleaseInfo) -> Result<(&AssetInfo, bool), UpdateError> {
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
            return Ok((asset, true));
        }
    }
    release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".exe") || a.name.ends_with(".msi"))
        .map(|a| (a, false))
        .ok_or_else(|| {
            UpdateError::new(
                UpdateErrorKind::Asset,
                format!("release {} 没有任何 .exe / .msi 资产", release.tag_name),
            )
        })
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
async fn fetch_expected_sha256(url: &str) -> Result<String, UpdateError> {
    if !is_allowed_download_url(url) {
        // 域名不在白名单 = 有东西试图把校验文件指到别处。这是安全事件，
        // 归 Blocked 而不是「取不到校验文件」—— 后者允许用户跳过。
        return Err(UpdateError::new(
            UpdateErrorKind::Blocked,
            format!("校验文件域名不在白名单: {}", url),
        ));
    }
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .header(USER_AGENT, "2-Pyramid-Updater/2.0")
        .send()
        .await
        .map_err(|e| UpdateError::from_reqwest("fetch sha256", e))?;
    if !resp.status().is_success() {
        return Err(UpdateError::from_status("fetch sha256", resp.status(), url));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| UpdateError::from_reqwest("read sha256", e))?;
    if bytes.len() > 4096 {
        return Err(UpdateError::new(
            UpdateErrorKind::ShaUnreachable,
            "校验文件超过 4KB，不像一份 SHA-256 清单",
        ));
    }
    let text = String::from_utf8_lossy(&bytes);
    let hex = text.split_whitespace().next().unwrap_or_default().trim();
    if hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(hex.to_ascii_lowercase())
    } else {
        Err(UpdateError::new(
            UpdateErrorKind::ShaUnreachable,
            format!("校验文件内容不是 64 位十六进制 SHA-256: {:?}", &text.chars().take(64).collect::<String>()),
        ))
    }
}

/// 取安装包的期望 SHA-256，并把两种失败按 kind 分开：
///   · `ShaMissing`     —— release 里没有这个资产（我们发布时漏了）
///   · `ShaUnreachable` —— 资产在，但取不到 / 读不出 / 解析不了
///
/// 界面上这两句措辞不同（「这个版本没有附校验文件」 vs 「校验文件下载失败」），
/// 但**都能**被用户显式跳过。白名单拒绝（`Blocked`）原样上抛，不参与跳过。
async fn resolve_expected_sha256(
    release: &ReleaseInfo,
    installer_name: &str,
) -> Result<String, UpdateError> {
    match find_sha256_asset(release, installer_name) {
        Some(sha_asset) => {
            fetch_expected_sha256(&sha_asset.browser_download_url)
                .await
                .map_err(|e| match e.kind {
                    UpdateErrorKind::Blocked => e,
                    _ => UpdateError::new(
                        UpdateErrorKind::ShaUnreachable,
                        format!("{} ; url={}", e.detail, sha_asset.browser_download_url),
                    ),
                })
        }
        None => Err(UpdateError::new(
            UpdateErrorKind::ShaMissing,
            format!(
                "release {} 的资产里没有 {}.sha256",
                release.tag_name, installer_name
            ),
        )),
    }
}

/// 下载安装包到临时目录并（默认）校验 SHA-256。
///
/// `allow_unverified_sha256` 是**用户显式选择**的降级：只有「拿不到校验文件」
/// 那一类失败会被它绕过，哈希不匹配永远拒绝（判据见
/// `UpdateErrorKind::allows_unverified_sha256`）。这个参数不落配置、不记忆 ——
/// 它只对本次下载有效，重试一次就回到强制校验。
async fn download_installer(
    app: &AppHandle,
    release: &ReleaseInfo,
    allow_unverified_sha256: bool,
) -> Result<String, UpdateError> {
    let (asset, strict_name) = find_windows_asset(release)?;

    // 1. 下载域名白名单校验（镜像响应可能伪造下载地址）
    if !is_allowed_download_url(&asset.browser_download_url) {
        return Err(UpdateError::new(
            UpdateErrorKind::Blocked,
            format!("安装包域名不在白名单，已拒绝下载: {}", asset.browser_download_url),
        ));
    }

    // 2. SHA-256 默认**强制**：没有校验文件就无法证明安装包完整，一律拒绝更新。
    //    唯一的例外是用户在界面上显式确认了「跳过校验」，且失败类型属于
    //    「拿不到校验文件」；这两种失败本身不是「字节不对」的证据。
    let mut skipped_sha = false;
    let expected_sha256 = match resolve_expected_sha256(release, &asset.name).await {
        Ok(hex) => hex,
        Err(err) => {
            if !allow_unverified_sha256 || !err.kind.allows_unverified_sha256() {
                // 注意这里**同时**挡住「哈希不匹配」「白名单拒绝」：
                // 跳过开关只对 ShaMissing / ShaUnreachable 生效。
                crate::log_error!("拒绝更新，校验不可用: {:?} — {}", err.kind, err.detail);
                return Err(err);
            }
            if !strict_name {
                // 文件名回退 + 跳过校验 = 两道边界同时失效，不放行
                crate::log_warn!(
                    "安装包名不符合发布规范，即使跳过校验也不放行: {}",
                    asset.name
                );
                return Err(UpdateError::new(
                    UpdateErrorKind::Blocked,
                    format!(
                        "安装包文件名不符合发布规范（{}），无法在跳过校验的情况下继续",
                        asset.name
                    ),
                ));
            }
            // 留痕：这条日志是事后**唯一**能证明「用户自己选择了降级」的证据，
            // 也是复盘「谁装进来的」时唯一的线索。所以版本、资产名、原因全写。
            crate::log_warn!(
                "用户显式跳过 SHA-256 校验：version={} asset={} reason={:?} detail={}",
                release.version,
                asset.name,
                err.kind,
                err.detail
            );
            skipped_sha = true;
            String::new()
        }
    };

    let temp_dir = std::env::temp_dir().join("2_pyramid_update");
    fs::create_dir_all(&temp_dir).map_err(|e| {
        UpdateError::new(
            UpdateErrorKind::Disk,
            format!("创建临时目录失败: {} ({})", temp_dir.display(), e),
        )
    })?;

    let file_path = temp_dir.join(&asset.name);
    let file_path_str = file_path.to_string_lossy().to_string();

    // 3. 先清掉上一次的半成品。
    //
    // `File::create` 会截断同名文件，但**只在能打开的前提下**：上次留下的
    // 半成品可能正被占用（杀毒在扫、上一轮取消后句柄还没释放），于是「重试」
    // 看起来什么都没发生，或者留下一个大小不对的文件被安装器拒绝。
    // 这里先删再建，删不掉就直接报磁盘错误 —— 把问题说清楚，而不是留个坏文件。
    //
    // 只删**本次的目标路径**，不清空整个 temp 目录：用户可能刚下完一份正在
    // 等手动安装，扫全目录迟早会把它删掉。
    if file_path.exists() {
        fs::remove_file(&file_path).map_err(|e| {
            UpdateError::new(
                UpdateErrorKind::Disk,
                format!("清理上次的临时文件失败: {} ({})", file_path_str, e),
            )
        })?;
        crate::log_info!("cleared previous partial installer: {}", file_path_str);
    }

    crate::log_info!("downloading {} ({} bytes)", asset.browser_download_url, asset.size);

    // 请求还没发出去就已被取消：别把几十 MB 白下一遍
    if CANCEL_REQUESTED.load(Ordering::SeqCst) {
        return Err(UpdateError::new(
            UpdateErrorKind::Cancelled,
            "下载在发出请求前被取消",
        ));
    }

    let client = reqwest::Client::new();
    let mut resp = client
        .get(&asset.browser_download_url)
        .header(USER_AGENT, "2-Pyramid-Updater/2.0")
        .send()
        .await
        .map_err(|e| UpdateError::from_reqwest("download installer", e))?;

    // 原来这里**不查状态码**：404 的 HTML 页面会被当成安装包写进 .exe，
    // 再由哈希校验兜住。兜得住，但用户看到的是「校验失败」，方向被带偏。
    // 现在提前判状态码并归到 Http。
    if !resp.status().is_success() {
        return Err(UpdateError::from_status(
            "download installer",
            resp.status(),
            &asset.browser_download_url,
        ));
    }

    let total = resp.content_length().unwrap_or(asset.size);
    let mut downloaded: u64 = 0;
    let mut last_emit: u64 = 0;
    let mut file = fs::File::create(&file_path).map_err(|e| {
        UpdateError::new(
            UpdateErrorKind::Disk,
            format!("创建文件失败: {} ({})", file_path_str, e),
        )
    })?;
    // sha2 的 new()/update()/finalize() 都来自 Digest trait，需先引入
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    let mut cancelled = false;

    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| UpdateError::from_reqwest("read installer body", e))?
    {
        // 取消只在数据块之间判：写到一半的半块扔不掉，那是「文件损坏」而不是
        // 「取消」。中断延迟 = 一个 chunk 的到达时间，通常 < 1s。
        if CANCEL_REQUESTED.load(Ordering::SeqCst) {
            cancelled = true;
            break;
        }
        file.write_all(&chunk).map_err(|e| {
            // 这里**不**顺手删文件：句柄还开着，Windows 上删不掉。
            // 第 3 步的清理会在下一次下载前把它收掉。
            UpdateError::new(
                UpdateErrorKind::Disk,
                format!("写文件失败: {} ({})", file_path_str, e),
            )
        })?;
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

    file.flush().map_err(|e| {
        UpdateError::new(
            UpdateErrorKind::Disk,
            format!("刷新文件失败: {} ({})", file_path_str, e),
        )
    })?;
    // 显式关句柄再删：Windows 上「打开中的文件」remove_file 会失败，
    // 否则取消后会留下一个半成品文件，和「已取消」的界面状态不一致。
    drop(file);

    if cancelled {
        let _ = fs::remove_file(&file_path);
        crate::log_warn!(
            "下载已取消（已下载 {} / {} 字节），临时文件已删除: {}",
            downloaded,
            total,
            file_path_str
        );
        return Err(UpdateError::new(
            UpdateErrorKind::Cancelled,
            format!("用户取消，已丢弃 {} 字节", downloaded),
        ));
    }

    // 长度对不上当传输中断处理。**跳过校验的分支尤其需要这一条**：
    // 那里没有哈希兜底，尺寸是唯一还能拦住「下了一半却被当成装好」的东西。
    if total > 0 && downloaded != total {
        let _ = fs::remove_file(&file_path);
        return Err(UpdateError::new(
            UpdateErrorKind::Network,
            format!("下载不完整：期望 {} 字节，实际 {}", total, downloaded),
        ));
    }

    // 4. 校验。跳过时仍然把实际哈希算完并写进日志 —— 那是事后唯一能比对的凭据。
    {
        let actual = format!("{:x}", hasher.finalize());
        if skipped_sha {
            crate::log_warn!(
                "未校验的安装包已落盘（用户显式跳过校验）: {} sha256={}",
                file_path_str,
                actual
            );
        } else if !actual.eq_ignore_ascii_case(&expected_sha256) {
            let _ = fs::remove_file(&file_path);
            crate::log_error!(
                "sha256 mismatch: expected={} actual={} file={}",
                expected_sha256,
                actual,
                file_path_str
            );
            return Err(UpdateError::new(
                UpdateErrorKind::ShaMismatch,
                format!("期望 {}，实际 {}（文件已删除）", expected_sha256, actual),
            ));
        } else {
            crate::log_info!("sha256 verified: {}", expected_sha256);
        }
    }

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
pub async fn check_for_update(app: AppHandle, channel: Option<String>) -> Result<UpdateCheckResult, UpdateError> {
    let channel = channel.unwrap_or_else(|| {
        read_config_file()
            .ok()
            .and_then(|c| c.update_channel)
            .unwrap_or_else(|| "master".to_string())
    });
    crate::log_info!("checking for update, channel={}", channel);
    check_github_releases(&channel, &current_version()).await
}

/// 下载更新。
///
/// `allowUnverifiedSha256`（前端 camelCase）是用户显式确认过的降级开关，
/// 缺省 `false` —— 旧前端不传这个参数时行为与改造前完全一致（强制校验）。
#[tauri::command]
pub async fn download_update(
    app: AppHandle,
    tag_name: String,
    allow_unverified_sha256: Option<bool>,
) -> Result<String, UpdateError> {
    let allow_unverified = allow_unverified_sha256.unwrap_or(false);
    crate::log_info!(
        "download_update requested for tag={} (skip_sha={})",
        tag_name,
        allow_unverified
    );

    // 互斥在**第一个 await 之前**拿到：否则两路请求会一起走到 fetch_releases，
    // 等它们都拿到守卫时早已各自浪费一次网络往返。
    let _guard = DownloadGuard::acquire()?;

    let releases = fetch_releases().await?;

    let (version_str, _priority) = parse_tag(&tag_name);
    let release = releases
        .iter()
        .find(|r| {
            let (v, _) = parse_tag(&r.tag_name);
            v == version_str
        })
        .ok_or_else(|| {
            UpdateError::new(
                UpdateErrorKind::Asset,
                format!("找不到 tag {} 对应的 release（可能已被撤回）", tag_name),
            )
        })?;

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

    download_installer(&app, &ri, allow_unverified).await
}

/// 取消正在进行的下载。
///
/// 没有下载在跑时返回 `false` 而不是报错 —— 界面上的取消按钮完全可能在下载
/// 刚好结束的那一帧被点到，那不该变成一个红色错误。
#[tauri::command]
pub fn cancel_update_download() -> bool {
    if !DOWNLOADING.load(Ordering::SeqCst) {
        return false;
    }
    CANCEL_REQUESTED.store(true, Ordering::SeqCst);
    crate::log_warn!("收到取消下载请求，将在下一个数据块处中止");
    true
}

#[tauri::command]
pub fn install_update(app: AppHandle, installer_path: String, new_version: String) -> Result<(), UpdateError> {
    if let Err(e) = write_update_marker(&new_version) {
        // 标记写不进去不该拦住安装，但必须留痕：读不到标记，下次启动就不会
        // 提示「已成功更新到 vX」，用户会以为没更新成。
        crate::log_warn!("更新标记写入失败（不阻断安装）: {}", e);
    }
    launch_installer(&installer_path)
        .map_err(|e| UpdateError::new(UpdateErrorKind::Install, e))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 造一个只带资产的 release，供查找类测试用。
    fn fixture(version: &str, assets: &[(&str, &str)]) -> ReleaseInfo {
        ReleaseInfo {
            tag_name: format!("Stable-{}", version),
            version: version.to_string(),
            name: format!("2-Pyramid {}", version),
            body: String::new(),
            published_at: "2026-01-01T00:00:00Z".to_string(),
            html_url: "https://github.com/realLivedInCorner/2-Pyramid/releases".to_string(),
            assets: assets
                .iter()
                .map(|(name, url)| AssetInfo {
                    name: (*name).to_string(),
                    browser_download_url: (*url).to_string(),
                    size: 1024,
                    content_type: "application/octet-stream".to_string(),
                })
                .collect(),
            priority: UpdatePriority::Optional,
            is_prerelease: false,
        }
    }

    // ── 安全边界：跳过校验只对「拿不到校验文件」生效 ──────────
    //
    // 这条测试是「不许降低安全边界」的可执行版本。以后有人想给 Mismatch
    // 也开一个跳过出口，会先在这里撞红。
    #[test]
    fn sha_skip_only_for_missing_or_unreachable() {
        assert!(UpdateErrorKind::ShaMissing.allows_unverified_sha256());
        assert!(UpdateErrorKind::ShaUnreachable.allows_unverified_sha256());

        assert!(!UpdateErrorKind::ShaMismatch.allows_unverified_sha256());
        assert!(!UpdateErrorKind::Blocked.allows_unverified_sha256());
        assert!(!UpdateErrorKind::Cancelled.allows_unverified_sha256());
        assert!(!UpdateErrorKind::Network.allows_unverified_sha256());
        assert!(!UpdateErrorKind::Disk.allows_unverified_sha256());
        assert!(!UpdateErrorKind::Other.allows_unverified_sha256());
    }

    // 前端按 kind 选文案，键名必须与 TS 侧的 UpdateErrorKind 字面量一致。
    // 序列化格式一旦漂移（比如变成 "ShaMissing"），界面会退回通用措辞，
    // 而这种失效在编译期看不出来 —— 只能靠这条测试钉住。
    #[test]
    fn error_serializes_with_camel_case_kind() {
        let json = serde_json::to_string(&UpdateError::new(UpdateErrorKind::ShaMismatch, "d"))
            .expect("serialize");
        assert!(json.contains("\"kind\":\"shaMismatch\""), "{json}");
        assert!(json.contains("\"detail\":\"d\""), "{json}");

        let json = serde_json::to_string(&UpdateError::new(UpdateErrorKind::ShaUnreachable, ""))
            .expect("serialize");
        assert!(json.contains("\"kind\":\"shaUnreachable\""), "{json}");
    }

    #[test]
    fn download_url_whitelist() {
        assert!(is_allowed_download_url(
            "https://github.com/realLivedInCorner/2-Pyramid/releases/download/x/a.exe"
        ));
        assert!(is_allowed_download_url(
            "https://objects.githubusercontent.com/github-production-release-asset/x"
        ));
        // 子域放行（GitHub 的下载会跳到 objects.githubusercontent.com 这类子域）
        assert!(is_allowed_download_url("https://api.github.com/x"));
        // 其它域名一律拒 —— 包括形似域名（evil-github.com 不是 .github.com）
        assert!(!is_allowed_download_url("https://evil.com/a.exe"));
        assert!(!is_allowed_download_url("https://evil-github.com/a.exe"));
        assert!(!is_allowed_download_url("https://github.com.evil.net/a.exe"));
        assert!(!is_allowed_download_url("file:///C:/a.exe"));
        assert!(!is_allowed_download_url("not a url"));
    }

    #[test]
    fn installer_name_must_carry_release_version() {
        assert!(is_expected_installer_name(
            "2-Pyramid-Installer-2.203.0.exe",
            "2.203.0"
        ));
        // 大写也认（比较前统一 lowercase）
        assert!(is_expected_installer_name(
            "2-PYRAMID-INSTALLER-2.203.0.EXE",
            "2.203.0"
        ));
        assert!(!is_expected_installer_name(
            "2-Pyramid-Installer-2.202.0.exe",
            "2.203.0"
        ));
        assert!(!is_expected_installer_name("setup.exe", "2.203.0"));
    }

    // 严格命名命中 → strict = true；只有历史命名的回退 → strict = false。
    // 跳过校验时调用方必须看到 false 并拒绝，所以这个布尔值不能省。
    #[test]
    fn find_windows_asset_reports_whether_name_is_strict() {
        let strict = fixture(
            "2.203.0",
            &[
                ("2-Pyramid-Installer-2.203.0.exe", "https://github.com/x/a.exe"),
                ("2-Pyramid-Installer-2.203.0.exe.sha256", "https://github.com/x/a.sha256"),
            ],
        );
        let (asset, is_strict) = find_windows_asset(&strict).expect("strict asset");
        assert_eq!(asset.name, "2-Pyramid-Installer-2.203.0.exe");
        assert!(is_strict);

        // 版本号不在文件名里：回退分支能找到，但必须标成「非严格」
        let legacy = fixture("2.203.0", &[("2-Pyramid-Setup.exe", "https://github.com/x/s.exe")]);
        let (asset, is_strict) = find_windows_asset(&legacy).expect("fallback asset");
        assert_eq!(asset.name, "2-Pyramid-Setup.exe");
        assert!(!is_strict, "回退命中的必须是 strict=false，否则跳过校验会绕过命名边界");

        let empty = fixture("2.203.0", &[]);
        let err = find_windows_asset(&empty).expect_err("no asset");
        assert_eq!(err.kind, UpdateErrorKind::Asset);
    }

    // .exe 优先于 .msi：EXE 是我们自己的安装向导，MSI 只是兜底
    #[test]
    fn find_windows_asset_prefers_exe() {
        let r = fixture(
            "2.203.0",
            &[
                ("2-Pyramid-Installer-2.203.0.msi", "https://github.com/x/a.msi"),
                ("2-Pyramid-Installer-2.203.0.exe", "https://github.com/x/a.exe"),
            ],
        );
        let (asset, _) = find_windows_asset(&r).expect("asset");
        assert_eq!(asset.name, "2-Pyramid-Installer-2.203.0.exe");
    }

    #[test]
    fn finds_matching_sha256_asset() {
        let r = fixture(
            "2.203.0",
            &[
                ("2-Pyramid-Installer-2.203.0.exe", "https://github.com/x/a.exe"),
                ("2-Pyramid-Installer-2.203.0.exe.sha256", "https://github.com/x/a.sha256"),
            ],
        );
        let sha = find_sha256_asset(&r, "2-Pyramid-Installer-2.203.0.exe").expect("sha asset");
        assert_eq!(sha.name, "2-Pyramid-Installer-2.203.0.exe.sha256");

        // 名字不匹配（多一个 .exe）时不能乱配一个哈希文件回去
        assert!(find_sha256_asset(&r, "2-Pyramid-Installer-2.203.0.msi").is_none());
    }

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
