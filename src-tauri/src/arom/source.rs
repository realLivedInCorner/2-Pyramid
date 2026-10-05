//! L0：内容来源（`Source`）。
//!
//! 设计要点（依据 `astray-arom-model.md` §3）：
//!
//! * **句柄常开**：`ZipSource` 在包的生命周期内持有容器句柄，解决了 Foray ROM
//!   「导出时必须重新打开源包」（`foray/export.rs:58–65`）的死结；
//! * **零驻留**：这里只保存条目元数据，**不保存字节**；字节按需读取（惰性）；
//! * **顺序契约**：`entries` 的索引顺序 == 源容器内的条目顺序，这是产物确定性的来源；
//! * **目录条目是一等公民**：现状会保留空目录（`converters/zip.rs:411–413`、测试 `zip.rs:707`），
//!   因此目录条目**不被跳过**（这点与 `foray/zip_safe.rs:125–127` 相反）；
//! * **原始压缩字节**：`copy_raw_to` 提供流式原始字节，用于 `Stored` 条目的零解压透传
//!   （D23：只有「源方法 == 白名单方法 == `Stored`」才允许透传）。

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

use super::error::AromError;
use super::limits::SafeLimits;

/// 句柄池上限：并发读时最多同时打开的容器句柄数（D21）。
const MAX_HANDLES: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMeta {
    pub is_dir: bool,
    /// `stored` / `deflated` / 其它（小写）。
    pub method: String,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub crc32: u32,
    /// 归一化的 zip 时间戳 `(年, 月, 日, 时, 分, 秒)`。
    pub mtime: Option<(u16, u8, u8, u8, u8, u8)>,
    pub unix_mode: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceEntry {
    /// 规范化 posix 路径（包根相对，**不含**尾斜杠；目录由 `meta.is_dir` 表示）。
    pub name: String,
    pub meta: SourceMeta,
    /// 在源容器内的索引（顺序契约的来源）。
    pub src_index: u32,
    /// 同一（规范化路径, 是否目录）在容器内出现多次。
    ///
    /// 现状管线解压时是「后者胜」，不会报错；因此这里也只**标记**不拒绝，
    /// 把语义决定权交给上层（Step 2/3）。
    pub duplicate: bool,
    /// 条目带符号链接位。
    ///
    /// 现状只在 `#[cfg(unix)]` 下拒绝符号链接（`converters/zip.rs:399–406`），
    /// Windows 构建连读都不读；这里改为**始终标记**（信息更全但不改变接受集合）。
    pub symlink: bool,
}

/// 条目字节的来源。
pub trait Source: Send + Sync {
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn entry(&self, idx: u32) -> Result<&SourceEntry, AromError>;

    fn name(&self, idx: u32) -> Result<&str, AromError> {
        Ok(self.entry(idx)?.name.as_str())
    }

    fn meta(&self, idx: u32) -> Result<&SourceMeta, AromError> {
        Ok(&self.entry(idx)?.meta)
    }

    /// 解压后的字节（惰性：调用时才读取）。
    fn read(&self, idx: u32) -> Result<Vec<u8>, AromError>;

    /// 解压后的字节，流式写入（大条目不必整体驻留）。
    fn copy_to(&self, idx: u32, out: &mut dyn Write) -> Result<(), AromError>;

    /// **原始压缩字节**，流式写入（不含本地文件头与数据描述符）。
    ///
    /// 仅当 `meta.method == "stored"` 时，其结果等于解压后的字节——这是 D23 允许
    /// 透传的唯一情形；`deflated` 条目原样透传会改变产物字节。
    fn copy_raw_to(&self, idx: u32, out: &mut dyn Write) -> Result<(), AromError>;
}

fn open_archive(path: &Path) -> Result<zip::ZipArchive<BufReader<File>>, AromError> {
    let file = File::open(path).map_err(|e| AromError::io(format!("open {}: {e}", path.display())))?;
    zip::ZipArchive::new(BufReader::new(file))
        .map_err(|e| AromError::zip(format!("read zip {}: {e}", path.display())))
}

fn method_name(method: zip::CompressionMethod) -> String {
    match method {
        zip::CompressionMethod::Stored => "stored".to_string(),
        zip::CompressionMethod::Deflated => "deflated".to_string(),
        other => format!("{other:?}").to_ascii_lowercase(),
    }
}

/// 把容器内路径规范化为包根相对 posix 路径；失败即表示逃逸或超限。
fn normalize_zip_path(raw: &str, limits: &SafeLimits) -> Result<(String, usize), AromError> {
    let raw = raw.replace('\\', "/");
    if raw.contains('\0') {
        return Err(AromError::path(format!("zip entry contains NUL: {raw}")));
    }
    if raw.starts_with('/') {
        return Err(AromError::path(format!("zip absolute path rejected: {raw}")));
    }
    let mut parts: Vec<String> = Vec::new();
    for comp in Path::new(&raw).components() {
        match comp {
            Component::Normal(s) => {
                let s = s.to_string_lossy();
                if s == ".." || s == "." {
                    return Err(AromError::path(format!("zip path traversal rejected: {raw}")));
                }
                parts.push(s.to_string());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(AromError::path(format!("zip path traversal rejected: {raw}")));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(AromError::path(format!("zip absolute path rejected: {raw}")));
            }
        }
    }
    if parts.is_empty() {
        return Err(AromError::path(format!("zip entry has empty path: {raw}")));
    }
    let depth = parts.len();
    limits.check_depth(depth, &raw)?;
    Ok((parts.join("/"), depth))
}

/// zip 容器来源：句柄常开 + 句柄池并发读（D21）。
pub struct ZipSource {
    path: PathBuf,
    entries: Vec<SourceEntry>,
    limits: SafeLimits,
    /// 空闲句柄池。`Mutex` 只护住池本身（取/放），读取期间不持锁，
    /// 因此并发读之间不会串行化。
    pool: Mutex<Vec<zip::ZipArchive<BufReader<File>>>>,
}

/// 手写 `Debug`：句柄池里的 `ZipArchive` 不实现 `Debug`，但诊断与测试都想要它。
impl std::fmt::Debug for ZipSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ZipSource")
            .field("path", &self.path)
            .field("entries", &self.entries.len())
            .finish()
    }
}

impl ZipSource {
    /// 打开并校验容器：**不落盘、不驻留字节**，只读元数据。
    pub fn open(path: &Path, limits: &SafeLimits) -> Result<Self, AromError> {
        let mut archive = open_archive(path)?;
        limits.check_entries(archive.len())?;

        let mut entries = Vec::with_capacity(archive.len());
        let mut seen: HashMap<(String, bool), u32> = HashMap::new();
        let mut total_bytes: u64 = 0;

        for i in 0..archive.len() {
            let entry = archive
                .by_index(i)
                .map_err(|e| AromError::zip(format!("entry {i}: {e}")))?;

            let (name, _) = normalize_zip_path(entry.name(), limits)?;
            let is_dir = entry.is_dir();
            let uncompressed_size = entry.size();
            let compressed_size = entry.compressed_size();

            if !is_dir {
                limits.check_file_bytes(uncompressed_size, &name)?;
                limits.check_ratio(uncompressed_size, compressed_size, &name)?;
                total_bytes = total_bytes.saturating_add(uncompressed_size);
                limits.check_total_bytes(total_bytes)?;
            }

            let unix_mode = entry.unix_mode();
            let symlink = unix_mode
                .map(|m| m & 0o170000 == 0o120000)
                .unwrap_or(false);
            let mtime = {
                let t = entry.last_modified();
                Some((
                    t.year(),
                    t.month(),
                    t.day(),
                    t.hour(),
                    t.minute(),
                    t.second(),
                ))
            };

            let duplicate = seen.insert((name.clone(), is_dir), i as u32).is_some();

            entries.push(SourceEntry {
                name,
                meta: SourceMeta {
                    is_dir,
                    method: method_name(entry.compression()),
                    compressed_size,
                    uncompressed_size,
                    crc32: entry.crc32(),
                    mtime,
                    unix_mode,
                },
                src_index: i as u32,
                duplicate,
                symlink,
            });
        }

        Ok(Self {
            path: path.to_path_buf(),
            entries,
            limits: limits.clone(),
            pool: Mutex::new(vec![archive]),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn limits(&self) -> &SafeLimits {
        &self.limits
    }

    /// 条目名（目录带尾斜杠）——回到容器内写法，供序列化与展示使用。
    pub fn container_name(&self, idx: u32) -> Result<String, AromError> {
        let entry = self.entry(idx)?;
        Ok(if entry.meta.is_dir {
            format!("{}/", entry.name)
        } else {
            entry.name.clone()
        })
    }

    /// 所有被标记为重复的条目索引。
    pub fn duplicate_indices(&self) -> Vec<u32> {
        self.entries
            .iter()
            .filter(|e| e.duplicate)
            .map(|e| e.src_index)
            .collect()
    }

    /// 所有被标记为符号链接的条目索引。
    pub fn symlink_indices(&self) -> Vec<u32> {
        self.entries
            .iter()
            .filter(|e| e.symlink)
            .map(|e| e.src_index)
            .collect()
    }

    /// 取一个容器句柄执行操作，用完放回池中（池空则新开，数量有上限）。
    fn with_archive<T>(
        &self,
        f: impl FnOnce(&mut zip::ZipArchive<BufReader<File>>) -> Result<T, AromError>,
    ) -> Result<T, AromError> {
        let pooled = {
            let mut pool = self
                .pool
                .lock()
                .map_err(|_| AromError::internal("zip handle pool poisoned"))?;
            pool.pop()
        };
        let mut archive = match pooled {
            Some(a) => a,
            None => open_archive(&self.path)?,
        };

        let result = f(&mut archive);

        if let Ok(mut pool) = self.pool.lock() {
            if pool.len() < MAX_HANDLES {
                pool.push(archive);
            }
        }
        result
    }

    fn expect_file(&self, idx: u32) -> Result<&SourceEntry, AromError> {
        let entry = self.entry(idx)?;
        if entry.meta.is_dir {
            return Err(AromError::path(format!(
                "entry {idx} ({}) is a directory",
                entry.name
            )));
        }
        Ok(entry)
    }
}

impl Source for ZipSource {
    fn len(&self) -> usize {
        self.entries.len()
    }

    fn entry(&self, idx: u32) -> Result<&SourceEntry, AromError> {
        self.entries
            .get(idx as usize)
            .ok_or_else(|| AromError::internal(format!("entry index out of range: {idx}")))
    }

    fn read(&self, idx: u32) -> Result<Vec<u8>, AromError> {
        let size = self.expect_file(idx)?.meta.uncompressed_size;
        self.with_archive(|archive| {
            let mut entry = archive
                .by_index(idx as usize)
                .map_err(|e| AromError::zip(format!("entry {idx}: {e}")))?;
            let mut buf = Vec::with_capacity(size as usize);
            entry
                .read_to_end(&mut buf)
                .map_err(|e| AromError::zip(format!("read entry {idx}: {e}")))?;
            Ok(buf)
        })
    }

    fn copy_to(&self, idx: u32, out: &mut dyn Write) -> Result<(), AromError> {
        self.expect_file(idx)?;
        self.with_archive(|archive| {
            let mut entry = archive
                .by_index(idx as usize)
                .map_err(|e| AromError::zip(format!("entry {idx}: {e}")))?;
            std::io::copy(&mut entry, out)
                .map_err(|e| AromError::io(format!("copy entry {idx}: {e}")))?;
            Ok(())
        })
    }

    fn copy_raw_to(&self, idx: u32, out: &mut dyn Write) -> Result<(), AromError> {
        self.expect_file(idx)?;
        self.with_archive(|archive| {
            let mut entry = archive
                .by_index_raw(idx as usize)
                .map_err(|e| AromError::zip(format!("raw entry {idx}: {e}")))?;
            std::io::copy(&mut entry, out)
                .map_err(|e| AromError::io(format!("raw copy entry {idx}: {e}")))?;
            Ok(())
        })
    }
}

/// 最小内存来源：L0 的第二个实现，用于测试与后续 Blob 后端（Step 4）。
///
/// 语义等同「全部条目都是 `stored`」，因此原始字节与解压字节相同。
pub struct MemSource {
    entries: Vec<(SourceEntry, Vec<u8>)>,
}

impl MemSource {
    /// `files`: (路径, 字节)。路径以 `/` 结尾者视为目录。
    pub fn new(files: Vec<(String, Vec<u8>)>) -> Result<Self, AromError> {
        let mut entries = Vec::with_capacity(files.len());
        for (i, (raw_name, bytes)) in files.into_iter().enumerate() {
            let is_dir = raw_name.ends_with('/');
            let len = bytes.len() as u64;
            entries.push((
                SourceEntry {
                    name: raw_name.trim_end_matches('/').to_string(),
                    meta: SourceMeta {
                        is_dir,
                        method: "stored".to_string(),
                        compressed_size: len,
                        uncompressed_size: len,
                        crc32: 0,
                        mtime: None,
                        unix_mode: None,
                    },
                    src_index: i as u32,
                    duplicate: false,
                    symlink: false,
                },
                bytes,
            ));
        }
        Ok(Self { entries })
    }
}

impl Source for MemSource {
    fn len(&self) -> usize {
        self.entries.len()
    }

    fn entry(&self, idx: u32) -> Result<&SourceEntry, AromError> {
        self.entries
            .get(idx as usize)
            .map(|(e, _)| e)
            .ok_or_else(|| AromError::internal(format!("entry index out of range: {idx}")))
    }

    fn read(&self, idx: u32) -> Result<Vec<u8>, AromError> {
        Ok(self
            .entries
            .get(idx as usize)
            .ok_or_else(|| AromError::internal(format!("entry index out of range: {idx}")))?
            .1
            .clone())
    }

    fn copy_to(&self, idx: u32, out: &mut dyn Write) -> Result<(), AromError> {
        out.write_all(&self.read(idx)?)
            .map_err(|e| AromError::io(format!("write entry {idx}: {e}")))
    }

    fn copy_raw_to(&self, idx: u32, out: &mut dyn Write) -> Result<(), AromError> {
        self.copy_to(idx, out)
    }
}

/// **目录来源**（§9.139）：把一棵**已解压的目录树**当作 `Source`。
///
/// ## 为什么需要它
///
/// 生产入口（`pack::version_converter`）在预检阶段**已经把输入 zip 解压到 `temp_dir`**
/// （日志里的 `extracting zip`）。但 A-ROM 驱动只吃 zip（`Pack::open_zip`），于是旧路径被迫
/// 把刚解开的那棵树**重新打包成 zip 再读回来**（实测 `pack=0.56s`，还要多写一份全量 zip）。
///
/// 有了 `DirSource`，管线可以直接吃那棵**已经在磁盘上的树**，省掉这趟往返。
/// 这不是权宜之计：`arom/mod.rs` 的 L0 源类型本来就写着
/// 「`ZipSource` · `BlobSource` · `DirSource`」——**`DirSource` 此前从未实现**，
/// 这里是把它补上。
///
/// ## 语义
///
/// * **路径**：目录相对路径（posix 分隔符），与 zip 源同一套规范化；
/// * **顺序**：**按路径排序**。zip 的顺序来自容器，而目录树没有天然顺序；
///   排序是这里唯一确定的契约，也是产物可复现的来源（调用方不应依赖 zip 的偶然顺序——
///   真实包实测两者产物指纹一致）；
/// * **`method` = `"stored"`**：目录里的字节本来就是未压缩的，因此
///   `copy_raw_to` 与 `copy_to` 等价（这正是 D23 允许零解压透传的情形）；
/// * **`crc32`**：按内容实算，与 zip 条目同一定义（产物写出时用它填 CRC 字段）；
/// * **空目录保留**：与 zip 源一致，目录条目是一等公民。
pub struct DirSource {
    root: PathBuf,
    entries: Vec<SourceEntry>,
}

impl DirSource {
    /// 扫描并校验目录树（**不驻留字节**，只读元数据；字节按需读取）。
    pub fn open(root: &Path, limits: &SafeLimits) -> Result<Self, AromError> {
        let mut found: Vec<(String, bool, u64, bool)> = Vec::new();
        let mut total_bytes: u64 = 0;

        for entry in walkdir::WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            let entry_path = entry.path();
            if entry_path == root {
                continue;
            }
            let rel = entry_path
                .strip_prefix(root)
                .map_err(|e| AromError::internal(e.to_string()))?
                .to_string_lossy()
                .replace('\\', "/");
            if rel.is_empty() {
                continue;
            }
            let is_dir = entry.file_type().is_dir();
            let size = if is_dir {
                0
            } else {
                entry.metadata().map(|m| m.len()).unwrap_or(0)
            };
            if !is_dir {
                limits.check_file_bytes(size, &rel)?;
                total_bytes = total_bytes.saturating_add(size);
                limits.check_total_bytes(total_bytes)?;
            }
            found.push((rel, is_dir, size, entry_is_symlink(entry_path)));
        }

        limits.check_entries(found.len())?;

        // 顺序契约：按路径排序（目录树没有容器顺序，排序是唯一确定的契约）。
        found.sort_by(|a, b| a.0.cmp(&b.0));

        let entries = found
            .into_iter()
            .enumerate()
            .map(|(i, (name, is_dir, size, symlink))| SourceEntry {
                name,
                meta: SourceMeta {
                    is_dir,
                    // 目录里的字节是未压缩的：标记 `stored` 使 D23 的零解压透传成立。
                    method: "stored".to_string(),
                    compressed_size: size,
                    uncompressed_size: size,
                    // 目录源不预扫内容：CRC 是**按需**算的（见 `DirSource::crc32`）。
                    // 唯一读 `meta.crc32` 的消费者是单测与 `pack::diff` 的**输入包**比较，
                    // 序列化器不读它（zip 的 CRC 由写出时的实际字节决定）。
                    crc32: 0,
                    mtime: None,
                    unix_mode: None,
                },
                src_index: i as u32,
                duplicate: false,
                symlink,
            })
            .collect();

        Ok(Self {
            root: root.to_path_buf(),
            entries,
        })
    }

    /// 条目的绝对路径。
    pub fn path_of(&self, idx: u32) -> Result<PathBuf, AromError> {
        let e = self
            .entries
            .get(idx as usize)
            .ok_or_else(|| AromError::internal(format!("entry index out of range: {idx}")))?;
        Ok(self.root.join(&e.name))
    }

    /// 按需算条目内容的 CRC32（与 zip 条目同一定义）。
    ///
    /// `meta.crc32` 在目录源里恒为 `0`：扫描阶段只读元数据、**不读内容**，
    /// 而 CRC 必须读内容才能得到（19 MB 的真实包上白白多读一遍）。需要 CRC 的调用方
    /// 走这里。序列化器不需要它——zip 写出的 CRC 由其实际写入的字节决定，
    /// 唯一读 `meta.crc32` 的是单测与 `pack::diff` 的**输入包**比较。
    pub fn crc32(&self, idx: u32) -> Result<u32, AromError> {
        let bytes = self.read(idx)?;
        let mut crc = flate2::Crc::new();
        crc.update(&bytes);
        Ok(crc.sum())
    }
}

fn entry_is_symlink(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

impl Source for DirSource {
    fn len(&self) -> usize {
        self.entries.len()
    }

    fn entry(&self, idx: u32) -> Result<&SourceEntry, AromError> {
        self.entries
            .get(idx as usize)
            .ok_or_else(|| AromError::internal(format!("entry index out of range: {idx}")))
    }

    fn read(&self, idx: u32) -> Result<Vec<u8>, AromError> {
        let path = self.path_of(idx)?;
        if self.entry(idx)?.meta.is_dir {
            return Ok(Vec::new());
        }
        std::fs::read(&path).map_err(|e| AromError::io(format!("read {}: {e}", path.display())))
    }

    fn copy_to(&self, idx: u32, out: &mut dyn Write) -> Result<(), AromError> {
        // 流式拷贝：大条目不必整体驻留内存。
        let path = self.path_of(idx)?;
        if self.entry(idx)?.meta.is_dir {
            return Ok(());
        }
        let mut f = File::open(&path)
            .map_err(|e| AromError::io(format!("open {}: {e}", path.display())))?;
        std::io::copy(&mut f, out)
            .map_err(|e| AromError::io(format!("copy {}: {e}", path.display())))?;
        Ok(())
    }

    fn copy_raw_to(&self, idx: u32, out: &mut dyn Write) -> Result<(), AromError> {
        // 目录源没有压缩层：原始字节 == 解压后字节，因此与 `copy_to` 同义。
        self.copy_to(idx, out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;


    /// **§9.139**：目录源与 zip 源必须给出**同一份视图**（同样的路径集合与内容）。
    ///
    /// 这是 `DirSource` 的唯一正确性契约：它存在的理由是省掉"把已解压的树重新打包成 zip"，
    /// 若两者视图不一致，省下的时间就是拿正确性换的。
    #[test]
    fn dir_source_matches_zip_source_for_same_tree() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("tree");
        // 目录、嵌套文件、空文件、二进制内容、需转义的路径分隔
        std::fs::create_dir_all(root.join("assets/minecraft/textures/item")).expect("mkdir");
        std::fs::write(root.join("pack.mcmeta"), br#"{"pack":{"pack_format":1}}"#).expect("w");
        std::fs::write(root.join("assets/minecraft/textures/item/a.png"), b"\x89PNG\r\n").expect("w");
        std::fs::write(root.join("assets/minecraft/textures/item/empty.bin"), b"").expect("w");
        std::fs::create_dir_all(root.join("assets/minecraft/empty_dir")).expect("mkdir");

        // 同一棵树打包成 zip，再分别用两个源打开
        let zip_path = tmp.path().join("t.zip");
        crate::pack::io::repack_resource_pack(
            &root.to_string_lossy(),
            &zip_path.to_string_lossy(),
        )
        .expect("repack");

        let limits = SafeLimits::preserving_current();
        let zip_src = ZipSource::open(&zip_path, &limits).expect("zip src");
        let dir_src = DirSource::open(&root, &limits).expect("dir src");

        let mut zip_entries: Vec<(String, bool, usize)> = (0..zip_src.len())
            .map(|i| {
                let e = zip_src.entry(i as u32).expect("e");
                (
                    e.name.clone(),
                    e.meta.is_dir,
                    if e.meta.is_dir { 0 } else { zip_src.read(i as u32).expect("read").len() },
                )
            })
            .collect();
        let mut dir_entries: Vec<(String, bool, usize)> = (0..dir_src.len())
            .map(|i| {
                let e = dir_src.entry(i as u32).expect("e");
                (
                    e.name.clone(),
                    e.meta.is_dir,
                    if e.meta.is_dir { 0 } else { dir_src.read(i as u32).expect("read").len() },
                )
            })
            .collect();
        zip_entries.sort();
        dir_entries.sort();
        assert_eq!(zip_entries, dir_entries, "目录源与 zip 源的条目集合/类型/大小必须一致");

        // 内容逐字节一致（含空文件）
        for i in 0..dir_src.len() {
            let name = dir_src.name(i as u32).expect("name").to_string();
            let is_dir = dir_src.meta(i as u32).expect("meta").is_dir;
            if is_dir {
                continue;
            }
            let zi = (0..zip_src.len())
                .find(|k| zip_src.name(*k as u32).map(|n| n == name).unwrap_or(false))
                .expect("zip has same path");
            assert_eq!(
                dir_src.read(i as u32).expect("dir read"),
                zip_src.read(zi as u32).expect("zip read"),
                "内容不一致: {name}"
            );
        }
        // 空目录被保留（与 zip 源同一契约）
        assert!(
            (0..dir_src.len()).any(|i| dir_src.name(i as u32).expect("n") == "assets/minecraft/empty_dir"),
            "空目录必须保留"
        );
    }

    /// 目录源的 `copy_raw_to` 必须等于 `copy_to`（无压缩层，D23 透传前提）。
    #[test]
    fn dir_source_raw_copy_equals_plain_copy() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("t");
        std::fs::create_dir_all(&root).expect("mkdir");
        std::fs::write(root.join("f.bin"), b"raw-bytes-here").expect("w");
        let limits = SafeLimits::preserving_current();
        let src = DirSource::open(&root, &limits).expect("open");
        let mut plain = Vec::new();
        let mut raw = Vec::new();
        src.copy_to(0, &mut plain).expect("copy");
        src.copy_raw_to(0, &mut raw).expect("raw");
        assert_eq!(plain, raw);
        assert_eq!(plain, b"raw-bytes-here");
        assert_eq!(src.crc32(0).expect("crc"), {
            let mut c = flate2::Crc::new();
            c.update(b"raw-bytes-here");
            c.sum()
        });
    }
    /// 可控夹具：目录条目、压缩方法、符号链接、重复路径、任意条目名（含 Zip Slip）。
    struct Spec {
        name: String,
        body: Option<Vec<u8>>,
        method: zip::CompressionMethod,
        symlink_target: Option<String>,
    }

    fn file(name: &str, body: &[u8]) -> Spec {
        Spec {
            name: name.to_string(),
            body: Some(body.to_vec()),
            method: zip::CompressionMethod::Stored,
            symlink_target: None,
        }
    }

    fn deflated(name: &str, body: &[u8]) -> Spec {
        Spec {
            method: zip::CompressionMethod::Deflated,
            ..file(name, body)
        }
    }

    fn dir(name: &str) -> Spec {
        Spec {
            name: format!("{}/", name.trim_end_matches('/')),
            body: None,
            method: zip::CompressionMethod::Stored,
            symlink_target: None,
        }
    }

    fn build_zip(path: &Path, specs: &[Spec]) {
        let f = File::create(path).expect("create zip");
        let mut zip = zip::ZipWriter::new(f);
        for spec in specs {
            let opts = zip::write::FileOptions::default().compression_method(spec.method);
            if let Some(target) = &spec.symlink_target {
                zip.add_symlink(spec.name.as_str(), target.as_str(), opts)
                    .expect("symlink");
                continue;
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

    fn temp_zip(dir: &Path, name: &str, specs: &[Spec]) -> PathBuf {
        let path = dir.join(name);
        build_zip(&path, specs);
        path
    }

    #[test]
    fn order_names_and_dir_entries_follow_the_container() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let zip = temp_zip(
            tmp.path(),
            "a.zip",
            &[
                file("z.txt", b"z"),
                dir("assets/empty"),
                file("assets/b.txt", b"b"),
                file("a.txt", b"a"),
            ],
        );

        let src = ZipSource::open(&zip, &SafeLimits::unlimited()).expect("open");
        assert_eq!(src.len(), 4, "目录条目必须收录（现状会保留空目录）");
        let names: Vec<&str> = (0..src.len() as u32)
            .map(|i| src.name(i).expect("name"))
            .collect();
        assert_eq!(
            names,
            vec!["z.txt", "assets/empty", "assets/b.txt", "a.txt"],
            "索引顺序必须等于容器内顺序（确定性契约）"
        );
        assert!(src.meta(1).expect("meta").is_dir, "目录条目要标记 is_dir");
        assert_eq!(
            src.container_name(1).expect("container name"),
            "assets/empty/",
            "序列化时需要还原容器内写法"
        );
    }

    #[test]
    fn read_and_raw_stream_match_for_stored_entries() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let payload = b"payload-bytes-for-stored-entry".to_vec();
        let zip = temp_zip(tmp.path(), "a.zip", &[file("t.txt", &payload)]);

        let src = ZipSource::open(&zip, &SafeLimits::unlimited()).expect("open");
        assert_eq!(src.meta(0).expect("meta").method, "stored");
        assert_eq!(src.read(0).expect("read"), payload);

        let mut raw = Vec::new();
        src.copy_raw_to(0, &mut raw).expect("raw");
        assert_eq!(
            raw, payload,
            "Stored 条目的原始压缩字节 == 内容字节（D23 透传前提）"
        );

        let mut streamed = Vec::new();
        src.copy_to(0, &mut streamed).expect("copy_to");
        assert_eq!(streamed, payload);
    }

    #[test]
    fn deflated_raw_stream_is_compressed_while_read_is_not() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let payload = vec![b'x'; 8192];
        let zip = temp_zip(tmp.path(), "a.zip", &[deflated("big.txt", &payload)]);

        let src = ZipSource::open(&zip, &SafeLimits::unlimited()).expect("open");
        let meta = src.meta(0).expect("meta").clone();
        assert_eq!(meta.method, "deflated");
        assert_eq!(src.read(0).expect("read"), payload);

        let mut raw = Vec::new();
        src.copy_raw_to(0, &mut raw).expect("raw");
        assert_eq!(
            raw.len() as u64, meta.compressed_size,
            "原始流长度必须等于容器内压缩大小"
        );
        assert!(raw.len() < payload.len(), "压缩数据应显著短于原文");
        assert_ne!(
            raw, payload,
            "Deflated 条目的原始字节不是内容字节 —— 原样透传会改变产物（D23）"
        );
    }

    #[test]
    fn limits_reject_count_depth_file_size_total_and_ratio() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let zip = temp_zip(
            tmp.path(),
            "a.zip",
            &[
                file("a.txt", b"aaaa"),
                file("deep/b.txt", b"bbbb"),
                deflated("zeros.bin", &vec![0u8; 4096]),
            ],
        );

        // 条目数
        let err = ZipSource::open(
            &zip,
            &SafeLimits {
                max_entries: 2,
                ..SafeLimits::unlimited()
            },
        )
        .expect_err("count limit");
        assert_eq!(err.kind(), "limit");

        // 深度
        let err = ZipSource::open(
            &zip,
            &SafeLimits {
                max_depth: 1,
                ..SafeLimits::unlimited()
            },
        )
        .expect_err("depth limit");
        assert_eq!(err.kind(), "limit");

        // 单文件大小
        let err = ZipSource::open(
            &zip,
            &SafeLimits {
                max_file_bytes: Some(3),
                ..SafeLimits::unlimited()
            },
        )
        .expect_err("file size limit");
        assert_eq!(err.kind(), "limit");

        // 总量
        let err = ZipSource::open(
            &zip,
            &SafeLimits {
                max_total_bytes: 5,
                ..SafeLimits::unlimited()
            },
        )
        .expect_err("total limit");
        assert_eq!(err.kind(), "limit");

        // 压缩比
        let err = ZipSource::open(
            &zip,
            &SafeLimits {
                max_compression_ratio: Some(2.0),
                ..SafeLimits::unlimited()
            },
        )
        .expect_err("ratio limit");
        assert_eq!(err.kind(), "limit");

        // 宽松档必须放行同一夹具（限额不误伤）
        assert!(ZipSource::open(&zip, &SafeLimits::preserving_current()).is_ok());
    }

    #[test]
    fn zip_slip_and_absolute_paths_are_rejected() {
        let tmp = tempfile::tempdir().expect("tempdir");

        let slip = temp_zip(tmp.path(), "slip.zip", &[file("../evil.txt", b"x")]);
        let err = ZipSource::open(&slip, &SafeLimits::unlimited()).expect_err("slip");
        assert_eq!(err.kind(), "path");
        assert!(err.to_string().contains("traversal"), "{err}");

        let abs = temp_zip(tmp.path(), "abs.zip", &[file("/etc/passwd", b"x")]);
        let err = ZipSource::open(&abs, &SafeLimits::unlimited()).expect_err("absolute");
        assert_eq!(err.kind(), "path");
        assert!(err.to_string().contains("absolute"), "{err}");

        let nested = temp_zip(tmp.path(), "nested.zip", &[file("a/../../b.txt", b"x")]);
        let err = ZipSource::open(&nested, &SafeLimits::unlimited()).expect_err("nested slip");
        assert_eq!(err.kind(), "path");
    }

    #[test]
    fn duplicate_and_symlink_entries_are_flagged_not_rejected() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let zip = temp_zip(
            tmp.path(),
            "a.zip",
            &[
                file("dup.txt", b"first"),
                file("dup.txt", b"second"),
                Spec {
                    name: "link".to_string(),
                    body: None,
                    method: zip::CompressionMethod::Stored,
                    symlink_target: Some("dup.txt".to_string()),
                },
            ],
        );

        let src = ZipSource::open(&zip, &SafeLimits::unlimited()).expect("open");
        assert_eq!(
            src.duplicate_indices(),
            vec![1],
            "重复路径只标记不拒绝（现状解压是后者胜，不报错）"
        );
        assert_eq!(
            src.symlink_indices(),
            vec![2],
            "符号链接只标记不拒绝（现状仅在 unix 下拒绝）"
        );
        assert_eq!(src.read(0).expect("read"), b"first");
        assert_eq!(src.read(1).expect("read"), b"second");
    }

    #[test]
    fn concurrent_reads_are_safe() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let specs: Vec<Spec> = (0..16)
            .map(|i| file(&format!("f{i}.txt"), format!("body-{i}").as_bytes()))
            .collect();
        let zip = temp_zip(tmp.path(), "a.zip", &specs);

        let src = ZipSource::open(&zip, &SafeLimits::preserving_current()).expect("open");
        std::thread::scope(|scope| {
            for i in 0..16u32 {
                let src = &src;
                scope.spawn(move || {
                    let bytes = src.read(i).expect("concurrent read");
                    assert_eq!(bytes, format!("body-{i}").as_bytes());
                });
            }
        });
        assert_eq!(src.len(), 16);
    }

    /// D21 的实测证据：句柄池 vs 每次重开容器。
    ///
    /// 默认 `#[ignore]`，用
    /// `cargo test --lib handle_pool_vs_reopen_timing -- --ignored --nocapture` 运行。
    #[test]
    #[ignore]
    fn handle_pool_vs_reopen_timing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let specs: Vec<Spec> = (0..1000)
            .map(|i| file(&format!("f{i:04}.txt"), format!("body-{i}").as_bytes()))
            .collect();
        let zip = temp_zip(tmp.path(), "many.zip", &specs);
        let src = ZipSource::open(&zip, &SafeLimits::preserving_current()).expect("open");

        let t0 = std::time::Instant::now();
        for i in 0..src.len() as u32 {
            let _ = src.read(i).expect("pooled read");
        }
        let pooled = t0.elapsed();

        let t1 = std::time::Instant::now();
        for i in 0..src.len() as u32 {
            let mut archive = open_archive(&zip).expect("reopen");
            let mut entry = archive.by_index(i as usize).expect("by_index");
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).expect("read");
        }
        let reopened = t1.elapsed();

        println!(
            "entries={} pooled={:?} reopen-per-call={:?}",
            src.len(),
            pooled,
            reopened
        );
        assert!(
            pooled < reopened,
            "句柄池必须比重开容器更快：pooled={pooled:?} reopened={reopened:?}"
        );
    }

    #[test]
    fn mem_source_is_a_minimal_second_impl() {
        let src = MemSource::new(vec![
            ("a.txt".to_string(), b"aaa".to_vec()),
            ("assets/".to_string(), Vec::new()),
        ])
        .expect("mem source");
        assert_eq!(src.len(), 2);
        assert!(src.meta(1).expect("meta").is_dir);
        assert_eq!(src.read(0).expect("read"), b"aaa");

        let mut raw = Vec::new();
        src.copy_raw_to(0, &mut raw).expect("raw");
        assert_eq!(raw, b"aaa", "内存来源语义等同 stored");
    }
}
