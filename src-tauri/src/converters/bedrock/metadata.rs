//! manifest / pack.mcmeta / lang / sounds / 剥离。

use std::fs;
use std::path::Path;

use crate::log_info;

use super::fsutil::{merge_dir, remove_dir_quiet, remove_file_quiet};
use super::mapping::{
    bedrock_lang_key_to_java, java_lang_key_to_bedrock, normalize_bedrock_lang_code,
    normalize_java_lang_code,
};

pub fn is_bedrock_resource_pack(root: &Path) -> bool {
    let manifest = root.join("manifest.json");
    if manifest.is_file() {
        if let Ok(raw) = fs::read_to_string(&manifest) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(mods) = v.get("modules").and_then(|m| m.as_array()) {
                    return mods
                        .iter()
                        .any(|m| m.get("type").and_then(|t| t.as_str()) == Some("resources"));
                }
            }
        }
    }
    root.join("pack_icon.png").is_file()
        && !root.join("pack.mcmeta").is_file()
        && (root.join("textures").is_dir() || root.join("manifest.json").is_file())
}

pub fn read_description_from_mcmeta(temp_dir: &Path) -> String {
    let mcmeta = temp_dir.join("pack.mcmeta");
    let Ok(content) = fs::read_to_string(&mcmeta) else {
        return "Converted by 2-Pyramid".to_string();
    };
    let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) else {
        return "Converted by 2-Pyramid".to_string();
    };
    match data
        .get("pack")
        .and_then(|p| p.get("description"))
        .and_then(|d| d.as_str())
    {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => "Converted by 2-Pyramid".to_string(),
    }
}

pub fn write_manifest(temp_dir: &Path, pack_name: &str, description: &str) -> Result<(), String> {
    let manifest = serde_json::json!({
        "format_version": 2,
        "header": {
            "description": description,
            "name": pack_name,
            "uuid": uuid::Uuid::new_v4().to_string(),
            "version": [1, 0, 0],
            "min_engine_version": [1, 20, 0],
        },
        "modules": [{
            "description": description,
            "type": "resources",
            "uuid": uuid::Uuid::new_v4().to_string(),
            "version": [1, 0, 0],
        }],
        "metadata": {
            "authors": ["2-Pyramid"],
            "generated_with": {
                "2-Pyramid": [env!("CARGO_PKG_VERSION")]
            }
        }
    });
    let pretty = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("serialize manifest failed: {}", e))?;
    fs::write(temp_dir.join("manifest.json"), pretty)
        .map_err(|e| format!("write manifest failed: {}", e))?;
    Ok(())
}

pub fn read_manifest_info(temp_dir: &Path) -> (String, String) {
    let default_name = "Converted Pack".to_string();
    let default_desc = "Converted by 2-Pyramid".to_string();
    let Ok(raw) = fs::read_to_string(temp_dir.join("manifest.json")) else {
        return (default_name, default_desc);
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return (default_name, default_desc);
    };
    let header = v.get("header");
    let name = header
        .and_then(|h| h.get("name"))
        .and_then(|n| n.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("Converted Pack")
        .to_string();
    let desc = header
        .and_then(|h| h.get("description"))
        .and_then(|d| d.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("Converted by 2-Pyramid")
        .to_string();
    (name, desc)
}

pub fn write_pack_mcmeta(temp_dir: &Path, pack_format: u32, description: &str) -> Result<(), String> {
    let mcmeta = serde_json::json!({
        "pack": { "pack_format": pack_format, "description": description }
    });
    let pretty = serde_json::to_string_pretty(&mcmeta)
        .map_err(|e| format!("serialize mcmeta failed: {}", e))?;
    fs::write(temp_dir.join("pack.mcmeta"), pretty)
        .map_err(|e| format!("write pack.mcmeta failed: {}", e))?;
    Ok(())
}

pub fn convert_lang_java_to_bedrock(minecraft: &Path, temp_dir: &Path) {
    let lang_src = minecraft.join("lang");
    if !lang_src.is_dir() {
        return;
    }
    let texts_dst = temp_dir.join("texts");
    if fs::create_dir_all(&texts_dst).is_err() {
        return;
    }
    let Ok(entries) = fs::read_dir(&lang_src) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let fname = entry.file_name().to_string_lossy().to_string();
        if !fname.to_ascii_lowercase().ends_with(".json") {
            continue;
        }
        let code = fname.trim_end_matches(".json").to_ascii_lowercase();
        let bedrock_code = normalize_bedrock_lang_code(&code);
        let Ok(raw) = fs::read_to_string(&path) else { continue };
        let Ok(map) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&raw)
        else {
            continue;
        };
        let mut lines = Vec::with_capacity(map.len());
        for (k, v) in &map {
            let Some(val) = v.as_str() else { continue };
            lines.push(format!(
                "{}={}",
                java_lang_key_to_bedrock(k),
                val.replace('\n', "\\n")
            ));
        }
        if lines.is_empty() {
            continue;
        }
        let _ = fs::write(texts_dst.join(format!("{}.lang", bedrock_code)), lines.join("\n"));
    }
    log_info!("OKAY bedrock [lang json -> texts/*.lang]");
}

pub fn convert_lang_bedrock_to_java(temp_dir: &Path, minecraft: &Path) {
    let texts = temp_dir.join("texts");
    if !texts.is_dir() {
        return;
    }
    let lang_dst = minecraft.join("lang");
    let Ok(entries) = fs::read_dir(&texts) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let fname = entry.file_name().to_string_lossy().to_string();
        if !fname.to_ascii_lowercase().ends_with(".lang") {
            continue;
        }
        let code = fname.trim_end_matches(".lang").to_string();
        let java_code = normalize_java_lang_code(&code);
        let Ok(raw) = fs::read_to_string(&path) else { continue };
        let mut map = serde_json::Map::new();
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            if let Some(jk) = bedrock_lang_key_to_java(k) {
                map.insert(jk, serde_json::Value::String(v.to_string()));
            }
        }
        if map.is_empty() {
            continue;
        }
        let _ = fs::create_dir_all(&lang_dst);
        if let Ok(pretty) = serde_json::to_string_pretty(&serde_json::Value::Object(map)) {
            let _ = fs::write(lang_dst.join(format!("{}.json", java_code)), pretty);
        }
    }
    log_info!("OKAY java [texts/*.lang -> lang/*.json]");
}

pub fn convert_sounds_java_to_bedrock(minecraft: &Path, temp_dir: &Path) {
    let java_sounds_dir = minecraft.join("sounds");
    let bedrock_sounds = temp_dir.join("sounds");
    if java_sounds_dir.exists() {
        let _ = fs::create_dir_all(&bedrock_sounds);
        if merge_dir(&java_sounds_dir, &bedrock_sounds).is_ok() {
            remove_dir_quiet(&java_sounds_dir);
            log_info!("OKAY bedrock [assets/sounds -> sounds/]");
        }
    }

    let sounds_json = minecraft.join("sounds.json");
    if !sounds_json.exists() {
        return;
    }
    let Ok(raw) = fs::read_to_string(&sounds_json) else { return };
    let Ok(map) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&raw) else {
        return;
    };
    let mut defs = serde_json::Map::new();
    for (event, val) in map {
        let category = val
            .get("category")
            .and_then(|c| c.as_str())
            .unwrap_or("block")
            .to_string();
        let mut sounds = Vec::new();
        if let Some(arr) = val.get("sounds").and_then(|s| s.as_array()) {
            for s in arr {
                let name = if let Some(obj) = s.as_object() {
                    obj.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string()
                } else {
                    s.as_str().unwrap_or("").to_string()
                };
                if name.is_empty() {
                    continue;
                }
                let path = if name.starts_with("sounds/") {
                    name
                } else {
                    format!("sounds/{}", name.trim_start_matches('/'))
                };
                sounds.push(serde_json::json!({ "name": path, "stream": false }));
            }
        }
        if sounds.is_empty() {
            continue;
        }
        defs.insert(event, serde_json::json!({ "category": category, "sounds": sounds }));
    }
    if defs.is_empty() {
        return;
    }
    let _ = fs::create_dir_all(&bedrock_sounds);
    let doc = serde_json::json!({
        "format_version": "1.14.0",
        "sound_definitions": defs
    });
    if let Ok(pretty) = serde_json::to_string_pretty(&doc) {
        let _ = fs::write(bedrock_sounds.join("sound_definitions.json"), &pretty);
        // 包根 sounds.json：与 definitions 同结构（部分加载路径读根目录）
        let _ = fs::write(temp_dir.join("sounds.json"), &pretty);
        log_info!("OKAY bedrock [sounds.json -> sound_definitions.json + root sounds.json]");
    }
    remove_file_quiet(&sounds_json);
}

pub fn convert_sounds_bedrock_to_java(temp_dir: &Path, minecraft: &Path) {
    let bedrock_sounds = temp_dir.join("sounds");
    if bedrock_sounds.exists() {
        let java_sounds = minecraft.join("sounds");
        let _ = fs::create_dir_all(&java_sounds);
        let Ok(entries) = fs::read_dir(&bedrock_sounds) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            if name == "sound_definitions.json" {
                continue;
            }
            let dst = java_sounds.join(&name);
            if path.is_dir() {
                let _ = merge_dir(&path, &dst);
            } else {
                let _ = fs::copy(&path, &dst);
            }
        }
        log_info!("OKAY java [sounds/ -> assets/minecraft/sounds/]");
    }

    // 优先读包根 sounds.json；解析失败则回退 sounds/sound_definitions.json
    let root_sounds = temp_dir.join("sounds.json");
    let defs_sounds = bedrock_sounds.join("sound_definitions.json");
    let mut raw = None;
    for candidate in [&root_sounds, &defs_sounds] {
        if !candidate.is_file() {
            continue;
        }
        if let Ok(s) = fs::read_to_string(candidate) {
            if serde_json::from_str::<serde_json::Value>(&s).is_ok() {
                raw = Some(s);
                break;
            }
        }
    }
    let Some(raw) = raw else {
        remove_dir_quiet(&bedrock_sounds);
        remove_file_quiet(&root_sounds);
        return;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
        remove_dir_quiet(&bedrock_sounds);
        remove_file_quiet(&root_sounds);
        return;
    };
    // 兼容：根文件可能是 {sound_definitions:{...}} 或直接 event map
    let defs_obj: Option<&serde_json::Map<String, serde_json::Value>> = v
        .get("sound_definitions")
        .and_then(|d| d.as_object())
        .or_else(|| v.as_object());
    let Some(defs) = defs_obj else {
        remove_dir_quiet(&bedrock_sounds);
        remove_file_quiet(&root_sounds);
        return;
    };
    let mut sounds_json = serde_json::Map::new();
    for (event, def) in defs {
        if event == "format_version" {
            continue;
        }
        let category = def
            .get("category")
            .and_then(|c| c.as_str())
            .unwrap_or("block")
            .to_string();
        let mut list = Vec::new();
        if let Some(arr) = def.get("sounds").and_then(|s| s.as_array()) {
            for s in arr {
                let name = if let Some(obj) = s.as_object() {
                    obj.get("name").and_then(|n| n.as_str()).unwrap_or("")
                } else {
                    s.as_str().unwrap_or("")
                };
                let rel = name.strip_prefix("sounds/").unwrap_or(name);
                if !rel.is_empty() {
                    list.push(serde_json::Value::String(rel.to_string()));
                }
            }
        }
        if list.is_empty() {
            continue;
        }
        sounds_json.insert(
            event.clone(),
            serde_json::json!({ "category": category, "sounds": list }),
        );
    }
    if !sounds_json.is_empty() {
        if let Ok(pretty) = serde_json::to_string_pretty(&serde_json::Value::Object(sounds_json)) {
            let _ = fs::write(minecraft.join("sounds.json"), pretty);
            log_info!("OKAY java [sound_definitions.json -> sounds.json]");
        }
    }
    remove_dir_quiet(&bedrock_sounds);
    remove_file_quiet(&root_sounds);
}

pub fn strip_java_only(minecraft: &Path, temp_dir: &Path) {
    // 只删 Java 定义目录；不要删 textures/models（盔甲穿戴层在 Bedrock 仍用）
    let java_dirs = [
        "blockstates",
        "models",
        "shaders",
        "atlases",
        "particles",
        "equipment",
        "items",
        "font",
        "post_effect",
        "waypoint_style",
        "optifine",
        "mcpatcher",
        "cit",
        "emissive",
    ];
    for d in java_dirs {
        remove_dir_quiet(&minecraft.join(d));
    }
    // textures 下仅剥离明确的 Java 专用子目录
    for d in ["optifine", "mcpatcher", "cit", "emissive"] {
        remove_dir_quiet(&minecraft.join("textures").join(d));
    }
    remove_dir_quiet(&temp_dir.join("optifine"));
    remove_dir_quiet(&temp_dir.join("mcpatcher"));
    remove_file_quiet(&minecraft.join("sounds.json"));
    remove_file_quiet(&minecraft.join("gpu_warnlist.json"));
    remove_file_quiet(&minecraft.join("regional_compliancies.json"));
}

pub fn strip_bedrock_only(temp_dir: &Path, minecraft: &Path) {
    for f in [
        "manifest.json",
        "textures_list.json",
        "terrain_texture.json",
        "item_texture.json",
        "blocks.json",
        "biomes_client.json",
        "splashes.json",
        "bug_pack_icon.png",
    ] {
        remove_file_quiet(&temp_dir.join(f));
    }
    if temp_dir.join("pack.png").exists() {
        remove_file_quiet(&temp_dir.join("pack_icon.png"));
    }
    let tex = minecraft.join("textures");
    for f in [
        "flipbook_textures.json",
        "textures_list.json",
        "terrain_texture.json",
        "item_texture.json",
    ] {
        remove_file_quiet(&temp_dir.join("textures").join(f));
        remove_file_quiet(&tex.join(f));
    }
    remove_file_quiet(&temp_dir.join("sounds.json"));
    remove_file_quiet(&temp_dir.join("sounds").join("sound_definitions.json"));
    for d in [
        "animation_controllers",
        "animations",
        "attachables",
        "biomes",
        "block_culling",
        "entity",
        "fogs",
        "items",
        "materials",
        "models",
        "particles",
        "render_controllers",
        "atmospherics",
        "color_grading",
        "cubemaps",
        "lighting",
        "local_lighting",
        "shadows",
        "pbr",
        "water",
    ] {
        remove_dir_quiet(&temp_dir.join(d));
    }
    remove_file_quiet(&temp_dir.join("texts").join("languages.json"));
    remove_file_quiet(&temp_dir.join("texts").join("language_names.json"));
    log_info!("OKAY java [strip bedrock-only]");
}

pub fn cleanup_empty_assets(temp_dir: &Path) {
    let assets = temp_dir.join("assets");
    if !assets.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(&assets) else { return };
    let mut only_minecraft = true;
    for entry in entries.flatten() {
        if entry.file_name() != "minecraft" {
            only_minecraft = false;
            break;
        }
    }
    if only_minecraft {
        remove_dir_quiet(&assets.join("minecraft"));
        let _ = fs::remove_dir(&assets);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_is_bedrock_resource_pack() {
        let temp = tempdir().unwrap();
        assert!(!is_bedrock_resource_pack(temp.path()));
        fs::write(
            temp.path().join("manifest.json"),
            r#"{"modules":[{"type":"resources"}]}"#,
        )
        .unwrap();
        assert!(is_bedrock_resource_pack(temp.path()));
    }

    #[test]
    fn test_manifest_and_mcmeta_roundtrip_fields() {
        let temp = tempdir().unwrap();
        write_manifest(temp.path(), "包名", "描述").unwrap();
        let (n, d) = read_manifest_info(temp.path());
        assert_eq!(n, "包名");
        assert_eq!(d, "描述");
        let v: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(temp.path().join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(v["header"]["min_engine_version"], serde_json::json!([1, 20, 0]));
        assert!(v["metadata"]["generated_with"]["2-Pyramid"].is_array());

        write_pack_mcmeta(temp.path(), 75, "描述2").unwrap();
        assert_eq!(read_description_from_mcmeta(temp.path()), "描述2");
    }

    #[test]
    fn test_lang_java_bedrock_roundtrip() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        let mc = root.join("assets/minecraft");
        fs::create_dir_all(mc.join("lang")).unwrap();
        fs::write(
            mc.join("lang/zh_cn.json"),
            r#"{"block.minecraft.stone":"石头"}"#,
        )
        .unwrap();
        convert_lang_java_to_bedrock(&mc, root);
        assert!(root.join("texts/zh_CN.lang").exists());

        // wipe java lang, convert back
        let _ = fs::remove_dir_all(mc.join("lang"));
        convert_lang_bedrock_to_java(root, &mc);
        let back: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(mc.join("lang/zh_cn.json")).unwrap()).unwrap();
        assert_eq!(back["block.minecraft.stone"], "石头");
    }

    #[test]
    fn test_sounds_java_bedrock_roundtrip() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        let mc = root.join("assets/minecraft");
        fs::create_dir_all(mc.join("sounds/ambient/cave")).unwrap();
        fs::write(mc.join("sounds/ambient/cave/cave1.ogg"), b"o").unwrap();
        fs::write(
            mc.join("sounds.json"),
            r#"{"ambient.cave":{"category":"ambient","sounds":["ambient/cave/cave1"]}}"#,
        )
        .unwrap();
        convert_sounds_java_to_bedrock(&mc, root);
        assert!(root.join("sounds/sound_definitions.json").exists());
        assert!(root.join("sounds.json").exists(), "根 sounds.json 双写");
        assert!(root.join("sounds/ambient/cave/cave1.ogg").exists());

        let _ = fs::remove_dir_all(mc.join("sounds"));
        let _ = fs::remove_file(mc.join("sounds.json"));
        // 根 sounds.json 仍在：b2j 应优先读它
        convert_sounds_bedrock_to_java(root, &mc);
        let sj: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(mc.join("sounds.json")).unwrap()).unwrap();
        assert_eq!(sj["ambient.cave"]["sounds"][0], "ambient/cave/cave1");
        assert!(mc.join("sounds/ambient/cave/cave1.ogg").exists());
        assert!(!root.join("sounds.json").exists());
    }
}
