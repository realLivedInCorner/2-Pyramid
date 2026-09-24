//! 安全解压门禁：拒绝 Zip Slip / Zip Bomb / Symlink，并限制解压规模。

use std::collections::HashSet;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};
use zip::ZipArchive;

/// 解压策略（默认收紧，可配置）。
#[derive(Debug, Clone)]
pub struct SafeLimits {
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
    pub max_entries: usize,
    pub max_depth: usize,
    pub max_compression_ratio: f64,
}

impl Default for SafeLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: 64 * 1024 * 1024,
            max_total_bytes: 1024 * 1024 * 1024,
            max_entries: 50_000,
            max_depth: 32,
            // 压缩包条目宣称大小 / 压缩后大小 过高 → bomb
            max_compression_ratio: 200.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SafeEntry {
    /// 规范化 posix 路径（相对包根）
    pub path: String,
    pub size: u64,
    pub compressed_size: u64,
    pub sha256: String,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct SafeArchive {
    pub entries: Vec<SafeEntry>,
}

/// 将 zip 内路径规范为包根相对 posix 路径；失败表示逃逸。
fn normalize_zip_path(raw: &str, max_depth: usize) -> Result<String, String> {
    let raw = raw.replace('\\', "/");
    if raw.starts_with('/') || raw.starts_with('\\') {
        return Err(format!("absolute path rejected: {raw}"));
    }
    if raw.contains('\0') {
        return Err("nul in path".into());
    }
    let path = Path::new(&raw);
    let mut parts: Vec<String> = Vec::new();
    for comp in path.components() {
        match comp {
            Component::Normal(s) => {
                let s = s.to_string_lossy();
                if s == ".." || s == "." {
                    return Err(format!("path traversal rejected: {raw}"));
                }
                parts.push(s.to_string());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(format!("path traversal rejected: {raw}"));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!("absolute path rejected: {raw}"));
            }
        }
    }
    if parts.is_empty() {
        return Err("empty path".into());
    }
    if parts.len() > max_depth {
        return Err(format!("path too deep: {raw}"));
    }
    Ok(parts.join("/"))
}

/// 打开并安全读取 zip。失败不落盘。
pub fn open_pack(path: &Path, limits: &SafeLimits) -> Result<SafeArchive, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("open zip: {e}"))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("read zip: {e}"))?;
    if archive.len() > limits.max_entries {
        return Err(format!(
            "too many entries: {} > {}",
            archive.len(),
            limits.max_entries
        ));
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut entries = Vec::with_capacity(archive.len());
    let mut total: u64 = 0;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("zip entry #{i}: {e}"))?;
        let name = entry.name().to_string();

        // Symlink / 特殊：zip 的 unix mode 高位含 symlink
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = entry.unix_mode() {
                if mode & 0o170000 == 0o120000 {
                    return Err(format!("symlink rejected: {name}"));
                }
            }
        }
        // Windows 下仍拒绝明显链接名模式
        if name.contains("./") && name.contains("/..") {
            return Err(format!("path traversal rejected: {name}"));
        }

        let is_dir = name.ends_with('/');
        let norm = normalize_zip_path(&name, limits.max_depth)?;
        if is_dir {
            continue;
        }
        if !seen.insert(norm.clone()) {
            return Err(format!("duplicate path rejected: {norm}"));
        }

        let compressed = entry.compressed_size();
        let declared = entry.size();
        if declared > limits.max_file_bytes {
            return Err(format!("file too large: {norm} ({declared} bytes)"));
        }
        if compressed > 0 && (declared as f64 / compressed as f64) > limits.max_compression_ratio {
            return Err(format!("zip bomb ratio too high: {norm}"));
        }

        let mut data = Vec::new();
        entry
            .take(limits.max_file_bytes + 1)
            .read_to_end(&mut data)
            .map_err(|e| format!("read entry {norm}: {e}"))?;
        if data.len() as u64 > limits.max_file_bytes {
            return Err(format!("file too large after inflate: {norm}"));
        }

        total = total.saturating_add(data.len() as u64);
        if total > limits.max_total_bytes {
            return Err("total inflated size exceeded".into());
        }

        let mut hasher = Sha256::new();
        hasher.update(&data);
        let sha256 = format!("{:x}", hasher.finalize());

        entries.push(SafeEntry {
            path: norm,
            size: data.len() as u64,
            compressed_size: compressed,
            sha256,
            data,
        });
    }

    Ok(SafeArchive { entries })
}

/// 从内存 zip 打开（测试与导出回读）。
pub fn open_pack_bytes(bytes: &[u8], limits: &SafeLimits) -> Result<SafeArchive, String> {
    let cursor = std::io::Cursor::new(bytes.to_vec());
    let mut archive = ZipArchive::new(cursor).map_err(|e| format!("read zip: {e}"))?;
    if archive.len() > limits.max_entries {
        return Err(format!(
            "too many entries: {} > {}",
            archive.len(),
            limits.max_entries
        ));
    }
    // 复用逻辑：写临时文件成本高，直接内联一份读取
    let mut seen: HashSet<String> = HashSet::new();
    let mut entries = Vec::new();
    let mut total: u64 = 0;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("zip entry #{i}: {e}"))?;
        let name = entry.name().to_string();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = entry.unix_mode() {
                if mode & 0o170000 == 0o120000 {
                    return Err(format!("symlink rejected: {name}"));
                }
            }
        }
        let is_dir = name.ends_with('/');
        let norm = normalize_zip_path(&name, limits.max_depth)?;
        if is_dir {
            continue;
        }
        if !seen.insert(norm.clone()) {
            return Err(format!("duplicate path rejected: {norm}"));
        }
        let compressed = entry.compressed_size();
        let declared = entry.size();
        if declared > limits.max_file_bytes {
            return Err(format!("file too large: {norm}"));
        }
        if compressed > 0 && (declared as f64 / compressed as f64) > limits.max_compression_ratio {
            return Err(format!("zip bomb ratio too high: {norm}"));
        }
        let mut data = Vec::new();
        entry
            .take(limits.max_file_bytes + 1)
            .read_to_end(&mut data)
            .map_err(|e| format!("read entry {norm}: {e}"))?;
        if data.len() as u64 > limits.max_file_bytes {
            return Err(format!("file too large after inflate: {norm}"));
        }
        total = total.saturating_add(data.len() as u64);
        if total > limits.max_total_bytes {
            return Err("total inflated size exceeded".into());
        }
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let sha256 = format!("{:x}", hasher.finalize());
        entries.push(SafeEntry {
            path: norm,
            size: data.len() as u64,
            compressed_size: compressed,
            sha256,
            data,
        });
    }
    Ok(SafeArchive { entries })
}

pub fn write_zip_file(path: &Path, entries: &[(String, Vec<u8>)]) -> Result<(), String> {
    use std::io::Write;
    use zip::write::FileOptions;
    let file = std::fs::File::create(path).map_err(|e| format!("create zip: {e}"))?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);
    for (name, data) in entries {
        zip.start_file(name.as_str(), opts)
            .map_err(|e| format!("zip start {name}: {e}"))?;
        zip.write_all(data).map_err(|e| format!("zip write {name}: {e}"))?;
    }
    zip.finish().map_err(|e| format!("zip finish: {e}"))?;
    Ok(())
}

pub fn pack_path_buf(s: &str) -> PathBuf {
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::FileOptions;

    fn make_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let opts = FileOptions::default();
            for (name, data) in files {
                zip.start_file(*name, opts).unwrap();
                zip.write_all(data).unwrap();
            }
            zip.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn rejects_path_traversal() {
        let bytes = make_zip(&[("../evil.txt", b"x")]);
        let err = open_pack_bytes(&bytes, &SafeLimits::default()).unwrap_err();
        assert!(err.contains("traversal") || err.contains("rejected"), "{err}");
    }

    #[test]
    fn rejects_absolute_path() {
        let bytes = make_zip(&[("/abs/evil.txt", b"x")]);
        assert!(open_pack_bytes(&bytes, &SafeLimits::default()).is_err());
    }

    #[test]
    fn accepts_normal_pack() {
        let bytes = make_zip(&[
            ("pack.mcmeta", b"{\"pack\":{\"pack_format\":34}}"),
            ("assets/minecraft/textures/item/a.png", b"png"),
        ]);
        let arc = open_pack_bytes(&bytes, &SafeLimits::default()).unwrap();
        assert_eq!(arc.entries.len(), 2);
        assert!(arc.entries.iter().any(|e| e.path == "pack.mcmeta"));
    }

    #[test]
    fn rejects_too_many_entries() {
        let files: Vec<(String, &[u8])> = (0..10)
            .map(|i| (format!("f{i}.txt"), &b"x"[..]))
            .collect();
        let refs: Vec<(&str, &[u8])> = files.iter().map(|(n, d)| (n.as_str(), *d)).collect();
        let bytes = make_zip(&refs);
        let mut limits = SafeLimits::default();
        limits.max_entries = 5;
        assert!(open_pack_bytes(&bytes, &limits).is_err());
    }

    #[test]
    fn normalize_rejects_dotdot() {
        assert!(normalize_zip_path("a/../b", 32).is_err());
        assert_eq!(normalize_zip_path("a/b/c.png", 32).unwrap(), "a/b/c.png");
    }
}
