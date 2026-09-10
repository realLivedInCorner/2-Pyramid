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
    // 穿戴盔甲层也在 models/armor 下改名
    let armor = textures_dst.join("models").join("armor");
    if armor.is_dir() {
        let n = rename_stems_in_dir(&armor, java_to_bedrock_stem)?;
        if n > 0 {
            log_info!("OKAY bedrock [models/armor 改名 {} 个]", n);
        }
    }

    let gui_dir = textures_dst.join("gui");
    let ui_dir = textures_dst.join("ui");
    if gui_dir.exists() {
        merge_dir(&gui_dir, &ui_dir)?;
        remove_dir_quiet(&gui_dir);
        log_info!("OKAY bedrock [textures/gui -> textures/ui]");
    }
    // gui/container/* 扁平到 ui/（Bedrock 读 ui/widgets.png 而非 ui/container/widgets.png）
    let container = ui_dir.join("container");
    if container.exists() {
        move_contents_up(&container, &ui_dir)?;
        remove_dir_quiet(&container);
        log_info!("OKAY bedrock [textures/ui/container 扁平化]");
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

    // 床：物品栏/手持用 items/bed.png；方块面贴图尽力铺 bed_*；并保留 bed_color 供现代 atlas
    write_bed_textures(textures_dst);
    // 弩：确保 standby 文件存在
    ensure_crossbow_standby_files(textures_dst);
    // 药水 overlay 等常见物品栏文件
    ensure_potion_item_files(textures_dst);
    // 实体：Java 嵌套路径旁再放 Bedrock 常用扁平名
    alias_entity_textures(textures_dst);

    // 经典 Bedrock 资源包靠文件路径覆盖（可用包均无 item_texture/terrain_texture）。
    // 自写 atlas 会覆盖 vanilla 查找，导致床/药水/弩等物品栏图标异常，故不再生成。
    write_textures_list(textures_dst)?;
    Ok(())
}

/// 床手持/物品图标：经典 Bedrock 使用 textures/items/bed.png（参考可用包）。
/// 同时把 Java 床贴图铺到 blocks/bed_*.png 面贴图（尽力），并复制 bed_color 到 items/。
fn write_bed_textures(textures_dst: &Path) {
    let blocks = textures_dst.join("blocks");
    let items = textures_dst.join("items");
    if !blocks.is_dir() {
        return;
    }
    let _ = fs::create_dir_all(&items);

    // 收集 blocks 下 bed_*.png
    let mut bed_files: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(&blocks) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let fname = entry.file_name().to_string_lossy().to_string();
            if fname.to_ascii_lowercase().ends_with(".png") && fname.starts_with("bed_") {
                bed_files.push(path);
            }
        }
    }
    if bed_files.is_empty() {
        return;
    }

    // 优先 white，否则取第一个，作为手持图标源
    bed_files.sort();
    let source = bed_files
        .iter()
        .find(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy() == "bed_white.png")
                .unwrap_or(false)
        })
        .cloned()
        .unwrap_or_else(|| bed_files[0].clone());

    // 1) 经典手持/物品图标
    let _ = fs::copy(&source, items.join("bed.png"));
    // 2) 现代 item atlas 仍用 bed_white 等
    for path in &bed_files {
        let fname = path.file_name().unwrap().to_string_lossy().to_string();
        let dst = items.join(&fname);
        if !dst.exists() {
            let _ = fs::copy(path, &dst);
        }
    }
    // 3) 方块面贴图：若缺失则用 source 铺满（Java 整床图 → Bedrock 分面 UV，近似）
    for face in [
        "bed_feet_end.png",
        "bed_feet_side.png",
        "bed_feet_top.png",
        "bed_head_end.png",
        "bed_head_side.png",
        "bed_head_top.png",
    ] {
        let dst = blocks.join(face);
        if !dst.exists() {
            let _ = fs::copy(&source, &dst);
        }
    }
    log_info!("OKAY bedrock [bed items/bed.png + face stubs]");
}

/// Java crossbow.png 已改名为 crossbow_standby；若仍有旧名则复制。
fn ensure_crossbow_standby_files(textures_dst: &Path) {
    let items = textures_dst.join("items");
    if !items.is_dir() {
        return;
    }
    // 若只有 crossbow.png（未改名成功等），补一份 standby
    let legacy = items.join("crossbow.png");
    let standby = items.join("crossbow_standby.png");
    if legacy.exists() && !standby.exists() {
        let _ = fs::copy(&legacy, &standby);
    }
    // 若只有 standby，再写回 crossbow.png 兼容
    if standby.exists() && !legacy.exists() {
        let _ = fs::copy(&standby, &legacy);
    }
}

/// 药水物品栏：确保 potion_bottle_* 与 potion_overlay 在 items/。
/// Java 的 overlay 可能在 gui/ 或 item/，缺文件时从常见位置补齐。
fn ensure_potion_item_files(textures_dst: &Path) {
    let items = textures_dst.join("items");
    let gui = textures_dst.join("gui");
    let _ = fs::create_dir_all(&items);

    // overlay：可能在 gui（已并入 ui）或 item/
    for src in [
        textures_dst.join("ui").join("potion_overlay.png"),
        textures_dst.join("ui").join("container").join("potion_overlay.png"),
        gui.join("potion_overlay.png"),
        items.join("potion_overlay.png"),
    ] {
        if src.is_file() {
            let dst = items.join("potion_overlay.png");
            if src != dst && !dst.exists() {
                let _ = fs::copy(&src, &dst);
            }
            break;
        }
    }

    // 若仍有未改名的 Java 药水文件（potion.png 等），补标准名
    let aliases: &[(&str, &str)] = &[
        ("potion.png", "potion_bottle_drinkable.png"),
        ("splash_potion.png", "potion_bottle_splash.png"),
        ("lingering_potion.png", "potion_bottle_lingering.png"),
        ("glass_bottle.png", "potion_bottle_empty.png"),
    ];
    for (from, to) in aliases {
        let src = items.join(from);
        let dst = items.join(to);
        if src.is_file() && !dst.exists() {
            let _ = fs::copy(&src, &dst);
        }
    }
}

/// 实体贴图别名：在嵌套 Java 路径旁放置 Bedrock 常见扁平文件名。
fn alias_entity_textures(textures_dst: &Path) {
    let entity = textures_dst.join("entity");
    if !entity.is_dir() {
        return;
    }
    // (子目录, 文件名, 根 entity/ 下别名)
    let aliases: &[(&str, &str, &str)] = &[
        ("zombie", "zombie.png", "zombie.png"),
        ("sheep", "sheep.png", "sheep.png"),
        ("sheep", "sheep_fur.png", "sheep_fur.png"),
        ("skeleton", "skeleton.png", "skeleton.png"),
        ("creeper", "creeper.png", "creeper.png"),
    ];
    for (folder, file, alias) in aliases {
        let src = entity.join(folder).join(file);
        let dst = entity.join(alias);
        if src.is_file() && !dst.exists() {
            let _ = fs::copy(&src, &dst);
        }
    }
    // 反向：扁平存在时补回嵌套（部分 Java 包用扁平）
    for (folder, file, alias) in aliases {
        let src = entity.join(alias);
        let dst = entity.join(folder).join(file);
        if src.is_file() && !dst.exists() {
            let _ = fs::create_dir_all(entity.join(folder));
            let _ = fs::copy(&src, &dst);
        }
    }
    log_info!("OKAY bedrock [entity texture aliases]");
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
        let Ok(rel) = path.strip_prefix(root.parent().unwrap_or(root)) else {
            continue;
        };
        let mut s = rel.to_string_lossy().replace('\\', "/");
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

/// j2b：生成 terrain_texture.json / item_texture.json。
/// 多变体物品使用 vanilla shortname（bed / bucket / bow_* 等），否则用文件名 stem。
pub fn write_bedrock_atlas_maps(textures_dst: &Path) -> Result<(), String> {
    write_atlas_file(&textures_dst.join("blocks"), "terrain_texture.json", textures_dst, Some("atlas.terrain"))?;
    write_atlas_file(&textures_dst.join("items"), "item_texture.json", textures_dst, None)?;
    Ok(())
}

const BED_VARIANT_ORDER: &[&str] = &[
    "white",
    "orange",
    "magenta",
    "light_blue",
    "yellow",
    "lime",
    "pink",
    "gray",
    "silver",
    "cyan",
    "purple",
    "blue",
    "brown",
    "green",
    "red",
    "black",
];

const BUCKET_VARIANT_ORDER: &[&str] = &[
    "empty",
    "milk",
    "water",
    "lava",
    "cod",
    "salmon",
    "tropical",
    "pufferfish",
    "powder_snow",
    "axolotl",
    "tadpole",
];

fn write_atlas_file(
    src_dir: &Path,
    out_name: &str,
    textures_dst: &Path,
    texture_name: Option<&str>,
) -> Result<(), String> {
    if !src_dir.is_dir() {
        return Ok(());
    }
    let folder = src_dir
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    // stem 列表
    let mut stems: Vec<String> = Vec::new();
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
        if lower.ends_with(".png") {
            let stem = fname[..fname.len() - 4].to_string();
            if !stem.is_empty() {
                stems.push(stem);
            }
        }
    }
    if stems.is_empty() {
        return Ok(());
    }
    stems.sort();

    let mut data = serde_json::Map::new();
    let mut consumed: std::collections::HashSet<String> = Default::default();

    // 1) vanilla 多变体组
    if folder == "items" {
        // bed
        let mut bed_paths = Vec::new();
        for color in BED_VARIANT_ORDER {
            let stem = format!("bed_{}", color);
            if stems.contains(&stem) {
                bed_paths.push(format!("textures/items/{}", stem));
                consumed.insert(stem);
            }
        }
        if !bed_paths.is_empty() {
            data.insert("bed".into(), serde_json::json!({ "textures": bed_paths }));
        }
        // 经典手持图标 bed.png 优先（可用包布局）；无 bed.png 时保留颜色数组
        if stems.contains(&"bed".to_string()) {
            data.insert(
                "bed".into(),
                serde_json::json!({ "textures": "textures/items/bed" }),
            );
            consumed.insert("bed".into());
        }
        // bucket
        let mut bucket_paths = Vec::new();
        for name in BUCKET_VARIANT_ORDER {
            let stem = format!("bucket_{}", name);
            if stems.contains(&stem) {
                bucket_paths.push(format!("textures/items/{}", stem));
                consumed.insert(stem);
            }
        }
        // 任意未在固定序中的 bucket_* 也挂到 bucket 数组末尾
        for s in &stems {
            if s.starts_with("bucket_") && !consumed.contains(s) {
                bucket_paths.push(format!("textures/items/{}", s));
                consumed.insert(s.clone());
            }
        }
        if !bucket_paths.is_empty() {
            data.insert("bucket".into(), serde_json::json!({ "textures": bucket_paths }));
        }
        // bow / crossbow
        if stems.contains(&"bow_standby".to_string()) {
            data.insert(
                "bow_standby".into(),
                serde_json::json!({ "textures": "textures/items/bow_standby" }),
            );
            consumed.insert("bow_standby".into());
        }
        let mut bow_pull = Vec::new();
        for i in 0..3 {
            let stem = format!("bow_pulling_{}", i);
            if stems.contains(&stem) {
                bow_pull.push(format!("textures/items/{}", stem));
                consumed.insert(stem);
            }
        }
        if !bow_pull.is_empty() {
            data.insert("bow_pulling".into(), serde_json::json!({ "textures": bow_pull }));
        }
        if stems.contains(&"crossbow_standby".to_string()) {
            data.insert(
                "crossbow_standby".into(),
                serde_json::json!({ "textures": "textures/items/crossbow_standby" }),
            );
            // 兼容仍查 crossbow 的路径
            data.insert(
                "crossbow".into(),
                serde_json::json!({ "textures": "textures/items/crossbow_standby" }),
            );
            consumed.insert("crossbow_standby".into());
            consumed.insert("crossbow".into());
        }
        let mut cross_pull: Vec<String> = Vec::new();
        for stem in ["crossbow_pulling_0", "crossbow_pulling_1", "crossbow_pulling_2", "crossbow_arrow", "crossbow_firework"] {
            if stems.contains(&stem.to_string()) {
                cross_pull.push(format!("textures/items/{}", stem));
                consumed.insert(stem.to_string());
            }
        }
        if !cross_pull.is_empty() {
            data.insert("crossbow_pulling".into(), serde_json::json!({ "textures": cross_pull }));
        }
    }

    // 2) 其余 stem → shortname=stem
    for stem in &stems {
        if consumed.contains(stem) {
            continue;
        }
        let rel = format!("textures/{}/{}", folder, stem);
        data.insert(stem.clone(), serde_json::json!({ "textures": rel }));
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

        // 不再自写 atlas（可用包均无这些文件；写错会覆盖 vanilla 查找）
        assert!(!root.join("textures/item_texture.json").exists());
        assert!(!root.join("textures/terrain_texture.json").exists());
    }

    #[test]
    fn test_bucket_bed_bow_classic_paths() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        let mc = root.join("assets/minecraft");
        let tex = mc.join("textures");
        fs::create_dir_all(tex.join("item")).unwrap();
        fs::create_dir_all(tex.join("block")).unwrap();
        fs::create_dir_all(tex.join("gui")).unwrap();
        fs::write(tex.join("item/water_bucket.png"), b"w").unwrap();
        fs::write(tex.join("item/bucket.png"), b"e").unwrap();
        fs::write(tex.join("item/bow.png"), b"b").unwrap();
        fs::write(tex.join("item/bow_pulling_0.png"), b"p").unwrap();
        fs::write(tex.join("item/crossbow.png"), b"c").unwrap();
        fs::write(tex.join("item/potion.png"), b"p").unwrap();
        fs::write(tex.join("gui/potion_overlay.png"), b"o").unwrap();
        fs::write(tex.join("block/red_bed.png"), b"r").unwrap();
        fs::write(tex.join("block/white_bed.png"), b"wh").unwrap();

        move_java_font_to_bedrock(&mc, root);
        reorganize_java_textures_for_bedrock(&mc, &root.join("textures")).unwrap();

        assert!(root.join("textures/items/bucket_water.png").exists());
        assert!(root.join("textures/items/bucket_empty.png").exists());
        assert!(root.join("textures/items/bow_standby.png").exists());
        assert!(root.join("textures/items/crossbow_standby.png").exists());
        assert!(root.join("textures/items/crossbow.png").exists());
        // 床
        assert!(root.join("textures/blocks/bed_red.png").exists());
        assert!(root.join("textures/items/bed_red.png").exists());
        assert!(root.join("textures/items/bed.png").exists());
        assert!(root.join("textures/blocks/bed_head_top.png").exists());
        // 药水
        assert!(root.join("textures/items/potion_bottle_drinkable.png").exists());
        assert!(root.join("textures/items/potion_overlay.png").exists());
        // 不生成 atlas
        assert!(!root.join("textures/item_texture.json").exists());
        assert!(!root.join("textures/terrain_texture.json").exists());
    }

    #[test]
    fn test_entity_aliases_and_crossbow_files() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        let mc = root.join("assets/minecraft");
        let tex = mc.join("textures");
        fs::create_dir_all(tex.join("entity/zombie")).unwrap();
        fs::create_dir_all(tex.join("entity/sheep")).unwrap();
        fs::create_dir_all(tex.join("item")).unwrap();
        fs::write(tex.join("entity/zombie/zombie.png"), b"z").unwrap();
        fs::write(tex.join("entity/sheep/sheep.png"), b"s").unwrap();
        fs::write(tex.join("entity/sheep/sheep_fur.png"), b"f").unwrap();
        fs::write(tex.join("item/crossbow.png"), b"c").unwrap();

        move_java_font_to_bedrock(&mc, root);
        reorganize_java_textures_for_bedrock(&mc, &root.join("textures")).unwrap();

        assert!(root.join("textures/entity/zombie.png").exists());
        assert!(root.join("textures/entity/sheep.png").exists());
        assert!(root.join("textures/entity/sheep_fur.png").exists());
        assert!(root.join("textures/items/crossbow_standby.png").exists());
        assert!(root.join("textures/items/crossbow.png").exists());
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
