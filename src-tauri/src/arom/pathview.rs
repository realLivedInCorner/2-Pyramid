//! M1：`PathView` 兼容适配层 —— 让**未迁移**的旧任务继续以「临时目录 + 文件路径」工作。
//!
//! **§9.130 更新**：`HurrayContext` 已整体退场（任务闭包不再接收它，
//! `workdir` / `pack_name` 改为注册期捕获）。本适配层仍然必要 —— 服务的是
//! 「A-ROM 对象模型」与「workdir 形态的驱动步骤」之间的往返（例如批次后的
//! `surgeon_cut_gui::run_in_workdir` 直连步骤），与旧引擎无关了。
//!
//! 1. [`materialize`]：把当前有效条目落到磁盘（布局与现状管线一致：`dest` 就是包根，
//!    含空目录——现状解压同样会建出它们）；
//! 2. 旧任务照常在 `dest` 上读写（它看到的只是一个普通目录）；
//! 3. [`harvest`]：对比磁盘与落盘基线，产出**一层**——新增/改动 → `Blob`，
//!    删除 → `Tombstone`，新增空目录 → `Body::Dir`；并报告落在声明范围之外的写入。
//!
//! **代价是诚实的**：适配层必然付「落盘 + 回读哈希」的成本——这正是现状管线每一步都在付的
//! 成本。它换来的是迁移期不需要一次性改完 92 个任务，且两边的产出可被同一把闸门比对。
//!
//! 范围声明：旧任务自己不知道读写范围，因此调用方用 [`decl_prefix`] / [`decl_any`] 声明。
//! 声明 `any` 的任务与任何任务都冲突（`task::plan` 会把它排成串行），这正确反映了
//! 「它可能碰任何东西」的事实。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use sha2::{Digest, Sha256};

use super::error::AromError;
use super::layer::{Body, Layer, Pack, PackView, Slot};
use super::task::{ScopeSet, TaskDecl, Tier};

/// 落盘基线：我们写下去时每个文件的长度与内容哈希。
#[derive(Debug, Clone, Default)]
pub struct Materialized {
    pub files: BTreeMap<String, (u64, String)>,
    pub dirs: BTreeSet<String>,
}

impl Materialized {
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn dir_count(&self) -> usize {
        self.dirs.len()
    }
}

/// 收获结果：一层变更 + 分类清单 + 越界写入诊断。
#[derive(Debug, Clone, Default)]
pub struct Harvest {
    pub layer: Layer,
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub removed: Vec<String>,
    /// 写入了但不在声明范围内的路径（诊断用；调用方决定告警还是失败）。
    pub undeclared: Vec<String>,
}

impl Harvest {
    pub fn changed(&self) -> usize {
        self.layer.writes().len()
    }
}

/// 未迁移任务的范围声明：读写都算作「该前缀整段」。
pub fn decl_prefix(name: &str, tier: Tier, prefix: &str) -> TaskDecl {
    TaskDecl::new(name, tier)
        .reads(ScopeSet::prefix(prefix))
        .writes(ScopeSet::prefix(prefix))
}

/// 读写范围不同的声明。
///
/// **目录改名类任务必须用它**：`mcpatcher → optifine` 这类改写的**目标**在源前缀之外，
/// 若只声明源前缀，适配层会在 `Harvest::undeclared` 里把目标整片报出来
/// （这不是误报——声明确实错了）。
pub fn decl_scopes(name: &str, tier: Tier, reads: ScopeSet, writes: ScopeSet) -> TaskDecl {
    TaskDecl::new(name, tier).reads(reads).writes(writes)
}

/// 无法确定范围时：整包读写（必然与所有任务冲突 → 串行）。
pub fn decl_any(name: &str, tier: Tier) -> TaskDecl {
    TaskDecl::new(name, tier)
        .reads(ScopeSet::any())
        .writes(ScopeSet::any())
}

/// 把当前有效条目落到 `dest`，返回基线。
pub fn materialize(view: &PackView<'_>, dest: &Path) -> Result<Materialized, AromError> {
    std::fs::create_dir_all(dest)
        .map_err(|e| AromError::io(format!("create {}: {e}", dest.display())))?;

    let mut baseline = Materialized::default();

    for dir in view.effective_dirs()? {
        let full = dest.join(&dir);
        std::fs::create_dir_all(&full)
            .map_err(|e| AromError::io(format!("mkdir {}: {e}", full.display())))?;
        baseline.dirs.insert(dir);
    }

    for res in view.entries()? {
        if res.is_dir {
            continue;
        }
        let bytes = view
            .read(&res.path)?
            .ok_or_else(|| AromError::internal(format!("entry vanished: {}", res.path)))?;
        let full = dest.join(&res.path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AromError::io(format!("mkdir {}: {e}", parent.display())))?;
        }
        std::fs::write(&full, &bytes)
            .map_err(|e| AromError::io(format!("write {}: {e}", full.display())))?;
        baseline
            .files
            .insert(res.path.clone(), (bytes.len() as u64, sha256_hex(&bytes)));
    }

    Ok(baseline)
}

/// 收获：把 `dest` 与基线对比，产出一层（不提交；调用方决定何时 `commit`）。
pub fn harvest(
    pack: &Pack,
    dest: &Path,
    baseline: &Materialized,
    decl: Option<&TaskDecl>,
) -> Result<Harvest, AromError> {
    let mut on_disk: BTreeSet<String> = BTreeSet::new();
    let mut dirs_on_disk: BTreeSet<String> = BTreeSet::new();

    for entry in walkdir::WalkDir::new(dest).into_iter().filter_map(Result::ok) {
        let rel = entry
            .path()
            .strip_prefix(dest)
            .map_err(|e| AromError::internal(e.to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        if rel.is_empty() {
            continue;
        }
        if entry.file_type().is_dir() {
            dirs_on_disk.insert(rel);
        } else {
            on_disk.insert(rel);
        }
    }

    let mut harvest = Harvest::default();
    let mut layer = Layer::new();

    // 新增 / 改动
    for path in &on_disk {
        let bytes = std::fs::read(dest.join(path))
            .map_err(|e| AromError::io(format!("read {}: {e}", path)))?;
        let fingerprint = (bytes.len() as u64, sha256_hex(&bytes));
        let unchanged = baseline
            .files
            .get(path)
            .map(|known| *known == fingerprint)
            .unwrap_or(false);
        if unchanged {
            continue;
        }
        if baseline.files.contains_key(path) {
            harvest.modified.push(path.clone());
        } else {
            harvest.added.push(path.clone());
        }
        let id = pack.blobs().put(bytes)?;
        layer.set(path, Slot::Present(Body::Blob(id)));
    }

    // 删除（文件）
    for path in baseline.files.keys() {
        if !on_disk.contains(path) {
            harvest.removed.push(path.clone());
            layer.set(path, Slot::Tombstone);
        }
    }

    // 新增目录（显式空目录也要能表达）
    for dir in &dirs_on_disk {
        if !baseline.dirs.contains(dir) {
            layer.set(dir, Slot::Present(Body::Dir));
        }
    }

    // 删除（目录）
    for dir in &baseline.dirs {
        if !dirs_on_disk.contains(dir) {
            harvest.removed.push(dir.clone());
            layer.set(dir, Slot::Tombstone);
        }
    }

    if let Some(decl) = decl {
        harvest.undeclared = layer
            .writes()
            .keys()
            .filter(|path| !decl.writes.contains(path))
            .cloned()
            .collect();
    }

    harvest.layer = layer;
    Ok(harvest)
}

/// 一次完整的适配层运行：落盘 → 旧任务 → 收获（不提交）。
pub fn run_legacy<F>(
    pack: &Pack,
    dest: &Path,
    decl: &TaskDecl,
    task: F,
) -> Result<Harvest, AromError>
where
    F: FnOnce(&Path) -> Result<(), String>,
{
    let baseline = materialize(&pack.view(), dest)?;
    task(dest).map_err(|e| AromError::internal(format!("legacy task `{}`: {e}", decl.name)))?;
    harvest(pack, dest, &baseline, Some(decl))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arom::serialize::{write_zip, SerializeOptions};
    use crate::arom::{Pack, SafeLimits};
    use crate::pack::diff::diff_containers;
    use std::io::Write as _;

    fn fixture(path: &Path) {
        let f = std::fs::File::create(path).expect("create");
        let mut zip = zip::ZipWriter::new(f);
        let opts = zip::write::FileOptions::default();
        let mut add = |name: &str, body: &[u8]| {
            zip.start_file(name, opts).expect("start");
            zip.write_all(body).expect("write");
        };
        add("pack.mcmeta", br#"{"pack":{"pack_format":34}}"#);
        add("assets/minecraft/mcpatcher/cit/a.properties", b"a=1");
        add("assets/minecraft/mcpatcher/cit/deep/b.properties", b"b=2");
        add("assets/minecraft/textures/x.png", b"not-a-real-png");
        add("assets/minecraft/lang/zh_cn.json", br#"{"a":"b"}"#);
        zip.add_directory("assets/empty/", opts).expect("dir");
        zip.finish().expect("finish");
    }

    fn open(path: &Path) -> Pack {
        Pack::open_zip(path, &SafeLimits::preserving_current(), None).expect("open")
    }

    #[test]
    fn materialize_round_trips_a_pack() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let zip_path = tmp.path().join("fixture.zip");
        fixture(&zip_path);
        let pack = open(&zip_path);

        let work = tmp.path().join("work");
        let view = pack.view();
        let baseline = materialize(&view, &work).expect("materialize");

        assert_eq!(baseline.file_count(), 5);
        assert!(baseline.dirs.contains("assets/empty"), "空目录要落盘");
        assert!(baseline.dirs.contains("assets/minecraft/mcpatcher/cit/deep"));
        assert_eq!(
            std::fs::read(work.join("assets/minecraft/lang/zh_cn.json")).expect("read"),
            br#"{"a":"b"}"#
        );
    }

    #[test]
    fn harvest_detects_add_modify_and_remove() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let zip_path = tmp.path().join("fixture.zip");
        fixture(&zip_path);
        let mut pack = open(&zip_path);

        let work = tmp.path().join("work");
        let baseline = {
            let view = pack.view();
            materialize(&view, &work).expect("materialize")
        };

        // 旧任务式的改动：新增、改写、删除
        std::fs::create_dir_all(work.join("new/deep")).expect("mkdir");
        std::fs::write(work.join("new/deep/added.txt"), b"added").expect("write");
        std::fs::write(work.join("assets/minecraft/textures/x.png"), b"rewritten").expect("write");
        std::fs::remove_file(work.join("assets/minecraft/lang/zh_cn.json")).expect("remove");

        let harvest = harvest(&pack, &work, &baseline, None).expect("harvest");
        assert_eq!(harvest.added, vec!["new/deep/added.txt"]);
        assert_eq!(harvest.modified, vec!["assets/minecraft/textures/x.png"]);
        assert!(harvest.removed.contains(&"assets/minecraft/lang/zh_cn.json".to_string()));
        assert_eq!(harvest.undeclared.len(), 0, "未给声明时不做越界判定");

        pack.commit(harvest.layer);
        let view = pack.view();
        assert_eq!(
            view.read("new/deep/added.txt").expect("read").expect("some"),
            b"added"
        );
        assert_eq!(
            view.read("assets/minecraft/textures/x.png").expect("read").expect("some"),
            b"rewritten"
        );
        assert!(view.resolve("assets/minecraft/lang/zh_cn.json").is_none());
    }


    #[test]
    fn undeclared_writes_are_reported() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let zip_path = tmp.path().join("fixture.zip");
        fixture(&zip_path);
        let pack = open(&zip_path);

        let work = tmp.path().join("work");
        let decl = decl_prefix("scope-limited", Tier::Architect, "assets/minecraft/textures");
        let harvest = run_legacy(&pack, &work, &decl, |dir| {
            std::fs::write(dir.join("outside.txt"), b"oops").map_err(|e| e.to_string())
        })
        .expect("run_legacy");

        assert_eq!(
            harvest.undeclared,
            vec!["outside.txt"],
            "越界写入必须被报出来（诊断；是否失败由调用方决定）"
        );
    }
}
