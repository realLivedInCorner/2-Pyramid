//! 目录合并、贴图后缀拆分、按映射表改名等文件工具。

use std::fs;
use std::path::Path;

pub fn merge_dir(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(dst).map_err(|e| format!("create dir failed: {}", e))?;
    for entry in fs::read_dir(src).map_err(|e| format!("read dir failed: {}", e))? {
        let entry = entry.map_err(|e| format!("read entry failed: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            merge_dir(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| format!("copy failed: {}", e))?;
        }
    }
    Ok(())
}

pub fn move_contents_up(src: &Path, dst: &Path) -> Result<(), String> {
    for entry in fs::read_dir(src).map_err(|e| format!("read dir failed: {}", e))? {
        let entry = entry.map_err(|e| format!("read entry failed: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            merge_dir(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| format!("copy failed: {}", e))?;
        }
    }
    Ok(())
}

pub fn rename_dir_if_absent(src: &Path, dst: &Path) -> Result<(), String> {
    if src.exists() && !dst.exists() {
        fs::rename(src, dst).map_err(|e| format!("rename {} failed: {}", src.display(), e))?;
    }
    Ok(())
}

pub fn split_texture_suffix(file_name: &str) -> Option<(String, String)> {
    let lower = file_name.to_ascii_lowercase();
    if lower.ends_with(".png.mcmeta") {
        Some((
            file_name[..file_name.len() - ".png.mcmeta".len()].to_string(),
            ".png.mcmeta".to_string(),
        ))
    } else if lower.ends_with(".png") {
        Some((
            file_name[..file_name.len() - ".png".len()].to_string(),
            ".png".to_string(),
        ))
    } else if lower.ends_with(".tga") {
        Some((
            file_name[..file_name.len() - ".tga".len()].to_string(),
            ".tga".to_string(),
        ))
    } else {
        None
    }
}

/// 对目录内 png / png.mcmeta / tga 成对改名。
pub fn rename_stems_in_dir(dir: &Path, map_fn: fn(&str) -> Option<String>) -> Result<usize, String> {
    let mut renamed = 0;
    if !dir.is_dir() {
        return Ok(0);
    }
    let entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| format!("read dir failed: {}", e))?
        .filter_map(|e| e.ok())
        .collect();
    for entry in entries {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        let Some((stem, suffix)) = split_texture_suffix(&file_name) else {
            continue;
        };
        let Some(new_stem) = map_fn(&stem) else { continue };
        let new_path = dir.join(format!("{}{}", new_stem, suffix));
        if new_path.exists() {
            let _ = fs::remove_file(&new_path);
        }
        fs::rename(&path, &new_path).map_err(|e| format!("rename id failed: {}", e))?;
        renamed += 1;
    }
    Ok(renamed)
}

pub fn remove_dir_quiet(p: &Path) {
    if p.is_dir() {
        let _ = fs::remove_dir_all(p);
    }
}

pub fn remove_file_quiet(p: &Path) {
    if p.is_file() {
        let _ = fs::remove_file(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_split_texture_suffix() {
        assert_eq!(
            split_texture_suffix("foo.png.mcmeta"),
            Some(("foo".into(), ".png.mcmeta".into()))
        );
        assert_eq!(
            split_texture_suffix("foo.png"),
            Some(("foo".into(), ".png".into()))
        );
        assert_eq!(split_texture_suffix("foo.json"), None);
    }

    #[test]
    fn test_merge_and_rename() {
        let temp = tempdir().unwrap();
        let a = temp.path().join("a");
        let b = temp.path().join("b");
        fs::create_dir_all(a.join("sub")).unwrap();
        fs::write(a.join("sub/x.png"), b"x").unwrap();
        fs::write(a.join("y.png"), b"y").unwrap();
        merge_dir(&a, &b).unwrap();
        assert!(b.join("sub/x.png").exists());
        assert!(b.join("y.png").exists());

        let n = rename_stems_in_dir(&b, |s| {
            if s == "y" {
                Some("z".into())
            } else {
                None
            }
        })
        .unwrap();
        assert_eq!(n, 1);
        assert!(b.join("z.png").exists());
        assert!(!b.join("y.png").exists());
    }
}
