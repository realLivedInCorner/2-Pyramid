//! L1b：层与事务（copy-on-write）。
//!
//! 语义（`astray-arom-model.md` §5）：
//!
//! * **层 = 前缀改名规则 + 路径覆盖写入**，层内顺序固定为 **先改名（作用于上一层结果）再写入**，
//!   因此任务写入的路径永远是**最终路径**；
//! * **目录级改名用前缀规则**，不物化子条目（有单测断言「改名后写入表仍为空」）；
//! * **未写入的路径自动字节透传**：`dirty` 概念不存在——不在任何层里出现就是未触碰；
//! * **回滚 = 丢弃整层**；**并行隔离**由层提供；**冲突**在合并时检测（默认拒绝）；
//! * 可观测性由层结构直接给出：谁在哪个层写了哪些路径。
//!
//! 实现取舍（与总纲 §5 的差异，如实记录）：层的键用**路径字符串**而不是 `PathId`，
//! 改名规则也用字符串前缀。原因是基础包（`BasePack`）的驻留表在构建后不可变，
//! 若层要用 `PathId` 就得引入共享可变驻留表或克隆整张表；而层的写入量远小于基础条目量
//! （几十 vs 几千），字符串键的代价可以忽略，且让「反向映射」退化为纯字符串运算。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

use image::RgbaImage;

use super::error::AromError;
use super::limits::SafeLimits;
use super::source::{Source, ZipSource};
use super::store::BasePack;

pub type BlobId = u32;

/// 任务生成/改写后的字节仓库（Step 4 接溢出后端）。
#[derive(Debug, Default)]
pub struct BlobStore {
    blobs: Vec<std::sync::Arc<Vec<u8>>>,
    bytes: u64,
    limit: Option<u64>,
}

impl BlobStore {
    pub fn new(limit: Option<u64>) -> Self {
        Self {
            blobs: Vec::new(),
            bytes: 0,
            limit,
        }
    }

    /// 写入一份新字节。超预算直接失败（溢出后端在 Step 4 接入）。
    pub fn put(&mut self, data: Vec<u8>) -> Result<BlobId, AromError> {
        let size = data.len() as u64;
        if let Some(limit) = self.limit {
            if self.bytes.saturating_add(size) > limit {
                return Err(AromError::Budget(format!(
                    "blob budget exceeded: {} + {} > {} bytes",
                    self.bytes, size, limit
                )));
            }
        }
        let id = self.blobs.len() as BlobId;
        self.bytes += size;
        self.blobs.push(std::sync::Arc::new(data));
        Ok(id)
    }

    pub fn get(&self, id: BlobId) -> Result<std::sync::Arc<Vec<u8>>, AromError> {
        self.blobs
            .get(id as usize)
            .cloned()
            .ok_or_else(|| AromError::internal(format!("blob id out of range: {id}")))
    }

    pub fn len(&self) -> usize {
        self.blobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blobs.is_empty()
    }

    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    pub fn limit(&self) -> Option<u64> {
        self.limit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameMode {
    /// 原路径消失。
    Move,
    /// 原路径保留（复制）。
    Copy,
}

/// 前缀改名规则：`from` 与其子树的路径映射到 `to` 之下。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefixRule {
    pub from: String,
    pub to: String,
    pub mode: RenameMode,
}

/// 条目内容的来源。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    /// 源容器条目（惰性读取，不驻留）。
    Base { src_idx: u32 },
    /// 层里写入的字节。
    Blob(BlobId),
    /// 指向另一条路径（复制/移动不复制字节）。
    Alias(String),
    /// 显式目录（`mkdir`）。
    Dir,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Slot {
    Present(Body),
    Tombstone,
}

/// 一层变更。
#[derive(Debug, Default, Clone)]
pub struct Layer {
    renames: Vec<PrefixRule>,
    writes: BTreeMap<String, Slot>,
}

impl Layer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.renames.is_empty() && self.writes.is_empty()
    }

    pub fn writes(&self) -> &BTreeMap<String, Slot> {
        &self.writes
    }

    pub fn renames(&self) -> &[PrefixRule] {
        &self.renames
    }

    pub fn set(&mut self, path: &str, slot: Slot) {
        self.writes.insert(normalize(path), slot);
    }

    pub fn add_rename(&mut self, rule: PrefixRule) -> Result<(), AromError> {
        if rule.from.is_empty() || rule.to.is_empty() {
            return Err(AromError::path("rename prefix must not be empty"));
        }
        if rule.from == rule.to {
            return Err(AromError::path(format!(
                "rename to the same prefix: {}",
                rule.from
            )));
        }
        self.renames.push(rule);
        Ok(())
    }

    /// 反向映射（解析用）：把本层输出命名空间里的路径映射回输入命名空间。
    ///
    /// `Move` 与 `Copy` 都要映射：`Copy` 规则下新路径同样只由规则产生，
    /// 而**原路径**不在任何规则的目标前缀之下，因此不会被误映射。
    fn map_back(&self, path: &str) -> String {
        let mut cur = path.to_string();
        for rule in self.renames.iter().rev() {
            if let Some(rest) = strip_prefix(&cur, &rule.to) {
                cur = join(&rule.from, &rest);
            }
        }
        cur
    }

    /// 本层是否把该**输出**路径隐藏了：它落在某个 `Move` 规则的来源前缀之下。
    ///
    /// 没有这一步，`Move` 之后原路径仍会穿透到下层（`Copy` 不会，原路径本就该保留）。
    fn hides_path(&self, path: &str) -> bool {
        self.renames
            .iter()
            .any(|rule| rule.mode == RenameMode::Move && strip_prefix(path, &rule.from).is_some())
    }

    /// 正向映射（枚举用）：一个输入路径在本层输出命名空间里可能对应多个路径（`Copy`）。
    fn map_forward_all(&self, path: &str) -> Vec<String> {
        let mut current = vec![path.to_string()];
        for rule in &self.renames {
            let mut next: Vec<String> = Vec::new();
            for p in current {
                match strip_prefix(&p, &rule.from) {
                    Some(rest) => {
                        next.push(join(&rule.to, &rest));
                        if rule.mode == RenameMode::Copy {
                            next.push(p);
                        }
                    }
                    None => next.push(p),
                }
            }
            current = next;
        }
        current
    }

    /// 本层对某路径的裁决：精确命中，或祖先被删除（整棵子树隐藏）。
    fn lookup(&self, path: &str) -> Option<Slot> {
        if let Some(slot) = self.writes.get(path) {
            return Some(slot.clone());
        }
        for anc in ancestors_of(path) {
            if let Some(Slot::Tombstone) = self.writes.get(anc) {
                return Some(Slot::Tombstone);
            }
        }
        None
    }
}

/// 解析结果：一条**有效条目**（基础条目或层写入）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub path: String,
    pub is_dir: bool,
    pub len: u64,
    pub body: Body,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictPolicy {
    /// 默认：并行层写入同一路径（或一方删除另一方的祖先）→ 报冲突。
    Reject,
    /// 后者胜（按提交顺序），并允许调用方记录告警。
    LastWins,
}

/// 一个受管资源包：不可变基座 + 已提交的层 + 字节仓库 + 类型化视图缓存。
///
/// `blobs` 与 `cache` 在 `Mutex` 后面：这样 `Tx` 只借用 `&Pack`，
/// **同一波次里的多个任务可以各自持有事务并发写入**（层本身互不相交，
/// 冲突由 [`Pack::commit_batch`] 在合并时检测）。
pub struct Pack {
    base: BasePack,
    source: Box<dyn Source>,
    blobs: Mutex<BlobStore>,
    layers: Vec<Layer>,
    version: u64,
    cache: Mutex<super::view::ViewCache>,
}

impl Pack {
    pub fn from_source(source: Box<dyn Source>, blob_limit: Option<u64>) -> Result<Self, AromError> {
        let base = BasePack::build(source.as_ref())?;
        Ok(Self {
            base,
            source,
            blobs: Mutex::new(BlobStore::new(blob_limit)),
            layers: Vec::new(),
            version: 0,
            cache: Mutex::new(super::view::ViewCache::default()),
        })
    }

    pub fn open_zip(path: &Path, limits: &SafeLimits, blob_limit: Option<u64>) -> Result<Self, AromError> {
        let source = Box::new(ZipSource::open(path, limits)?);
        Self::from_source(source, blob_limit)
    }

    pub fn base(&self) -> &BasePack {
        &self.base
    }

    pub fn source(&self) -> &dyn Source {
        self.source.as_ref()
    }

    /// 字节仓库。锁只在取/放单个 blob 期间持有，不跨读取。
    pub fn blobs(&self) -> std::sync::MutexGuard<'_, BlobStore> {
        self.blobs.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// 每次提交自增；视图缓存（Step 5）以此为失效依据。
    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn view(&self) -> PackView<'_> {
        PackView {
            pack: self,
            pending: None,
        }
    }

    /// 提交一层（原样保留其改名规则与写入）。
    pub fn commit(&mut self, layer: Layer) {
        if layer.is_empty() {
            return;
        }
        self.layers.push(layer);
        self.version += 1;
    }

    /// 提交一批并行产生的层：先按策略做冲突检测，再按给定顺序提交。
    pub fn commit_batch(
        &mut self,
        layers: Vec<Layer>,
        policy: ConflictPolicy,
    ) -> Result<(), AromError> {
        if policy == ConflictPolicy::Reject {
            if let Some((a, b, path)) = find_conflict(&layers) {
                return Err(AromError::Conflict(format!(
                    "layers #{a} and #{b} both touch `{path}`"
                )));
            }
        }
        for layer in layers {
            self.commit(layer);
        }
        Ok(())
    }

    /// 读取一个内容体的字节（`Alias` 会沿链解析）。
    pub fn read_body(&self, body: &Body) -> Result<Vec<u8>, AromError> {
        match body {
            Body::Base { src_idx } => self.source.read(*src_idx),
            Body::Blob(id) => Ok(self.blobs().get(*id)?.as_ref().clone()),
            Body::Alias(target) => {
                let resolved = self.view().resolve(target).ok_or_else(|| {
                    AromError::internal(format!("alias target not found: {target}"))
                })?;
                if resolved.is_dir {
                    return Err(AromError::path(format!("alias target is a directory: {target}")));
                }
                self.read_body(&resolved.body)
            }
            Body::Dir => Ok(Vec::new()),
        }
    }

    fn blob_len(&self, id: BlobId) -> Result<u64, AromError> {
        Ok(self.blobs().get(id)?.len() as u64)
    }

    /// 视图缓存读取（版本变化即整体作废）。
    pub(crate) fn cached_mcmeta(&self, path: &str) -> Option<Arc<super::view::PackMeta>> {
        let mut cache = self.cache.lock().ok()?;
        cache.sync(self.version);
        cache.get_mcmeta(path)
    }

    pub(crate) fn store_mcmeta(&self, path: &str, meta: Arc<super::view::PackMeta>) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.sync(self.version);
            cache.put_mcmeta(path, meta);
        }
    }

    pub(crate) fn cached_image(&self, path: &str) -> Option<Arc<RgbaImage>> {
        let mut cache = self.cache.lock().ok()?;
        cache.sync(self.version);
        cache.get_image(path)
    }

    pub(crate) fn store_image(&self, path: &str, img: Arc<RgbaImage>) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.sync(self.version);
            cache.put_image(path, img);
        }
    }

    /// 诊断/测试：缓存中的条目数。
    pub fn cached_mcmeta_count(&self) -> usize {
        match self.cache.lock() {
            Ok(mut cache) => {
                cache.sync(self.version);
                cache.cached_mcmeta()
            }
            Err(_) => 0,
        }
    }

    pub fn cached_image_count(&self) -> usize {
        match self.cache.lock() {
            Ok(mut cache) => {
                cache.sync(self.version);
                cache.cached_images()
            }
            Err(_) => 0,
        }
    }

    /// 把一个写入体描述成有效条目（补齐 `is_dir` / `len`）。
    fn describe(&self, path: &str, body: &Body) -> Result<Resolved, AromError> {
        Ok(match body {
            Body::Base { src_idx } => {
                // 写入表里引用源条目：元数据从基座取
                let entry = self
                    .base
                    .source_order()
                    .find(|(_, e)| e.src_idx == *src_idx)
                    .map(|(_, e)| e)
                    .ok_or_else(|| {
                        AromError::internal(format!("base src_idx not found: {src_idx}"))
                    })?;
                Resolved {
                    path: path.to_string(),
                    is_dir: entry.is_dir,
                    len: entry.len,
                    body: body.clone(),
                }
            }
            Body::Blob(id) => Resolved {
                path: path.to_string(),
                is_dir: false,
                len: self.blob_len(*id)?,
                body: body.clone(),
            },
            Body::Alias(target) => {
                let resolved = self.view().resolve(target).ok_or_else(|| {
                    AromError::path(format!("alias target not found: {target}"))
                })?;
                Resolved {
                    path: path.to_string(),
                    is_dir: resolved.is_dir,
                    len: resolved.len,
                    body: body.clone(),
                }
            }
            Body::Dir => Resolved {
                path: path.to_string(),
                is_dir: true,
                len: 0,
                body: body.clone(),
            },
        })
    }

    /// 开始一个事务。只借用 `&self`：**多个任务可以同时持事务并发写**，
    /// 各自的层互不相交，冲突在 [`Pack::commit_batch`] 合并时检测。
    pub fn tx(&self, origin: &str) -> Tx<'_> {
        Tx {
            pack: self,
            layer: Layer::new(),
            origin: origin.to_string(),
        }
    }
}

/// 只读视图：基座 + 已提交层（+ 可选的在途层）。
pub struct PackView<'a> {
    /// 供同模块树的视图层（`view.rs`）访问缓存与基座。
    pub(crate) pack: &'a Pack,
    pending: Option<&'a Layer>,
}

impl<'a> PackView<'a> {
    pub fn version(&self) -> u64 {
        self.pack.version
    }

    /// 视图是否包含在途事务层。在途视图**不参与**类型化缓存。
    pub fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    pub fn base(&self) -> &BasePack {
        &self.pack.base
    }

    fn layers_top_down(&self) -> impl Iterator<Item = &'a Layer> {
        self.pack
            .layers
            .iter()
            .rev()
            .chain(self.pending.into_iter())
    }

    fn layers_bottom_up(&self) -> impl Iterator<Item = &'a Layer> {
        self.pending.into_iter().chain(self.pack.layers.iter())
    }

    /// 按最终命名空间里的路径解析。惰性：沿途只做表查找与字符串前缀运算。
    pub fn resolve(&self, path: &str) -> Option<Resolved> {
        let path = normalize(path);
        let mut cursor = path.clone();
        for layer in self.layers_top_down() {
            match layer.lookup(&cursor) {
                Some(Slot::Present(body)) => {
                    return self.pack.describe(&path, &body).ok();
                }
                Some(Slot::Tombstone) => return None,
                None => {
                    if layer.hides_path(&cursor) {
                        return None;
                    }
                    cursor = layer.map_back(&cursor);
                }
            }
        }
        let entry_id = self.pack.base.resolve(&cursor)?;
        let entry = self.pack.base.entry(entry_id).ok()?;
        Some(Resolved {
            path,
            is_dir: entry.is_dir,
            len: entry.len,
            body: Body::Base {
                src_idx: entry.src_idx,
            },
        })
    }

    pub fn read(&self, path: &str) -> Result<Option<Vec<u8>>, AromError> {
        match self.resolve(path) {
            None => Ok(None),
            Some(res) => {
                if res.is_dir {
                    return Err(AromError::path(format!("`{path}` is a directory")));
                }
                Ok(Some(self.pack.read_body(&res.body)?))
            }
        }
    }

    /// 物化全部有效条目（顺序：路径排序，确定性）。枚举是 O(条目数)，解析仍是惰性的。
    pub fn entries(&self) -> Result<Vec<Resolved>, AromError> {
        let mut map: BTreeMap<String, Resolved> = BTreeMap::new();
        for (_, entry) in self.pack.base.unique_entries() {
            let path = self.pack.base.path_str(entry.path)?.to_string();
            map.insert(
                path.clone(),
                Resolved {
                    path,
                    is_dir: entry.is_dir,
                    len: entry.len,
                    body: Body::Base {
                        src_idx: entry.src_idx,
                    },
                },
            );
        }

        for layer in self.layers_bottom_up() {
            if !layer.renames.is_empty() {
                let mut next: BTreeMap<String, Resolved> = BTreeMap::new();
                for res in map.into_values() {
                    for mapped in layer.map_forward_all(&res.path) {
                        let mut copy = res.clone();
                        copy.path = mapped.clone();
                        next.insert(mapped, copy);
                    }
                }
                map = next;
            }
            for (path, slot) in layer.writes() {
                match slot {
                    Slot::Tombstone => {
                        map.remove(path);
                        map.retain(|p, _| !is_under(p, path));
                    }
                    Slot::Present(body) => {
                        let res = self.pack.describe(path, body)?;
                        map.insert(path.clone(), res);
                    }
                }
            }
        }

        Ok(map.into_values().collect())
    }

    /// 序列化用的目录集合：显式目录 ∪ 各自隐含的祖先目录。
    ///
    /// 现状解压会在磁盘上把这些目录都建出来（文件的父目录、以及显式目录自身的父目录），
    /// 重新打包时它们都会成为目录条目——序列化必须补齐，否则闸门会报「目录条目单边存在」。
    pub fn effective_dirs(&self) -> Result<BTreeSet<String>, AromError> {
        let mut dirs: BTreeSet<String> = BTreeSet::new();
        let entries = self.entries()?;
        for res in &entries {
            if res.is_dir {
                dirs.insert(res.path.clone());
            }
        }
        for res in &entries {
            let mut cursor = parent_of(&res.path);
            while let Some(p) = cursor {
                if !p.is_empty() {
                    dirs.insert(p.to_string());
                }
                cursor = parent_of(p);
            }
        }
        Ok(dirs)
    }
}

/// 事务：读走视图（含自己的在途写入），写只落到自己的层。
pub struct Tx<'a> {
    pack: &'a Pack,
    layer: Layer,
    origin: String,
}

impl<'a> Tx<'a> {
    pub fn origin(&self) -> &str {
        &self.origin
    }

    pub fn layer(&self) -> &Layer {
        &self.layer
    }

    pub fn view(&self) -> PackView<'_> {
        PackView {
            pack: self.pack,
            pending: Some(&self.layer),
        }
    }

    pub fn resolve(&self, path: &str) -> Option<Resolved> {
        self.view().resolve(path)
    }

    pub fn read(&self, path: &str) -> Result<Option<Vec<u8>>, AromError> {
        self.view().read(path)
    }

    // ── 只读访问器：一律返回 owned 数据，因此「先读后写」不会与 `&mut self` 冲突 ──

    pub fn exists(&self, path: &str) -> bool {
        self.view().resolve(path).is_some()
    }

    /// 「这个前缀里有东西吗」——显式条目**或**任意子条目。
    ///
    /// 容器里常常没有目录的显式条目（目录由文件隐含），因此判断「目录是否存在」
    /// 不能只看 `exists`；旧任务在磁盘上看到的目录永远是存在的。
    pub fn has_prefix(&self, prefix: &str) -> Result<bool, AromError> {
        if self.exists(prefix) {
            return Ok(true);
        }
        Ok(!self.list(prefix)?.is_empty())
    }

    /// 列出前缀下的条目（owned）。
    pub fn list(&self, prefix: &str) -> Result<Vec<Resolved>, AromError> {
        let prefix = prefix.trim_end_matches('/').to_string();
        let entries = self.view().entries()?;
        Ok(entries
            .into_iter()
            .filter(|r| is_under(&r.path, &prefix))
            .collect())
    }

    pub fn text(&self, path: &str) -> Result<String, AromError> {
        self.view().text(path)
    }

    pub fn json(&self, path: &str) -> Result<serde_json::Value, AromError> {
        self.view().json(path)
    }

    pub fn image(&self, path: &str) -> Result<Arc<RgbaImage>, AromError> {
        self.view().image(path)
    }

    pub fn put(&mut self, path: &str, bytes: Vec<u8>) -> Result<(), AromError> {
        let id = self.pack.blobs().put(bytes)?;
        self.layer.set(path, Slot::Present(Body::Blob(id)));
        Ok(())
    }

    /// 编码为 PNG 后写入。编码设置与 `image` 的默认保存路径一致（迁移转换器时需逐位核对）。
    pub fn put_image(&mut self, path: &str, img: &RgbaImage) -> Result<(), AromError> {
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgba8(img.clone())
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .map_err(|e| AromError::internal(format!("encode png `{path}`: {e}")))?;
        self.put(path, buf)
    }

    /// 复制/移动不复制字节：目标路径只是指向来源路径的别名。
    pub fn alias(&mut self, from: &str, to: &str) -> Result<(), AromError> {
        let from = normalize(from);
        if from == normalize(to) {
            return Err(AromError::path("alias source and target are the same"));
        }
        if self.view().resolve(&from).is_none() {
            return Err(AromError::path(format!("alias source not found: {from}")));
        }
        self.layer.set(to, Slot::Present(Body::Alias(from)));
        Ok(())
    }

    /// 删除文件或整棵子树。
    pub fn remove(&mut self, path: &str) -> Result<(), AromError> {
        self.layer.set(path, Slot::Tombstone);
        Ok(())
    }

    /// 目录整体改名（前缀规则，不物化子条目）。
    pub fn rename_dir(&mut self, from: &str, to: &str) -> Result<(), AromError> {
        self.add_prefix_rule(from, to, RenameMode::Move)
    }

    /// 目录整体复制（前缀规则，不复制字节）。
    pub fn copy_dir(&mut self, from: &str, to: &str) -> Result<(), AromError> {
        self.add_prefix_rule(from, to, RenameMode::Copy)
    }

    fn add_prefix_rule(&mut self, from: &str, to: &str, mode: RenameMode) -> Result<(), AromError> {
        let from = normalize(from);
        let to = normalize(to);
        // 目录可能只由子条目隐含存在（容器里没有显式目录条目）→ 用 has_prefix 判定
        if !self.has_prefix(&from)? {
            return Err(AromError::path(format!("rename source not found: {from}")));
        }
        self.layer.add_rename(PrefixRule { from, to, mode })
    }

    /// 建目录（显式空目录——现状会保留空目录，序列化必须能表达）。
    pub fn mkdir(&mut self, path: &str) -> Result<(), AromError> {
        self.layer.set(path, Slot::Present(Body::Dir));
        Ok(())
    }

    /// 取出层；交给 [`Pack::commit`] 或 [`Pack::commit_batch`] 合并。
    ///
    /// 事务只持有 `&Pack`（这样同一波次的任务可以**并发**持事务），因此提交必须由
    /// `Pack` 侧发起：
    ///
    /// ```text
    /// let layer = tx.into_layer();
    /// pack.commit(layer);
    /// ```
    pub fn into_layer(self) -> Layer {
        self.layer
    }

    /// 丢弃（回滚）：层被 drop，包保持原状。
    pub fn abort(self) {}
}

/// 两两检测并行层冲突：同一路径被两个层写入，或一方删除的子树覆盖另一方的写入。
fn find_conflict(layers: &[Layer]) -> Option<(usize, usize, String)> {
    for i in 0..layers.len() {
        for j in (i + 1)..layers.len() {
            for (path, slot_a) in layers[i].writes() {
                if layers[j].writes().contains_key(path) {
                    return Some((i, j, path.clone()));
                }
                if let Slot::Tombstone = slot_a {
                    for other in layers[j].writes().keys() {
                        if is_under(other, path) {
                            return Some((i, j, other.clone()));
                        }
                    }
                }
            }
            for (path, slot_b) in layers[j].writes() {
                if let Slot::Tombstone = slot_b {
                    for other in layers[i].writes().keys() {
                        if is_under(other, path) {
                            return Some((i, j, other.clone()));
                        }
                    }
                }
            }
        }
    }
    None
}

fn normalize(path: &str) -> String {
    path.trim_matches('/').replace('\\', "/")
}

fn is_under(path: &str, prefix: &str) -> bool {
    path.len() > prefix.len() && path.starts_with(prefix) && path.as_bytes()[prefix.len()] == b'/'
}

/// `path` 落在 `prefix` 之下时返回剩余部分（同为前缀时返回空串）。
fn strip_prefix(path: &str, prefix: &str) -> Option<String> {
    if path == prefix {
        return Some(String::new());
    }
    path.strip_prefix(prefix)
        .and_then(|rest| rest.strip_prefix('/'))
        .map(|rest| rest.to_string())
}

fn join(prefix: &str, rest: &str) -> String {
    if rest.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}/{rest}")
    }
}

fn parent_of(path: &str) -> Option<&str> {
    path.rfind('/').map(|i| &path[..i])
}

fn ancestors_of(path: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut cursor = parent_of(path);
    while let Some(p) = cursor {
        out.push(p);
        cursor = parent_of(p);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arom::source::MemSource;

    fn pack_with(files: Vec<(&str, Vec<u8>)>) -> Pack {
        let src = MemSource::new(files.into_iter().map(|(p, b)| (p.to_string(), b)).collect())
            .expect("mem source");
        Pack::from_source(Box::new(src), None).expect("pack")
    }

    fn tree() -> Pack {
        pack_with(vec![
            ("assets/", Vec::new()),
            ("assets/a.txt", b"alpha".to_vec()),
            ("assets/sub/b.txt", b"beta".to_vec()),
            ("root.txt", b"root".to_vec()),
        ])
    }

    #[test]
    fn resolve_sees_pending_and_committed_writes() {
        let mut pack = tree();

        // 包视图：提交前必须是原始内容
        assert_eq!(
            pack.view().read("assets/a.txt").expect("read").expect("some"),
            b"alpha",
            "提交前包内容不变"
        );

        // 在途写入：事务自己能看到自己的写入
        let mut tx = pack.tx("t1");
        tx.put("assets/a.txt", b"changed".to_vec()).expect("put");
        assert_eq!(
            tx.read("assets/a.txt").expect("read").expect("some"),
            b"changed"
        );
        {
            let layer = tx.into_layer();
            pack.commit(layer);
        }

        assert_eq!(
            pack.view().read("assets/a.txt").expect("read").expect("some"),
            b"changed"
        );
        assert_eq!(pack.version(), 1, "提交必须递增版本（缓存失效依据）");
    }

    #[test]
    fn tombstone_hides_entry_and_its_subtree() {
        let mut pack = tree();
        let mut tx = pack.tx("eraser");
        tx.remove("assets").expect("remove");
        {
            let layer = tx.into_layer();
            pack.commit(layer);
        }

        let view = pack.view();
        assert!(view.resolve("assets/a.txt").is_none(), "子树一并隐藏");
        assert!(view.resolve("assets/sub/b.txt").is_none());
        assert!(view.resolve("assets").is_none());
        assert!(view.resolve("root.txt").is_some(), "无关条目不受影响");

        let paths: Vec<String> = view.entries().expect("entries").into_iter().map(|e| e.path).collect();
        assert_eq!(paths, vec!["root.txt"]);
    }

    #[test]
    fn prefix_rename_is_a_rule_not_materialized_writes() {
        let mut pack = tree();
        let mut tx = pack.tx("bedrock");
        tx.rename_dir("assets", "x/assets").expect("rename");
        {
            let layer = tx.into_layer();
            pack.commit(layer);
        }

        assert!(
            pack.layers()[0].writes().is_empty(),
            "目录改名必须是指令而不是物化写入（否则 3k 条目会炸）"
        );
        assert_eq!(pack.layers()[0].renames().len(), 1);

        let view = pack.view();
        assert!(view.resolve("x/assets/a.txt").is_some(), "改名后可见");
        assert!(view.resolve("x/assets/sub/b.txt").is_some(), "子树跟着走");
        assert!(view.resolve("assets/a.txt").is_none(), "Move：原路径消失");

        // 「改名不新增条目」：与同一棵未改名的树逐条目相同
        // （条目集合里除了文件与显式目录，还包含由文件隐含而显式化的祖先目录）
        let count = view.entries().expect("entries").len();
        let plain = tree();
        let same = plain.view().entries().expect("entries").len();
        assert_eq!(count, same, "改名不新增条目");
    }

    #[test]
    fn copy_rule_keeps_the_original_path() {
        let mut pack = tree();
        let mut tx = pack.tx("backup");
        tx.copy_dir("assets", "backup").expect("copy");
        {
            let layer = tx.into_layer();
            pack.commit(layer);
        }

        let view = pack.view();
        assert!(view.resolve("assets/a.txt").is_some(), "Copy：原路径保留");
        assert!(view.resolve("backup/a.txt").is_some());
        assert_eq!(
            view.read("backup/a.txt").expect("read").expect("some"),
            b"alpha",
            "复制不复制字节，内容一致"
        );
    }

    #[test]
    fn alias_shares_bytes_without_new_blob() {
        let mut pack = tree();
        let mut tx = pack.tx("alias");
        tx.alias("assets/a.txt", "copy.txt").expect("alias");
        {
            let layer = tx.into_layer();
            pack.commit(layer);
        }

        assert_eq!(pack.blobs().len(), 0, "别名不产生新字节");
        assert_eq!(pack.blobs().bytes(), 0);

        let view = pack.view();
        assert_eq!(view.read("copy.txt").expect("read").expect("some"), b"alpha");
        assert_eq!(
            view.resolve("copy.txt").expect("resolved").len,
            5,
            "长度由目标条目给出"
        );
    }

    #[test]
    fn alias_to_missing_target_is_rejected() {
        let mut pack = tree();
        let mut tx = pack.tx("alias");
        let err = tx.alias("nope.txt", "copy.txt").expect_err("must reject");
        assert_eq!(err.kind(), "path");
    }

    #[test]
    fn commit_batch_rejects_conflicts_and_lastwins_merges() {
        let mut pack = tree();
        let l1 = {
            let mut tx = pack.tx("t1");
            tx.put("same.txt", b"one".to_vec()).expect("put");
            tx.into_layer()
        };
        let l2 = {
            let mut tx = pack.tx("t2");
            tx.put("same.txt", b"two".to_vec()).expect("put");
            tx.into_layer()
        };

        let err = pack
            .commit_batch(vec![l1.clone(), l2.clone()], ConflictPolicy::Reject)
            .expect_err("conflict must be reported");
        assert_eq!(err.kind(), "conflict");
        assert!(pack.layers().is_empty(), "冲突时不得留下半成品层");

        pack.commit_batch(vec![l1, l2], ConflictPolicy::LastWins)
            .expect("last wins");
        assert_eq!(
            pack.view().read("same.txt").expect("read").expect("some"),
            b"two"
        );
    }

    #[test]
    fn abort_keeps_the_pack_unchanged() {
        let mut pack = tree();
        let before = pack.version();
        {
            let mut tx = pack.tx("t1");
            tx.put("assets/a.txt", b"changed".to_vec()).expect("put");
            tx.abort();
        }
        assert_eq!(pack.version(), before);
        assert!(pack.layers().is_empty());
        assert_eq!(
            pack.view().read("assets/a.txt").expect("read").expect("some"),
            b"alpha",
            "中止后内容必须原样"
        );
        assert_eq!(pack.blobs().len(), 1, "已写入的字节留在仓库里，但没有任何层引用它");
    }

    #[test]
    fn blob_budget_is_enforced() {
        let src = MemSource::new(vec![("a.txt".to_string(), b"a".to_vec())]).expect("mem");
        let mut pack = Pack::from_source(Box::new(src), Some(4)).expect("pack");

        let mut tx = pack.tx("t1");
        let err = tx.put("big.txt", vec![0u8; 5]).expect_err("budget");
        assert_eq!(err.kind(), "budget");
        tx.put("ok.txt", vec![0u8; 4]).expect("within budget");
    }

    #[test]
    fn mkdir_and_put_toggle_dir_state() {
        let mut pack = tree();
        let mut tx = pack.tx("t1");
        tx.mkdir("new/empty").expect("mkdir");
        {
            let layer = tx.into_layer();
            pack.commit(layer);
        }

        let view = pack.view();
        let dir = view.resolve("new/empty").expect("resolved");
        assert!(dir.is_dir);

        let mut tx = pack.tx("t2");
        tx.put("new/empty", b"now a file".to_vec()).expect("put");
        {
            let layer = tx.into_layer();
            pack.commit(layer);
        }
        let file = pack.view().resolve("new/empty").expect("resolved");
        assert!(!file.is_dir, "后写入的文件覆盖目录语义");
        assert_eq!(file.len, 10, "`now a file` 共 10 字节");
    }

    #[test]
    fn effective_dirs_cover_explicit_and_synthesized() {
        let pack = pack_with(vec![
            ("assets/empty/", Vec::new()),
            ("assets/deep/x/y.txt", b"y".to_vec()),
        ]);
        let dirs: Vec<String> = pack
            .view()
            .effective_dirs()
            .expect("dirs")
            .into_iter()
            .collect();
        assert_eq!(
            dirs,
            vec!["assets", "assets/deep", "assets/deep/x", "assets/empty"],
            "显式空目录 + 文件隐含祖先目录（现状解压会在磁盘上建出它们）"
        );
    }
}
