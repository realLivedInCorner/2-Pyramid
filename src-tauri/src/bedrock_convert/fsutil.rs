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
        // **恒等映射必须跳过**（§9.125 实测缺陷）：映射表里有 `bamboo_block → bamboo_block`
        // 这类**恒等项**（它们的作用是声明"这个名字已在覆盖范围内"，不是要改名）。
        // 若照常执行：`new_path == path` ⇒ 下面那句 `remove_file(new_path)` 把**源文件删掉**，
        // 紧接着的 `rename` 必然报 `NotFound`（Windows os error 2）——
        // **整条 j2b 转换就此中止**。修复前它一直潜伏，只因为以前没人拿"含恒等映射项"的真实包跑过 j2b。
        if new_stem == stem {
            continue;
        }
        let new_path = dir.join(format!("{}{}", new_stem, suffix));
        if new_path.exists() {
            let _ = fs::remove_file(&new_path);
        }
        fs::rename(&path, &new_path).map_err(|e| {
            // 保留现场读数：这类失败以前只有一句 "rename id failed"，无法定位是哪一个文件
            format!(
                "rename id failed: {} [dir={} from={:?} to={:?} exists_after={}]",
                e,
                dir.display(),
                path,
                new_path,
                path.exists()
            )
        })?;
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

    /// **恒等映射必须安全**（§9.125 实测缺陷的回归用例）。
    ///
    /// 映射表里有 `bamboo_block → bamboo_block` 这类**恒等项**（声明"已在覆盖范围内"，
    /// 不是要改名）。修复前该函数会走「先删目标、再改名」——而目标就是源本身 ⇒
    /// **源被删掉、rename 报 NotFound**，整条 j2b 转换中止。真实包（TapL 16x）实测：
    /// `textures/blocks/bamboo_block.png` 正是第一个撞上的文件。
    #[test]
    fn test_identity_mapping_does_not_delete_the_file() {
        let temp = tempdir().unwrap();
        let d = temp.path().join("blocks");
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("bamboo_block.png"), b"keep me").unwrap();
        fs::write(d.join("bamboo_block.png.mcmeta"), b"meta").unwrap();
        fs::write(d.join("stone.png"), b"stone").unwrap();

        // 只有 stone 真改名；bamboo_block 是恒等映射（此前会让整个调用报错）
        let n = rename_stems_in_dir(&d, |s| match s {
            "bamboo_block" => Some("bamboo_block".into()),
            "stone" => Some("rock".into()),
            _ => None,
        })
        .expect("恒等映射不得导致 rename 失败（此前会报 NotFound）");

        assert_eq!(n, 1, "只有 stone 真被改名；恒等项不计入");
        assert!(d.join("bamboo_block.png").exists(), "恒等映射的文件必须还在");
        assert!(
            d.join("bamboo_block.png.mcmeta").exists(),
            "恒等映射的成对 mcmeta 也必须还在"
        );
        assert!(!d.join("stone.png").exists());
        assert!(d.join("rock.png").exists());
    }
}
