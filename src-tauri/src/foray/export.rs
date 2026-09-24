//! 导出：默认另存副本；原地覆盖需二次确认并写 .bak。
//!
//! 契约（S2.7）：
//! - 未 dirty 的条目 **字节级保留**（复用原始 zip 压缩条目）
//! - 原地覆盖先写临时文件再原子替换；失败从 .bak 回滚

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use super::rom::{Rom, RomDir};
use super::zip_safe::{SafeEntry, write_zip_file};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportMode {
    SaveAs,
    InPlace,
}

fn collect_files(dir: &RomDir, out: &mut Vec<(String, Vec<u8>, bool)>) {
    for f in &dir.files {
        out.push((f.path.clone(), f.data.clone(), f.dirty));
    }
    for d in &dir.dirs {
        collect_files(d, out);
    }
}

/// 保留原始压缩字节的导出：未 dirty 条目直接 `raw_copy`，dirty 条目重压缩。
fn export_preserving_bytes(
    rom: &Rom,
    out_path: &Path,
    original: Option<&Path>,
) -> Result<(), String> {
    let file = std::fs::File::create(out_path).map_err(|e| format!("create zip: {e}"))?;
    let mut zip = ZipWriter::new(file);
    let opts = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    let mut clean: Vec<(String, Vec<u8>, bool)> = Vec::new();
    if !rom.meta.raw.is_empty() {
        // pack.mcmeta 可能被解析后改写 —— dirty 与否以 raw 是否变化难以判断，统一重写（小文件）
        zip.start_file("pack.mcmeta", opts)
            .map_err(|e| e.to_string())?;
        zip.write_all(rom.meta.raw.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    if let Some(icon) = &rom.icon {
        zip.start_file("pack.png", opts).map_err(|e| e.to_string())?;
        zip.write_all(&icon.data).map_err(|e| e.to_string())?;
    }
    collect_files(&rom.root, &mut clean);
    clean.sort_by(|a, b| a.0.cmp(&b.0));

    // 打开源 zip 以便 raw_copy 未修改条目
    let mut src_zip = match original {
        Some(p) if p.exists() => {
            let f = std::fs::File::open(p).map_err(|e| format!("open source zip: {e}"))?;
            Some(ZipArchive::new(f).map_err(|e| format!("source zip: {e}"))?)
        }
        _ => None,
    };

    for (path, data, dirty) in &clean {
        if *dirty {
            zip.start_file(path.as_str(), opts)
                .map_err(|e| format!("zip start {path}: {e}"))?;
            zip.write_all(data)
                .map_err(|e| format!("zip write {path}: {e}"))?;
        } else if let Some(archive) = src_zip.as_mut() {
            // 字节级保留：从源 zip raw_copy
            let entry = archive
                .by_name(path.as_str())
                .map_err(|e| format!("source entry {path}: {e}"))?;
            zip.raw_copy_file(entry)
                .map_err(|e| format!("raw copy {path}: {e}"))?;
        } else {
            zip.start_file(path.as_str(), opts)
                .map_err(|e| format!("zip start {path}: {e}"))?;
            zip.write_all(data)
                .map_err(|e| format!("zip write {path}: {e}"))?;
        }
    }

    zip.finish().map_err(|e| format!("zip finish: {e}"))?;
    Ok(())
}

pub fn export_pack(rom: &Rom, mode: ExportMode, dest_dir: Option<&Path>) -> Result<PathBuf, String> {
    let source = Path::new(&rom.source_path);
    let original = if source.exists() && source.extension().is_some() {
        Some(source)
    } else {
        None
    };

    let out_path = match mode {
        ExportMode::SaveAs => {
            let dir = dest_dir
                .map(|p| p.to_path_buf())
                .or_else(|| source.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from("."));
            let stem = source
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("pack");
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            dir.join(format!("{stem}_foray_{ts}.zip"))
        }
        ExportMode::InPlace => {
            if source.as_os_str().is_empty() || !source.exists() {
                return Err("source path empty or missing, cannot overwrite".into());
            }
            source.to_path_buf()
        }
    };

    if let Some(parent) = out_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    match mode {
        ExportMode::SaveAs => {
            export_preserving_bytes(rom, &out_path, original)?;
            Ok(out_path)
        }
        ExportMode::InPlace => {
            let bak = source.with_extension("zip.bak");
            std::fs::copy(source, &bak).map_err(|e| format!("backup failed: {e}"))?;
            // 先写临时文件，成功后再替换，避免 truncate 源包
            let tmp = source.with_extension("zip.foray_tmp");
            if let Err(e) = export_preserving_bytes(rom, &tmp, Some(source)) {
                let _ = std::fs::remove_file(&tmp);
                return Err(format!("export failed (source intact): {e}"));
            }
            // 原子替换：Windows 上先删目标再改名；失败从 bak 恢复
            let replace = std::fs::remove_file(source)
                .and_then(|_| std::fs::rename(&tmp, source));
            if let Err(e) = replace {
                let _ = std::fs::remove_file(&tmp);
                let _ = std::fs::rename(&bak, source);
                return Err(format!("replace failed (restored backup): {e}"));
            }
            Ok(source.to_path_buf())
        }
    }
}

/// 兼容测试：整包重写（无源 zip 时）。
pub fn export_pack_recompress(rom: &Rom, dest: &Path) -> Result<(), String> {
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    if !rom.meta.raw.is_empty() {
        entries.push(("pack.mcmeta".into(), rom.meta.raw.clone().into_bytes()));
    }
    if let Some(icon) = &rom.icon {
        entries.push(("pack.png".into(), icon.data.clone()));
    }
    let mut files = Vec::new();
    collect_files(&rom.root, &mut files);
    for (path, data, _) in files {
        entries.push((path, data));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    write_zip_file(dest, &entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foray::rom::build;
    use crate::foray::zip_safe::{open_pack_bytes, SafeLimits};

    fn zip_bytes() -> Vec<u8> {
        use std::io::Write as _;
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            let o = zip::write::FileOptions::default();
            z.start_file("pack.mcmeta", o).unwrap();
            z.write_all(br#"{"pack":{"pack_format":34,"description":"t"}}"#)
                .unwrap();
            z.start_file("assets/a.txt", o).unwrap();
            z.write_all(b"hello").unwrap();
            z.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn save_as_creates_new_file() {
        let arc = open_pack_bytes(&zip_bytes(), &SafeLimits::default()).unwrap();
        let rom = build(&arc, "pack.zip");
        let dir = tempfile::tempdir().unwrap();
        let out = export_pack(&rom, ExportMode::SaveAs, Some(dir.path())).unwrap();
        assert!(out.exists());
        assert!(out.file_name().unwrap().to_string_lossy().contains("_foray_"));
    }

    #[test]
    fn in_place_writes_backup() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("pack.zip");
        std::fs::write(&src, zip_bytes()).unwrap();
        let arc = open_pack_bytes(&std::fs::read(&src).unwrap(), &SafeLimits::default()).unwrap();
        let rom = build(&arc, src.to_str().unwrap());
        let out = export_pack(&rom, ExportMode::InPlace, None).unwrap();
        assert_eq!(out, src);
        assert!(src.with_extension("zip.bak").exists());
    }

    #[test]
    fn in_place_keeps_clean_entry_content() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("pack.zip");
        std::fs::write(&src, zip_bytes()).unwrap();
        let arc = open_pack_bytes(&std::fs::read(&src).unwrap(), &SafeLimits::default()).unwrap();
        let mut rom = build(&arc, src.to_str().unwrap());
        // dirty one file
        if let Some(f) = rom.find_file_mut("assets/a.txt") {
            f.data = b"HELLO".to_vec();
            f.dirty = true;
        }
        export_pack(&rom, ExportMode::InPlace, None).unwrap();
        let arc2 = open_pack_bytes(&std::fs::read(&src).unwrap(), &SafeLimits::default()).unwrap();
        let a = arc2
            .entries
            .iter()
            .find(|e| e.path == "assets/a.txt")
            .unwrap();
        assert_eq!(&a.data, b"HELLO");
    }
}
