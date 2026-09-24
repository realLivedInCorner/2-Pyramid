//! Resource-pack Object Model — 仅 Foray 使用的内存对象树。

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use super::mcmeta::PackMeta;
use super::zip_safe::{SafeArchive, SafeEntry};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    Texture,
    Model,
    Shader,
    Json,
    Sound,
    Lang,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParseStatus {
    Ok,
    Warn,
    Error,
    Skipped,
}

#[derive(Debug, Clone, Serialize)]
pub struct RomFile {
    pub path: String,
    pub kind: FileKind,
    pub size: u64,
    pub sha256: String,
    pub parse: ParseStatus,
    pub paintable: bool,
    pub dirty: bool,
    #[serde(skip)]
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RomDir {
    pub name: String,
    pub path: String,
    pub dirs: Vec<RomDir>,
    pub files: Vec<RomFile>,
}

impl RomDir {
    fn new(name: &str, path: &str) -> Self {
        Self {
            name: name.to_string(),
            path: path.to_string(),
            dirs: Vec::new(),
            files: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct KindStat {
    pub kind: String,
    pub count: usize,
    pub bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct RomStats {
    pub files: usize,
    pub bytes: u64,
    pub by_kind: Vec<KindStat>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueLevel {
    Info,
    Warn,
    Danger,
}

#[derive(Debug, Clone, Serialize)]
pub struct RomIssue {
    pub level: IssueLevel,
    pub source: String,
    pub path: String,
    pub message: String,
}

pub struct Rom {
    pub meta: PackMeta,
    pub icon: Option<RomFile>,
    pub root: RomDir,
    pub stats: RomStats,
    pub issues: Vec<RomIssue>,
    pub source_path: String,
}

impl Rom {
    pub fn find_file(&self, path: &str) -> Option<&RomFile> {
        find_file_in(&self.root, path)
    }

    pub fn find_file_mut(&mut self, path: &str) -> Option<&mut RomFile> {
        find_file_mut_in(&mut self.root, path)
    }
}

fn find_file_in<'a>(dir: &'a RomDir, path: &str) -> Option<&'a RomFile> {
    for f in &dir.files {
        if f.path == path {
            return Some(f);
        }
    }
    for d in &dir.dirs {
        if let Some(f) = find_file_in(d, path) {
            return Some(f);
        }
    }
    None
}

fn find_file_mut_in<'a>(dir: &'a mut RomDir, path: &str) -> Option<&'a mut RomFile> {
    for f in &mut dir.files {
        if f.path == path {
            return Some(f);
        }
    }
    for d in &mut dir.dirs {
        if let Some(f) = find_file_mut_in(d, path) {
            return Some(f);
        }
    }
    None
}

pub fn classify(path: &str) -> FileKind {
    let lower = path.to_ascii_lowercase();
    let ext = Path::new(&lower)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if matches!(ext, "png" | "tga" | "jpg" | "jpeg" | "webp") {
        return FileKind::Texture;
    }
    if matches!(ext, "vsh" | "fsh" | "glsl" | "json")
        && (lower.contains("/shaders/") || lower.contains("\\shaders\\"))
    {
        if ext == "json" {
            return FileKind::Shader;
        }
        return FileKind::Shader;
    }
    if lower.contains("/sounds/") || ext == "ogg" || ext == "wav" {
        return FileKind::Sound;
    }
    if lower.contains("/lang/") {
        return FileKind::Lang;
    }
    if lower.contains("/models/") || lower.contains("/blockstates/") {
        return FileKind::Model;
    }
    if ext == "json" {
        return FileKind::Json;
    }
    FileKind::Other
}

fn is_paintable_png(entry: &SafeEntry) -> bool {
    if !entry.path.to_ascii_lowercase().ends_with(".png") {
        return false;
    }
    // 最小 PNG 签名
    entry.data.len() > 8 && entry.data.starts_with(&[0x89, b'P', b'N', b'G'])
}

fn check_png_dims(data: &[u8]) -> Result<(u32, u32), String> {
    // IHDR: 16..24 width/height big-endian after signature+len+type
    if data.len() < 24 {
        return Err("png truncated".into());
    }
    if &data[12..16] != b"IHDR" {
        return Err("png missing IHDR".into());
    }
    let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    if w == 0 || h == 0 || w > 8192 || h > 8192 {
        return Err(format!("png dims out of range: {w}x{h}"));
    }
    Ok((w, h))
}

/// 扫描 PNG 辅助块：超大 / 可疑 iCCP、tEXt、zTXt 给出告警（规格 S2.3）。
pub fn scan_png_aux_chunks(data: &[u8]) -> Vec<String> {
    let mut warns = Vec::new();
    if data.len() < 8 || !data.starts_with(&[0x89, b'P', b'N', b'G']) {
        return warns;
    }
    let mut i = 8usize;
    while i + 8 <= data.len() {
        let len = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
        let typ = &data[i + 4..i + 8];
        let typ_s = String::from_utf8_lossy(typ).to_string();
        let data_start = i + 8;
        let data_end = data_start.saturating_add(len);
        if data_end + 4 > data.len() {
            warns.push(format!("png chunk {typ_s} truncated"));
            break;
        }
        if matches!(typ_s.as_str(), "iCCP" | "zTXt" | "tEXt" | "iTXt") && len > 64 * 1024 {
            warns.push(format!("png chunk {typ_s} unusually large ({len} bytes)"));
        }
        if typ_s == "iCCP" && len > 0 {
            // 压缩 ICC 配置过大也告警
            if len > 256 * 1024 {
                warns.push("iCCP profile very large".to_string());
            }
        }
        // IEND
        if typ_s == "IEND" {
            break;
        }
        i = data_end + 4;
        if warns.len() >= 8 {
            break;
        }
    }
    warns
}

fn insert_file(dir: &mut RomDir, file: RomFile) {
    let path = file.path.clone();
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() == 1 {
        dir.files.push(file);
        return;
    }
    // 创建/进入子目录链
    let mut cur = dir;
    let mut acc = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            cur.files.push(file);
            return;
        }
        if !acc.is_empty() {
            acc.push('/');
        }
        acc.push_str(part);
        let idx = match cur.dirs.iter().position(|d| d.name == *part) {
            Some(i) => i,
            None => {
                cur.dirs.push(RomDir::new(part, &acc));
                cur.dirs.len() - 1
            }
        };
        cur = &mut cur.dirs[idx];
    }
}

fn kind_name(k: FileKind) -> &'static str {
    match k {
        FileKind::Texture => "texture",
        FileKind::Model => "model",
        FileKind::Shader => "shader",
        FileKind::Json => "json",
        FileKind::Sound => "sound",
        FileKind::Lang => "lang",
        FileKind::Other => "other",
    }
}

/// 从安全解压结果构建 ROM。
pub fn build(archive: &SafeArchive, source_path: &str) -> Rom {
    let mut meta = PackMeta::default();
    let mut icon = None;
    let mut root = RomDir::new("", "");
    let mut issues = Vec::new();
    let mut kind_bytes: BTreeMap<&'static str, (usize, u64)> = BTreeMap::new();

    for e in &archive.entries {
        if e.path == "pack.mcmeta" {
            match super::mcmeta::parse_mcmeta(&e.data) {
                Ok(m) => {
                    if let Some(w) = m.warn.clone() {
                        issues.push(RomIssue {
                            level: IssueLevel::Warn,
                            source: "parse".into(),
                            path: e.path.clone(),
                            message: w,
                        });
                    }
                    meta = m;
                }
                Err(err) => {
                    issues.push(RomIssue {
                        level: IssueLevel::Danger,
                        source: "parse".into(),
                        path: e.path.clone(),
                        message: err,
                    });
                    meta.raw = String::from_utf8_lossy(&e.data).to_string();
                }
            }
            continue;
        }

        let kind = classify(&e.path);
        let paintable = is_paintable_png(e);
        let mut parse = ParseStatus::Ok;
        if e.path.to_ascii_lowercase().ends_with(".png") {
            if let Err(msg) = check_png_dims(&e.data) {
                parse = ParseStatus::Error;
                issues.push(RomIssue {
                    level: IssueLevel::Danger,
                    source: "parse".into(),
                    path: e.path.clone(),
                    message: msg,
                });
            }
            for w in scan_png_aux_chunks(&e.data) {
                issues.push(RomIssue {
                    level: IssueLevel::Warn,
                    source: "parse".into(),
                    path: e.path.clone(),
                    message: w,
                });
            }
        } else if e.path.to_ascii_lowercase().ends_with(".json") {
            if serde_json::from_slice::<serde_json::Value>(&e.data).is_err() {
                parse = ParseStatus::Warn;
                issues.push(RomIssue {
                    level: IssueLevel::Warn,
                    source: "parse".into(),
                    path: e.path.clone(),
                    message: "json parse failed".into(),
                });
            }
        } else if e.data.is_empty() {
            parse = ParseStatus::Warn;
            issues.push(RomIssue {
                level: IssueLevel::Warn,
                source: "parse".into(),
                path: e.path.clone(),
                message: "empty file".into(),
            });
        }

        let file = RomFile {
            path: e.path.clone(),
            kind,
            size: e.size,
            sha256: e.sha256.clone(),
            parse,
            paintable,
            dirty: false,
            data: e.data.clone(),
        };

        if e.path == "pack.png" {
            icon = Some(file);
        } else {
            let slot = kind_bytes.entry(kind_name(kind)).or_insert((0, 0));
            slot.0 += 1;
            slot.1 += e.size;
            insert_file(&mut root, file);
        }
    }

    let mut by_kind: Vec<KindStat> = kind_bytes
        .into_iter()
        .map(|(kind, (count, bytes))| KindStat {
            kind: kind.to_string(),
            count,
            bytes,
        })
        .collect();
    by_kind.sort_by(|a, b| b.bytes.cmp(&a.bytes));

    let stats = RomStats {
        files: archive.entries.iter().filter(|e| e.path != "pack.mcmeta" && e.path != "pack.png").count()
            + usize::from(icon.is_some()),
        bytes: archive.entries.iter().map(|e| e.size).sum(),
        by_kind,
    };

    if meta.description.is_empty() && meta.pack_format.is_none() && meta.min_format.is_none() {
        issues.push(RomIssue {
            level: IssueLevel::Warn,
            source: "rom".into(),
            path: "pack.mcmeta".into(),
            message: "missing pack.mcmeta content".into(),
        });
    }

    Rom {
        meta,
        icon,
        root,
        stats,
        issues,
        source_path: source_path.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foray::zip_safe::{open_pack_bytes, SafeLimits};

    fn zip_bytes(files: &[(&str, &[u8])]) -> Vec<u8> {
        use std::io::Write;
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            let o = zip::write::FileOptions::default();
            for (n, d) in files {
                z.start_file(*n, o).unwrap();
                z.write_all(d).unwrap();
            }
            z.finish().unwrap();
        }
        buf.into_inner()
    }

    fn tiny_png() -> Vec<u8> {
        // 1x1 PNG
        vec![
            0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R',
            0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0, 0x90, 0x77, 0x53, 0xDE,
        ]
    }

    #[test]
    fn builds_tree_and_stats() {
        let png = tiny_png();
        let bytes = zip_bytes(&[
            ("pack.mcmeta", br#"{"pack":{"description":"t","pack_format":34}}"#),
            ("pack.png", &png),
            ("assets/minecraft/textures/block/stone.png", &png),
            ("assets/minecraft/models/item/stick.json", br#"{"parent":"item/generated"}"#),
        ]);
        let arc = open_pack_bytes(&bytes, &SafeLimits::default()).unwrap();
        let rom = build(&arc, "mem.zip");
        assert_eq!(rom.meta.pack_format, Some(34));
        assert!(rom.icon.is_some());
        assert!(rom.find_file("assets/minecraft/textures/block/stone.png").is_some());
        assert!(rom.find_file("assets/minecraft/models/item/stick.json").is_some());
        assert!(rom.stats.files >= 3);
    }

    #[test]
    fn classify_kinds() {
        assert_eq!(classify("assets/minecraft/textures/a.png"), FileKind::Texture);
        assert_eq!(classify("assets/minecraft/shaders/core/x.vsh"), FileKind::Shader);
        assert_eq!(classify("assets/minecraft/models/item/a.json"), FileKind::Model);
    }
}
