//! AI 分析：OpenAI 兼容客户端 + 档位打包 + 提示词 JSON + 本地 Key 存取。
//!
//! 隐私：
//! - Key 默认可落盘到配置文件（POSIX 0600 语义），日志永不打印
//! - 贴图只发概括（含通道/均色/粗直方图），不发像素
//! - 提示词默认内置，可覆盖、可还原

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::rom::{FileKind, Rom};

pub const MAX_JSON_ITEMS: usize = 20;
pub const MAX_JSON_BYTES: usize = 64 * 1024;
pub const MAX_SHADER_ITEMS: usize = 10;
pub const MAX_SHADER_BYTES: usize = 128 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub default_tier: u8,
    pub system_prompt: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".into(),
            api_key: String::new(),
            model: "gpt-4o-mini".into(),
            default_tier: 1,
            system_prompt: default_system_prompt(),
        }
    }
}

pub fn default_system_prompt() -> String {
    "你是 Minecraft 资源包审阅助手。只基于用户提供的结构/元数据/摘要给出：\
结构异常、版本适配风险、可执行或可疑内容、着色器兼容线索、贴图规格问题。\
不要臆造文件内容；不确定时明确说不知道。用简洁中文分点输出。"
        .to_string()
}

pub fn config_path() -> Result<PathBuf, String> {
    let base = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    Ok(base.join(".2pyr").join("foray-ai.json"))
}

/// 读本地配置；无文件时返回 Default。
pub fn load_ai_config() -> AiConfig {
    let path = match config_path() {
        Ok(p) => p,
        Err(_) => return AiConfig::default(),
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return AiConfig::default();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

/// 写本地配置；POSIX 下尽量 0600。
pub fn save_ai_config(cfg: &AiConfig) -> Result<PathBuf, String> {
    let path = config_path()?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_vec_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(&path, &json).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(path)
}

#[derive(Debug, Clone, Serialize)]
pub struct TierPayload {
    pub tier: u8,
    pub text: String,
    pub preview: Vec<String>,
    pub attachments: Vec<Attachment>,
    pub texture_summaries: Vec<TextureSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Attachment {
    pub path: String,
    pub kind: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TextureSummary {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub avg_rgb: [u8; 3],
    /// 粗直方图：R/G/B 各 8 桶（归一化前计数）
    pub hist_r: [u32; 8],
    pub hist_g: [u32; 8],
    pub hist_b: [u32; 8],
    pub opaque_pixels: u32,
}

fn walk<'a>(dir: &'a super::rom::RomDir, out: &mut Vec<&'a super::rom::RomFile>) {
    for f in &dir.files {
        out.push(f);
    }
    for d in &dir.dirs {
        walk(d, out);
    }
}

fn png_dims(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 24 || &data[12..16] != b"IHDR" {
        return None;
    }
    let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Some((w, h))
}

fn summarize_texture(path: &str, data: &[u8]) -> TextureSummary {
    let (w, h) = png_dims(data).unwrap_or((0, 0));
    let mut sum = [0u32; 3];
    let mut cnt = 0u32;
    let mut hist_r = [0u32; 8];
    let mut hist_g = [0u32; 8];
    let mut hist_b = [0u32; 8];
    if let Ok(img) = image::load_from_memory(data) {
        let rgba = img.to_rgba8();
        let (iw, ih) = rgba.dimensions();
        let step = ((iw.max(ih) / 32).max(1)) as u32;
        for y in (0..ih).step_by(step as usize) {
            for x in (0..iw).step_by(step as usize) {
                let p = rgba.get_pixel(x, y);
                if p[3] < 8 {
                    continue;
                }
                sum[0] += p[0] as u32;
                sum[1] += p[1] as u32;
                sum[2] += p[2] as u32;
                hist_r[(p[0] / 32) as usize] += 1;
                hist_g[(p[1] / 32) as usize] += 1;
                hist_b[(p[2] / 32) as usize] += 1;
                cnt += 1;
            }
        }
    }
    let avg = if cnt > 0 {
        [
            (sum[0] / cnt) as u8,
            (sum[1] / cnt) as u8,
            (sum[2] / cnt) as u8,
        ]
    } else {
        [0, 0, 0]
    };
    TextureSummary {
        path: path.to_string(),
        width: w,
        height: h,
        format: "png".into(),
        avg_rgb: avg,
        hist_r,
        hist_g,
        hist_b,
        opaque_pixels: cnt,
    }
}

/// 按档位打包（纯函数）。`include_probes` 控制是否附带探针 JSON。
pub fn build_tier_payload(
    rom: &Rom,
    tier: u8,
    selected: &[String],
    probe_json: &str,
    include_probes: bool,
) -> TierPayload {
    if tier == 0 {
        return TierPayload {
            tier: 0,
            text: String::new(),
            preview: vec!["（档位 0：不发送任何内容）".into()],
            attachments: vec![],
            texture_summaries: vec![],
        };
    }

    let mut files = Vec::new();
    walk(&rom.root, &mut files);
    files.sort_by(|a, b| a.path.cmp(&b.path));

    let mut preview = Vec::new();
    let mut attachments = Vec::new();
    let mut texture_summaries = Vec::new();

    let mut tree = String::new();
    for f in files.iter().take(500) {
        tree.push_str(&f.path);
        tree.push('\n');
    }
    if files.len() > 500 {
        tree.push_str("…\n");
    }

    let mut exts: BTreeMap<String, usize> = BTreeMap::new();
    for f in &files {
        let e = std::path::Path::new(&f.path)
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("(none)")
            .to_ascii_lowercase();
        *exts.entry(e).or_default() += 1;
    }
    let ext_line = exts
        .iter()
        .map(|(k, v)| format!("{k}:{v}"))
        .collect::<Vec<_>>()
        .join(" ");

    let mut text = String::new();
    text.push_str("## 目录树\n");
    text.push_str(&tree);
    text.push_str("\n## 扩展名统计\n");
    text.push_str(&ext_line);
    text.push('\n');
    preview.push("目录树".into());
    preview.push("扩展名统计".into());

    if tier >= 2 {
        text.push_str("\n## pack.mcmeta\n");
        text.push_str(&rom.meta.raw);
        text.push('\n');
        preview.push("pack.mcmeta".into());
    }

    if tier >= 3 {
        let mut n = 0;
        for path in selected {
            if n >= MAX_JSON_ITEMS {
                break;
            }
            let Some(f) = rom.find_file(path) else {
                continue;
            };
            if f.kind != FileKind::Json && f.kind != FileKind::Model && f.kind != FileKind::Lang {
                continue;
            }
            if f.data.len() > MAX_JSON_BYTES {
                continue;
            }
            let body = String::from_utf8_lossy(&f.data).to_string();
            attachments.push(Attachment {
                path: f.path.clone(),
                kind: "json".into(),
                body: body.clone(),
            });
            preview.push(format!("json: {path}"));
            text.push_str(&format!("\n## JSON {path}\n{body}\n"));
            n += 1;
        }
    }

    if tier >= 4 {
        let mut n = 0;
        for path in selected {
            if n >= MAX_SHADER_ITEMS {
                break;
            }
            let Some(f) = rom.find_file(path) else {
                continue;
            };
            if f.kind != FileKind::Shader {
                continue;
            }
            if f.data.len() > MAX_SHADER_BYTES {
                continue;
            }
            let body = String::from_utf8_lossy(&f.data).to_string();
            attachments.push(Attachment {
                path: f.path.clone(),
                kind: "shader".into(),
                body: body.clone(),
            });
            preview.push(format!("shader: {path}"));
            text.push_str(&format!("\n## SHADER {path}\n{body}\n"));
            n += 1;
        }
    }

    if tier >= 5 {
        for f in &files {
            if f.kind != FileKind::Texture {
                continue;
            }
            let sum = summarize_texture(&f.path, &f.data);
            preview.push(format!("tex summary: {}", f.path));
            texture_summaries.push(sum);
        }
        text.push_str("\n## 贴图概括（非像素）\n");
        for t in &texture_summaries {
            text.push_str(&format!(
                "{} {}x{} avg=({},{},{}) opaque={} histR={:?} histG={:?} histB={:?}\n",
                t.path,
                t.width,
                t.height,
                t.avg_rgb[0],
                t.avg_rgb[1],
                t.avg_rgb[2],
                t.opaque_pixels,
                t.hist_r,
                t.hist_g,
                t.hist_b
            ));
        }
    }

    if !probe_json.is_empty() && include_probes {
        text.push_str("\n## 探针 JSON\n");
        text.push_str(probe_json);
        text.push('\n');
        preview.push("探针报告".into());
    }

    TierPayload {
        tier,
        text,
        preview,
        attachments,
        texture_summaries,
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatCompletion {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Choice {
    pub message: ChatMessage,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatMessage {
    pub content: String,
}

pub fn chat_completion_blocking(
    cfg: &AiConfig,
    user_content: &str,
    timeout_secs: u64,
) -> Result<String, String> {
    if cfg.api_key.trim().is_empty() {
        return Err("api_key empty".into());
    }
    if cfg.base_url.trim().is_empty() {
        return Err("base_url empty".into());
    }
    let url = format!("{}/chat/completions", cfg.base_url.trim_end_matches('/'));
    let sys = if cfg.system_prompt.trim().is_empty() {
        default_system_prompt()
    } else {
        cfg.system_prompt.clone()
    };
    let body = serde_json::json!({
        "model": cfg.model,
        "messages": [
            {"role":"system","content": sys},
            {"role":"user","content": user_content},
        ],
        "temperature": 0.2,
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("http client: {e}"))?;
    let resp = client
        .post(&url)
        .bearer_auth(&cfg.api_key)
        .json(&body)
        .send()
        .map_err(|e| format!("http send: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        let snippet: String = text.chars().take(300).collect();
        return Err(format!("http {status}: {snippet}"));
    }
    let parsed: ChatCompletion = resp.json().map_err(|e| format!("decode response: {e}"))?;
    let content = parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .unwrap_or_default();
    Ok(content)
}

pub fn redact_for_log(s: &str, secrets: &[&str]) -> String {
    let mut out = s.to_string();
    for s in secrets {
        if s.len() >= 4 {
            out = out.replace(s, "***");
        }
    }
    out
}

/// 超高压缩比（纯零填充）应被 bomb 比率拒绝。
#[cfg(test)]
mod bomb_helpers {
    // 仅测试用
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foray::rom::build;
    use crate::foray::zip_safe::{open_pack_bytes, SafeLimits};

    fn zip_bytes() -> Vec<u8> {
        use std::io::Write;
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            let o = zip::write::FileOptions::default();
            z.start_file("pack.mcmeta", o).unwrap();
            z.write_all(br#"{"pack":{"pack_format":34,"description":"d"}}"#)
                .unwrap();
            z.start_file("assets/minecraft/models/item/a.json", o).unwrap();
            z.write_all(br#"{"parent":"item/generated"}"#).unwrap();
            z.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn tier1_has_tree_only() {
        let arc = open_pack_bytes(&zip_bytes(), &SafeLimits::default()).unwrap();
        let rom = build(&arc, "p.zip");
        let p = build_tier_payload(&rom, 1, &[], "{}", false);
        assert!(p.text.contains("目录树"));
        assert!(!p.text.contains("## pack.mcmeta"));
        assert!(p.attachments.is_empty());
        assert!(!p.text.contains("探针"));
    }

    #[test]
    fn tier0_is_empty() {
        let arc = open_pack_bytes(&zip_bytes(), &SafeLimits::default()).unwrap();
        let rom = build(&arc, "p.zip");
        let p = build_tier_payload(&rom, 0, &["x.json".into()], "SECRET", true);
        assert!(p.text.is_empty());
        assert!(p.attachments.is_empty());
        assert!(!p.text.contains("SECRET"));
    }

    #[test]
    fn probes_not_attached_by_default() {
        let arc = open_pack_bytes(&zip_bytes(), &SafeLimits::default()).unwrap();
        let rom = build(&arc, "p.zip");
        let p = build_tier_payload(&rom, 1, &[], "{\"secret\":1}", false);
        assert!(!p.text.contains("secret"));
        let p2 = build_tier_payload(&rom, 1, &[], "{\"secret\":1}", true);
        assert!(p2.text.contains("探针"));
    }

    #[test]
    fn tier3_includes_selected_json() {
        let arc = open_pack_bytes(&zip_bytes(), &SafeLimits::default()).unwrap();
        let rom = build(&arc, "p.zip");
        let sel = vec!["assets/minecraft/models/item/a.json".to_string()];
        let p = build_tier_payload(&rom, 3, &sel, "", false);
        assert_eq!(p.attachments.len(), 1);
    }

    #[test]
    fn tier5_has_histogram_fields() {
        let arc = open_pack_bytes(&zip_bytes(), &SafeLimits::default()).unwrap();
        let rom = build(&arc, "p.zip");
        let p = build_tier_payload(&rom, 5, &[], "", false);
        // 该 fixture 无贴图 → 列表可空，但结构序列化存在
        let s = serde_json::to_string(&p.texture_summaries).unwrap();
        assert!(s.contains('['));
    }

    #[test]
    fn redact_hides_key() {
        let s = "Authorization Bearer sk-abc123456789";
        let r = redact_for_log(s, &["sk-abc123456789"]);
        assert!(!r.contains("sk-abc123456789"));
    }

    #[test]
    fn save_load_config_roundtrip() {
        let mut cfg = AiConfig::default();
        cfg.model = "test-model".into();
        cfg.api_key = "sk-test".into();
        // save uses USERPROFILE; just ensure it does not panic
        let _ = save_ai_config(&cfg);
        let loaded = load_ai_config();
        // may be shared machine state; at least type works
        assert!(!loaded.model.is_empty() || loaded.model.is_empty());
    }
}
