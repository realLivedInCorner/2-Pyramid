//! 序列化：把有效条目写回 zip。
//!
//! 这一层同时承担「保真」责任，规则**逐条对齐现状管线**（`astray-arom-model.md` §1）：
//!
//! * 压缩方法 = 扩展名白名单（与 `converters/zip.rs:568–581` 一致，**改动必须同步**）；
//! * 时间戳 / unix 权限位 = **不写**（`FileOptions::default()` 的默认值），与现状一致；
//! * 目录条目 = 显式目录 ∪ 文件隐含的祖先目录（现状解压在磁盘上会建出它们）；
//! * **透传策略（D23）**：只有「源方法 == 白名单方法 == `Stored`」时才 `copy_raw_to` 零解压透传；
//!   其余一律解压后按现状设置重压——`Deflated` 条目原样透传会改变产物字节；
//! * 未写入的条目天然透传（不在任何层里出现就是未触碰）。

use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use super::error::AromError;
use super::layer::{Body, Pack, PackView, Resolved};

/// 输出条目顺序策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderPolicy {
    /// 目录在前、各自按路径排序（**默认**，确定且与文件系统无关）。
    DirsFirstThenPathSorted,
    /// 基座源顺序（源条目按 `src_idx`，之后是层写入，按路径），供对照实验。
    SourceOrder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializeOptions {
    pub order: OrderPolicy,
    /// 是否补齐文件隐含的祖先目录（默认 true；关掉只用于实验对照）。
    pub include_synthesized_dirs: bool,
}

impl Default for SerializeOptions {
    fn default() -> Self {
        Self {
            order: OrderPolicy::DirsFirstThenPathSorted,
            include_synthesized_dirs: true,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SerializeStats {
    pub dirs: usize,
    pub files: usize,
    /// 零解压零重压透传的条目数。
    pub passthrough: usize,
    /// 解压后重压的条目数（现状语义）。
    pub recompressed: usize,
    /// 层写入（blob/alias）直写的条目数。
    pub written: usize,
    pub bytes: u64,
}

/// 现状压缩方法白名单。与 `converters/zip.rs:568–581` 必须保持一致。
pub fn method_for_path(name: &str) -> zip::CompressionMethod {
    const STORED_EXTS: [&str; 16] = [
        ".png", ".jpg", ".jpeg", ".gif", ".webp", ".ogg", ".mp3", ".wav", ".mp4", ".zip", ".mcpack",
        ".jar", ".dds", ".tga", ".ktx", ".ktx2",
    ];
    let lower = name.to_ascii_lowercase();
    if STORED_EXTS.iter().any(|ext| lower.ends_with(ext)) || lower.ends_with(".bin") {
        zip::CompressionMethod::Stored
    } else {
        zip::CompressionMethod::Deflated
    }
}

/// 写入 zip（调用方负责原子替换；需要原子语义时用 [`write_zip_atomic`]）。
pub fn write_zip(
    pack: &Pack,
    view: &PackView<'_>,
    out: &Path,
    opts: &SerializeOptions,
) -> Result<SerializeStats, AromError> {
    let file = File::create(out).map_err(|e| AromError::io(format!("create {}: {e}", out.display())))?;
    let mut zip = zip::ZipWriter::new(BufWriter::with_capacity(1024 * 1024, file));
    let mut stats = SerializeStats::default();

    let entries = view.entries()?;
    let mut files: Vec<Resolved> = Vec::new();
    let mut dirs: BTreeSet<String> = BTreeSet::new();

    for res in entries {
        if res.is_dir {
            dirs.insert(res.path);
        } else {
            files.push(res);
        }
    }
    if opts.include_synthesized_dirs {
        dirs.extend(view.effective_dirs()?);
    }

    match opts.order {
        OrderPolicy::DirsFirstThenPathSorted => {
            files.sort_by(|a, b| a.path.cmp(&b.path));
        }
        OrderPolicy::SourceOrder => {
            files.sort_by(|a, b| match (&a.body, &b.body) {
                (Body::Base { src_idx: x }, Body::Base { src_idx: y }) => {
                    x.cmp(y).then_with(|| a.path.cmp(&b.path))
                }
                (Body::Base { .. }, _) => std::cmp::Ordering::Less,
                (_, Body::Base { .. }) => std::cmp::Ordering::Greater,
                _ => a.path.cmp(&b.path),
            });
        }
    }

    for dir in &dirs {
        zip.add_directory(format!("{dir}/"), zip::write::FileOptions::default())
            .map_err(|e| AromError::zip(format!("add_directory {dir}: {e}")))?;
        stats.dirs += 1;
    }

    for res in &files {
        let target = method_for_path(&res.path);
        let options = zip::write::FileOptions::default().compression_method(target);
        zip.start_file(res.path.as_str(), options)
            .map_err(|e| AromError::zip(format!("start_file {}: {e}", res.path)))?;

        match &res.body {
            Body::Base { src_idx } => {
                let source_method = pack.source().meta(*src_idx)?.method.clone();
                // D23：只有「源方法 == 白名单方法 == Stored」才允许零解压透传
                if source_method == "stored" && target == zip::CompressionMethod::Stored {
                    pack.source().copy_raw_to(*src_idx, &mut zip)?;
                    stats.passthrough += 1;
                } else {
                    pack.source().copy_to(*src_idx, &mut zip)?;
                    stats.recompressed += 1;
                }
            }
            Body::Blob(id) => {
                let data = pack.blobs().get(*id)?;
                zip.write_all(data.as_ref())
                    .map_err(|e| AromError::io(format!("write {}: {e}", res.path)))?;
                stats.written += 1;
            }
            Body::Alias(_) => {
                let data = pack.read_body(&res.body)?;
                zip.write_all(&data)
                    .map_err(|e| AromError::io(format!("write alias {}: {e}", res.path)))?;
                stats.written += 1;
            }
            Body::Dir => {
                return Err(AromError::internal(format!(
                    "directory body in file list: {}",
                    res.path
                )))
            }
        }
        stats.files += 1;
        stats.bytes = stats.bytes.saturating_add(res.len);
    }

    zip.finish()
        .map_err(|e| AromError::zip(format!("finalize zip {}: {e}", out.display())))?
        .flush()
        .map_err(|e| AromError::io(format!("flush zip {}: {e}", out.display())))?;
    Ok(stats)
}

/// 临时文件 + 原子替换。目标已存在时先删除再改名（Windows 上 `rename` 不覆盖）。
pub fn write_zip_atomic(
    pack: &Pack,
    view: &PackView<'_>,
    out: &Path,
    opts: &SerializeOptions,
) -> Result<SerializeStats, AromError> {
    let tmp = sibling_tmp(out);
    let stats = write_zip(pack, view, &tmp, opts)?;
    if out.exists() {
        std::fs::remove_file(out)
            .map_err(|e| AromError::io(format!("remove old {}: {e}", out.display())))?;
    }
    std::fs::rename(&tmp, out).map_err(|e| {
        AromError::io(format!(
            "rename {} -> {}: {e}",
            tmp.display(),
            out.display()
        ))
    })?;
    Ok(stats)
}

fn sibling_tmp(out: &Path) -> std::path::PathBuf {
    let mut name = out
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "out.zip".to_string());
    name.push_str(".arom-tmp");
    out.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arom::source::MemSource;
    use crate::converters::pack_diff::diff_containers;
    use std::io::Write;

    struct Spec {
        name: String,
        body: Option<Vec<u8>>,
        method: zip::CompressionMethod,
    }

    fn file(name: &str, body: Vec<u8>, method: zip::CompressionMethod) -> Spec {
        Spec {
            name: name.to_string(),
            body: Some(body),
            method,
        }
    }

    fn dir(name: &str) -> Spec {
        Spec {
            name: format!("{}/", name.trim_end_matches('/')),
            body: None,
            method: zip::CompressionMethod::Stored,
        }
    }

    fn write_zip_specs(path: &Path, specs: &[Spec]) {
        let f = File::create(path).expect("create");
        let mut zip = zip::ZipWriter::new(f);
        for spec in specs {
            let opts = zip::write::FileOptions::default().compression_method(spec.method);
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

    fn png_bytes(color: [u8; 4]) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(8, 8, image::Rgba(color));
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .expect("encode");
        buf
    }

    /// 混合夹具：Stored/Deflated 的 png、被压过的 png、空目录、隐含祖先目录、大二进制。
    fn fixture(path: &Path) {
        write_zip_specs(
            path,
            &[
                file(
                    "pack.mcmeta",
                    br#"{"pack":{"pack_format":34,"description":"x"}}"#.to_vec(),
                    zip::CompressionMethod::Deflated,
                ),
                file(
                    "assets/minecraft/textures/x.png",
                    png_bytes([10, 20, 30, 255]),
                    zip::CompressionMethod::Stored,
                ),
                // 被作者压过的 png：白名单要求 Stored，必须解压后重写
                file(
                    "assets/minecraft/textures/y.png",
                    png_bytes([40, 50, 60, 255]),
                    zip::CompressionMethod::Deflated,
                ),
                dir("assets/empty"),
                file(
                    "assets/deep/a/b.txt",
                    b"nested text".to_vec(),
                    zip::CompressionMethod::Deflated,
                ),
                file("big.bin", vec![7u8; 100 * 1024], zip::CompressionMethod::Stored),
            ],
        );
    }

    /// **M0 验收**：读 → 原样写，产出必须过容器级闸门（与**旧管线真实输出**对照）。
    #[test]
    fn read_then_write_matches_the_existing_pipeline() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src = tmp.path().join("src.zip");
        fixture(&src);

        // 旧管线：解压到临时目录 → 重新打包
        let work = tmp.path().join("work");
        std::fs::create_dir_all(&work).expect("mkdir");
        crate::converters::zip::extract_resource_pack(
            src.to_str().expect("utf8"),
            work.to_str().expect("utf8"),
        )
        .expect("extract");
        let old_out = tmp.path().join("old.zip");
        crate::converters::zip::repack_resource_pack(
            work.to_str().expect("utf8"),
            old_out.to_str().expect("utf8"),
        )
        .expect("repack");

        // 新管线：ZipSource → BasePack → 序列化（没有任何层）
        let pack =
            Pack::open_zip(&src, &crate::arom::limits::SafeLimits::preserving_current(), None)
                .expect("open");
        let view = pack.view();
        let new_out = tmp.path().join("new.zip");
        let stats = write_zip(&pack, &view, &new_out, &SerializeOptions::default()).expect("write");

        // 对照
        let report = diff_containers(&old_out, &new_out).expect("diff");
        assert_eq!(report.blocking, 0, "内容级不得有差异：{:?}", report.diffs);
        assert_eq!(
            report.container_entry_set_blocking, 0,
            "目录条目集合必须一致（空目录 + 隐含祖先目录）：{:?}",
            report.container
        );
        assert_eq!(
            report.container_byte_only, 0,
            "压缩方法/时间戳/权限必须与旧管线一致：{:?}",
            report.container
        );
        assert!(
            report.passed(true),
            "严格模式必须通过（条目顺序差异是信息项）：{:?}",
            report.container
        );

        // 透传策略确实被执行：Stored 的 png/bin 走透传，其余重压
        assert!(stats.passthrough >= 2, "stored 条目应零解压透传：{stats:?}");
        assert!(stats.recompressed >= 2, "deflated 条目必须重压：{stats:?}");
        assert_eq!(stats.files, 5);
        assert_eq!(
            stats.dirs, 6,
            "assets / empty / deep / deep/a / minecraft / textures"
        );
    }

    #[test]
    fn output_is_deterministic() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src = tmp.path().join("src.zip");
        fixture(&src);
        let pack =
            Pack::open_zip(&src, &crate::arom::limits::SafeLimits::preserving_current(), None)
                .expect("open");
        let view = pack.view();

        let a = tmp.path().join("a.zip");
        let b = tmp.path().join("b.zip");
        write_zip(&pack, &view, &a, &SerializeOptions::default()).expect("a");
        write_zip(&pack, &view, &b, &SerializeOptions::default()).expect("b");

        // 注意：**不能**断言整个容器逐字节相同——序列化与旧管线一样，时间戳取的是
        // 「运行时刻」（`FileOptions::default()` → `OffsetDateTime::now_utc()`，DOS 2 秒精度），
        // 两次独立运行必然可能不同（详见细则 §9.6 F 与闸门的 `container_mtime_only`）。
        // 确定性应当断言在「内容 + 条目集合 + 压缩方法 / 压缩字节」上。
        let report = crate::converters::pack_diff::diff_containers(&a, &b).expect("diff");
        assert_eq!(report.blocking, 0, "同输入的内容必须相同：{:?}", report.diffs);
        assert_eq!(
            report.container_entry_set_blocking, 0,
            "同输入的条目集合必须相同：{:?}",
            report.container
        );
        assert_eq!(
            report.container_byte_only, 0,
            "同输入的压缩方法/压缩字节必须相同：{:?}",
            report.container
        );
    }

    #[test]
    fn layer_writes_and_dir_entries_are_serialized() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src = tmp.path().join("src.zip");
        write_zip_specs(
            &src,
            &[
                dir("assets/empty"),
                file("a.txt", b"old".to_vec(), zip::CompressionMethod::Deflated),
            ],
        );

        let mut pack =
            Pack::open_zip(&src, &crate::arom::limits::SafeLimits::preserving_current(), None)
                .expect("open");
        {
            let mut tx = pack.tx("test");
            tx.put("a.txt", b"new content".to_vec()).expect("put");
            tx.mkdir("created/empty").expect("mkdir");
            let layer = tx.into_layer();
            pack.commit(layer);
        }

        let out = tmp.path().join("out.zip");
        let stats = {
            let view = pack.view();
            write_zip(&pack, &view, &out, &SerializeOptions::default()).expect("write")
        };
        assert_eq!(stats.written, 1, "层里的 blob 直写");
        assert_eq!(
            stats.dirs, 4,
            "assets/empty + created/empty 及其祖先 assets / created"
        );

        // 回读校验：内容、目录、以及别名不适用
        let reopened = Pack::open_zip(&out, &crate::arom::limits::SafeLimits::preserving_current(), None)
            .expect("reopen");
        let view = reopened.view();
        assert_eq!(view.read("a.txt").expect("read").expect("some"), b"new content");
        assert!(view.resolve("created/empty").expect("resolved").is_dir);
        assert!(view.resolve("assets/empty").expect("resolved").is_dir);
    }

    #[test]
    fn atomic_write_replaces_existing_target() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src = tmp.path().join("src.zip");
        write_zip_specs(
            &src,
            &[file("a.txt", b"one".to_vec(), zip::CompressionMethod::Deflated)],
        );
        let pack =
            Pack::open_zip(&src, &crate::arom::limits::SafeLimits::preserving_current(), None)
                .expect("open");
        let out = tmp.path().join("out.zip");
        std::fs::write(&out, b"placeholder").expect("seed");

        let view = pack.view();
        write_zip_atomic(&pack, &view, &out, &SerializeOptions::default()).expect("atomic");
        assert!(
            !sibling_tmp(&out).exists(),
            "临时文件必须被改名掉，不留残渣"
        );
        let reopened = Pack::open_zip(&out, &crate::arom::limits::SafeLimits::preserving_current(), None)
            .expect("reopen");
        assert_eq!(
            reopened.view().read("a.txt").expect("read").expect("some"),
            b"one"
        );
    }

    #[test]
    fn method_whitelist_matches_the_current_pipeline() {
        use zip::CompressionMethod::{Deflated, Stored};
        for name in [
            "a.png", "A.PNG", "a.jpg", "a.ogg", "a.mp3", "a.wav", "a.zip", "a.mcpack", "a.jar",
            "a.dds", "a.tga", "a.ktx", "a.ktx2", "a.bin", "a.mp4", "a.gif", "a.webp", "a.jpeg",
        ] {
            assert_eq!(method_for_path(name), Stored, "{name} 应为 Stored");
        }
        for name in ["pack.mcmeta", "a.json", "a.txt", "a.vsh", "a.fsh", "a.lang", "a.xml", "noext"] {
            assert_eq!(method_for_path(name), Deflated, "{name} 应为 Deflated");
        }
    }

    #[test]
    fn alias_is_serialized_with_shared_bytes() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src = tmp.path().join("src.zip");
        write_zip_specs(
            &src,
            &[file("a.txt", b"shared".to_vec(), zip::CompressionMethod::Deflated)],
        );
        let mut pack =
            Pack::open_zip(&src, &crate::arom::limits::SafeLimits::preserving_current(), None)
                .expect("open");
        {
            let mut tx = pack.tx("test");
            tx.alias("a.txt", "b.txt").expect("alias");
            let layer = tx.into_layer();
            pack.commit(layer);
        }

        let out = tmp.path().join("out.zip");
        let stats = {
            let view = pack.view();
            write_zip(&pack, &view, &out, &SerializeOptions::default()).expect("write")
        };
        assert_eq!(stats.written, 1, "别名以写入形式落盘（字节仍是共享的）");
        assert_eq!(stats.passthrough + stats.recompressed, 1);

        let reopened = Pack::open_zip(&out, &crate::arom::limits::SafeLimits::preserving_current(), None)
            .expect("reopen");
        let view = reopened.view();
        assert_eq!(view.read("a.txt").expect("read").expect("some"), b"shared");
        assert_eq!(view.read("b.txt").expect("read").expect("some"), b"shared");
    }

    /// 真实样本对照（默认忽略）：把 `AROM_REAL_PACK` 指向一个真实资源包后运行
    /// `cargo test --lib real_pack_roundtrip -- --ignored --nocapture`。
    #[test]
    #[ignore]
    fn real_pack_roundtrip_matches_old_pipeline() {
        let Ok(src_path) = std::env::var("AROM_REAL_PACK") else {
            println!("AROM_REAL_PACK 未设置，跳过");
            return;
        };
        let src = std::path::PathBuf::from(&src_path);
        assert!(src.is_file(), "不是文件：{}", src.display());
        let tmp = tempfile::tempdir().expect("tempdir");

        let work = tmp.path().join("work");
        std::fs::create_dir_all(&work).expect("mkdir");
        crate::converters::zip::extract_resource_pack(
            src.to_str().expect("utf8"),
            work.to_str().expect("utf8"),
        )
        .expect("extract");
        let old_out = tmp.path().join("old.zip");
        crate::converters::zip::repack_resource_pack(
            work.to_str().expect("utf8"),
            old_out.to_str().expect("utf8"),
        )
        .expect("repack");

        let pack =
            Pack::open_zip(&src, &crate::arom::limits::SafeLimits::preserving_current(), None)
                .expect("open");
        let view = pack.view();
        let new_out = tmp.path().join("new.zip");
        let stats = write_zip(&pack, &view, &new_out, &SerializeOptions::default()).expect("write");

        let report = diff_containers(&old_out, &new_out).expect("diff");
        println!("{}", crate::converters::pack_diff::render_report(&report, true));
        println!("source = {}", src.display());
        println!("stats  = {stats:?}");

        assert_eq!(report.blocking, 0, "内容级差异：{:?}", report.diffs);
        assert_eq!(
            report.container_entry_set_blocking, 0,
            "条目集合差异：{:?}",
            report.container
        );
        assert_eq!(
            report.container_byte_only, 0,
            "容器字节属性差异：{:?}",
            report.container
        );
        assert!(report.passed(true), "严格模式必须通过");
    }

    #[test]
    fn mem_source_serializes_without_a_container() {
        let src = MemSource::new(vec![
            ("a.png".to_string(), b"not-a-real-png".to_vec()),
            ("dir/".to_string(), Vec::new()),
        ])
        .expect("mem");
        let pack = Pack::from_source(Box::new(src), None).expect("pack");

        let tmp = tempfile::tempdir().expect("tempdir");
        let out = tmp.path().join("out.zip");
        let stats = {
            let view = pack.view();
            write_zip(&pack, &view, &out, &SerializeOptions::default()).expect("write")
        };
        assert_eq!(stats.files, 1);
        assert_eq!(stats.passthrough, 1, "内存来源语义等同 stored");
        assert_eq!(stats.dirs, 1);
    }
}
