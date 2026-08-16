use std::fs;
use std::path::{Path, PathBuf};

use chrono::Local;
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::{json, Value};
use walkdir::WalkDir;

use crate::converters::zip::{extract_resource_pack, repack_resource_pack};
use crate::{log_info, log_warn};

const PACK_FORMAT_LABELS: [&str; 26] = [
    "Java 1.6-1.8",
    "Java 1.9-1.10",
    "Java 1.11-1.12",
    "Java 1.13-1.14",
    "Java 1.15-1.16.1",
    "Java 1.16.2-1.16.5",
    "Java 1.17",
    "Java 1.18",
    "Java 1.19-1.19.2",
    "Java 1.19.3",
    "Java 1.19.4",
    "Java 1.20-1.20.1",
    "Java 1.20.2",
    "Java 1.20.3-1.20.4",
    "Java 1.20.5-1.20.6",
    "Java 1.21-1.21.1",
    "Java 1.21.2-1.21.3",
    "Java 1.21.4",
    "Java 1.21.5",
    "Java 1.21.6",
    "Java 1.21.7-1.21.8",
    "Java 1.21.9-1.21.10",
    "Java 1.21.11",
    "Java 26.1-26.1.2",
    "Java 26.2",
    "Bedrock Latest",
];

fn pack_format_label(pack_format: u32) -> &'static str {
    match pack_format {
        1 => "Java 1.6-1.8",
        2 => "Java 1.9-1.10",
        3 => "Java 1.11-1.12",
        4 => "Java 1.13-1.14",
        5 => "Java 1.15-1.16.1",
        6 => "Java 1.16.2-1.16.5",
        7 => "Java 1.17",
        8 => "Java 1.18",
        9 => "Java 1.19-1.19.2",
        12 => "Java 1.19.3",
        13 => "Java 1.19.4",
        15 => "Java 1.20-1.20.1",
        18 => "Java 1.20.2",
        22 => "Java 1.20.3-1.20.4",
        32 => "Java 1.20.5-1.20.6",
        34 => "Java 1.21-1.21.1",
        42 => "Java 1.21.2-1.21.3",
        46 => "Java 1.21.4",
        55 => "Java 1.21.5",
        63 => "Java 1.21.6",
        64 => "Java 1.21.7-1.21.8",
        69 => "Java 1.21.9-1.21.10",
        75 => "Java 1.21.11",
        84 => "Java 26.1-26.1.2",
        88 => "Java 26.2",
        1000 => "Bedrock Latest",
        _ => "Unknown",
    }
}

pub fn pack_format_label_for_output(pack_format: u32) -> &'static str {
    pack_format_label(pack_format)
}

lazy_static! {
    static ref VERSION_PREFIX_RE: Regex = {
        let patterns = PACK_FORMAT_LABELS
            .iter()
            .map(|label| regex::escape(label))
            .collect::<Vec<_>>()
            .join("|");
        Regex::new(&format!(r"^\[({})\]", patterns)).unwrap()
    };
}

fn strip_version_prefix(name: &str) -> String {
    let cleaned = VERSION_PREFIX_RE.replace(name, "");
    cleaned.trim().to_string()
}

fn read_text_with_fallback(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    match String::from_utf8(bytes.clone()) {
        Ok(text) => Ok(text),
        Err(_) => Ok(bytes.iter().map(|&b| b as char).collect()),
    }
}

fn find_pack_mcmeta(temp_dir: &Path) -> Option<PathBuf> {
    let direct = temp_dir.join("pack.mcmeta");
    if direct.exists() {
        return Some(direct);
    }

    for entry in WalkDir::new(temp_dir).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() && entry.file_name().to_string_lossy().eq_ignore_ascii_case("pack.mcmeta") {
            return Some(entry.into_path());
        }
    }

    None
}

fn read_pack_format(pack_meta_path: &Path) -> Result<u32, String> {
    let content = read_text_with_fallback(pack_meta_path)?;
    let data: Value = serde_json::from_str(&content)
        .map_err(|e| format!("failed to parse {}: {}", pack_meta_path.display(), e))?;

    let pack_value = data.get("pack").and_then(|p| p.get("pack_format"));

    if let Some(value) = pack_value {
        if let Some(num) = value.as_u64() {
            return Ok(num as u32);
        }
        if let Some(text) = value.as_str() {
            if let Ok(num) = text.parse::<u32>() {
                return Ok(num);
            }
        }
    }

    Ok(1)
}

fn normalize_description(value: &Value) -> String {
    let raw = match value {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::String(text) => text.clone(),
                _ => item.to_string(),
            })
            .collect::<Vec<_>>()
            .join(" "),
        _ => value.to_string(),
    };

    raw.replace('\n', " ").replace('\r', " ")
}

fn write_pack_format(pack_meta_path: &Path, target_version: u32) -> Result<(), String> {
    let content = if pack_meta_path.exists() {
        read_text_with_fallback(pack_meta_path)?
    } else {
        String::new()
    };

    let mut data: Value = if content.trim().is_empty() {
        json!({"pack": {"pack_format": target_version, "description": "Converted by 2-Pyramid"}})
    } else {
        serde_json::from_str(&content).unwrap_or_else(|_| {
            json!({"pack": {"pack_format": target_version, "description": "Converted by 2-Pyramid"}})
        })
    };

    let description = data
        .get("pack")
        .and_then(|pack| pack.get("description"))
        .map(normalize_description)
        .unwrap_or_else(|| "Converted by 2-Pyramid".to_string());

    if target_version >= 69 {
        data = json!({
            "pack": {
                "pack_format": 34,
                "supported_formats": [34, target_version],
                "min_format": [34, 0],
                "max_format": [target_version, 0],
                "description": description
            }
        });
    } else {
        if data.get("pack").is_none() {
            data["pack"] = json!({});
        }
        data["pack"]["pack_format"] = json!(target_version);
        data["pack"]["description"] = json!(description);
    }

    let pretty = serde_json::to_string_pretty(&data)
        .map_err(|e| format!("failed to serialize pack.mcmeta: {}", e))?;
    fs::write(pack_meta_path, pretty)
        .map_err(|e| format!("failed to write {}: {}", pack_meta_path.display(), e))
}

pub fn build_output_path_for_batch(
    input_zip: &Path,
    target_version: u32,
    parent_folder_path: Option<&str>,
    output_dir_override: Option<&str>,
) -> Result<PathBuf, String> {
    build_output_path(input_zip, target_version, parent_folder_path, output_dir_override)
}

pub fn build_output_path(
    input_zip: &Path,
    target_version: u32,
    parent_folder_path: Option<&str>,
    output_dir_override: Option<&str>,
) -> Result<PathBuf, String> {
    let label = pack_format_label(target_version);
    let base_name = input_zip
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("resource_pack");
    let cleaned_base = strip_version_prefix(base_name);

    // Naming style is user-configurable:
    //   * "default"   → `[tag]name.zip`, numeric suffix on collision
    //   * "timestamp" → `[tag]name_YYYYMMDD-HHMMSS.zip`
    //   * "overwrite" → `[tag]name.zip`, replaces an existing file
    let naming = crate::commands::read_config_file()
        .ok()
        .and_then(|c| c.output_naming)
        .unwrap_or_else(|| "default".to_string());

    let mut file_name = match naming.as_str() {
        "timestamp" => {
            let stamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
            format!("[{}]{}_{}.zip", label, cleaned_base, stamp)
        }
        _ => format!("[{}]{}.zip", label, cleaned_base),
    };

    let output_dir = if let Some(override_dir) = output_dir_override {
        Path::new(override_dir).to_path_buf()
    } else if let Some(parent_folder) = parent_folder_path {
        let parent_path = Path::new(parent_folder);
        let parent_name = parent_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("resource_packs");
        let cleaned_parent = strip_version_prefix(parent_name);
        let grandparent = parent_path.parent().unwrap_or(parent_path);
        grandparent.join(format!("[{}]{}", label, cleaned_parent))
    } else {
        input_zip.parent().unwrap_or_else(|| Path::new(".")).to_path_buf()
    };

    fs::create_dir_all(&output_dir)
        .map_err(|e| format!("failed to create output directory {}: {}", output_dir.display(), e))?;

    let mut output_path = output_dir.join(&file_name);
    // "overwrite" replaces an existing file; the other styles never
    // clobber — append a numeric suffix until the path is free.
    if naming != "overwrite" {
        let mut counter = 1;
        while output_path.exists() {
            file_name = format!("[{}]{}_{}.zip", label, cleaned_base, counter);
            output_path = output_dir.join(&file_name);
            counter += 1;
        }
    }

    Ok(output_path)
}

pub fn process_extracted_dir_only(
    input_zip: &Path,
    temp_dir: &Path,
    target_version: u32,
) -> Result<(), String> {
    let pack_meta_path = find_pack_mcmeta(temp_dir).unwrap_or_else(|| temp_dir.join("pack.mcmeta"));
    let source_version = read_pack_format(&pack_meta_path).unwrap_or(1);

    crate::invoke_conversion::invoke_conversion(
        input_zip,
        temp_dir,
        target_version,
        source_version,
    )
    .map_err(|e| format!("conversion pipeline failed: {}", e))?;

    write_pack_format(&pack_meta_path, target_version)?;

    Ok(())
}

pub fn convert_resource_pack(file_path: &str, target_version: u32) -> Result<String, String> {
    process_zip(file_path, target_version, None, 1.0, None, None)
}

pub fn process_zip(
    original_file_path: &str,
    pack_format2: u32,
    _progress_callback: Option<fn(f64, &str)>,
    _file_weight: f64,
    parent_folder_path: Option<&str>,
    output_dir_override: Option<&str>,
) -> Result<String, String> {
    let input_zip = Path::new(original_file_path);
    if !input_zip.exists() {
        return Err(format!("input file not found: {}", input_zip.display()));
    }

    if pack_format2 == 1000 {
        return Err("Bedrock conversion is not implemented in Rust yet. Convert to pack_format 75 first.".to_string());
    }

    let temp_dir = tempfile::tempdir().map_err(|e| format!("failed to create temp dir: {}", e))?;
    let temp_dir_path = temp_dir.path().to_string_lossy().to_string();

    extract_resource_pack(original_file_path, &temp_dir_path)?;

    let pack_meta_path = find_pack_mcmeta(temp_dir.path()).unwrap_or_else(|| temp_dir.path().join("pack.mcmeta"));
    if !pack_meta_path.exists() {
        log_warn!("pack.mcmeta not found, creating a new one at {}", pack_meta_path.display());
    }

    let source_version = read_pack_format(&pack_meta_path).unwrap_or(1);
    log_info!("detected pack_format: {}", source_version);

    // Always run conversion pipeline regardless of version difference
    // This ensures all conversion tasks are executed even if version hasn't changed
    crate::invoke_conversion::invoke_conversion(
        input_zip,
        temp_dir.path(),
        pack_format2,
        source_version,
    )
    .map_err(|e| format!("conversion pipeline failed: {}", e))?;

    write_pack_format(&pack_meta_path, pack_format2)?;

    let output_path = build_output_path(input_zip, pack_format2, parent_folder_path, output_dir_override)?;

    repack_resource_pack(&temp_dir_path, &output_path.to_string_lossy())?;

    if !output_path.exists() {
        return Err(format!("output file not found after repack: {}", output_path.display()));
    }

    Ok(output_path.to_string_lossy().to_string())
}

pub fn register_task(_engine: &mut crate::hurray::engine::HurrayEngine) {
    // Not a standalone task: this module orchestrates end-to-end ZIP conversion.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// End-to-end smoke test: copy the Pika 5K 16x resource pack's container
    /// folder into a tempdir, run the full conversion pipeline targeting
    /// pack_format 34 (1.21), and verify the result has the expected
    /// structure (sprite atlas files exist, pack.mcmeta updated, container
    /// files modified).
    #[test]
    fn test_full_pipeline_pika5k_to_121() {
        let rp_root = r"D:\GameTime\.minecraft\versions\1.20.1-OptiFine_I6\resourcepacks\!   §1§b§lPika 5K - 16x";
        if !std::path::Path::new(rp_root).exists() {
            eprintln!("SKIP: Pika 5K not found at {}", rp_root);
            return;
        }

        let temp = tempdir().expect("tempdir");
        let src_container = std::path::Path::new(rp_root)
            .join("assets/minecraft/textures/gui/container");
        let dst_container = temp.path().join("assets/minecraft/textures/gui/container");
        fs::create_dir_all(&dst_container).expect("mkdir container");
        for entry in fs::read_dir(&src_container).expect("read src") {
            let entry = entry.expect("entry");
            let ft = entry.file_type().expect("ft");
            if ft.is_file()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".png")
            {
                fs::copy(entry.path(), dst_container.join(entry.file_name()))
                    .expect("copy");
            }
        }

        // Drop a pack.mcmeta declaring pack_format=6 (1.16-1.20 era)
        let mcmeta = temp.path().join("pack.mcmeta");
        fs::write(
            &mcmeta,
            r#"{"pack":{"pack_format":6,"description":"Pika 5K - 16x (test fixture)"}}"#,
        )
        .expect("write mcmeta");

        let src_zip = std::path::Path::new(rp_root).join("pack.mcmeta");
        if let Err(e) = process_extracted_dir_only(&src_zip, temp.path(), 34) {
            panic!("process_extracted_dir_only failed: {}", e);
        }

        // 1. pack.mcmeta updated
        let new_mcmeta = fs::read_to_string(&mcmeta).expect("read mcmeta");
        assert!(
            new_mcmeta.contains("pack_format") && new_mcmeta.contains(": 34"),
            "pack.mcmeta should be updated to pack_format 34: {}",
            new_mcmeta
        );
        assert!(
            new_mcmeta.contains("Pika 5K"),
            "original description should be preserved: {}",
            new_mcmeta
        );

        // 2. Sprite atlas populated by cut_gui / GuiSurgeon. The
        //    container-level smithing.png / cartography_table.png were
        //    generated by fix_smithing2_villager2_ui / fix_machinery_ui
        //    EARLIER in the pipeline, then consumed by cut_gui to make
        //    sprites, then DELETED by the deferred cleanup step (1.21
        //    sprite mode doesn't keep the legacy atlas PNGs). The fact
        //    that they were deleted is itself a sign the pipeline ran end
        //    to end. Verify the resulting sprites instead.
        let sprites = temp
            .path()
            .join("assets/minecraft/textures/gui/sprites/container");
        assert!(
            sprites.join("smithing/template_slot.png").exists(),
            "smithing/template_slot.png must be produced by cut_gui from fix_smithing2_villager2_ui's smithing.png"
        );
        assert!(
            sprites.join("smithing/error.png").exists(),
            "smithing/error.png must be produced by cut_gui"
        );
        assert!(
            sprites.join("cartography_table/duplicated_map.png").exists(),
            "cartography_table/duplicated_map.png must be produced by cut_gui from fix_machinery_ui's cartography_table.png"
        );
        assert!(
            sprites.join("grindstone/input_slot.png").exists(),
            "grindstone/input_slot.png must be produced by cut_gui from fix_machinery_ui's grindstone.png"
        );

        // 3. Confirm the deferred cleanup actually removed legacy atlas PNGs.
        //    If cut_gui deferred them but cleanup didn't run, the legacy
        //    PNGs would still be present.
        assert!(
            !dst_container.join("smithing.png").exists(),
            "smithing.png should have been cleaned up after cut_gui"
        );
        assert!(
            !dst_container.join("anvil.png").exists(),
            "anvil.png should have been cleaned up after cut_gui"
        );
    }
}
