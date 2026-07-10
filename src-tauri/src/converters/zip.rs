use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use zip::write::FileOptions;

// Prevent ZIP bomb: max total uncompressed size = 500 MB
const ZIP_BOMB_LIMIT: u64 = 500 * 1024 * 1024;

/// 通用 ZIP 解压函数（含 ZIP bomb 防护 + 缓冲 I/O + 进度日志）。
/// 项目中所有 ZIP 解压都应调用此函数或 `extract_resource_pack`。
pub fn extract_zip_to_dir(zip_path: &Path, dest_dir: &Path) -> Result<(), String> {
    crate::log_info!("extracting zip: {} -> {}", zip_path.display(), dest_dir.display());

    let file = File::open(zip_path)
        .map_err(|e| format!("failed to open zip {}: {}", zip_path.display(), e))?;
    let mut archive = zip::ZipArchive::new(std::io::BufReader::with_capacity(1024 * 1024, file))
        .map_err(|e| format!("failed to read zip archive {}: {}", zip_path.display(), e))?;

    fs::create_dir_all(dest_dir)
        .map_err(|e| format!("failed to create extraction directory {}: {}", dest_dir.display(), e))?;

    let total_entries = archive.len();
    let mut total_written: u64 = 0;
    for i in 0..total_entries {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("failed to read zip entry {}: {}", i, e))?;

        let out_path = dest_dir.join(entry.name());

        if entry.name().ends_with('/') {
            fs::create_dir_all(&out_path)
                .map_err(|e| format!("failed to create directory {}: {}", out_path.display(), e))?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create parent {}: {}", parent.display(), e))?;
        }

        let mut out_file = std::io::BufWriter::with_capacity(64 * 1024, File::create(&out_path)
            .map_err(|e| format!("failed to create file {}: {}", out_path.display(), e))?);
        let written = std::io::copy(&mut entry, &mut out_file)
            .map_err(|e| format!("failed to extract {}: {}", out_path.display(), e))?;
        out_file.flush()
            .map_err(|e| format!("failed to flush extracted file {}: {}", out_path.display(), e))?;

        total_written = total_written.saturating_add(written);
        if total_written > ZIP_BOMB_LIMIT {
            return Err(format!(
                "Extracted file too large (>{:.0}MB), possible ZIP bomb. Extraction aborted.",
                ZIP_BOMB_LIMIT as f64 / 1024.0 / 1024.0
            ));
        }

        if (i + 1) % 50 == 0 || i + 1 == total_entries {
            let percent = ((i + 1) * 100) / total_entries;
            crate::log_info!("Progress: {}/{} ({}%) - Extracting", i + 1, total_entries, percent);
        }
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
