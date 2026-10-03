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
//! # 容器级比对
//!
//! 内容级比对把 zip 读成「路径 → 字节」，因此**看不见**三件事：目录条目、
//! 压缩方法、以及 zip 元数据（时间戳 / unix 权限位）。容器级比对补上这一层
//! （两侧都是 zip 时才启用；任一侧是目录则跳过并在 `note` 说明）：
//!
//!   * `dirOnlyInA` / `dirOnlyInB`  —— 目录条目单边存在 → **两种模式都失败**
//!     （内容级比对跳过目录，这是唯一能发现它的地方）；
//!   * `METHOD-DIFF` / `COMPRESSED-BYTES-DIFF` —— 压缩方法或压缩结果不同 → 仅 `--strict` 失败；
//!   * `MTIME-DIFF` / `MODE-DIFF`   —— 时间戳 / unix 权限位不同 → 仅 `--strict` 失败；
//!   * `order-only`                 —— 条目顺序不同 → **不判失败**，仅信息项。
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

/// 容器级差异分级：内容级比对看不见的部分。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ContainerDiffKind {
    /// 目录条目只存在于 A（内容级比对会跳过目录条目）。
    DirEntryOnlyInA,
    /// 目录条目只存在于 B。
    DirEntryOnlyInB,
    /// 压缩方法不同（例如 Stored vs Deflated）。
    MethodDiff,
    /// 内容相同（CRC 一致）但压缩结果字节不同（编码器/级别差异）。
    CompressedBytesDiff,
    /// 时间戳不同。
    MtimeDiff,
    /// unix 权限位不同。
    ModeDiff,
    /// 仅条目顺序不同（不判失败）。
    OrderOnly,
}

impl ContainerDiffKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ContainerDiffKind::DirEntryOnlyInA => "dir-only-in-A",
            ContainerDiffKind::DirEntryOnlyInB => "dir-only-in-B",
            ContainerDiffKind::MethodDiff => "METHOD-DIFF",
            ContainerDiffKind::CompressedBytesDiff => "COMPRESSED-BYTES-DIFF",
            ContainerDiffKind::MtimeDiff => "MTIME-DIFF",
            ContainerDiffKind::ModeDiff => "MODE-DIFF",
            ContainerDiffKind::OrderOnly => "order-only",
        }
    }

    /// 影响条目集合的差异：两种模式都判失败。
    pub fn is_entry_set(self) -> bool {
        matches!(
            self,
            ContainerDiffKind::DirEntryOnlyInA | ContainerDiffKind::DirEntryOnlyInB
        )
    }

    /// 只影响容器字节、不影响内容的差异：仅 `--strict` 判失败。
    pub fn is_byte_only(self) -> bool {
        matches!(
            self,
            ContainerDiffKind::MethodDiff
                | ContainerDiffKind::CompressedBytesDiff
                | ContainerDiffKind::MtimeDiff
                | ContainerDiffKind::ModeDiff
        )
    }
}

/// zip 条目的容器属性（目录条目同样收录）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerEntry {
    pub path: String,
    pub is_dir: bool,
    pub method: Option<String>,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub crc32: Option<u32>,
    pub mtime: Option<String>,
    pub unix_mode: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerMeta {
    pub path: String,
    pub entries: Vec<ContainerEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerDiff {
    pub path: String,
    pub kind: ContainerDiffKind,
    pub detail: Option<String>,
}

/// 容器级比对结果。
///
/// * `entry_set_blocking` —— 目录条目单边存在：两种模式都失败；
/// * `byte_only`          —— 方法/压缩字节/时间戳/权限：仅 `--strict` 失败；
/// * `order_changed`      —— 条目顺序变化：仅信息项（顺序不是现状承诺）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerSummary {
    pub entries_a: usize,
    pub entries_b: usize,
    pub dirs_a: usize,
    pub dirs_b: usize,
    pub entry_set_blocking: usize,
    pub byte_only: usize,
    pub order_changed: bool,
    pub diffs: Vec<ContainerDiff>,
    /// 未做容器级比对时的原因（例如一侧是目录）。
    pub note: Option<String>,
}

/// 读取容器（zip）的条目级元数据。
///
/// 目录返回 `Ok(None)`：目录没有压缩方法/时间戳语义，因此容器级比对只在
/// **两侧都是 zip** 时启用（避免「目录 vs zip」产生假阳性）。
pub fn read_container_meta(root: &Path) -> Result<Option<ContainerMeta>, String> {
    if root.is_dir() {
        return Ok(None);
    }
    let file = std::fs::File::open(root).map_err(|e| format!("open {}: {}", root.display(), e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("read zip: {}", e))?;
    let mut entries = Vec::with_capacity(archive.len());
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| format!("entry {}: {}", i, e))?;
        let mtime = entry.last_modified();
        entries.push(ContainerEntry {
            path: entry.name().to_string(),
            is_dir: entry.is_dir(),
            method: Some(method_name(entry.compression())),
            compressed_size: entry.compressed_size(),
            uncompressed_size: entry.size(),
            crc32: Some(entry.crc32()),
            mtime: Some(format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
                mtime.year(),
                mtime.month(),
                mtime.day(),
                mtime.hour(),
                mtime.minute(),
                mtime.second()
            )),
            unix_mode: entry.unix_mode(),
        });
    }
    Ok(Some(ContainerMeta {
        path: root.to_string_lossy().to_string(),
        entries,
    }))
}

fn method_name(method: zip::CompressionMethod) -> String {
    match method {
        zip::CompressionMethod::Stored => "stored".to_string(),
        zip::CompressionMethod::Deflated => "deflated".to_string(),
        other => format!("{:?}", other).to_ascii_lowercase(),
    }
}

/// 容器级比对。
pub fn diff_container_meta(a: &ContainerMeta, b: &ContainerMeta) -> ContainerSummary {
    let map_a: BTreeMap<&str, &ContainerEntry> =
        a.entries.iter().map(|e| (e.path.as_str(), e)).collect();
    let map_b: BTreeMap<&str, &ContainerEntry> =
        b.entries.iter().map(|e| (e.path.as_str(), e)).collect();

    let mut diffs: Vec<ContainerDiff> = Vec::new();
    let mut entry_set_blocking = 0usize;
    let mut byte_only = 0usize;

    // 1) 目录条目单边存在 —— 内容级比对跳过目录，只有这里能发现
    for (path, ea) in &map_a {
        if ea.is_dir && !map_b.contains_key(path) {
            diffs.push(ContainerDiff {
                path: (*path).to_string(),
                kind: ContainerDiffKind::DirEntryOnlyInA,
                detail: Some("目录条目只存在于 A（内容级比对会跳过目录条目）".into()),
            });
            entry_set_blocking += 1;
        }
    }
    for (path, eb) in &map_b {
        if eb.is_dir && !map_a.contains_key(path) {
            diffs.push(ContainerDiff {
                path: (*path).to_string(),
                kind: ContainerDiffKind::DirEntryOnlyInB,
                detail: Some("目录条目只存在于 B（内容级比对会跳过目录条目）".into()),
            });
            entry_set_blocking += 1;
        }
    }

    // 2) 共有条目的容器属性
    for (path, ea) in &map_a {
        let Some(eb) = map_b.get(path) else { continue };
        if ea.method != eb.method {
            diffs.push(ContainerDiff {
                path: (*path).to_string(),
                kind: ContainerDiffKind::MethodDiff,
                detail: Some(format!(
                    "压缩方法不同：A {} / B {}",
                    ea.method.as_deref().unwrap_or("?"),
                    eb.method.as_deref().unwrap_or("?")
                )),
            });
            byte_only += 1;
        } else if ea.crc32.is_some()
            && ea.crc32 == eb.crc32
            && ea.compressed_size != eb.compressed_size
        {
            diffs.push(ContainerDiff {
                path: (*path).to_string(),
                kind: ContainerDiffKind::CompressedBytesDiff,
                detail: Some(format!(
                    "内容相同（CRC 一致）但压缩字节不同：A {} / B {} bytes",
                    ea.compressed_size, eb.compressed_size
                )),
            });
            byte_only += 1;
        }
        if ea.mtime != eb.mtime {
            diffs.push(ContainerDiff {
                path: (*path).to_string(),
                kind: ContainerDiffKind::MtimeDiff,
                detail: Some(format!(
                    "时间戳不同：A {} / B {}",
                    ea.mtime.as_deref().unwrap_or("-"),
                    eb.mtime.as_deref().unwrap_or("-")
                )),
            });
            byte_only += 1;
        }
        if ea.unix_mode != eb.unix_mode {
            diffs.push(ContainerDiff {
                path: (*path).to_string(),
                kind: ContainerDiffKind::ModeDiff,
                detail: Some(format!(
                    "unix 权限位不同：A {:?} / B {:?}",
                    ea.unix_mode, eb.unix_mode
                )),
            });
            byte_only += 1;
        }
    }

    // 3) 条目顺序（仅当条目集合一致时才判定，否则是集合差异的噪声）
    let set_equal = map_a.keys().eq(map_b.keys());
    let order_a: Vec<&str> = a.entries.iter().map(|e| e.path.as_str()).collect();
    let order_b: Vec<&str> = b.entries.iter().map(|e| e.path.as_str()).collect();
    let order_changed = set_equal && order_a != order_b;
    if order_changed {
        diffs.push(ContainerDiff {
            path: String::new(),
            kind: ContainerDiffKind::OrderOnly,
            detail: Some(format!(
                "条目顺序不同（A {} 条 / B {} 条）—— 顺序不是现状承诺，仅作信息项",
                order_a.len(),
                order_b.len()
            )),
        });
    }

    ContainerSummary {
        entries_a: a.entries.len(),
        entries_b: b.entries.len(),
        dirs_a: a.entries.iter().filter(|e| e.is_dir).count(),
        dirs_b: b.entries.iter().filter(|e| e.is_dir).count(),
        entry_set_blocking,
        byte_only,
        order_changed,
        diffs,
        note: None,
    }
}

/// 两侧都是 zip 时做容器级比对；否则返回带说明的空摘要。
fn build_container_summary(a: &Path, b: &Path) -> ContainerSummary {
    let meta_a = read_container_meta(a).ok().flatten();
    let meta_b = read_container_meta(b).ok().flatten();
    match (meta_a, meta_b) {
        (Some(ma), Some(mb)) => diff_container_meta(&ma, &mb),
        _ => ContainerSummary {
            note: Some("一侧是目录（或读取失败），未做容器级比对".into()),
            ..Default::default()
        },
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
    /// 容器级比对（两侧都是 zip 时才有内容）。
    pub container: Option<ContainerSummary>,
    /// 目录条目单边存在 —— 两种模式都判失败。
    pub container_entry_set_blocking: usize,
    /// 方法/压缩字节/时间戳/权限差异 —— 仅 `--strict` 判失败。
    pub container_byte_only: usize,
    /// 条目顺序是否变化（信息项，不判失败）。
    pub container_order_changed: bool,
}

impl PackDiffReport {
    /// 容器级摘要片段（未做容器级比对时为空串）。
    pub fn container_clause(&self) -> String {
        match &self.container {
            None => String::new(),
            Some(c) => format!(
                " | container: entries A={} B={} (dirs A={} B={}), entry-set={}, byte-only={}, order-changed={}",
                c.entries_a, c.entries_b, c.dirs_a, c.dirs_b, c.entry_set_blocking, c.byte_only, c.order_changed
            ),
        }
    }

    /// `--strict` 通过时，区分「逐字节一致」与「顺序不同，因此不构成逐字节一致」。
    pub fn strict_pass_label(&self) -> &'static str {
        if self.container_order_changed {
            "PASS（内容与容器属性一致；条目顺序不同 → 不构成逐字节一致）"
        } else {
            "PASS（字节级完全一致）"
        }
    }

    pub fn passed(&self, strict: bool) -> bool {
        if strict {
            // 严格模式：每个文件 identical，且容器不得有仅字节差异 / 目录条目单边存在
            self.diffs.iter().all(|d| d.kind == DiffKind::Identical)
                && self.container_entry_set_blocking == 0
                && self.container_byte_only == 0
        } else {
            self.blocking == 0 && self.container_entry_set_blocking == 0
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "A={} files, B={} files, blocking={}, diffs={}{}",
            self.files_a,
            self.files_b,
            self.blocking,
            self.diffs.len(),
            self.container_clause()
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

    let container = build_container_summary(a, b);
    let container_entry_set_blocking = container.entry_set_blocking;
    let container_byte_only = container.byte_only;
    let container_order_changed = container.order_changed;

    Ok(PackDiffReport {
        a: a.to_string_lossy().to_string(),
        b: b.to_string_lossy().to_string(),
        files_a: map_a.len(),
        files_b: map_b.len(),
        identical_or_equivalent: equivalent,
        blocking,
        diffs,
        container: Some(container),
        container_entry_set_blocking,
        container_byte_only,
        container_order_changed,
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
            if strict {
                report.strict_pass_label()
            } else {
                "PASS（无内容差异；允许仅编码差异）"
            }
        } else {
            "FAIL（存在内容或容器差异）"
        }
    ));
    if let Some(c) = &report.container {
        s.push_str("\n容器级:\n");
        s.push_str(&format!(
            "  条目 A={} B={}（目录 A={} B={}）· 条目集合差异 {} · 字节属性差异 {} · 顺序变化 {}\n",
            c.entries_a, c.entries_b, c.dirs_a, c.dirs_b, c.entry_set_blocking, c.byte_only, c.order_changed
        ));
        if let Some(note) = &c.note {
            s.push_str(&format!("  注: {}\n", note));
        }
        for d in c.diffs.iter().take(200) {
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
    }
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
        assert_eq!(report.container_byte_only, 0, "同管线产物容器属性应一致");
        assert!(!report.container_order_changed);
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

    /// 可控条目集合：目录条目、压缩方法、unix 权限位、时间戳、写入顺序。
    struct ZSpec {
        name: String,
        body: Option<Vec<u8>>,
        method: zip::CompressionMethod,
        mode: Option<u32>,
        mtime: Option<zip::DateTime>,
    }

    fn zfile(name: &str, body: &[u8]) -> ZSpec {
        ZSpec {
            name: name.to_string(),
            body: Some(body.to_vec()),
            method: zip::CompressionMethod::Stored,
            mode: None,
            mtime: None,
        }
    }

    fn zdir(name: &str) -> ZSpec {
        ZSpec {
            name: format!("{}/", name.trim_end_matches('/')),
            body: None,
            method: zip::CompressionMethod::Stored,
            mode: None,
            mtime: None,
        }
    }

    fn write_zip_specs(path: &Path, specs: &[ZSpec]) {
        let file = std::fs::File::create(path).expect("create");
        let mut zip = zip::ZipWriter::new(file);
        for spec in specs {
            let mut opts =
                zip::write::FileOptions::default().compression_method(spec.method);
            if let Some(mode) = spec.mode {
                opts = opts.unix_permissions(mode);
            }
            if let Some(mtime) = spec.mtime {
                opts = opts.last_modified_time(mtime);
            }
            match &spec.body {
                Some(body) => {
                    zip.start_file(spec.name.as_str(), opts).expect("start");
                    zip.write_all(body).expect("write");
                }
                None => {
                    zip.add_directory(spec.name.as_str(), opts).expect("dir");
                }
            }
        }
        zip.finish().expect("finish");
    }

    fn centry(
        path: &str,
        is_dir: bool,
        method: &str,
        csize: u64,
        crc: u32,
        mtime: &str,
        mode: u32,
    ) -> ContainerEntry {
        ContainerEntry {
            path: path.to_string(),
            is_dir,
            method: Some(method.to_string()),
            compressed_size: csize,
            uncompressed_size: 100,
            crc32: Some(crc),
            mtime: Some(mtime.to_string()),
            unix_mode: Some(mode),
        }
    }

    #[test]
    fn container_kinds_are_classified() {
        let a = ContainerMeta {
            path: "a".into(),
            entries: vec![
                centry("empty/", true, "stored", 0, 0, "2026-01-01T00:00:00", 0o644),
                centry("same-size.txt", false, "stored", 10, 7, "2026-01-01T00:00:00", 0o644),
                centry("method.txt", false, "stored", 10, 11, "2026-01-01T00:00:00", 0o644),
                centry("mtime.txt", false, "deflated", 10, 13, "2026-01-01T00:00:00", 0o644),
                centry("mode.txt", false, "deflated", 10, 17, "2026-01-01T00:00:00", 0o644),
            ],
        };
        let b = ContainerMeta {
            path: "b".into(),
            entries: vec![
                // CRC 相同、压缩字节不同 → CompressedBytesDiff
                centry("same-size.txt", false, "stored", 12, 7, "2026-01-01T00:00:00", 0o644),
                centry("method.txt", false, "deflated", 9, 11, "2026-01-01T00:00:00", 0o644),
                centry("mtime.txt", false, "deflated", 10, 13, "2026-02-02T00:00:00", 0o644),
                centry("mode.txt", false, "deflated", 10, 17, "2026-01-01T00:00:00", 0o600),
            ],
        };

        let s = diff_container_meta(&a, &b);
        assert_eq!(s.entry_set_blocking, 1, "empty/ 只在 A → 目录条目单边存在");
        assert_eq!(s.byte_only, 4, "压缩字节/方法/时间戳/权限各一条");
        assert!(!s.order_changed, "条目集合不同时不做顺序判定");
        let kinds: Vec<ContainerDiffKind> = s.diffs.iter().map(|d| d.kind).collect();
        for k in [
            ContainerDiffKind::DirEntryOnlyInA,
            ContainerDiffKind::CompressedBytesDiff,
            ContainerDiffKind::MethodDiff,
            ContainerDiffKind::MtimeDiff,
            ContainerDiffKind::ModeDiff,
        ] {
            assert!(kinds.contains(&k), "缺少 {:?}：{:?}", k, kinds);
        }

        // 集合一致、仅顺序不同 → 只报 order-only，不判失败
        let o1 = ContainerMeta {
            path: "a".into(),
            entries: vec![
                centry("x.txt", false, "stored", 5, 1, "t", 0o644),
                centry("y.txt", false, "stored", 5, 2, "t", 0o644),
            ],
        };
        let o2 = ContainerMeta {
            path: "b".into(),
            entries: vec![
                centry("y.txt", false, "stored", 5, 2, "t", 0o644),
                centry("x.txt", false, "stored", 5, 1, "t", 0o644),
            ],
        };
        let os = diff_container_meta(&o1, &o2);
        assert!(os.order_changed);
        assert_eq!(os.byte_only, 0);
        assert_eq!(os.entry_set_blocking, 0);
        assert_eq!(os.diffs.len(), 1);
        assert_eq!(os.diffs[0].kind, ContainerDiffKind::OrderOnly);
    }

    /// 旧闸门看不见的回归：空目录条目丢失（内容级比对跳过目录条目）。
    #[test]
    fn container_missing_dir_entry_blocks_both_modes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let a = dir.path().join("a.zip");
        let b = dir.path().join("b.zip");
        write_zip_specs(&a, &[zfile("a.txt", b"same"), zdir("assets/empty")]);
        write_zip_specs(&b, &[zfile("a.txt", b"same")]);

        let report = diff_containers(&a, &b).expect("diff");
        assert_eq!(report.blocking, 0, "文件内容一致（目录条目不参与内容比对）");
        assert_eq!(report.container_entry_set_blocking, 1, "丢失的空目录条目必须被发现");
        assert!(!report.passed(false), "非严格模式下条目集合差异也判失败");
        assert!(!report.passed(true));
        let kinds: Vec<ContainerDiffKind> = report
            .container
            .as_ref()
            .expect("container summary")
            .diffs
            .iter()
            .map(|d| d.kind)
            .collect();
        assert!(kinds.contains(&ContainerDiffKind::DirEntryOnlyInA), "{:?}", kinds);

        // 人类可读输出必须把这条差异摆出来（CLI 用户只看文本）
        let text = render_report(&report, false);
        assert!(text.contains("容器级"), "{}", text);
        assert!(text.contains("dir-only-in-A"), "{}", text);
        assert!(text.contains("FAIL"), "{}", text);
    }

    #[test]
    fn container_method_diff_fails_strict_only() {
        let dir = tempfile::tempdir().expect("tempdir");
        let a = dir.path().join("a.zip");
        let b = dir.path().join("b.zip");
        let mut spec_a = zfile("t.txt", b"payload");
        spec_a.method = zip::CompressionMethod::Stored;
        let mut spec_b = zfile("t.txt", b"payload");
        spec_b.method = zip::CompressionMethod::Deflated;
        write_zip_specs(&a, &[spec_a]);
        write_zip_specs(&b, &[spec_b]);

        let report = diff_containers(&a, &b).expect("diff");
        assert_eq!(report.blocking, 0, "内容字节相同");
        assert_eq!(report.container_byte_only, 1, "压缩方法不同");
        assert!(report.passed(false), "非严格模式：内容等价即通过");
        assert!(!report.passed(true), "严格模式：压缩方法不同 → 字节不同");
        let kinds: Vec<ContainerDiffKind> = report
            .container
            .as_ref()
            .expect("container summary")
            .diffs
            .iter()
            .map(|d| d.kind)
            .collect();
        assert!(kinds.contains(&ContainerDiffKind::MethodDiff), "{:?}", kinds);
    }

    #[test]
    fn container_mtime_and_mode_diffs_fail_strict_only() {
        let dir = tempfile::tempdir().expect("tempdir");
        let a = dir.path().join("a.zip");
        let b = dir.path().join("b.zip");
        let dt_a = zip::DateTime::from_date_and_time(2020, 1, 1, 0, 0, 0).expect("dt");
        let dt_b = zip::DateTime::from_date_and_time(2021, 6, 6, 6, 6, 6).expect("dt");

        let mut spec_a = zfile("t.txt", b"same");
        spec_a.mode = Some(0o644);
        spec_a.mtime = Some(dt_a);
        let mut spec_b = zfile("t.txt", b"same");
        spec_b.mode = Some(0o600);
        spec_b.mtime = Some(dt_b);
        write_zip_specs(&a, &[spec_a]);
        write_zip_specs(&b, &[spec_b]);

        let report = diff_containers(&a, &b).expect("diff");
        assert_eq!(report.blocking, 0, "内容字节相同");
        assert_eq!(report.container_byte_only, 2, "时间戳 + 权限位各一条");
        assert!(report.passed(false));
        assert!(!report.passed(true));
        let kinds: Vec<ContainerDiffKind> = report
            .container
            .as_ref()
            .expect("container summary")
            .diffs
            .iter()
            .map(|d| d.kind)
            .collect();
        assert!(kinds.contains(&ContainerDiffKind::MtimeDiff), "{:?}", kinds);
        assert!(kinds.contains(&ContainerDiffKind::ModeDiff), "{:?}", kinds);
    }

    #[test]
    fn container_order_change_is_informational() {
        let dir = tempfile::tempdir().expect("tempdir");
        let a = dir.path().join("a.zip");
        let b = dir.path().join("b.zip");
        write_zip_specs(&a, &[zfile("x.txt", b"1"), zfile("y.txt", b"2")]);
        write_zip_specs(&b, &[zfile("y.txt", b"2"), zfile("x.txt", b"1")]);

        let report = diff_containers(&a, &b).expect("diff");
        assert_eq!(report.blocking, 0);
        assert!(report.container_order_changed, "顺序变化必须被发现");
        assert_eq!(report.container_byte_only, 0, "顺序不产生字节属性差异");
        assert!(report.passed(false));
        assert!(report.passed(true), "顺序不是现状承诺 → 不判失败");
        assert!(
            report.strict_pass_label().contains("顺序"),
            "严格通过的措辞必须说明顺序不同：{}",
            report.strict_pass_label()
        );
    }

    #[test]
    fn container_comparison_skipped_for_directory_side() {
        let dir = tempfile::tempdir().expect("tempdir");
        let zip_path = dir.path().join("a.zip");
        write_zip_specs(&zip_path, &[zfile("t.txt", b"same")]);

        let extracted = dir.path().join("tree");
        std::fs::create_dir_all(&extracted).expect("mkdir");
        std::fs::write(extracted.join("t.txt"), b"same").expect("write");

        let report = diff_containers(&zip_path, &extracted).expect("diff");
        assert_eq!(report.blocking, 0);
        assert!(report.passed(false));
        let c = report.container.as_ref().expect("container summary");
        assert!(c.note.is_some(), "一侧是目录时必须说明未做容器级比对");
        assert_eq!(c.byte_only, 0);
        assert_eq!(c.entry_set_blocking, 0);
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
