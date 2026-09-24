//! AI 分析：OpenAI 兼容客户端 + 档位打包 + 提示词 JSON。
//!
//! 隐私：
//! - Key 仅在本模块持有，不写日志
//! - 贴图只发概括，不发像素
//! - 提示词默认内置，可被用户 JSON 覆盖

use std::collections::BTreeMap;

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
    /// 0..=5
    pub default_tier: u8,
    /// 用户覆盖提示词（system）。空则用内置。
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

#[derive(Debug, Clone, Serialize)]
pub struct TierPayload {
    pub tier: u8,
    pub text: String,
    /// 界面「将发送」列表
    pub preview: Vec<String>,
    /// 额外附件（json / shader 副本）
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

/// 按档位打包（纯函数，便于单测）。
/// `selected` 为用户勾选的路径（档位 ≥3 时使用）。
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

    // 目录树（paths）
    let mut tree = String::new();
    for f in files.iter().take(500) {
        tree.push_str(&f.path);
        tree.push('\n');
    }
    if files.len() > 500 {
        tree.push_str("…\n");
    }

    // 扩展名统计
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
            let (w, h) = png_dims(&f.data).unwrap_or((0, 0));
            let mut sum = [0u32; 3];
            let mut cnt = 0u32;
            // 粗采样均色：每隔 stride 取像素（若有 image 解码失败则跳过均色）
            if let Ok(img) = image::load_from_memory(&f.data) {
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
            texture_summaries.push(TextureSummary {
                path: f.path.clone(),
                width: w,
                height: h,
                format: "png".into(),
                avg_rgb: avg,
            });
            preview.push(format!("tex summary: {}", f.path));
        }
        text.push_str("\n## 贴图概括（非像素）\n");
        for t in &texture_summaries {
            text.push_str(&format!(
                "{} {}x{} avg=({},{},{})\n",
                t.path, t.width, t.height, t.avg_rgb[0], t.avg_rgb[1], t.avg_rgb[2]
            ));
        }
    }

    // 探针 JSON 仅在显式请求时附带（隐私档位表未包含默认探针）
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

/// OpenAI 兼容 `chat/completions`（阻塞 reqwest；在 async 命令里 spawn_blocking）。
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
    let url = format!(
        "{}/chat/completions",
        cfg.base_url.trim_end_matches('/')
    );
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
        // 不回显 key；仅状态与截断 body
        let snippet: String = text.chars().take(300).collect();
        return Err(format!("http {status}: {snippet}"));
    }
    let parsed: ChatCompletion = resp
        .json()
        .map_err(|e| format!("decode response: {e}"))?;
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
    }

    #[test]
    fn tier3_includes_selected_json() {
        let arc = open_pack_bytes(&zip_bytes(), &SafeLimits::default()).unwrap();
        let rom = build(&arc, "p.zip");
        let sel = vec!["assets/minecraft/models/item/a.json".to_string()];
        let p = build_tier_payload(&rom, 3, &sel, "{}", false);
        assert_eq!(p.attachments.len(), 1);
        assert!(p.preview.iter().any(|x| x.starts_with("json:")));
    }

    #[test]
    fn redact_hides_key() {
        let s = "Authorization Bearer sk-abc123456789";
        let r = redact_for_log(s, &["sk-abc123456789"]);
        assert!(r.contains("***"));
        assert!(!r.contains("sk-abc123456789"));
    }
}
