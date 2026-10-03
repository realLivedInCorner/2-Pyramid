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

pub fn register_task(_engine: &mut crate::hurray::engine::HurrayEngine) {
    // ZIP utilities are orchestrated directly by version_converter.
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
