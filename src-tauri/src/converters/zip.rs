use rayon::prelude::*;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use zip::write::FileOptions;

// Prevent ZIP bomb: max total uncompressed size = 500 MB
const ZIP_BOMB_LIMIT: u64 = 500 * 1024 * 1024;
const ZIP_MAX_ENTRIES: usize = 100_000;
const ZIP_MAX_DEPTH: usize = 64;

/// Sanitize a zip entry name into a path under `dest_dir`.
/// Rejects absolute paths, drive prefixes, and `..` traversal (Zip Slip).
fn safe_out_path(dest_dir: &Path, raw_name: &str) -> Result<PathBuf, String> {
    let raw = raw_name.replace('\\', "/");
    if raw.contains('\0') {
        return Err(format!("zip entry contains NUL: {raw_name}"));
    }
    if raw.starts_with('/') || raw.starts_with('\\') {
        return Err(format!("zip absolute path rejected: {raw_name}"));
    }

    let mut parts: Vec<String> = Vec::new();
    for comp in Path::new(&raw).components() {
        match comp {
            Component::Normal(s) => {
                let s = s.to_string_lossy();
                if s == ".." || s == "." {
                    return Err(format!("zip path traversal rejected: {raw_name}"));
                }
                parts.push(s.into_owned());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(format!("zip path traversal rejected: {raw_name}"));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!("zip absolute path rejected: {raw_name}"));
            }
        }
    }
    if parts.is_empty() {
        return Err(format!("zip empty path rejected: {raw_name}"));
    }
    if parts.len() > ZIP_MAX_DEPTH {
        return Err(format!("zip path too deep: {raw_name}"));
    }

    let mut out = dest_dir.to_path_buf();
    for p in &parts {
        out.push(p);
    }

    // Defense in depth: resolved path must stay under dest_dir.
    let dest_str = dest_dir.to_string_lossy();
    let out_str = out.to_string_lossy();
    let dest_norm = dest_str.replace('\\', "/");
    let out_norm = out_str.replace('\\', "/");
    if out_norm != dest_norm && !out_norm.starts_with(&format!("{dest_norm}/")) {
        return Err(format!("zip entry escapes destination: {raw_name}"));
    }

    Ok(out)
}

/// 小文件走并行写盘的上限（单个条目）；超过此大小的条目在读取线程内流式写盘，
/// 避免为一个大文件占用大量内存。
const EXTRACT_PARALLEL_MAX_ENTRY: u64 = 4 * 1024 * 1024;
/// 并行写盘的线程数上限（再多只会抢磁盘队列）。
const EXTRACT_MAX_WRITERS: usize = 8;
/// 待写队列深度上限（限制峰值内存：深度 × 单条目上限）。
const EXTRACT_QUEUE_DEPTH: usize = 32;

/// 并行删除目录内容（Windows 上删除几千个小文件与杀软扫描同样昂贵）。
///
/// 做法：先并发删除一级子项（目录走 `remove_dir_all`、文件走 `remove_file`），
/// 再删除根目录本身。比 `remove_dir_all` 的顺序遍历快数倍。
pub fn remove_dir_parallel(root: &Path) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }

    let mut entries: Vec<std::path::PathBuf> = Vec::new();
    let read = fs::read_dir(root)
        .map_err(|e| format!("failed to read dir {}: {}", root.display(), e))?;
    for entry in read {
        match entry {
            Ok(e) => entries.push(e.path()),
            Err(e) => return Err(format!("failed to read entry in {}: {}", root.display(), e)),
        }
    }

    let failures: Vec<String> = entries
        .par_iter()
        .filter_map(|path| {
            let result = if path.is_dir() {
                fs::remove_dir_all(path)
            } else {
                fs::remove_file(path)
            };
            result.err().map(|e| format!("failed to remove {}: {}", path.display(), e))
        })
        .collect();
    if let Some(first) = failures.into_iter().next() {
        return Err(first);
    }

    fs::remove_dir_all(root)
        .map_err(|e| format!("failed to remove dir {}: {}", root.display(), e))
}

/// 本工具的临时工作目录前缀（用于安全清理残留）。
pub const WORK_DIR_PREFIX: &str = ".2pyr-work-";

/// 清理陈旧的临时工作目录（异常退出/后台删除未完成留下的残留）。
///
/// 只处理**本工具前缀**且修改时间早于 `max_age` 的目录，避免误删其他程序
/// 的临时文件或正在使用的目录。返回清理数量。
pub fn sweep_stale_work_dirs(root: &Path, max_age: std::time::Duration) -> usize {
    let Ok(read) = fs::read_dir(root) else {
        return 0;
    };
    let mut removed = 0usize;
    for entry in read.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with(WORK_DIR_PREFIX) {
            continue;
        }
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let too_old = entry
            .metadata()
            .and_then(|m| m.modified())
            .map(|modified| {
                modified
                    .elapsed()
                    .map(|age| age > max_age)
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        if too_old && remove_dir_parallel(&path).is_ok() {
            removed += 1;
        }
    }
    removed
}

lazy_static::lazy_static! {
    static ref CLEANUP_PENDING: std::sync::atomic::AtomicUsize =
        std::sync::atomic::AtomicUsize::new(0);
    static ref CLEANUP_HANDLES: std::sync::Mutex<Vec<std::thread::JoinHandle<()>>> =
        std::sync::Mutex::new(Vec::new());
}

/// 临时目录清理策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupMode {
    /// 当前线程同步删除（基准测试用，耗时计入 cleanup_s）。
    Sync,
    /// 后台线程删除；进程退出会中断，只适合 GUI 这类长命进程。
    Background,
    /// 交给**脱离本进程**的系统子进程删除（`rmdir /S /Q`）：本进程立即退出，
    /// 删除在后台继续完成——短命进程（CLI/脚本）也不留残留、也不等待。
    Detached,
}

lazy_static::lazy_static! {
    static ref CLEANUP_MODE: std::sync::Mutex<CleanupMode> =
        std::sync::Mutex::new(CleanupMode::Background);
}

/// 设置清理策略（CLI 启动时设为 `Detached`）。
pub fn set_cleanup_mode(mode: CleanupMode) {
    if let Ok(mut m) = CLEANUP_MODE.lock() {
        *m = mode;
    }
}

/// 当前清理策略。环境变量 `2PYR_SYNC_CLEANUP=1` 优先（强制同步，便于基准测试）。
pub fn cleanup_mode() -> CleanupMode {
    if std::env::var("2PYR_SYNC_CLEANUP").is_ok() {
        return CleanupMode::Sync;
    }
    CLEANUP_MODE
        .lock()
        .map(|m| *m)
        .unwrap_or(CleanupMode::Background)
}

/// 按当前策略派发清理。返回 `true` 表示已完成（同步、耗时需计入），
/// `false` 表示已交给后台（不计入耗时）。
pub fn dispatch_cleanup(root: &Path) -> bool {
    let mode = cleanup_mode();
    crate::log_info!("temp cleanup dispatch: mode={:?} path={}", mode, root.display());
    match mode {
        CleanupMode::Sync => {
            if let Err(e) = remove_dir_parallel(root) {
                crate::log_warn!("temp cleanup failed: {}", e);
            }
            true
        }
        CleanupMode::Background => {
            if remove_dir_in_background(root) {
                false
            } else {
                // 派发失败就同步删掉，别留下垃圾
                if let Err(e) = remove_dir_parallel(root) {
                    crate::log_warn!("temp cleanup failed: {}", e);
                }
                true
            }
        }
        CleanupMode::Detached => {
            if remove_dir_detached(root) {
                false
            } else if remove_dir_in_background(root) {
                false
            } else {
                let _ = remove_dir_parallel(root);
                true
            }
        }
    }
}

/// 派发一个**脱离本进程**的删除：Windows 用 `cmd /C rmdir /S /Q`，Unix 用 `rm -rf`。
/// 子进程不等待、不继承标准流、无窗口；本进程退出后它继续执行到完成。
pub fn remove_dir_detached(root: &Path) -> bool {
    use std::process::{Command, Stdio};
    if !root.exists() {
        return true;
    }

    #[cfg(windows)]
    let mut cmd = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        let mut c = Command::new("cmd");
        c.arg("/C").arg("rmdir").arg("/S").arg("/Q").arg(root);
        c.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = Command::new("rm");
        c.arg("-rf").arg(root);
        c
    };

    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}

/// 后台异步删除工作目录：删除几千个小文件在 Windows 上要 1.5–2.5 s
/// （杀软逐个扫描），把它挪出用户等待路径。返回是否成功派发。
///
/// 句柄会被登记，`wait_for_cleanups` 可在进程退出前等待它们完成——否则
/// 短命进程（如 CLI）退出时会杀掉后台线程并留下残留目录。
pub fn remove_dir_in_background(root: &Path) -> bool {
    let path = root.to_path_buf();
    CLEANUP_PENDING.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let spawned = std::thread::Builder::new()
        .name("workdir-cleanup".into())
        .spawn(move || {
            if let Err(e) = remove_dir_parallel(&path) {
                crate::log_warn!("background temp cleanup failed: {}", e);
            }
            CLEANUP_PENDING.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        });
    match spawned {
        Ok(handle) => {
            if let Ok(mut handles) = CLEANUP_HANDLES.lock() {
                handles.retain(|h| !h.is_finished());
                handles.push(handle);
            }
            true
        }
        Err(_) => {
            CLEANUP_PENDING.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            false
        }
    }
}

/// 仍在进行中的后台清理数量。
pub fn pending_cleanups() -> usize {
    CLEANUP_PENDING.load(std::sync::atomic::Ordering::SeqCst)
}

/// 等待后台清理完成（最多 `timeout`）。返回是否全部完成。
/// 供 CLI 等短命进程在退出前调用，避免留下临时目录。
pub fn wait_for_cleanups(timeout: std::time::Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    while pending_cleanups() > 0 {
        if std::time::Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    // 回收已结束的句柄，避免长命进程（GUI）里累积
    if let Ok(mut handles) = CLEANUP_HANDLES.lock() {
        handles.retain(|h| !h.is_finished());
    }
    true
}

/// 通用 ZIP 解压函数（含 Zip Slip / ZIP bomb 防护 + 缓冲 I/O + 进度日志）。
/// 项目中所有 ZIP 解压都应调用此函数或 `extract_resource_pack`。
///
/// 性能说明：对"几千个小文件"的资源包，瓶颈是**逐文件创建 + 杀软实时扫描**，
/// 而不是 Deflate 解压本身。因此这里采用「读取线程顺序解压 → 有界队列 →
/// 多个写线程并行落盘」：小文件并行写，大文件在读取线程内流式写（内存有界）。
/// 输出与串行版本逐字节相同，只是写盘并发化。
pub fn extract_zip_to_dir(zip_path: &Path, dest_dir: &Path) -> Result<(), String> {
    crate::log_info!("extracting zip: {} -> {}", zip_path.display(), dest_dir.display());

    let file = File::open(zip_path)
        .map_err(|e| format!("failed to open zip {}: {}", zip_path.display(), e))?;
    let mut archive = zip::ZipArchive::new(std::io::BufReader::with_capacity(1024 * 1024, file))
        .map_err(|e| format!("failed to read zip archive {}: {}", zip_path.display(), e))?;

    if archive.len() > ZIP_MAX_ENTRIES {
        return Err(format!(
            "zip has too many entries: {} > {}",
            archive.len(),
            ZIP_MAX_ENTRIES
        ));
    }

    fs::create_dir_all(dest_dir)
        .map_err(|e| format!("failed to create extraction directory {}: {}", dest_dir.display(), e))?;

    // ── 并行写盘工作线程 ────────────────────────────────────────
    let worker_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(2, EXTRACT_MAX_WRITERS);
    let (tx, rx) = std::sync::mpsc::sync_channel::<(std::path::PathBuf, Vec<u8>)>(EXTRACT_QUEUE_DEPTH);
    let rx = std::sync::Arc::new(std::sync::Mutex::new(rx));

    let mut workers = Vec::with_capacity(worker_count);
    for idx in 0..worker_count {
        let rx = std::sync::Arc::clone(&rx);
        workers.push(
            std::thread::Builder::new()
                .name(format!("extract-writer-{}", idx))
                .spawn(move || -> Result<(), String> {
                    loop {
                        let job = {
                            match rx.lock() {
                                Ok(guard) => guard.recv(),
                                Err(_) => break,
                            }
                        };
                        match job {
                            Ok((path, bytes)) => {
                                fs::write(&path, &bytes).map_err(|e| {
                                    format!("failed to write extracted file {}: {}", path.display(), e)
                                })?;
                            }
                            Err(_) => break,
                        }
                    }
                    Ok(())
                })
                .map_err(|e| format!("failed to spawn extraction worker: {}", e))?,
        );
    }

    // 已创建目录缓存：避免对同一父目录重复 create_dir_all
    // （3631 个文件往往只落在几百个目录里，重复调用纯属浪费）
    let mut created_dirs: std::collections::HashSet<std::path::PathBuf> =
        std::collections::HashSet::new();
    let mut ensure_dir = |dir: &Path| -> Result<(), String> {
        if created_dirs.insert(dir.to_path_buf()) {
            fs::create_dir_all(dir)
                .map_err(|e| format!("failed to create directory {}: {}", dir.display(), e))?;
        }
        Ok(())
    };

    let total_entries = archive.len();
    let mut total_written: u64 = 0;
    let mut dispatch_error: Option<String> = None;

    for i in 0..total_entries {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("failed to read zip entry {}: {}", i, e))?;

        #[cfg(unix)]
        {
            if let Some(mode) = entry.unix_mode() {
                if mode & 0o170000 == 0o120000 {
                    return Err(format!("zip symlink rejected: {}", entry.name()));
                }
            }
        }

        let raw_name = entry.name().to_string();
        let out_path = safe_out_path(dest_dir, &raw_name)?;

        if raw_name.ends_with('/') {
            ensure_dir(&out_path)?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            ensure_dir(parent)?;
        }

        let entry_size = entry.size();
        if entry_size <= EXTRACT_PARALLEL_MAX_ENTRY {
            // 小文件：读到内存后交给写线程（并行落盘）
            let mut buf = Vec::with_capacity(entry_size as usize);
            entry
                .read_to_end(&mut buf)
                .map_err(|e| format!("failed to extract {}: {}", out_path.display(), e))?;
            total_written = total_written.saturating_add(buf.len() as u64);
            if total_written > ZIP_BOMB_LIMIT {
                return Err(format!(
                    "Extracted file too large (>{:.0}MB), possible ZIP bomb. Extraction aborted.",
                    ZIP_BOMB_LIMIT as f64 / 1024.0 / 1024.0
                ));
            }
            if tx.send((out_path, buf)).is_err() {
                dispatch_error = Some("extraction worker channel closed unexpectedly".to_string());
                break;
            }
        } else {
            // 大文件：留在读取线程流式写，内存占用恒定
            let mut out_file = std::io::BufWriter::with_capacity(256 * 1024, File::create(&out_path)
                .map_err(|e| format!("failed to create file {}: {}", out_path.display(), e))?);
            let written = std::io::copy(&mut entry, &mut out_file)
                .map_err(|e| format!("failed to extract {}: {}", out_path.display(), e))?;
            out_file
                .flush()
                .map_err(|e| format!("failed to flush extracted file {}: {}", out_path.display(), e))?;
            total_written = total_written.saturating_add(written);
            if total_written > ZIP_BOMB_LIMIT {
                return Err(format!(
                    "Extracted file too large (>{:.0}MB), possible ZIP bomb. Extraction aborted.",
                    ZIP_BOMB_LIMIT as f64 / 1024.0 / 1024.0
                ));
            }
        }

        if (i + 1) % 50 == 0 || i + 1 == total_entries {
            let percent = ((i + 1) * 100) / total_entries;
            crate::log_info!("Progress: {}/{} ({}%) - Extracting", i + 1, total_entries, percent);
        }
    }

    // 关闭发送端 → 写线程排空队列后退出
    drop(tx);
    let mut worker_error: Option<String> = None;
    for handle in workers {
        match handle.join() {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                if worker_error.is_none() {
                    worker_error = Some(e);
                }
            }
            Err(_) => {
                if worker_error.is_none() {
                    worker_error = Some("extraction worker panicked".to_string());
                }
            }
        }
    }

    if let Some(e) = dispatch_error.or(worker_error) {
        return Err(e);
    }

    Ok(())
}

/// 从字符串路径解压（向后兼容的便捷封装）
pub fn extract_resource_pack(zip_path: &str, target_dir: &str) -> Result<(), String> {
    extract_zip_to_dir(Path::new(zip_path), Path::new(target_dir))
}

pub fn repack_resource_pack(source_dir: &str, target_zip: &str) -> Result<(), String> {
    crate::log_info!("Repacking started for: {}", target_zip);

    let source_path = Path::new(source_dir);
    if !source_path.exists() {
        return Err(format!("source directory not found: {}", source_dir));
    }

    if let Some(parent) = Path::new(target_zip).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create output directory {}: {}", parent.display(), e))?;
    }

    crate::log_info!("Scanning files in source directory...");
    let entries: Vec<_> = walkdir::WalkDir::new(source_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .collect();
    
    let total_entries = entries.len();
    crate::log_info!("Found {} files to repack.", total_entries);

    let file = File::create(target_zip)
        .map_err(|e| format!("failed to create output zip {}: {}", target_zip, e))?;
    let mut zip_writer = zip::ZipWriter::new(std::io::BufWriter::with_capacity(1024 * 1024, file));
    let default_options = FileOptions::default();

    for (i, entry) in entries.into_iter().enumerate() {
        let path = entry.path();
        let relative = path
            .strip_prefix(source_path)
            .map_err(|e| format!("failed to strip prefix for {}: {}", path.display(), e))?;
        let name = relative.to_string_lossy().replace('\\', "/");

        if name.is_empty() {
            continue;
        }

        if path.is_file() {
            let method = compression_method_for_path(&name);
            let options = default_options.compression_method(method);
            zip_writer
                .start_file(&name, options)
                .map_err(|e| format!("failed to start zip file entry {}: {}", name, e))?;
            let mut f = std::io::BufReader::with_capacity(64 * 1024, File::open(path)
                .map_err(|e| format!("failed to open source file {}: {}", path.display(), e))?);
            std::io::copy(&mut f, &mut zip_writer)
                .map_err(|e| format!("failed to write zip file entry {}: {}", name, e))?;
        } else {
            zip_writer
                .add_directory(format!("{}/", name), default_options)
                .map_err(|e| format!("failed to add zip directory entry {}: {}", name, e))?;
        }

        // Emit progress more frequently
        if (i + 1) % 10 == 0 || i + 1 == total_entries {
            let percent = ((i + 1) * 100) / total_entries;
            crate::log_info!("Progress: {}/{} ({}%) - Repacking", i + 1, total_entries, percent);
        }
    }

    crate::log_info!("Finalizing ZIP archive (this may take a while for large packs)...");
    zip_writer
        .finish()
        .map_err(|e| format!("failed to finalize zip {}: {}", target_zip, e))?
        .flush()
        .map_err(|e| format!("failed to flush zip output: {}", e))?;

    Ok(())
}

fn compression_method_for_path(name: &str) -> zip::CompressionMethod {
    let lower = name.to_ascii_lowercase();
    let stored_exts = [
        ".png", ".jpg", ".jpeg", ".gif", ".webp",
        ".ogg", ".mp3", ".wav", ".mp4",
        ".zip", ".mcpack", ".jar",
        ".dds", ".tga", ".ktx", ".ktx2",
        ".bin",
    ];
    if stored_exts.iter().any(|ext| lower.ends_with(ext)) {
        return zip::CompressionMethod::Stored;
    }
    zip::CompressionMethod::Deflated
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// 并行删除 + 残留清扫：只清理本工具前缀且足够陈旧的目录。
    #[test]
    fn parallel_removal_and_stale_sweep() {
        let root = tempfile::tempdir().expect("tempdir");

        // 造一个多层目录树用于并行删除
        let tree = root.path().join("tree");
        for i in 0..40 {
            let dir = tree.join(format!("d{}", i % 5)).join(format!("sub{}", i % 3));
            std::fs::create_dir_all(&dir).expect("mkdir");
            std::fs::write(dir.join(format!("f{}.bin", i)), b"x").expect("write");
        }
        remove_dir_parallel(&tree).expect("remove tree");
        assert!(!tree.exists(), "parallel removal must delete the whole tree");

        // sweep：本工具前缀 + 足够陈旧 → 删除；新目录 / 其他前缀 → 保留
        let stale = root.path().join(format!("{}old", WORK_DIR_PREFIX));
        let fresh = root.path().join(format!("{}new", WORK_DIR_PREFIX));
        let other = root.path().join(".tmp_other_app");
        for d in [&stale, &fresh, &other] {
            std::fs::create_dir_all(d).expect("mkdir");
            std::fs::write(d.join("f.bin"), b"x").expect("write");
        }

        // sweep 规则①：阈值 0 → 一定"过期"，应删除本工具前缀目录、绝不动其他前缀
        let removed_all =
            sweep_stale_work_dirs(root.path(), std::time::Duration::ZERO);
        assert_eq!(
            removed_all, 2,
            "both work dirs (stale + fresh) expire under a zero threshold"
        );
        assert!(!stale.exists());
        assert!(!fresh.exists());
        assert!(other.exists(), "foreign temp dirs must never be touched");

        // sweep 规则②：正常阈值（2 小时）下，刚创建的目录必须保留
        let kept = root.path().join(format!("{}recent", WORK_DIR_PREFIX));
        std::fs::create_dir_all(&kept).expect("mkdir");
        let removed_none =
            sweep_stale_work_dirs(root.path(), std::time::Duration::from_secs(2 * 60 * 60));
        assert_eq!(removed_none, 0, "fresh work dirs must be kept");
        assert!(kept.exists());
        assert!(other.exists());
    }

    /// 后台清理可被等待（CLI 退出前调用），且不会留下目录。
    #[test]
    fn background_cleanup_can_be_awaited() {
        let root = tempfile::tempdir().expect("tempdir");
        let work = root.path().join(format!("{}bg", WORK_DIR_PREFIX));
        std::fs::create_dir_all(work.join("assets")).expect("mkdir");
        for i in 0..50 {
            std::fs::write(work.join("assets").join(format!("f{}.bin", i)), b"x").expect("write");
        }

        assert!(remove_dir_in_background(&work), "dispatch must succeed");
        assert!(
            wait_for_cleanups(std::time::Duration::from_secs(20)),
            "cleanup must finish within the timeout"
        );
        assert!(!work.exists(), "background cleanup must remove the directory");
        assert_eq!(pending_cleanups(), 0);
    }

    /// 并行解压回归：小文件走写线程池、大文件走读取线程流式分支、空目录仍要建，
    /// 且每个文件内容必须逐字节一致。
    #[test]
    fn parallel_extraction_preserves_all_entries() {
        use std::io::Write as _;

        let dir = tempfile::tempdir().expect("tempdir");
        let zip_path = dir.path().join("many.zip");
        let out_dir = dir.path().join("out");

        // 300 个小文件（分散在多层目录）+ 1 个空目录 + 1 个 5 MB 大文件
        let mut expected: Vec<(String, Vec<u8>)> = Vec::new();
        {
            let file = std::fs::File::create(&zip_path).expect("create zip");
            let mut zip = zip::ZipWriter::new(file);
            let stored = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            let deflated = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);

            for i in 0..300 {
                let name = format!("assets/minecraft/textures/gui/dir{}/file{}.bin", i % 7, i);
                let body = format!("payload-{}", i).into_bytes();
                zip.start_file(&name, deflated).expect("start_file");
                zip.write_all(&body).expect("write");
                expected.push((name, body));
            }

            // 空目录条目
            zip.add_directory("assets/empty_dir/", stored).expect("add_directory");

            // 大文件（> EXTRACT_PARALLEL_MAX_ENTRY，走流式写盘分支）
            let big: Vec<u8> = (0..5 * 1024 * 1024).map(|i| (i % 251) as u8).collect();
            zip.start_file("assets/big.bin", stored).expect("start_file");
            zip.write_all(&big).expect("write");
            expected.push(("assets/big.bin".to_string(), big));

            zip.finish().expect("finish");
        }

        extract_zip_to_dir(&zip_path, &out_dir).expect("extract");

        for (name, body) in &expected {
            let path = out_dir.join(name.replace('/', std::path::MAIN_SEPARATOR_STR));
            let got = std::fs::read(&path)
                .unwrap_or_else(|e| panic!("missing extracted file {}: {}", path.display(), e));
            assert_eq!(
                got.len(),
                body.len(),
                "size mismatch for {}",
                name
            );
            assert!(got == *body, "content mismatch for {}", name);
        }
        assert!(
            out_dir.join("assets").join("empty_dir").is_dir(),
            "empty directory entry must still be created"
        );
    }

    #[test]
    fn rejects_zip_slip_and_absolute_paths() {
        let dest = Path::new("/tmp/pack");
        assert!(safe_out_path(dest, "../evil.txt").is_err());
        assert!(safe_out_path(dest, "a/../../evil.txt").is_err());
        assert!(safe_out_path(dest, "..\\evil.txt").is_err());
        assert!(safe_out_path(dest, "/etc/passwd").is_err());
        assert!(safe_out_path(dest, "C:/Windows/evil.dll").is_err());
        assert!(safe_out_path(dest, "a\0b").is_err());
    }

    #[test]
    fn accepts_normal_relative_paths() {
        let dest = Path::new("/tmp/pack");
        let p = safe_out_path(dest, "assets/minecraft/textures/block/stone.png").unwrap();
        assert!(p.ends_with("assets/minecraft/textures/block/stone.png"));
        let p2 = safe_out_path(dest, "./pack.mcmeta").unwrap();
        assert!(p2.ends_with("pack.mcmeta"));
    }
}
