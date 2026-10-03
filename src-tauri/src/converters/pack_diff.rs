//! 输出对比（黄金样本 diff）——提速改动的质量闸门。
//!
//! 用途：同一输入包，用优化前/优化后各转一次，然后比较两个产物是否
//! **内容等价**。判定分级：
//!
//!   * `identical`          —— 字节完全相同；
//!   * `encodingOnly`       —— PNG 像素完全相同，只是编码字节不同（可接受的提速副作用）；
//!   * `jsonEquivalent`     —— JSON 语义相同（键序/空白不同）；
//!   * `contentDiff`        —— **内容真的不同**（像素/JSON 语义/其他字节）→ 闸门失败；
//!   * `onlyInA` / `onlyInB`—— 文件只存在于一侧。
//!
//! CLI：`2-pyramid.exe --pack-diff <A> <B> [--strict] [--json <file>]`
//! 退出码 0 = 无内容差异（`--strict` 下要求字节级完全相同），1 = 有内容差异。

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DiffKind {
    Identical,
    EncodingOnly,
    JsonEquivalent,
    ContentDiff,
    OnlyInA,
    OnlyInB,
}

impl DiffKind {
    pub fn is_blocking(self) -> bool {
        matches!(self, DiffKind::ContentDiff | DiffKind::OnlyInA | DiffKind::OnlyInB)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            DiffKind::Identical => "identical",
            DiffKind::EncodingOnly => "encoding-only",
            DiffKind::JsonEquivalent => "json-equivalent",
            DiffKind::ContentDiff => "CONTENT-DIFF",
            DiffKind::OnlyInA => "only-in-A",
            DiffKind::OnlyInB => "only-in-B",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub path: String,
    pub kind: DiffKind,
    /// 额外说明（像素差异数量、尺寸不同、JSON 差异摘要等）。
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackDiffReport {
    pub a: String,
    pub b: String,
    pub files_a: usize,
    pub files_b: usize,
    /// 完全一致（含仅编码差异）的文件数。
    pub identical_or_equivalent: usize,
    /// 内容有实质差异或缺失的文件数。
    pub blocking: usize,
    pub diffs: Vec<FileDiff>,
}

impl PackDiffReport {
    pub fn passed(&self, strict: bool) -> bool {
        if strict {
            // 严格模式：要求每一个文件都是 identical
            self.diffs.iter().all(|d| d.kind == DiffKind::Identical)
        } else {
            self.blocking == 0
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "A={} files, B={} files, blocking={}, diffs={}",
            self.files_a,
            self.files_b,
            self.blocking,
            self.diffs.len()
        )
    }
}

/// 读取 zip（或目录）为 `相对路径 → 内容`（内存里比对，样本包通常不大；
/// 超大包请用 `--strict` 之外的场景自行分批）。
fn read_container(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut out = BTreeMap::new();
    if root.is_dir() {
        for entry in walkdir::WalkDir::new(root).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = std::fs::read(entry.path()).map_err(|e| format!("read {}: {}", rel, e))?;
            out.insert(rel, bytes);
        }
        return Ok(out);
    }

    let file = std::fs::File::open(root).map_err(|e| format!("open {}: {}", root.display(), e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("read zip: {}", e))?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| format!("entry {}: {}", i, e))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().replace('\\', "/");
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry
            .read_to_end(&mut buf)
            .map_err(|e| format!("read {}: {}", name, e))?;
        out.insert(name, buf);
    }
    Ok(out)
}

/// 比较两个产物（zip 或目录）。
pub fn diff_containers(a: &Path, b: &Path) -> Result<PackDiffReport, String> {
    let map_a = read_container(a)?;
    let map_b = read_container(b)?;

    let mut diffs: Vec<FileDiff> = Vec::new();
    let mut blocking = 0usize;
    let mut equivalent = 0usize;

    // A 侧文件
    for (path, bytes_a) in &map_a {
        match map_b.get(path) {
            None => {
                diffs.push(FileDiff {
                    path: path.clone(),
                    kind: DiffKind::OnlyInA,
                    detail: None,
                });
                blocking += 1;
            }
            Some(bytes_b) => {
                let (kind, detail) = compare_file(path, bytes_a, bytes_b);
                if kind.is_blocking() {
                    blocking += 1;
                } else {
                    equivalent += 1;
                }
                if kind != DiffKind::Identical {
                    diffs.push(FileDiff {
                        path: path.clone(),
                        kind,
                        detail,
                    });
                }
            }
        }
    }
    // B 侧多出来的文件
    for path in map_b.keys() {
        if !map_a.contains_key(path) {
            diffs.push(FileDiff {
                path: path.clone(),
                kind: DiffKind::OnlyInB,
                detail: None,
            });
            blocking += 1;
        }
    }

    Ok(PackDiffReport {
        a: a.to_string_lossy().to_string(),
        b: b.to_string_lossy().to_string(),
        files_a: map_a.len(),
        files_b: map_b.len(),
        identical_or_equivalent: equivalent,
        blocking,
        diffs,
    })
}

/// 单文件比较：PNG 走像素级，JSON 走语义级，其余字节级。
fn compare_file(path: &str, a: &[u8], b: &[u8]) -> (DiffKind, Option<String>) {
    let lower = path.to_ascii_lowercase();

    if a == b {
        return (DiffKind::Identical, None);
    }

    if lower.ends_with(".png") {
        return compare_png(a, b);
    }
    if lower.ends_with(".json") || lower.ends_with(".mcmeta") {
        let ja: Result<serde_json::Value, _> = serde_json::from_slice(a);
        let jb: Result<serde_json::Value, _> = serde_json::from_slice(b);
        return match (ja, jb) {
            (Ok(va), Ok(vb)) => {
                if va == vb {
                    (DiffKind::JsonEquivalent, Some("JSON 语义相同，仅格式/键序不同".into()))
                } else {
                    let detail = format!(
                        "JSON 语义不同（A 顶层键 {:?} / B 顶层键 {:?}）",
                        top_keys(&va),
                        top_keys(&vb)
                    );
                    (DiffKind::ContentDiff, Some(detail))
                }
            }
            _ => (
                DiffKind::ContentDiff,
                Some(format!(
                    "JSON 解析失败或不可比（A {} bytes / B {} bytes）",
                    a.len(),
                    b.len()
                )),
            ),
        };
    }

    (
        DiffKind::ContentDiff,
        Some(format!("字节不同（A {} bytes / B {} bytes）", a.len(), b.len())),
    )
}

fn top_keys(v: &serde_json::Value) -> Vec<String> {
    v.as_object()
        .map(|o| o.keys().take(6).cloned().collect())
        .unwrap_or_default()
}

/// PNG 像素级比较：尺寸不同 → 内容差异；像素相同但字节不同 → 仅编码差异。
fn compare_png(a: &[u8], b: &[u8]) -> (DiffKind, Option<String>) {
    let ia = image::load_from_memory_with_format(a, image::ImageFormat::Png);
    let ib = image::load_from_memory_with_format(b, image::ImageFormat::Png);
    let (ia, ib) = match (ia, ib) {
        (Ok(x), Ok(y)) => (x, y),
        _ => return (DiffKind::ContentDiff, Some("PNG 解码失败".into())),
    };
    if ia.width() != ib.width() || ia.height() != ib.height() {
        return (
            DiffKind::ContentDiff,
            Some(format!(
                "尺寸不同：{}x{} vs {}x{}",
                ia.width(),
                ia.height(),
                ib.width(),
                ib.height()
            )),
        );
    }
    let ra = ia.to_rgba8();
    let rb = ib.to_rgba8();
    let mut diff_px = 0usize;
    let mut max_delta = 0i32;
    for (pa, pb) in ra.pixels().zip(rb.pixels()) {
        if pa != pb {
            diff_px += 1;
            for c in 0..4 {
                let d = (pa[c] as i32 - pb[c] as i32).abs();
                if d > max_delta {
                    max_delta = d;
                }
            }
        }
    }
    if diff_px == 0 {
        (
            DiffKind::EncodingOnly,
            Some(format!(
                "像素完全相同（{}x{}），仅编码字节不同（A {} B {} bytes）",
                ia.width(),
                ia.height(),
                a.len(),
                b.len()
            )),
        )
    } else {
        let total = (ia.width() * ia.height()) as usize;
        (
            DiffKind::ContentDiff,
            Some(format!(
                "像素不同：{}/{} 像素，最大通道差 {}",
                diff_px, total, max_delta
            )),
        )
    }
}

/// 便捷入口：把报告渲染成人类可读文本。
pub fn render_report(report: &PackDiffReport, strict: bool) -> String {
    let mut s = String::new();
    s.push_str(&format!("A: {}\nB: {}\n", report.a, report.b));
    s.push_str(&format!("{}\n", report.summary()));
    s.push_str(&format!(
        "结论: {}\n",
        if report.passed(strict) {
            if strict { "PASS（字节级完全一致）" } else { "PASS（无内容差异；允许仅编码差异）" }
        } else {
            "FAIL（存在内容差异）"
        }
    ));
    if !report.diffs.is_empty() {
        s.push_str("\n差异清单:\n");
        for d in report.diffs.iter().take(200) {
            s.push_str(&format!(
                "  [{}] {}{}\n",
                d.kind.as_str(),
                d.path,
                d.detail
                    .as_ref()
                    .map(|x| format!("  — {}", x))
                    .unwrap_or_default()
            ));
        }
        if report.diffs.len() > 200 {
            s.push_str(&format!("  … 共 {} 条，仅显示前 200\n", report.diffs.len()));
        }
    }
    s
}

/// 把 zip/目录统一读成内存映射（CLI 需要时可复用）。
pub fn read_as_map(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    read_container(root)
}

/// 归一化路径（便于 CLI 打印）。
pub fn normalize(p: &str) -> PathBuf {
    PathBuf::from(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn png_bytes(color: [u8; 4]) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(8, 8, image::Rgba(color));
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut buf, image::ImageFormat::Png)
            .expect("encode");
        buf.into_inner()
    }

    fn write_zip(path: &Path, files: &[(&str, Vec<u8>)]) {
        let file = std::fs::File::create(path).expect("create");
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::FileOptions::default();
        for (name, body) in files {
            zip.start_file(*name, opts).expect("start");
            zip.write_all(body).expect("write");
        }
        zip.finish().expect("finish");
    }

    #[test]
    fn identical_packs_pass() {
        let dir = tempfile::tempdir().expect("tempdir");
        let a = dir.path().join("a.zip");
        let b = dir.path().join("b.zip");
        let files: Vec<(&str, Vec<u8>)> = vec![
            ("pack.mcmeta", br#"{"pack":{"pack_format":34}}"#.to_vec()),
            ("assets/x.png", png_bytes([10, 20, 30, 255])),
        ];
        write_zip(&a, &files);
        write_zip(&b, &files);

        let report = diff_containers(&a, &b).expect("diff");
        assert_eq!(report.blocking, 0);
        assert!(report.passed(false));
        assert!(report.passed(true), "字节相同应通过严格模式");
    }

    #[test]
    fn reencoded_png_is_encoding_only_not_blocking() {
        let dir = tempfile::tempdir().expect("tempdir");
        let a = dir.path().join("a.zip");
        let b = dir.path().join("b.zip");

        // 同一像素内容，但用不同压缩级别重新编码 → 字节不同、像素相同
        let img = image::RgbaImage::from_pixel(16, 16, image::Rgba([200, 10, 10, 255]));
        let encode = |level: image::codecs::png::CompressionType| -> Vec<u8> {
            use image::ImageEncoder;
            let mut buf = Vec::new();
            let enc = image::codecs::png::PngEncoder::new_with_quality(
                &mut buf,
                level,
                image::codecs::png::FilterType::Adaptive,
            );
            enc.write_image(img.as_raw(), 16, 16, image::ColorType::Rgba8)
                .expect("encode");
            buf
        };
        let fast = encode(image::codecs::png::CompressionType::Fast);
        let best = encode(image::codecs::png::CompressionType::Best);
        assert_ne!(fast, best, "不同压缩级别应产生不同字节");

        write_zip(&a, &[("t.png", fast)]);
        write_zip(&b, &[("t.png", best)]);

        let report = diff_containers(&a, &b).expect("diff");
        assert_eq!(report.blocking, 0, "像素相同不应算内容差异：{:?}", report.diffs);
        assert!(report.passed(false));
        assert!(!report.passed(true), "严格模式应因字节不同而失败");
        assert_eq!(report.diffs[0].kind, DiffKind::EncodingOnly);
    }

    #[test]
    fn pixel_change_is_blocking() {
        let dir = tempfile::tempdir().expect("tempdir");
        let a = dir.path().join("a.zip");
        let b = dir.path().join("b.zip");
        write_zip(&a, &[("t.png", png_bytes([10, 10, 10, 255]))]);
        write_zip(&b, &[("t.png", png_bytes([11, 10, 10, 255]))]);

        let report = diff_containers(&a, &b).expect("diff");
        assert_eq!(report.blocking, 1);
        assert!(!report.passed(false));
        assert_eq!(report.diffs[0].kind, DiffKind::ContentDiff);
    }

    #[test]
    fn json_semantic_equivalence_and_missing_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let a = dir.path().join("a.zip");
        let b = dir.path().join("b.zip");
        write_zip(
            &a,
            &[
                ("pack.mcmeta", br#"{"pack":{"pack_format":34,"description":"x"}}"#.to_vec()),
                ("only_a.txt", b"a".to_vec()),
            ],
        );
        write_zip(
            &b,
            &[
                // 键序不同 + 空白不同 → 语义相同
                ("pack.mcmeta", br#"{ "pack": {"description":"x", "pack_format":34} }"#.to_vec()),
                ("only_b.txt", b"b".to_vec()),
            ],
        );

        let report = diff_containers(&a, &b).expect("diff");
        let meta = report
            .diffs
            .iter()
            .find(|d| d.path == "pack.mcmeta")
            .expect("mcmeta should be reported (not byte-identical)");
        assert_eq!(meta.kind, DiffKind::JsonEquivalent);
        assert_eq!(report.blocking, 2, "两个单边文件算 blocking");
        assert!(!report.passed(false));
    }
}
