//! M2：混合运行驱动。
//!
//! 目标：让 **A-ROM 接管一次真实转换的读入与写出**，而任务仍按既定节奏迁移。
//! 第一版（本文件）刻意不要求迁移任何任务——全部旧任务经 [`pathview`](crate::arom::pathview)
//! 适配层跑，用来回答一个必须先回答的问题：
//!
//! > 把「解压到临时目录 → 跑任务 → 重新打包」换成
//! > 「建 Pack → 落盘给任务 → 收成层 → A-ROM 序列化」，
//! > **产物是不是一模一样？**
//!
//! 流程：
//!
//! ```text
//! Pack::open(输入)
//!   → materialize(有效条目落盘 + 基线哈希)
//!   → legacy_run(workdir)        // 旧执行器，签名不变：给它一个目录
//!   → harvest(对比基线产出一层) → commit
//!   → write_zip(A-ROM 序列化)
//! ```
//!
//! 与旧管线的唯一差别就是「字节从哪里来、写到哪里去」，因此两者的产物必须逐项一致——
//! 这正是本模块的验收方式（见文件末尾用例）。
//!
//! 迁移期往后，本驱动会逐步把注册表里的任务换成原生实现（`arom::task::plan` 负责排序与并行），
//! 未迁移的继续走适配层；编译期开关按模块控制（已裁决：D6 按模块）。

use std::path::Path;

use crate::arom::pathview::{harvest, materialize};
use crate::arom::serialize::{write_zip, SerializeOptions, SerializeStats};
use crate::arom::{AromError, Pack, SafeLimits};

#[derive(Debug, Clone)]
pub struct MixedRunOptions {
    /// 读入限额。已裁决取宽松值（不改变今天能转的包）。
    pub limits: SafeLimits,
    pub serialize: SerializeOptions,
    /// 层内字节预算（`None` = 不限；溢出后端在后续里程碑接入）。
    pub blob_limit: Option<u64>,
}

impl Default for MixedRunOptions {
    fn default() -> Self {
        Self {
            limits: SafeLimits::preserving_current(),
            serialize: SerializeOptions::default(),
            blob_limit: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MixedRunReport {
    pub materialized_files: usize,
    pub materialized_dirs: usize,
    /// 旧任务改动到的条目数（层里的写入数）。
    pub harvested_changes: usize,
    pub added: usize,
    pub modified: usize,
    pub removed: usize,
    /// 层写入中落在声明范围之外者（本版传入的声明为 `None`，故为空）。
    pub undeclared: Vec<String>,
    pub stats: SerializeStats,
}

/// 用 A-ROM 接管读入与写出，任务由 `legacy_run` 在 `workdir` 上执行。
///
/// `legacy_run` 的签名刻意与旧管线一致：**给它一个目录，它自己跑完所有任务**
/// （例如 `invoke_conversion_ex(...)` + 收尾的 `pack.mcmeta` 改写）。
pub fn run_with_legacy_tasks<F>(
    input: &Path,
    workdir: &Path,
    output: &Path,
    opts: &MixedRunOptions,
    legacy_run: F,
) -> Result<MixedRunReport, AromError>
where
    F: FnOnce(&Path) -> Result<(), String>,
{
    ensure_empty_dir(workdir)?;

    let mut pack = Pack::open_zip(input, &opts.limits, opts.blob_limit)?;

    // 1) 落盘（含空目录——旧执行器在磁盘上看到的目录必须完整）
    let baseline = {
        let view = pack.view();
        materialize(&view, workdir)?
    };

    // 2) 旧任务照常读写目录
    legacy_run(workdir).map_err(AromError::internal)?;

    // 3) 收成一层
    let harvested = harvest(&pack, workdir, &baseline, None)?;

    let report = MixedRunReport {
        materialized_files: baseline.file_count(),
        materialized_dirs: baseline.dir_count(),
        harvested_changes: harvested.changed(),
        added: harvested.added.len(),
        modified: harvested.modified.len(),
        removed: harvested.removed.len(),
        undeclared: harvested.undeclared.clone(),
        stats: SerializeStats::default(),
    };
    pack.commit(harvested.layer);

    // 4) A-ROM 序列化
    let stats = {
        let view = pack.view();
        write_zip(&pack, &view, output, &opts.serialize)?
    };

    Ok(MixedRunReport { stats, ..report })
}

/// 工作目录必须是空目录：残留文件会被 `harvest` 误判成「任务新增」。
fn ensure_empty_dir(dir: &Path) -> Result<(), AromError> {
    if dir.exists() {
        let mut entries = std::fs::read_dir(dir)
            .map_err(|e| AromError::io(format!("read {}: {e}", dir.display())))?;
        if entries.next().is_some() {
            return Err(AromError::io(format!(
                "work dir must be empty: {}",
                dir.display()
            )));
        }
    } else {
        std::fs::create_dir_all(dir)
            .map_err(|e| AromError::io(format!("create {}: {e}", dir.display())))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::converters::pack_diff::diff_containers;
    use crate::converters::version_converter::write_pack_format;
    use std::io::Write as _;
    use std::path::PathBuf;

    fn png(size: (u32, u32)) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(size.0, size.1, image::Rgba([120, 60, 30, 255]));
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .expect("encode png");
        buf
    }

    /// 夹具刻意带上几个会被真实任务处理的结构（mcpatcher 目录、font 目录、
    /// item 下的动画 mcmeta、旧贴图路径）。
    fn fixture(path: &Path) {
        let f = std::fs::File::create(path).expect("create fixture");
        let mut zip = zip::ZipWriter::new(f);
        let opts = zip::write::FileOptions::default();
        let mut add = |name: &str, body: Vec<u8>| {
            zip.start_file(name, opts).expect("start");
            zip.write_all(&body).expect("write");
        };
        add(
            "pack.mcmeta",
            br#"{"pack":{"pack_format":34,"description":"fixture"}}"#.to_vec(),
        );
        add("assets/minecraft/mcpatcher/cit/a.properties", b"a=1".to_vec());
        add("assets/minecraft/font/default.json", b"{}".to_vec());
        add("assets/minecraft/terrain.png", png((16, 16)));
        add("assets/minecraft/gui/items.png", png((16, 32)));
        add("assets/minecraft/textures/item/water.png", png((32, 64)));
        add(
            "assets/minecraft/textures/item/water.png.mcmeta",
            br#"{"animation": {}}"#.to_vec(),
        );
        add("assets/minecraft/lang/zh_cn.json", br#"{"a":"b"}"#.to_vec());
        zip.add_directory("assets/empty/", opts).expect("dir");
        zip.finish().expect("finish");
    }

    /// 旧管线的等价物：解压 → 跑任务 → 重打包（与 `process_zip_timed` 同序）。
    fn legacy_output(input: &Path, tmp: &Path, target: u32, source: u32) -> PathBuf {
        let work = tmp.join("legacy_work");
        std::fs::create_dir_all(&work).expect("mkdir");
        crate::converters::zip::extract_resource_pack(
            input.to_str().expect("utf8"),
            work.to_str().expect("utf8"),
        )
        .expect("extract");

        crate::invoke_conversion::invoke_conversion_ex(input, &work, target, source, true, false, true)
            .expect("legacy pipeline");
        let mcmeta = work.join("pack.mcmeta");
        if mcmeta.exists() {
            write_pack_format(&mcmeta, target).expect("write_pack_format");
        }

        let out = tmp.join("legacy.zip");
        crate::converters::zip::repack_resource_pack(
            work.to_str().expect("utf8"),
            out.to_str().expect("utf8"),
        )
        .expect("repack");
        out
    }

    fn mixed_output(input: &Path, tmp: &Path, target: u32, source: u32) -> (PathBuf, MixedRunReport) {
        let work = tmp.join("mixed_work");
        let out = tmp.join("mixed.zip");
        let report = run_with_legacy_tasks(
            input,
            &work,
            &out,
            &MixedRunOptions::default(),
            |dir| {
                // `target_path` 与旧管线一致地传输入包（`invoke_conversion_ex` 用它推导包名），
                // 否则两边会因为包名不同而产生差异——那与「谁负责 IO」无关。
                crate::invoke_conversion::invoke_conversion_ex(input, dir, target, source, true, false, true)
                    .map_err(|e| e.to_string())?;
                let mcmeta = dir.join("pack.mcmeta");
                if mcmeta.exists() {
                    write_pack_format(&mcmeta, target).map_err(|e| e.to_string())?;
                }
                Ok(())
            },
        )
        .expect("mixed run");
        (out, report)
    }

    fn assert_equivalent(legacy: &Path, mixed: &Path) -> crate::converters::pack_diff::PackDiffReport {
        let report = diff_containers(legacy, mixed).expect("diff");
        assert_eq!(report.blocking, 0, "内容差异：{:?}", report.diffs);
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
        assert!(report.passed(true), "{:?}", report.container);
        report
    }

    #[test]
    fn a_rom_owns_io_of_a_real_conversion_on_a_fixture() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let input = tmp.path().join("fixture.zip");
        fixture(&input);

        let legacy = legacy_output(&input, tmp.path(), 97, 34);
        let (mixed, report) = mixed_output(&input, tmp.path(), 97, 34);

        assert_equivalent(&legacy, &mixed);
        assert!(report.materialized_files >= 8, "{report:?}");
        assert!(report.materialized_dirs >= 5, "空目录也要落盘：{report:?}");
        assert!(report.stats.files > 0, "{report:?}");
    }

    #[test]
    fn work_dir_must_be_empty() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let input = tmp.path().join("fixture.zip");
        fixture(&input);
        let work = tmp.path().join("dirty");
        std::fs::create_dir_all(&work).expect("mkdir");
        std::fs::write(work.join("leftover.txt"), b"x").expect("write");

        let err = run_with_legacy_tasks(
            &input,
            &work,
            &tmp.path().join("out.zip"),
            &MixedRunOptions::default(),
            |_| Ok(()),
        )
        .expect_err("must reject dirty work dir");
        assert_eq!(err.kind(), "io");
    }

    /// 真实包上的等价性（默认忽略）：
    /// `AROM_REAL_PACK=<包> [AROM_TARGET=97] cargo test --lib mixed_run -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn a_rom_owns_io_of_a_real_conversion_on_a_real_pack() {
        let Ok(src) = std::env::var("AROM_REAL_PACK") else {
            println!("AROM_REAL_PACK 未设置，跳过");
            return;
        };
        let input = PathBuf::from(&src);
        assert!(input.is_file(), "不是文件：{}", input.display());
        let target: u32 = std::env::var("AROM_TARGET")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(97);

        // 源格式从包里读（L2 视图）
        let source = {
            let pack = Pack::open_zip(&input, &SafeLimits::preserving_current(), None)
                .expect("open for source format");
            pack.view()
                .mcmeta()
                .ok()
                .and_then(|m| m.effective_format())
                .unwrap_or(34)
        };
        println!("source format = {source}, target = {target}");

        let tmp = tempfile::tempdir().expect("tempdir");
        let legacy = legacy_output(&input, tmp.path(), target, source);
        let (mixed, report) = mixed_output(&input, tmp.path(), target, source);
        let diff = assert_equivalent(&legacy, &mixed);
        println!("mixed run = {report:?}");
        println!("diff = {}", diff.summary());
    }
}
