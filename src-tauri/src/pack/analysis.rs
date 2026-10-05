//! 资源包**结构分析**（只读）。
//!
//! 目的：在决定「多版本资源包怎么转换」之前，先把包的真实结构解析出来。
//! 本模块不修改任何文件、不参与转换，只回答几个问题：
//!
//!   1. 这个包是单版本，还是**分层**（官方 `overlays`）？
//!   2. 它是否声明了 `supported_formats` 区间（"一份内容适用多版本"）？
//!   3. 是否存在**非标准的版本折叠目录**（如 `textures/item/1.20/…`、
//!      `assets_1_21/…`）——第三方多版本包的常见做法？
//!   4. 一个压缩包里是不是塞了**多个包根**（多合一打包）？
//!   5. 每个覆盖层实际有多少文件、其中多少会**覆盖基础层同路径文件**
//!      （覆盖计数决定"压平"时信息损失有多大）。
//!
//! 输出 `PackAnalysis` 既可序列化给前端/测试查看，也能打一行摘要进日志，
//! 便于拿真实包做验证后再定转换语义。

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::Path;

use serde::Serialize;

/// 包结构形态（可同时命中多个）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PackShape {
    /// 单层单版本：只有 `pack.pack_format`。
    SingleVersion,
    /// 官方分层：`overlays.entries[]` 指向若干覆盖层目录。
    OverlayLayers,
    /// 声明了 `supported_formats` 区间（同内容适用多版本）。
    FormatRange,
    /// 非标准版本折叠目录（目录名含版本号）。
    FoldingDirectories,
    /// 一个压缩包里存在多个包根。
    MultiRootPack,
}

impl PackShape {
    pub fn as_str(self) -> &'static str {
        match self {
            PackShape::SingleVersion => "single-version",
            PackShape::OverlayLayers => "overlay-layers",
            PackShape::FormatRange => "format-range",
            PackShape::FoldingDirectories => "folding-dirs",
            PackShape::MultiRootPack => "multi-root",
        }
    }
}

/// 一层（基础层或覆盖层 / 折叠目录）的统计。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerInfo {
    /// 相对包根的目录（基础层为 `""`）。
    pub directory: String,
    /// 声明适用的 pack_format（折叠目录为空）。
    pub formats: Vec<u32>,
    /// 目录是否真实存在于压缩包内。
    pub exists: bool,
    /// 该层文件数。
    pub file_count: usize,
    /// 与基础层同路径（会覆盖基础层）的文件数。
    pub override_count: usize,
    /// 覆盖示例（最多 5 条，便于人工判断冲突性质）。
    pub override_preview: Vec<String>,
}

/// 分析结果。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackAnalysis {
    /// 被分析的对象（zip 路径）。
    pub source: String,
    /// 命中的形态（按判定顺序）。
    pub shapes: Vec<PackShape>,
    /// `pack.pack_format`（若有）。
    pub pack_format: Option<u32>,
    /// `supported_formats` 原文（数字或区间），用于人工核对。
    pub supported_formats: Option<String>,
    /// 包根在压缩包内的前缀（嵌套目录时为 `xxx/`，根目录为 `""`）。
    pub root_prefix: String,
    /// 其余候选包根（多根包时非空）。
    pub multi_roots: Vec<String>,
    /// 官方 overlays 分层（含基础层，索引 0 为基础层）。
    pub layers: Vec<LayerInfo>,
    /// 非标准版本折叠目录。
    pub folding_dirs: Vec<LayerInfo>,
    /// 压缩包内文件总数。
    pub total_files: usize,
    /// 需要人工注意的问题。
    pub warnings: Vec<String>,
}

impl PackAnalysis {
    /// 一行摘要（写日志用）。
    pub fn summary(&self) -> String {
        let shapes = self
            .shapes
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("+");
        let overlays = self.layers.len().saturating_sub(1);
        format!(
            "shape={} format={:?} root={:?} overlays={} folding={} files={}{}",
            shapes,
            self.pack_format,
            self.root_prefix,
            overlays,
            self.folding_dirs.len(),
            self.total_files,
            if self.warnings.is_empty() {
                String::new()
            } else {
                format!(" warnings={}", self.warnings.len())
            }
        )
    }
}

/// 版本样式目录名：`1.20` / `1.20.4` / `1_20` / `1_20_4` / `v1.21` / `mc1.21` /
/// `overlay_1_21` / `assets_1_20` / `1.21+` 等。
fn looks_like_version_dir(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let core = lower
        .trim_start_matches("overlay_")
        .trim_start_matches("assets_")
        .trim_start_matches("mc")
        .trim_start_matches('v')
        .trim_end_matches('+');
    // 归一化 "1_20_4" → "1.20.4"
    let dotted = core.replace('_', ".");
    let mut parts = dotted.split('.');
    let Some(major) = parts.next() else { return false };
    if major != "1" {
        return false;
    }
    let Some(minor) = parts.next() else { return false };
    if minor.is_empty() || !minor.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let rest: Vec<&str> = parts.collect();
    // 只接受 1.x / 1.x.y（再多段就不像 MC 版本目录了，例如 1.2.3.4.5.6）
    if rest.len() > 1 {
        return false;
    }
    if rest.iter().any(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_digit())) {
        return false;
    }
    // 段值合理性：minor / patch 不超过两位数（1.20 / 1.21.4 …）
    let minor_num: u32 = minor.parse().unwrap_or(u32::MAX);
    if minor_num > 99 {
        return false;
    }
    if let Some(patch) = rest.first() {
        if patch.parse::<u32>().unwrap_or(u32::MAX) > 99 {
            return false;
        }
    }
    true
}

/// 从 zip 读取一次目录清单，然后分析（不解压内容，只读 pack.mcmeta 正文）。
pub fn analyze_zip(zip_path: &Path) -> Result<PackAnalysis, String> {
    let file = std::fs::File::open(zip_path)
        .map_err(|e| format!("failed to open {}: {}", zip_path.display(), e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("failed to read zip: {}", e))?;

    let mut names: Vec<String> = Vec::with_capacity(archive.len());
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| format!("failed to read zip entry {}: {}", i, e))?;
        if entry.is_dir() {
            continue;
        }
        names.push(entry.name().replace('\\', "/"));
    }

    // pack.mcmeta 正文按需从压缩包读取（只读这一个文件）
    let source = zip_path.to_string_lossy().to_string();
    analyze_with(&source, names, |meta_name| {
        let mut content = String::new();
        match archive.by_name(meta_name) {
            Ok(mut entry) => {
                let _ = entry.read_to_string(&mut content);
                Some(content)
            }
            Err(_) => None,
        }
    })
}

/// 对**已解压**的包目录做同样的分析（转换流程内部记录日志用，或人工排查用）。
pub fn analyze_dir(dir: &Path) -> Result<PackAnalysis, String> {
    let mut names: Vec<String> = Vec::new();
    for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() {
            if let Ok(rel) = entry.path().strip_prefix(dir) {
                names.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let source = dir.to_string_lossy().to_string();
    analyze_with(&source, names, |meta_name| {
        std::fs::read_to_string(dir.join(meta_name)).ok()
    })
}

/// 共用判定核心：`names` 为相对路径集合，`read_meta` 按需提供 pack.mcmeta 正文。
fn analyze_with<F>(source: &str, names: Vec<String>, mut read_meta: F) -> Result<PackAnalysis, String>
where
    F: FnMut(&str) -> Option<String>,
{
    // ── 定位包根（pack.mcmeta 所在目录） ─────────────────────────
    let mut meta_dirs: BTreeSet<String> = BTreeSet::new();
    for n in &names {
        if n.ends_with("pack.mcmeta") {
            let prefix = n.trim_end_matches("pack.mcmeta").trim_end_matches('/');
            meta_dirs.insert(prefix.to_string());
        }
    }

    let mut warnings: Vec<String> = Vec::new();
    let root_prefix = match meta_dirs.iter().min_by_key(|d| d.matches('/').count()) {
        Some(d) => d.clone(),
        None => {
            warnings.push("未找到 pack.mcmeta".to_string());
            String::new()
        }
    };
    let multi_roots: Vec<String> = meta_dirs
        .iter()
        .filter(|d| **d != root_prefix)
        .cloned()
        .collect();

    let root = if root_prefix.is_empty() {
        String::new()
    } else {
        format!("{}/", root_prefix)
    };

    // ── 读 pack.mcmeta 正文 ────────────────────────────────────
    let meta_name = format!("{}pack.mcmeta", root);
    let (pack_format, supported_formats, overlay_entries) = match read_meta(&meta_name) {
        Some(content) => parse_meta(&content, &mut warnings),
        None => (None, None, Vec::new()),
    };

    // ── 路径集合（相对包根） ───────────────────────────────────
    let rel_files: Vec<String> = names
        .iter()
        .filter_map(|n| n.strip_prefix(&root).map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect();
    let total_files = rel_files.len();

    let overlay_dirs: BTreeSet<String> = overlay_entries
        .iter()
        .map(|(dir, _)| dir.trim_matches('/').to_string())
        .collect();

    let is_under_overlay = |rel: &str| -> bool {
        overlay_dirs
            .iter()
            .any(|d| !d.is_empty() && (rel == d || rel.starts_with(&format!("{}/", d))))
    };

    let base_files: BTreeSet<String> = rel_files
        .iter()
        .filter(|rel| !is_under_overlay(rel) && !rel.ends_with("pack.mcmeta"))
        .cloned()
        .collect();

    // ── 逐层统计 ──────────────────────────────────────────────
    let mut layers: Vec<LayerInfo> = Vec::new();
    layers.push(LayerInfo {
        directory: String::new(),
        formats: pack_format.into_iter().collect(),
        exists: true,
        file_count: base_files.len(),
        override_count: 0,
        override_preview: Vec::new(),
    });

    for (dir, formats) in &overlay_entries {
        let dir_clean = dir.trim_matches('/').to_string();
        let prefix = format!("{}/", dir_clean);
        let mut file_count = 0usize;
        let mut override_count = 0usize;
        let mut preview: Vec<String> = Vec::new();
        for rel in &rel_files {
            if let Some(inner) = rel.strip_prefix(&prefix) {
                if inner.is_empty() {
                    continue;
                }
                file_count += 1;
                if base_files.contains(inner) {
                    override_count += 1;
                    if preview.len() < 5 {
                        preview.push(inner.to_string());
                    }
                }
            }
        }
        if file_count == 0 {
            warnings.push(format!("覆盖层目录不存在或为空：{}", dir_clean));
        }
        if override_count > 0 {
            warnings.push(format!(
                "覆盖层 {} 会覆盖基础层 {} 个文件（压平将丢失基础层对应内容）",
                dir_clean, override_count
            ));
        }
        layers.push(LayerInfo {
            directory: dir_clean,
            formats: formats.clone(),
            exists: file_count > 0,
            file_count,
            override_count,
            override_preview: preview,
        });
    }

    // ── 非标准「版本折叠目录」检测 ─────────────────────────────
    let mut folding: BTreeMap<String, usize> = BTreeMap::new();
    for rel in &rel_files {
        let parts: Vec<&str> = rel.split('/').collect();
        for (idx, seg) in parts.iter().enumerate() {
            if idx == parts.len() - 1 {
                continue;
            }
            if looks_like_version_dir(seg) {
                *folding.entry(parts[..=idx].join("/")).or_insert(0) += 1;
            }
        }
    }
    let folding_dirs: Vec<LayerInfo> = folding
        .into_iter()
        .map(|(dir, count)| LayerInfo {
            directory: dir,
            formats: Vec::new(),
            exists: true,
            file_count: count,
            override_count: 0,
            override_preview: Vec::new(),
        })
        .collect();
    if folding_dirs.len() > 20 {
        warnings.push(format!(
            "疑似版本折叠目录过多（{} 个），可能是常规目录名被误判，请人工核对",
            folding_dirs.len()
        ));
    }

    // ── 形态判定 ──────────────────────────────────────────────
    let mut shapes: Vec<PackShape> = Vec::new();
    if !multi_roots.is_empty() {
        shapes.push(PackShape::MultiRootPack);
    }
    if !overlay_entries.is_empty() {
        shapes.push(PackShape::OverlayLayers);
    }
    if supported_formats.is_some() {
        shapes.push(PackShape::FormatRange);
    }
    if !folding_dirs.is_empty() {
        shapes.push(PackShape::FoldingDirectories);
    }
    if shapes.is_empty() {
        shapes.push(PackShape::SingleVersion);
    }

    if supported_formats.is_some() && overlay_entries.is_empty() {
        warnings.push(
            "包声明 supported_formats 区间但只有一份内容：转换后需按目标版本改写/清除该声明"
                .to_string(),
        );
    }
    if !overlay_entries.is_empty() {
        warnings.push(
            "当前转换管线只处理基础层，覆盖层目录会原样保留（本分析仅为决策提供依据）"
                .to_string(),
        );
    }

    Ok(PackAnalysis {
        source: source.to_string(),
        shapes,
        pack_format,
        supported_formats,
        root_prefix,
        multi_roots,
        layers,
        folding_dirs,
        total_files,
        warnings,
    })
}

/// 解析 pack.mcmeta：返回 (pack_format, supported_formats 原文, overlays 条目)。
fn parse_meta(
    content: &str,
    warnings: &mut Vec<String>,
) -> (Option<u32>, Option<String>, Vec<(String, Vec<u32>)>) {
    let data: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(e) => {
            warnings.push(format!("pack.mcmeta 解析失败：{}", e));
            return (None, None, Vec::new());
        }
    };

    let pack_format = data
        .get("pack")
        .and_then(|p| p.get("pack_format"))
        .and_then(|v| {
            v.as_u64()
                .map(|n| n as u32)
                .or_else(|| v.as_str().and_then(|s| s.trim().parse::<u32>().ok()))
        });

    let supported_formats = data
        .get("pack")
        .and_then(|p| p.get("supported_formats"))
        .map(|v| {
            if let Some(arr) = v.as_array() {
                let nums: Vec<String> = arr
                    .iter()
                    .map(|x| x.to_string())
                    .collect();
                format!("[{}]", nums.join(","))
            } else {
                v.to_string()
            }
        });

    let mut overlays: Vec<(String, Vec<u32>)> = Vec::new();
    if let Some(entries) = data
        .get("overlays")
        .and_then(|o| o.get("entries"))
        .and_then(|e| e.as_array())
    {
        for (idx, entry) in entries.iter().enumerate() {
            let Some(dir) = entry.get("directory").and_then(|d| d.as_str()) else {
                warnings.push(format!("overlays.entries[{}] 缺少 directory 字段", idx));
                continue;
            };
            let mut formats: Vec<u32> = Vec::new();
            if let Some(f) = entry.get("formats") {
                if let Some(arr) = f.as_array() {
                    for item in arr {
                        if let Some(n) = item.as_u64() {
                            formats.push(n as u32);
                        } else if let Some(s) = item.as_str() {
                            if let Ok(n) = s.trim().parse::<u32>() {
                                formats.push(n);
                            }
                        }
                    }
                } else if let Some(n) = f.as_u64() {
                    formats.push(n as u32);
                }
            }
            if formats.is_empty() {
                warnings.push(format!("覆盖层 {} 未声明 formats", dir));
            }
            overlays.push((dir.to_string(), formats));
        }
    }

    (pack_format, supported_formats, overlays)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::FileOptions;

    fn write_zip(path: &Path, files: &[(&str, &str)]) {
        let file = std::fs::File::create(path).expect("create zip");
        let mut zip = zip::ZipWriter::new(file);
        let opts = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, body) in files {
            zip.start_file(*name, opts).expect("start_file");
            zip.write_all(body.as_bytes()).expect("write");
        }
        zip.finish().expect("finish");
    }

    fn temp_zip(name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(name);
        (dir, path)
    }

    #[test]
    fn detects_single_version_pack() {
        let (_d, zip_path) = temp_zip("single.zip");
        write_zip(
            &zip_path,
            &[
                ("pack.mcmeta", r#"{"pack":{"pack_format":34,"description":"x"}}"#),
                ("assets/minecraft/textures/item/apple.png", "x"),
            ],
        );
        let a = analyze_zip(&zip_path).expect("analyze");
        assert_eq!(a.shapes, vec![PackShape::SingleVersion]);
        assert_eq!(a.pack_format, Some(34));
        assert_eq!(a.total_files, 2);
        assert!(a.layers[0].file_count == 1, "base layer should count 1 file");
    }

    #[test]
    fn detects_overlay_layers_with_override_counts() {
        let (_d, zip_path) = temp_zip("overlay.zip");
        write_zip(
            &zip_path,
            &[
                (
                    "pack.mcmeta",
                    r#"{"pack":{"pack_format":15,"description":"x"},
                        "overlays":{"entries":[
                          {"formats":[34],"directory":"overlay_34"},
                          {"formats":[46],"directory":"overlay_46"}]}}"#,
                ),
                ("assets/minecraft/textures/item/apple.png", "base"),
                ("overlay_34/assets/minecraft/textures/item/apple.png", "new"),
                ("overlay_34/assets/minecraft/textures/item/extra.png", "new"),
                ("overlay_46/assets/minecraft/textures/item/apple.png", "newer"),
            ],
        );
        let a = analyze_zip(&zip_path).expect("analyze");
        assert!(a.shapes.contains(&PackShape::OverlayLayers));
        assert_eq!(a.layers.len(), 3, "base + 2 overlays");
        let o34 = a.layers.iter().find(|l| l.directory == "overlay_34").unwrap();
        assert_eq!(o34.file_count, 2);
        assert_eq!(o34.override_count, 1, "apple.png overrides base");
        assert_eq!(o34.formats, vec![34]);
        let o46 = a.layers.iter().find(|l| l.directory == "overlay_46").unwrap();
        assert_eq!(o46.override_count, 1);
        assert!(a.warnings.iter().any(|w| w.contains("覆盖层")));
    }

    #[test]
    fn detects_supported_formats_range() {
        let (_d, zip_path) = temp_zip("range.zip");
        write_zip(
            &zip_path,
            &[
                (
                    "pack.mcmeta",
                    r#"{"pack":{"pack_format":15,"supported_formats":[15,34],"description":"x"}}"#,
                ),
                ("assets/minecraft/textures/item/apple.png", "x"),
            ],
        );
        let a = analyze_zip(&zip_path).expect("analyze");
        assert!(a.shapes.contains(&PackShape::FormatRange));
        assert_eq!(a.supported_formats.as_deref(), Some("[15,34]"));
        assert!(a.warnings.iter().any(|w| w.contains("supported_formats")));
    }

    #[test]
    fn detects_folding_directories() {
        let (_d, zip_path) = temp_zip("fold.zip");
        write_zip(
            &zip_path,
            &[
                ("pack.mcmeta", r#"{"pack":{"pack_format":34,"description":"x"}}"#),
                ("assets/minecraft/textures/item/1.20/apple.png", "x"),
                ("assets/minecraft/textures/item/1.21/apple.png", "x"),
                ("assets/minecraft/textures/item/root.png", "x"),
            ],
        );
        let a = analyze_zip(&zip_path).expect("analyze");
        assert!(a.shapes.contains(&PackShape::FoldingDirectories));
        let dirs: Vec<&str> = a.folding_dirs.iter().map(|l| l.directory.as_str()).collect();
        assert!(dirs.contains(&"assets/minecraft/textures/item/1.20"), "{:?}", dirs);
        assert!(dirs.contains(&"assets/minecraft/textures/item/1.21"), "{:?}", dirs);
    }

    #[test]
    fn detects_multi_root_archive() {
        let (_d, zip_path) = temp_zip("multi.zip");
        write_zip(
            &zip_path,
            &[
                ("PackA/pack.mcmeta", r#"{"pack":{"pack_format":15}}"#),
                ("PackA/assets/minecraft/textures/item/a.png", "x"),
                ("PackB/pack.mcmeta", r#"{"pack":{"pack_format":34}}"#),
                ("PackB/assets/minecraft/textures/item/b.png", "x"),
            ],
        );
        let a = analyze_zip(&zip_path).expect("analyze");
        assert!(a.shapes.contains(&PackShape::MultiRootPack));
        assert_eq!(a.root_prefix, "PackA");
        assert_eq!(a.multi_roots, vec!["PackB".to_string()]);
    }

    #[test]
    fn version_dir_heuristics() {
        for ok in ["1.20", "1_20_4", "v1.21", "mc1.21", "overlay_1_21", "assets_1_20", "1.21+"] {
            assert!(looks_like_version_dir(ok), "{ok} should look like a version dir");
        }
        for no in ["item", "textures", "blocks", "1.2.3.4.5.6", "model", "gui"] {
            assert!(!looks_like_version_dir(no), "{no} should NOT look like a version dir");
        }
    }
}
