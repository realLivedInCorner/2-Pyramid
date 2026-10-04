//! L1a：不可变基座（`BasePack`）—— arena + 路径驻留 + 索引。
//!
//! 设计要点（`astray-arom-model.md` §4）：
//!
//! * **arena + 稳定 ID**：条目放在 `Vec` 里，索引即 `EntryId`；不嵌套持有，不做全树递归查找。
//! * **路径驻留**：每个路径字符串只存一份，条目用 `PathId` 引用（对比 Foray ROM 每个节点各存一份路径字符串）。
//! * **零字节驻留、零哈希**：构建期**不读任何字节**，只记录 `len` 与来源索引（有单测用计数来源验证）。
//! * **目录是一等条目**：显式目录条目保留（现状会保留空目录）；文件隐含的祖先目录单独由
//!   [`BasePack::synthesized_dirs`] 给出——现状解压会在磁盘上建出这些目录并在重新打包时
//!   写成目录条目，所以序列化阶段必须补上它们。
//! * **索引 = 唯一路径**：源容器内同路径重复时，索引指向**最后**一个（对齐现状解压的「后者胜」），
//!   被遮蔽的条目标记 `shadowed` 但仍留在 arena 里供分析。

use std::collections::{BTreeMap, BTreeSet};

use super::error::AromError;
use super::source::Source;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PathId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryId(pub u32);

/// 路径驻留表。
#[derive(Debug, Default)]
pub struct PathInterner {
    by_id: Vec<String>,
    by_path: BTreeMap<String, PathId>,
}

impl PathInterner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, path: &str) -> PathId {
        if let Some(id) = self.by_path.get(path) {
            return *id;
        }
        let id = PathId(self.by_id.len() as u32);
        self.by_id.push(path.to_string());
        self.by_path.insert(path.to_string(), id);
        id
    }

    pub fn get(&self, id: PathId) -> Result<&str, AromError> {
        self.by_id
            .get(id.0 as usize)
            .map(|s| s.as_str())
            .ok_or_else(|| AromError::internal(format!("path id out of range: {}", id.0)))
    }

    pub fn id_of(&self, path: &str) -> Option<PathId> {
        self.by_path.get(path).copied()
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// 按驻留顺序（= 首次出现顺序）遍历；顺序确定。
    pub fn iter(&self) -> impl Iterator<Item = (PathId, &str)> {
        self.by_id
            .iter()
            .enumerate()
            .map(|(i, s)| (PathId(i as u32), s.as_str()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseEntry {
    /// 完整路径（驻留）。
    pub path: PathId,
    /// 父目录路径（驻留）；根下条目为 `None`。
    pub parent: Option<PathId>,
    /// 最后一段名字（驻留）。
    pub name: PathId,
    /// 文件字节数；目录为 0。
    pub len: u64,
    /// 内容来源：源容器内的条目索引。
    pub src_idx: u32,
    pub is_dir: bool,
    /// 同一路径在源容器内先出现过、本条目把它遮蔽了（分析用途）。
    pub shadowed: bool,
}

/// 不可变基座：一次转换的「源包视图」。`SYNTHETIC_SRC_IDX` 标记「没有源条目」的合成目录。
pub const SYNTHETIC_SRC_IDX: u32 = u32::MAX;

pub struct BasePack {
    entries: Vec<BaseEntry>,
    index: BTreeMap<PathId, EntryId>,
    paths: PathInterner,
    source_order: Vec<EntryId>,
}

impl BasePack {
    /// 从来源构建。**不读字节、不算哈希**。
    pub fn build(source: &dyn Source) -> Result<Self, AromError> {
        let mut paths = PathInterner::new();
        let mut entries: Vec<BaseEntry> = Vec::with_capacity(source.len());
        let mut index: BTreeMap<PathId, EntryId> = BTreeMap::new();
        let mut source_order: Vec<EntryId> = Vec::with_capacity(source.len());

        for i in 0..source.len() {
            let entry = source.entry(i as u32)?;

            // 祖先全部驻留（合成目录与改名规则都要用）
            let mut ancestors: Vec<PathId> = Vec::new();
            let mut cursor = parent_of(&entry.name);
            while let Some(p) = cursor {
                ancestors.push(paths.intern(p));
                cursor = parent_of(p);
            }

            let path = paths.intern(&entry.name);
            let name = paths.intern(basename_of(&entry.name));
            let id = EntryId(entries.len() as u32);

            if let Some(prev) = index.insert(path, id) {
                if let Some(slot) = entries.get_mut(prev.0 as usize) {
                    slot.shadowed = true;
                }
            }

            entries.push(BaseEntry {
                path,
                parent: ancestors.first().copied(),
                name,
                len: entry.meta.uncompressed_size,
                src_idx: entry.src_index,
                is_dir: entry.meta.is_dir,
                shadowed: false,
            });
            source_order.push(id);
        }

        // 输入里由文件**隐含**的祖先目录显式化。
        //
        // 现状管线把源包解压到磁盘（为每个文件建父目录），打包时目录遍历会把那些目录
        // 也写成条目——也就是说「文件被删掉后，它的空父目录仍然留在产物里」。
        // 视图必须复刻这一点，否则「删文件」会顺带剪掉父目录，产物条目集合与现状不一致
        // （M2 实测：夹具里 `textures/entity` 与 `textures/misc` 各差一个目录条目）。
        //
        // 不加入 `source_order`：它们不是源条目，只是基座视图的一部分。
        let implied_paths: Vec<String> = {
            let mut implied: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
            for entry in &entries {
                let mut cursor = entry.parent;
                while let Some(pid) = cursor {
                    if !index.contains_key(&pid) {
                        if let Ok(p) = paths.get(pid) {
                            implied.insert(p.to_string());
                        }
                    }
                    cursor = paths
                        .get(pid)
                        .ok()
                        .and_then(parent_of)
                        .and_then(|p| paths.id_of(p));
                }
            }
            implied.into_iter().collect()
        };
        for path_str in implied_paths {
            let name_str = basename_of(&path_str).to_string();
            let parent = parent_of(&path_str).and_then(|p| paths.id_of(p));
            let path = paths.intern(&path_str);
            let id = EntryId(entries.len() as u32);
            index.insert(path, id);
            entries.push(BaseEntry {
                path,
                parent,
                name: paths.intern(&name_str),
                len: 0,
                src_idx: SYNTHETIC_SRC_IDX,
                is_dir: true,
                shadowed: false,
            });
        }

        Ok(Self {
            entries,
            index,
            paths,
            source_order,
        })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 唯一路径数（索引大小）。
    pub fn unique_len(&self) -> usize {
        self.index.len()
    }

    pub fn paths(&self) -> &PathInterner {
        &self.paths
    }

    pub fn entry(&self, id: EntryId) -> Result<&BaseEntry, AromError> {
        self.entries
            .get(id.0 as usize)
            .ok_or_else(|| AromError::internal(format!("entry id out of range: {}", id.0)))
    }

    pub fn path_str(&self, id: PathId) -> Result<&str, AromError> {
        self.paths.get(id)
    }

    pub fn path_of(&self, id: EntryId) -> Result<&str, AromError> {
        self.paths.get(self.entry(id)?.path)
    }

    pub fn resolve(&self, path: &str) -> Option<EntryId> {
        self.paths.id_of(path).and_then(|p| self.resolve_id(p))
    }

    pub fn resolve_id(&self, path: PathId) -> Option<EntryId> {
        self.index.get(&path).copied()
    }

    pub fn is_dir(&self, id: EntryId) -> Result<bool, AromError> {
        Ok(self.entry(id)?.is_dir)
    }

    /// 源顺序（含被遮蔽条目）—— 分析用途。
    pub fn source_order(&self) -> impl Iterator<Item = (EntryId, &BaseEntry)> {
        self.source_order
            .iter()
            .map(move |id| (*id, &self.entries[id.0 as usize]))
    }

    /// 唯一路径视图，按 `PathId`（= 首次出现）顺序 —— 顺序确定。
    pub fn unique_entries(&self) -> impl Iterator<Item = (EntryId, &BaseEntry)> {
        self.index.iter().map(move |(_, id)| (*id, &self.entries[id.0 as usize]))
    }

    pub fn file_count(&self) -> usize {
        self.index
            .values()
            .filter(|id| !self.entries[id.0 as usize].is_dir)
            .count()
    }

    pub fn dir_count(&self) -> usize {
        self.index
            .values()
            .filter(|id| self.entries[id.0 as usize].is_dir)
            .count()
    }

    /// 被同名条目遮蔽的条目（源容器内重复路径）。
    pub fn shadowed(&self) -> Vec<EntryId> {
        self.source_order
            .iter()
            .copied()
            .filter(|id| self.entries[id.0 as usize].shadowed)
            .collect()
    }

    /// 文件路径隐含、但源容器里没有显式目录条目的祖先目录，**按路径排序**返回。
    ///
    /// 现状解压会在磁盘上建出这些目录，重新打包时它们会成为目录条目——序列化阶段必须补上。
    pub fn synthesized_dirs(&self) -> Vec<PathId> {
        let mut out: BTreeSet<PathId> = BTreeSet::new();
        for (_, entry) in self.unique_entries() {
            if entry.is_dir {
                continue;
            }
            let mut cursor = entry.parent;
            while let Some(pid) = cursor {
                if !self.index.contains_key(&pid) {
                    out.insert(pid);
                }
                cursor = self
                    .paths
                    .get(pid)
                    .ok()
                    .and_then(parent_of)
                    .and_then(|p| self.paths.id_of(p));
            }
        }
        let mut sorted: Vec<PathId> = out.into_iter().collect();
        sorted.sort_by(|a, b| {
            let pa = self.paths.get(*a).unwrap_or("");
            let pb = self.paths.get(*b).unwrap_or("");
            pa.cmp(pb)
        });
        sorted
    }
}

/// 最后一段名字。
pub fn basename_of(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[i + 1..],
        None => path,
    }
}

/// 父目录路径（根下条目返回 `None`）。
pub fn parent_of(path: &str) -> Option<&str> {
    path.rfind('/').map(|i| &path[..i])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arom::source::{MemSource, SourceEntry};
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn mem(files: Vec<(&str, Vec<u8>)>) -> MemSource {
        MemSource::new(files.into_iter().map(|(p, b)| (p.to_string(), b)).collect())
            .expect("mem source")
    }

    /// 计数来源：任何一次字节读取都会被记下——用来证明构建期零驻留。
    struct ReadCounter<'a> {
        inner: &'a MemSource,
        reads: AtomicUsize,
    }

    impl Source for ReadCounter<'_> {
        fn len(&self) -> usize {
            self.inner.len()
        }

        fn entry(&self, idx: u32) -> Result<&SourceEntry, AromError> {
            self.inner.entry(idx)
        }

        fn read(&self, idx: u32) -> Result<Vec<u8>, AromError> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            self.inner.read(idx)
        }

        fn copy_to(&self, idx: u32, out: &mut dyn Write) -> Result<(), AromError> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            self.inner.copy_to(idx, out)
        }

        fn copy_raw_to(&self, idx: u32, out: &mut dyn Write) -> Result<(), AromError> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            self.inner.copy_raw_to(idx, out)
        }
    }

    #[test]
    fn build_is_zero_residency() {
        let src = mem(vec![
            ("small.txt", b"hello".to_vec()),
            ("big.bin", vec![7u8; 2 * 1024 * 1024]),
        ]);
        let counter = ReadCounter {
            inner: &src,
            reads: AtomicUsize::new(0),
        };

        let pack = BasePack::build(&counter).expect("build");
        assert_eq!(
            counter.reads.load(Ordering::SeqCst),
            0,
            "构建期不得读取任何字节（零驻留）"
        );
        assert_eq!(pack.len(), 2);
        assert_eq!(
            pack.entry(pack.resolve("big.bin").expect("resolve")).expect("entry").len,
            2 * 1024 * 1024,
            "长度必须来自元数据而不是读取内容"
        );
    }

    #[test]
    fn index_resolves_paths_and_keeps_source_order() {
        let src = mem(vec![
            ("z.txt", b"z".to_vec()),
            ("assets/b.txt", b"b".to_vec()),
            ("a.txt", b"a".to_vec()),
        ]);
        let pack = BasePack::build(&src).expect("build");

        // 显式条目 + 由 `assets/b.txt` 隐含而显式化的 `assets` 目录
        assert_eq!(pack.unique_len(), 4);
        assert_eq!(pack.file_count(), 3);
        assert_eq!(pack.dir_count(), 1, "隐含祖先目录被显式化");
        assert!(pack.resolve("assets/b.txt").is_some());
        assert!(pack.resolve("assets").is_some(), "隐含目录成为条目");
        assert!(pack.resolve("nope.txt").is_none());

        let order: Vec<&str> = pack
            .source_order()
            .map(|(_, e)| pack.path_str(e.path).expect("path"))
            .collect();
        assert_eq!(order, vec!["z.txt", "assets/b.txt", "a.txt"], "源顺序契约只含源条目");

        // 祖先被驻留
        assert_eq!(pack.paths().id_of("assets").map(|id| pack.path_str(id).expect("p")), Some("assets"));
    }

    #[test]
    fn explicit_dirs_are_first_class_and_empty_dirs_survive() {
        let src = mem(vec![
            ("assets/", Vec::new()),
            ("assets/empty/", Vec::new()),
            ("assets/keep.txt", b"k".to_vec()),
        ]);
        let pack = BasePack::build(&src).expect("build");

        let empty = pack.resolve("assets/empty").expect("empty dir entry");
        assert!(pack.is_dir(empty).expect("is_dir"), "空目录条目必须存在");
        assert_eq!(pack.dir_count(), 2, "assets 与 assets/empty 都是显式目录");
        assert!(
            pack.synthesized_dirs().is_empty(),
            "目录都已显式存在，不应再合成"
        );
    }

    /// 输入里由文件隐含的祖先目录必须**成为条目**：现状管线解压时创建它们、
    /// 打包时把它们写成条目，所以「删掉文件」不应连带剪掉父目录。
    #[test]
    fn missing_ancestors_are_made_explicit() {
        let src = mem(vec![("a/b/c/d.txt", b"d".to_vec())]);
        let pack = BasePack::build(&src).expect("build");

        for path in ["a", "a/b", "a/b/c"] {
            let id = pack.resolve(path).unwrap_or_else(|| panic!("{path} 应成为条目"));
            assert!(pack.is_dir(id).expect("is_dir"), "{path} 应是目录");
        }
        assert_eq!(pack.dir_count(), 3, "三个隐含祖先目录都成为条目");
        assert_eq!(pack.file_count(), 1);
        assert!(
            pack.synthesized_dirs().is_empty(),
            "已显式化，无需再合成"
        );
        assert_eq!(
            pack.source_order().count(),
            1,
            "合成目录不进入源顺序"
        );

        // 显式目录存在时，不重复插入
        let src = mem(vec![("a/b/c/d.txt", b"d".to_vec()), ("a/b/", Vec::new())]);
        let pack = BasePack::build(&src).expect("build");
        assert_eq!(pack.dir_count(), 3, "a、a/b、a/b/c 各一个条目");
    }

    #[test]
    fn duplicate_paths_resolve_to_the_last_one() {
        let src = mem(vec![
            ("dup.txt", b"first".to_vec()),
            ("other.txt", b"x".to_vec()),
            ("dup.txt", b"second".to_vec()),
        ]);
        let pack = BasePack::build(&src).expect("build");

        assert_eq!(pack.unique_len(), 2, "索引按唯一路径");
        assert_eq!(pack.len(), 3, "arena 保留两个同名条目用于分析");

        let shadowed = pack.shadowed();
        assert_eq!(shadowed.len(), 1);
        let first = shadowed[0];
        assert_eq!(pack.path_of(first).expect("path"), "dup.txt");
        assert_eq!(
            pack.entry(first).expect("entry").src_idx,
            0,
            "被遮蔽的是先出现的那一条"
        );
        let resolved = pack.resolve("dup.txt").expect("resolve");
        assert_eq!(
            pack.entry(resolved).expect("entry").src_idx,
            2,
            "索引指向最后一个（对齐现状解压的后胜语义）"
        );
    }

    #[test]
    fn path_helpers_handle_roots_and_nesting() {
        assert_eq!(parent_of("a.txt"), None);
        assert_eq!(parent_of("a/b.txt"), Some("a"));
        assert_eq!(parent_of("a/b/c.txt"), Some("a/b"));
        assert_eq!(basename_of("a.txt"), "a.txt");
        assert_eq!(basename_of("a/b/c.txt"), "c.txt");
    }

    #[test]
    fn interner_stores_each_path_once() {
        let mut p = PathInterner::new();
        let a = p.intern("assets/x.txt");
        let b = p.intern("assets/x.txt");
        let c = p.intern("assets/y.txt");
        assert_eq!(a, b, "同路径必须复用同一个 PathId");
        assert_ne!(a, c);
        assert_eq!(p.len(), 2, "驻留表里只有两份字符串");
        assert_eq!(p.id_of("assets/y.txt"), Some(c));
        assert!(p.get(PathId(99)).is_err(), "越界 id 报错而不是 panic");
    }
}
