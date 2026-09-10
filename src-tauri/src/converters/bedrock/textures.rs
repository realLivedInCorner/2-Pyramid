//! 贴图目录重组与 flipbook 动画收集。

use std::fs;
use std::path::Path;

use crate::log_info;

use super::fsutil::{
    merge_dir, move_contents_up, remove_dir_quiet, remove_file_quiet, rename_dir_if_absent,
    rename_stems_in_dir,
};
use super::mapping::{bedrock_to_java_stem, java_to_bedrock_stem};

/// j2b：提升 textures、item/block → items/blocks、改名、gui→ui、flipbook。
pub fn reorganize_java_textures_for_bedrock(minecraft: &Path, textures_dst: &Path) -> Result<(), String> {
    let textures_src = minecraft.join("textures");
    if textures_src.exists() {
        merge_dir(&textures_src, textures_dst)?;
        let _ = fs::remove_dir_all(&textures_src);
        log_info!("OKAY bedrock [minecraft/textures -> textures/]");
    }

    rename_dir_if_absent(&textures_dst.join("item"), &textures_dst.join("items"))?;
    rename_dir_if_absent(&textures_dst.join("block"), &textures_dst.join("blocks"))?;
    for d in ["items", "blocks"] {
        let n = rename_stems_in_dir(&textures_dst.join(d), java_to_bedrock_stem)?;
        if n > 0 {
            log_info!("OKAY bedrock [{} 改名 {} 个]", d, n);
        }
    }

    let gui_dir = textures_dst.join("gui");
    let ui_dir = textures_dst.join("ui");
    if gui_dir.exists() {
        merge_dir(&gui_dir, &ui_dir)?;
        remove_dir_quiet(&gui_dir);
        log_info!("OKAY bedrock [textures/gui -> textures/ui]");
    }
    let creative = ui_dir.join("creative_inventory");
    if creative.exists() {
        move_contents_up(&creative, &ui_dir)?;
        remove_dir_quiet(&creative);
    }

    let flipbooks = collect_flipbooks(textures_dst);
    if !flipbooks.is_empty() {
        let pretty = serde_json::to_string_pretty(&flipbooks)
            .map_err(|e| format!("serialize flipbook failed: {}", e))?;
        fs::write(textures_dst.join("flipbook_textures.json"), pretty)
            .map_err(|e| format!("write flipbook failed: {}", e))?;
        log_info!("OKAY bedrock [flipbook_textures.json × {}]", flipbooks.len());
    }

    // Java colormap → Bedrock colormaps
    rename_dir_if_absent(&textures_dst.join("colormap"), &textures_dst.join("colormaps"))?;

    // T6/T7: 贴图缓存 + atlas 短名表
    write_textures_list(textures_dst)?;
    write_bedrock_atlas_maps(textures_dst)?;
    Ok(())
}

/// j2b：生成 `textures/textures_list.json`（无扩展名相对包根路径）。
pub fn write_textures_list(textures_dst: &Path) -> Result<(), String> {
    let mut paths = Vec::new();
    collect_texture_rel_paths(textures_dst, textures_dst, &mut paths);
    paths.sort();
    paths.dedup();
    let pretty = serde_json::to_string_pretty(&paths)
        .map_err(|e| format!("serialize textures_list failed: {}", e))?;
    fs::write(textures_dst.join("textures_list.json"), pretty)
        .map_err(|e| format!("write textures_list failed: {}", e))?;
    log_info!("OKAY bedrock [textures_list.json × {}]", paths.len());
    Ok(())
}

fn collect_texture_rel_paths(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_texture_rel_paths(root, &path, out);
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if !(name.ends_with(".png") || name.ends_with(".tga")) {
            continue;
        }
        // 跳过 mcmeta 配对文件本身；flipbook/atlas json 不在 textures 子目录树的 png 里
        let Ok(rel) = path.strip_prefix(root.parent().unwrap_or(root)) else {
            // root = textures/，包根 = parent
            continue;
        };
        let mut s = rel.to_string_lossy().replace('\\', "/");
        // 大小写不敏感去扩展名
        let lower = s.to_ascii_lowercase();
        if lower.ends_with(".png") {
            s.truncate(s.len() - 4);
        } else if lower.ends_with(".tga") {
            s.truncate(s.len() - 4);
        }
        if !s.is_empty() {
            out.push(s);
        }
    }
}

/// j2b：生成 terrain_texture.json / item_texture.json（shortname=文件名 stem）。
pub fn write_bedrock_atlas_maps(textures_dst: &Path) -> Result<(), String> {
    write_atlas_file(&textures_dst.join("blocks"), "terrain_texture.json", textures_dst, Some("atlas.terrain"))?;
    write_atlas_file(&textures_dst.join("items"), "item_texture.json", textures_dst, None)?;
    Ok(())
}

fn write_atlas_file(
    src_dir: &Path,
    out_name: &str,
    textures_dst: &Path,
    texture_name: Option<&str>,
) -> Result<(), String> {
    if !src_dir.is_dir() {
        return Ok(());
    }
    let mut data = serde_json::Map::new();
    let Ok(entries) = fs::read_dir(src_dir) else { return Ok(()) };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let fname = entry.file_name().to_string_lossy().to_string();
        if !fname.to_ascii_lowercase().ends_with(".png") {
            continue;
        }
        let lower = fname.to_ascii_lowercase();
        let stem = if lower.ends_with(".png") {
            fname[..fname.len() - 4].to_string()
        } else {
            continue;
        };
        if stem.is_empty() {
            continue;
        }
        let folder = src_dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let rel = format!("textures/{}/{}", folder, stem);
        data.insert(
            stem,
            serde_json::json!({ "textures": rel }),
        );
    }
    if data.is_empty() {
        return Ok(());
    }
    let mut root = serde_json::Map::new();
    if let Some(name) = texture_name {
        root.insert("texture_name".into(), serde_json::Value::String(name.into()));
    }
    root.insert("texture_data".into(), serde_json::Value::Object(data));
    let pretty = serde_json::to_string_pretty(&serde_json::Value::Object(root))
        .map_err(|e| format!("serialize atlas failed: {}", e))?;
    fs::write(textures_dst.join(out_name), pretty)
        .map_err(|e| format!("write {} failed: {}", out_name, e))?;
    log_info!("OKAY bedrock [{}]", out_name);
    Ok(())
}

/// j2b：字体目录搬到包根 font/，ascii→default8。
pub fn move_java_font_to_bedrock(minecraft: &Path, pack_root: &Path) {
    let font_src = minecraft.join("textures").join("font");
    let font_dst = pack_root.join("font");
    if !font_src.exists() {
        return;
    }
    remove_dir_quiet(&font_dst);
    if fs::rename(&font_src, &font_dst).is_err() {
        let _ = merge_dir(&font_src, &font_dst);
        remove_dir_quiet(&font_src);
    }
    let ascii = font_dst.join("ascii.png");
    let default8 = font_dst.join("default8.png");
    if ascii.exists() && !default8.exists() {
        let _ = fs::rename(&ascii, &default8);
    }
    log_info!("OKAY bedrock [textures/font -> font/]");
}

/// b2j：textures 提升到 assets/minecraft/textures，items/blocks 单数化并反向改名。
pub fn reorganize_bedrock_textures_for_java(pack_root: &Path, minecraft: &Path) -> Result<(), String> {
    let textures_src = pack_root.join("textures");
    let textures_dst = minecraft.join("textures");
    if textures_src.exists() {
        fs::create_dir_all(&textures_dst)
            .map_err(|e| format!("create textures dst failed: {}", e))?;
        merge_dir(&textures_src, &textures_dst)?;
        remove_dir_quiet(&textures_src);
        log_info!("OKAY java [textures -> assets/minecraft/textures]");
    }

    rename_dir_if_absent(&textures_dst.join("items"), &textures_dst.join("item"))?;
    rename_dir_if_absent(&textures_dst.join("blocks"), &textures_dst.join("block"))?;
    // Bedrock colormaps → Java colormap
    if textures_dst.join("colormaps").exists() {
        let cmap = textures_dst.join("colormap");
        if cmap.exists() {
            let _ = merge_dir(&textures_dst.join("colormaps"), &cmap);
            remove_dir_quiet(&textures_dst.join("colormaps"));
        } else {
            rename_dir_if_absent(&textures_dst.join("colormaps"), &cmap)?;
        }
    }
    // b2j：丢弃 Bedrock 专用贴图索引（T6/T7）
    for f in [
        "textures_list.json",
        "terrain_texture.json",
        "item_texture.json",
        "flipbook_textures.json",
    ] {
        remove_file_quiet(&textures_dst.join(f));
    }
    for d in ["item", "block"] {
        let n = rename_stems_in_dir(&textures_dst.join(d), bedrock_to_java_stem)?;
        if n > 0 {
            log_info!("OKAY java [{} 反向改名 {} 个]", d, n);
        }
    }
    Ok(())
}

/// b2j：包根 font/ → textures/font/，default8→ascii。
pub fn move_bedrock_font_to_java(pack_root: &Path, minecraft: &Path) {
    let font_src = pack_root.join("font");
    if !font_src.exists() {
        return;
    }
    let font_dst = minecraft.join("textures").join("font");
    let _ = fs::create_dir_all(&font_dst);
    let _ = merge_dir(&font_src, &font_dst);
    remove_dir_quiet(&font_src);
    let default8 = font_dst.join("default8.png");
    let ascii = font_dst.join("ascii.png");
    if default8.exists() && !ascii.exists() {
        let _ = fs::rename(&default8, &ascii);
    }
    log_info!("OKAY java [font/ -> textures/font/]");
}

/// b2j：textures/ui → gui/container，去掉 json。
pub fn move_bedrock_ui_to_java_gui(minecraft: &Path) {
    let ui_src = minecraft.join("textures").join("ui");
    if !ui_src.exists() {
        return;
    }
    let gui_container = minecraft.join("textures").join("gui").join("container");
    let _ = fs::create_dir_all(&gui_container);
    let _ = merge_dir(&ui_src, &gui_container);
    remove_dir_quiet(&ui_src);
    strip_json_in_dir(&gui_container);
    log_info!("OKAY java [textures/ui -> textures/gui/container]");
}

fn strip_json_in_dir(dir: &Path) {
    if !dir.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            strip_json_in_dir(&path);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("json"))
            .unwrap_or(false)
        {
            let _ = fs::remove_file(&path);
        }
    }
}

fn collect_flipbooks(textures_dst: &Path) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    walk_flipbooks(textures_dst, textures_dst, &mut out);
    out
}

fn walk_flipbooks(root: &Path, dir: &Path, out: &mut Vec<serde_json::Value>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_flipbooks(root, &path, out);
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.to_ascii_lowercase().ends_with(".png.mcmeta") {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else { continue };
        let Ok(meta) = serde_json::from_str::<serde_json::Value>(&content) else {
            continue;
        };
        let Some(anim) = meta.get("animation") else { continue };
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let flipbook_texture = rel.trim_end_matches(".png.mcmeta").to_string();
        let tile = flipbook_texture
            .rsplit('/')
            .next()
            .unwrap_or(&flipbook_texture)
            .to_string();
        let ticks = anim
            .get("frametime")
            .and_then(|v| v.as_u64())
            .unwrap_or(1)
            .max(1);
        out.push(serde_json::json!({
            "flipbook_texture": flipbook_texture,
            "atlas_tile": tile,
            "ticks_per_frame": ticks,
        }));
        remove_file_quiet(&path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_j2b_textures_reorg() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        let mc = root.join("assets/minecraft");
        let tex = mc.join("textures");
        fs::create_dir_all(tex.join("item")).unwrap();
        fs::create_dir_all(tex.join("block")).unwrap();
        fs::create_dir_all(tex.join("gui")).unwrap();
        fs::write(tex.join("item/golden_apple.png"), b"g").unwrap();
        fs::write(tex.join("block/water_still.png"), b"w").unwrap();
        fs::write(
            tex.join("block/water_still.png.mcmeta"),
            r#"{"animation":{"frametime":3}}"#,
        )
        .unwrap();
        fs::write(tex.join("gui/icons.png"), b"i").unwrap();
        fs::create_dir_all(tex.join("font")).unwrap();
        fs::write(tex.join("font/ascii.png"), b"a").unwrap();

        let textures_dst = root.join("textures");
        // 与 j2b.rs 顺序一致：先抽 font，再提升 textures
        move_java_font_to_bedrock(&mc, root);
        reorganize_java_textures_for_bedrock(&mc, &textures_dst).unwrap();

        assert!(root.join("textures/items/apple_golden.png").exists());
        assert!(root.join("textures/blocks/water_still.png").exists());
        assert!(root.join("textures/ui/icons.png").exists());
        assert!(root.join("font/default8.png").exists());
        let flip: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(root.join("textures/flipbook_textures.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(flip[0]["ticks_per_frame"], 3);
        assert!(!root.join("textures/blocks/water_still.png.mcmeta").exists());

        // T6 textures_list
        let list: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(root.join("textures/textures_list.json")).unwrap(),
        )
        .unwrap();
        let arr = list.as_array().unwrap();
        assert!(arr.iter().any(|v| v.as_str() == Some("textures/blocks/water_still")));
        assert!(arr.iter().any(|v| v.as_str() == Some("textures/items/apple_golden")));

        // T7 atlas
        let terrain: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(root.join("textures/terrain_texture.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            terrain["texture_data"]["water_still"]["textures"],
            "textures/blocks/water_still"
        );
        let item: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(root.join("textures/item_texture.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            item["texture_data"]["apple_golden"]["textures"],
            "textures/items/apple_golden"
        );
    }

    #[test]
    fn test_b2j_textures_reorg() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        let mc = root.join("assets/minecraft");
        fs::create_dir_all(root.join("textures/items")).unwrap();
        fs::create_dir_all(root.join("textures/blocks")).unwrap();
        fs::create_dir_all(root.join("textures/ui")).unwrap();
        fs::create_dir_all(root.join("font")).unwrap();
        fs::write(root.join("textures/items/apple_golden.png"), b"g").unwrap();
        fs::write(root.join("textures/blocks/stone.png"), b"s").unwrap();
        fs::write(root.join("textures/ui/widgets.png"), b"w").unwrap();
        fs::write(root.join("textures/ui/x.json"), b"{}").unwrap();
        fs::write(root.join("font/default8.png"), b"f").unwrap();
        fs::create_dir_all(root.join("textures/colormaps")).unwrap();
        fs::write(root.join("textures/colormaps/grass.png"), b"c").unwrap();
        fs::write(root.join("textures/textures_list.json"), b"[]").unwrap();
        fs::write(root.join("textures/terrain_texture.json"), b"{}").unwrap();
        fs::write(root.join("textures/item_texture.json"), b"{}").unwrap();

        reorganize_bedrock_textures_for_java(root, &mc).unwrap();
        move_bedrock_font_to_java(root, &mc);
        move_bedrock_ui_to_java_gui(&mc);

        assert!(mc.join("textures/item/golden_apple.png").exists());
        assert!(mc.join("textures/block/stone.png").exists());
        assert!(mc.join("textures/gui/container/widgets.png").exists());
        assert!(!mc.join("textures/gui/container/x.json").exists());
        assert!(mc.join("textures/font/ascii.png").exists());
        assert!(mc.join("textures/colormap/grass.png").exists());
        assert!(!mc.join("textures/textures_list.json").exists());
        assert!(!mc.join("textures/terrain_texture.json").exists());
        assert!(!mc.join("textures/item_texture.json").exists());
        assert!(!root.join("textures").exists());
        assert!(!root.join("font").exists());
    }
}
