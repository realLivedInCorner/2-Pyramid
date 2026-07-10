use std::path::{Path, PathBuf};
use std::fs;
use std::io::{self, Read, Write};
use uuid::Uuid;
use base64::{Engine as _, engine::general_purpose};
use flate2::write::ZlibEncoder;
use flate2::read::ZlibDecoder;
use flate2::Compression;
use zip::ZipArchive;
use zip::write::FileOptions;

use crate::overlay::{overlay_config_path, overlay_templates_dir_from_app, overlay_temp_dir, copy_dir_all};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OverlayProject {
    pub id: String,
    pub name: String,
    pub parent_pack_path: String,
    pub updated_at: i64,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct OverlayConfig {
    pub projects: Vec<OverlayProject>,
    pub settings: Option<serde_json::Value>,
}

#[derive(serde::Deserialize)]
pub struct OverlayInitRequest {
    pub name: String,
    #[serde(alias = "parentPackPath")]
    pub parent_pack_path: Option<String>,
}

fn write_overlay_config(cfg: &OverlayConfig) -> Result<(), String> {
    let path = overlay_config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    let content = serde_json::to_string_pretty(cfg).map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Failed to write config: {}", e))?;
    Ok(())
}

fn read_overlay_config() -> Result<OverlayConfig, String> {
    let path = overlay_config_path()?;
    if !path.exists() {
        return Ok(OverlayConfig::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("Failed to read config: {}", e))?;
    let cfg: OverlayConfig = serde_json::from_str(&content).unwrap_or_default();
    Ok(cfg)
}

fn create_default_overlay_structure(temp_dir: &PathBuf) -> Result<(), String> {
    let folders = [
        temp_dir.join("assets").join("minecraft").join("models"),
        temp_dir.join("assets").join("minecraft").join("shaders").join("core"),
        temp_dir.join("assets").join("minecraft").join("lang"),
        temp_dir.join("assets").join("minecraft").join("textures").join("models").join("item"),
        temp_dir.join("assets").join("minecraft").join("textures").join("misc"),
    ];
    for folder in folders.iter() {
        fs::create_dir_all(folder).map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    Ok(())
}

fn extract_zip_file(zip_path: &PathBuf, dest_dir: &PathBuf) -> Result<(), String> {
    crate::converters::zip::extract_zip_to_dir(zip_path, dest_dir)
}

#[tauri::command]
pub fn get_overlay_projects() -> Result<Vec<OverlayProject>, String> {
    let cfg = read_overlay_config()?;
    Ok(cfg.projects)
}

#[tauri::command]
pub fn delete_overlay_project(id: String) -> Result<(), String> {
    let mut cfg = read_overlay_config()?;
    let project = cfg.projects.iter().find(|p| p.id == id).cloned();

    cfg.projects.retain(|p| p.id != id);
    write_overlay_config(&cfg)?;

    if let Some(p) = project {
        let temp_dir = overlay_temp_dir(Some(&p.name))?;
        if temp_dir.exists() {
            let _ = fs::remove_dir_all(temp_dir);
        }
    }

    Ok(())
}

#[tauri::command]
pub fn overlay_init(request: OverlayInitRequest) -> Result<OverlayProject, String> {
    let mut cfg = read_overlay_config()?;
    let name = request.name.trim().to_string();

    if name.is_empty() {
        return Err("Project name cannot be empty".to_string());
    }

    if cfg.projects.iter().any(|p| p.name == name) {
        return Err(format!("Project name '{}' already exists", name));
    }

    let temp_dir = overlay_temp_dir(Some(&name))?;

    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).map_err(|e| format!("Failed to clean overlay project directory: {}", e))?;
    }
    fs::create_dir_all(&temp_dir).map_err(|e| format!("Failed to create overlay project directory: {}", e))?;

    create_default_overlay_structure(&temp_dir)?;

    let project_id = Uuid::new_v4().to_string();
    let new_project = OverlayProject {
        id: project_id,
        name: name.clone(),
        parent_pack_path: request.parent_pack_path.unwrap_or_default(),
        updated_at: chrono::Utc::now().timestamp_millis(),
    };

    cfg.projects.push(new_project.clone());
    write_overlay_config(&cfg)?;

    Ok(new_project)
}

#[tauri::command]
pub fn import_lang_from_parent(project_name: String, lang_code: String) -> Result<serde_json::Value, String> {
    let cfg = read_overlay_config()?;
    let project = cfg.projects.iter().find(|p| p.name == project_name).ok_or("Project not found")?;
    let parent_path = &project.parent_pack_path;

    if parent_path.is_empty() {
        return Err("No parent pack selected".to_string());
    }

    let path = Path::new(parent_path);
    if !path.exists() {
        return Err("Parent pack file does not exist".to_string());
    }

    let file = fs::File::open(path).map_err(|e| format!("Failed to open parent pack: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Failed to read parent pack archive: {}", e))?;

    let target_file = format!("assets/minecraft/lang/{}.json", lang_code);
    let mut file = archive.by_name(&target_file).map_err(|_| format!("Language file not found in parent pack: {}", target_file))?;

    let mut content = String::new();
    file.read_to_string(&mut content).map_err(|e| format!("Failed to read language file: {}", e))?;

    let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| format!("Failed to parse language file: {}", e))?;

    let temp_dir = overlay_temp_dir(Some(&project_name))?;
    let lang_dir = temp_dir.join("assets").join("minecraft").join("lang");
    fs::create_dir_all(&lang_dir).map_err(|e| format!("Failed to create directory: {}", e))?;
    fs::write(lang_dir.join(format!("{}.json", lang_code)), &content).map_err(|e| format!("Failed to save language file: {}", e))?;

    Ok(json)
}

#[tauri::command]
pub fn get_overlay_lang() -> Result<serde_json::Value, String> {
    let cfg = read_overlay_config()?;
    let settings = cfg.settings.unwrap_or_else(|| serde_json::json!({}));
    Ok(settings.get("lang").cloned().unwrap_or_else(|| serde_json::json!("zh_cn")))
}

#[tauri::command]
pub fn save_overlay_lang(lang: String) -> Result<serde_json::Value, String> {
    let mut cfg = read_overlay_config()?;
    let mut settings = cfg.settings.unwrap_or_else(|| serde_json::json!({}));
    if let serde_json::Value::Object(ref mut map) = settings {
        map.insert("lang".to_string(), serde_json::Value::String(lang));
    }
    cfg.settings = Some(settings);
    write_overlay_config(&cfg)?;
    serde_json::to_value(cfg).map_err(|e| format!("Failed to return config: {}", e))
}

#[tauri::command]
pub fn read_lang_file(app: tauri::AppHandle, lang_code: String) -> Result<serde_json::Value, String> {
    let overlay_dir = overlay_templates_dir_from_app(&app)?;
    let lang_file = overlay_dir.join("lang").join(format!("{}.json", lang_code));

    crate::log_info!("Attempting to read language file: {}", lang_file.display());

    if !lang_file.exists() {
        return Err(format!("Language file does not exist: {}", lang_file.display()));
    }

    let content = fs::read_to_string(&lang_file).map_err(|e| format!("Failed to read language file: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| format!("Failed to parse language file: {}", e))?;
    Ok(json)
}

#[tauri::command]
pub fn get_overlay_json(project_name: String) -> Result<serde_json::Value, String> {
    let temp_dir = overlay_temp_dir(Some(&project_name))?;
    let overlay_json = temp_dir.join("overlay.json");
    if !overlay_json.exists() {
        return Ok(serde_json::json!({}));
    }
    let content = fs::read_to_string(&overlay_json).map_err(|e| format!("Failed to read overlay.json: {}", e))?;
    let data: serde_json::Value = serde_json::from_str(&content).map_err(|e| format!("Failed to parse overlay.json: {}", e))?;
    Ok(data)
}

#[tauri::command]
pub fn save_overlay_json(project_name: String, data: serde_json::Value) -> Result<(), String> {
    let temp_dir = overlay_temp_dir(Some(&project_name))?;
    if !temp_dir.exists() {
        fs::create_dir_all(&temp_dir).map_err(|e| format!("Failed to create temp project directory: {}", e))?;
    }
    let overlay_json = temp_dir.join("overlay.json");
    let content = serde_json::to_string_pretty(&data).map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(&overlay_json, content).map_err(|e| format!("Failed to write overlay.json: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn export_overlay_share_code(project_name: String) -> Result<String, String> {
    let temp_dir = overlay_temp_dir(Some(&project_name))?;
    let overlay_json_path = temp_dir.join("overlay.json");

    let data = if overlay_json_path.exists() {
        let content = fs::read_to_string(&overlay_json_path).map_err(|e| format!("Failed to read config: {}", e))?;
        serde_json::from_str::<serde_json::Value>(&content).map_err(|e| format!("Failed to parse config: {}", e))?
    } else {
        serde_json::json!({})
    };

    let share_data = serde_json::json!({
        "version": "1.0",
        "name": project_name,
        "config": data
    });

    let json_str = serde_json::to_string(&share_data).map_err(|e| format!("Failed to serialize: {}", e))?;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(json_str.as_bytes()).map_err(|e| format!("Compression failed: {}", e))?;
    let compressed = encoder.finish().map_err(|e| format!("Failed to finish compression: {}", e))?;

    let code = general_purpose::STANDARD.encode(compressed);
    Ok(format!("HRCN-{}", code))
}

#[tauri::command]
pub fn import_overlay_share_code(share_code: String) -> Result<OverlayProject, String> {
    if !share_code.starts_with("HRCN-") {
        return Err("Invalid share code format".to_string());
    }

    let code = &share_code[5..];

    let compressed = general_purpose::STANDARD.decode(code).map_err(|e| format!("Decode failed: {}", e))?;

    let mut decoder = ZlibDecoder::new(&compressed[..]);
    let mut json_str = String::new();
    decoder.read_to_string(&mut json_str).map_err(|e| format!("Decompress failed: {}", e))?;

    let share_data: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse data: {}", e))?;

    let name = share_data["name"].as_str().ok_or("Project name missing from share code")?.to_string();
    let config = share_data["config"].clone();

    let mut cfg = read_overlay_config()?;
    let mut final_name = name.clone();
    let mut counter = 1;

    while cfg.projects.iter().any(|p| p.name == final_name) {
        final_name = format!("{}_{}", name, counter);
        counter += 1;
    }

    let temp_dir = overlay_temp_dir(Some(&final_name))?;
    fs::create_dir_all(&temp_dir).map_err(|e| format!("Failed to create directory: {}", e))?;
    create_default_overlay_structure(&temp_dir)?;

    let overlay_json = temp_dir.join("overlay.json");
    let content = serde_json::to_string_pretty(&config).map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(&overlay_json, content).map_err(|e| format!("Failed to save config: {}", e))?;

    let project_id = Uuid::new_v4().to_string();
    let new_project = OverlayProject {
        id: project_id,
        name: final_name,
        parent_pack_path: "".to_string(),
        updated_at: chrono::Utc::now().timestamp_millis(),
    };

    cfg.projects.push(new_project.clone());
    write_overlay_config(&cfg)?;

    Ok(new_project)
}

#[tauri::command]
pub fn overlay_package(app: tauri::AppHandle, project_name: String) -> Result<String, String> {
    let temp_dir = overlay_temp_dir(Some(&project_name))?;
    let name = project_name.clone();

    if !temp_dir.exists() {
        return Err("Overlay project directory does not exist".to_string());
    }

    let overlay_dir = overlay_templates_dir_from_app(&app)?;
    apply_overlay_changes(&project_name, &overlay_dir)?;

    let overlay_json_path = temp_dir.join("overlay.json");
    let workspace_root = if overlay_json_path.exists() {
        let content = fs::read_to_string(&overlay_json_path).unwrap_or_default();
        let config: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
        config
            .get("workspace")
            .and_then(|w| w.get("path"))
            .and_then(|p| p.as_str())
            .map(|s| PathBuf::from(s))
    } else {
        None
    };

    if let Some(ws_root) = workspace_root {
        if !ws_root.exists() {
            return Err("Parent pack workspace directory does not exist".to_string());
        }

        let original_name = ws_root.file_name().and_then(|n| n.to_str()).unwrap_or(&name);
        let output_name = format!("[Overlay]{}.zip", original_name);
        let output_path = temp_dir.join(&output_name);

        crate::log_info!("packaging from workspace: {} -> {}", ws_root.display(), output_path.display());

        let file = fs::File::create(&output_path).map_err(|e| format!("Failed to create archive: {}", e))?;
        let mut zip = zip::ZipWriter::new(file);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let mut buffer = Vec::new();

        for entry in walkdir::WalkDir::new(&ws_root) {
            let entry = entry.map_err(|e| format!("Failed to walk workspace: {}", e))?;
            let path = entry.path();
            if path.is_dir() {
                continue;
            }
            let name_in_zip = path.strip_prefix(ws_root.parent().unwrap_or(&ws_root))
                .map_err(|e| format!("Path processing failed: {}", e))?;
            let name_str = name_in_zip.to_string_lossy().replace('\\', "/");
            zip.start_file(&name_str, options).map_err(|e| format!("Failed to write archive: {}", e))?;
            let mut f = fs::File::open(path).map_err(|e| format!("Failed to read file: {}", e))?;
            buffer.clear();
            f.read_to_end(&mut buffer).map_err(|e| format!("Failed to read file: {}", e))?;
            zip.write_all(&buffer).map_err(|e| format!("Failed to write archive: {}", e))?;
        }

        zip.finish().map_err(|e| format!("Failed to finish archive: {}", e))?;

        let mut config: serde_json::Value = serde_json::from_str(&fs::read_to_string(&overlay_json_path).unwrap_or_default()).unwrap_or_default();
        if let Some(obj) = config.as_object_mut() {
            obj.remove("workspace");
        }
        let _ = fs::write(&overlay_json_path, serde_json::to_string_pretty(&config).unwrap_or_default());

        Ok(output_path.to_string_lossy().to_string())
    } else {
        let pack_mcmeta = temp_dir.join("pack.mcmeta");
        let mcmeta = serde_json::json!({
            "pack": {
                "pack_format": 15,
                "description": name
            }
        });
        fs::write(&pack_mcmeta, serde_json::to_string(&mcmeta).unwrap_or_default())
            .map_err(|e| format!("Failed to write pack.mcmeta: {}", e))?;

        let output_name = format!("{}.zip", name);
        let output_path = temp_dir.join(&output_name);
        let assets_dir = temp_dir.join("assets");

        let file = fs::File::create(&output_path).map_err(|e| format!("Failed to create archive: {}", e))?;
        let mut zip = zip::ZipWriter::new(file);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let mut buffer = Vec::new();

        zip.start_file("pack.mcmeta", options).map_err(|e| format!("Failed to write archive: {}", e))?;
        let mut f = fs::File::open(&pack_mcmeta).map_err(|e| format!("Failed to read pack.mcmeta: {}", e))?;
        buffer.clear();
        f.read_to_end(&mut buffer).map_err(|e| format!("Failed to read pack.mcmeta: {}", e))?;
        zip.write_all(&buffer).map_err(|e| format!("Failed to write archive: {}", e))?;

        if assets_dir.exists() {
            for entry in walkdir::WalkDir::new(&assets_dir) {
                let entry = entry.map_err(|e| format!("Failed to walk assets: {}", e))?;
                let path = entry.path();
                if path.is_dir() {
                    continue;
                }
                let name = path.strip_prefix(&temp_dir).map_err(|e| format!("Path processing failed: {}", e))?;
                let name_str = name.to_string_lossy().replace('\\', "/");
                zip.start_file(name_str, options).map_err(|e| format!("Failed to write archive: {}", e))?;
                let mut f = fs::File::open(path).map_err(|e| format!("Failed to read file: {}", e))?;
                buffer.clear();
                f.read_to_end(&mut buffer).map_err(|e| format!("Failed to read file: {}", e))?;
                zip.write_all(&buffer).map_err(|e| format!("Failed to write archive: {}", e))?;
            }
        }

        zip.finish().map_err(|e| format!("Failed to finish archive: {}", e))?;
        let _ = fs::remove_file(&pack_mcmeta);
        Ok(output_path.to_string_lossy().to_string())
    }
}

/// Normalize overlay.json config keys for backward compatibility with Python reference.
/// Python's overlay.py writes different key names than the Rust/Vue frontend.
/// This function adds aliases so both formats are accepted transparently.
fn normalize_overlay_config_keys(mut config: serde_json::Value) -> serde_json::Value {
    if let Some(obj) = config.as_object_mut() {
        // Alias: Python uses "lang_itemname", Rust/Vue uses "custom_names"
        if !obj.contains_key("custom_names") {
            if let Some(v) = obj.remove("lang_itemname") {
                obj.insert("custom_names".to_string(), v);
                crate::log_info!("normalize: lang_itemname -> custom_names");
            }
        }
        // Alias: Python uses "big_items" (plural), Rust/Vue uses "big_item" (singular)
        if !obj.contains_key("big_item") {
            if let Some(v) = obj.remove("big_items") {
                obj.insert("big_item".to_string(), v);
                crate::log_info!("normalize: big_items -> big_item");
            }
        }
        // Alias: Python uses "core_shadow" object with "enabled" bool, Rust/Vue uses bare "no_shadow" bool
        if !obj.contains_key("no_shadow") {
            if let Some(core_shadow) = obj.get("core_shadow") {
                if core_shadow.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false) {
                    obj.insert("no_shadow".to_string(), serde_json::Value::Bool(true));
                    crate::log_info!("normalize: core_shadow.enabled -> no_shadow");
                }
            }
        }
        // Alias: Python uses "core_outline_rainbow" / "core_outline" objects,
        //        Rust/Vue uses unified "outline_type" string
        if !obj.contains_key("outline_type") {
            if obj.get("core_outline_rainbow")
                .and_then(|v| v.get("enabled"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                obj.insert("outline_type".to_string(), serde_json::Value::String("rainbow".to_string()));
                crate::log_info!("normalize: core_outline_rainbow.enabled -> outline_type=rainbow");
            } else if obj.get("core_outline")
                .and_then(|v| v.get("enabled"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                obj.insert("outline_type".to_string(), serde_json::Value::String("default".to_string()));
                crate::log_info!("normalize: core_outline.enabled -> outline_type=default");
            }
        }
        // Preserve per-project core_outline for color/thickness (used in outline_type=="default" path)
        // Also handle Python's "selected_language" key
        if !obj.contains_key("selected_language") {
            // If not present, we'll use the per-project lang code from global settings
            // Keep the key as-is if it already exists (Python may have written it)
        }
    }
    config
}

fn apply_overlay_changes(project_name: &str, overlay_dir: &Path) -> Result<(), String> {
    let temp_dir = overlay_temp_dir(Some(project_name))?;
    let overlay_json_path = temp_dir.join("overlay.json");
    if !overlay_json_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&overlay_json_path).map_err(|e| format!("Failed to read overlay.json: {}", e))?;
    let config: serde_json::Value = serde_json::from_str(&content).map_err(|e| format!("Failed to parse overlay.json: {}", e))?;

    // Normalize config: add aliases for Python-style JSON keys so both Python and Rust/Vue
    // key formats are accepted. This keeps backward compatibility with overlay.json files
    // generated by the Python reference implementation.
    let config = normalize_overlay_config_keys(config);

    let (output_root, is_workspace_mode) = {
        let proj_cfg = read_overlay_config()?;
        if let Some(project) = proj_cfg.projects.iter().find(|p| p.name == project_name) {
            if !project.parent_pack_path.is_empty() && Path::new(&project.parent_pack_path).exists() {
                match crate::overlay::process_parent_pack_workspace(&temp_dir, &project.parent_pack_path) {
                    Ok(ws) => {
                        crate::log_info!("using parent pack workspace: {}", ws.display());
                        (ws, true)
                    }
                    Err(e) => {
                        crate::log_info!("parent pack workspace extraction failed ({}), falling back to asset mode", e);
                        (temp_dir.clone(), false)
                    }
                }
            } else {
                (temp_dir.clone(), false)
            }
        } else {
            (temp_dir.clone(), false)
        }
    };

    if let Some(custom_names) = config.get("custom_names").and_then(|v| v.as_object()) {
        let lang_dir = output_root.join("assets").join("minecraft").join("lang");
        fs::create_dir_all(&lang_dir).map_err(|e| format!("Failed to create lang directory: {}", e))?;

        // Read selected_language from per-project overlay.json (Python-style),
        // fall back to "zh_cn" if not specified
        let selected_lang = config.get("selected_language")
            .and_then(|v| v.as_str())
            .unwrap_or("zh_cn");

        let lang_filename = format!("{}.json", selected_lang);
        let lang_path = lang_dir.join(&lang_filename);
        let base_lang_path = overlay_dir.join("lang").join(&lang_filename);
        let mut lang_data: serde_json::Value = if lang_path.exists() {
            let lang_content = fs::read_to_string(&lang_path).map_err(|e| format!("Failed to read zh_cn.json: {}", e))?;
            serde_json::from_str(&lang_content).unwrap_or_else(|_| serde_json::json!({}))
        } else if base_lang_path.exists() {
            let lang_content = fs::read_to_string(&base_lang_path).map_err(|e| format!("Failed to read base language file: {}", e))?;
            serde_json::from_str(&lang_content).unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        if let serde_json::Value::Object(ref mut map) = lang_data {
            for (k, v) in custom_names {
                map.insert(k.clone(), v.clone());
            }
        }

        let lang_content = serde_json::to_string_pretty(&lang_data).map_err(|e| format!("Failed to serialize language file: {}", e))?;
        fs::write(&lang_path, lang_content).map_err(|e| format!("Failed to write language file: {}", e))?;
    }

    let big_item_templates = overlay_dir.join("big_item");

    if let Some(big_items) = config.get("big_item").and_then(|v| v.as_object()) {
        let models_dir = output_root.join("assets").join("minecraft").join("models").join("item");
        fs::create_dir_all(&models_dir).map_err(|e| format!("Failed to create models directory: {}", e))?;

        for (item_id, _settings) in big_items {
            let template_path = big_item_templates.join(format!("{}.json", item_id));
            if template_path.exists() {
                let dest_path = models_dir.join(format!("{}.json", item_id));
                fs::copy(&template_path, &dest_path).map_err(|e| format!("Failed to copy model template: {}", e))?;
                crate::overlay::fix_json_placeholders(&dest_path)?;
            } else {
                if item_id == "compass" {
                    let compass_template = big_item_templates.join("compass_json").join("compass.json");
                    if compass_template.exists() {
                        let dest_path = models_dir.join("compass.json");
                        fs::copy(&compass_template, &dest_path).map_err(|e| format!("Failed to copy compass template: {}", e))?;
                        crate::overlay::fix_json_placeholders(&dest_path)?;

                        let src_dir = big_item_templates.join("compass_json");
                        let parent_dir = models_dir.parent().unwrap_or(&models_dir);
                        let dest_dir = parent_dir.join("compass_json");
                        let _ = copy_dir_all(&src_dir, &dest_dir);
                        if dest_dir.exists() {
                            for entry in fs::read_dir(&dest_dir).unwrap_or_else(|_| fs::read_dir(".").unwrap()) {
                                if let Ok(entry) = entry {
                                    let _ = crate::overlay::fix_json_placeholders(&entry.path());
                                }
                            }
                        }
                    }
                }
            }
        }

        crate::overlay::apply_scale_to_json_files(&models_dir, &serde_json::to_value(big_items).unwrap_or_default())?;

        let compass_dir = models_dir.parent().unwrap_or(&models_dir).join("compass_json");
        if compass_dir.exists() {
            crate::overlay::apply_scale_to_json_files(&compass_dir, &serde_json::to_value(big_items).unwrap_or_default())?;
        }
    }

    if let Some(small_items) = config.get("small_item").and_then(|v| v.as_object()) {
        let textures_item_dir = output_root.join("assets").join("minecraft").join("textures").join("models").join("item");
        fs::create_dir_all(&textures_item_dir).map_err(|e| format!("Failed to create textures/models/item directory: {}", e))?;

        for (item_name, item_config) in small_items {
            if item_config.get("type").and_then(|v| v.as_str()) == Some("zoom_out")
                && item_config.get("should_shrink").and_then(|v| v.as_bool()).unwrap_or(false)
            {
                let source_file = big_item_templates.join(format!("{}.json", item_name));
                if source_file.exists() {
                    let dest_file = textures_item_dir.join(format!("{}.json", item_name));
                    fs::copy(&source_file, &dest_file).map_err(|e| format!("Failed to copy shrink item: {}", e))?;
                    crate::overlay::fix_json_placeholders(&dest_file)?;
                    crate::log_info!("copied small_item: {} to textures/models/item/", item_name);
                }
            }
        }
    }

    let shaders_dir = output_root.join("assets").join("minecraft").join("shaders").join("core");

    if config.get("no_shadow").and_then(|v| v.as_bool()).unwrap_or(false) {
        fs::create_dir_all(&shaders_dir).map_err(|e| e.to_string())?;
        let src = overlay_dir.join("core_inventory").join("rendertype_gui.vsh");
        if src.exists() {
            fs::copy(&src, shaders_dir.join("rendertype_gui.vsh")).map_err(|e| e.to_string())?;
        }
    }

    if let Some(outline_type) = config.get("outline_type").and_then(|v| v.as_str()) {
        if outline_type != "none" {
            fs::create_dir_all(&shaders_dir).map_err(|e| e.to_string())?;
            let src_dir_name = match outline_type {
                "rainbow" => "core_rainbow_outline",
                "rainbow_hexian" => "core_rainbow_outline_hexian",
                _ => "core_outline"
            };
            let src_dir = overlay_dir.join(src_dir_name);
            if src_dir.exists() {
                for entry in fs::read_dir(&src_dir).map_err(|e| e.to_string())? {
                    let entry = entry.map_err(|e| e.to_string())?;
                    let path = entry.path();
                    if path.is_dir() || path.file_name().is_none() {
                        continue;
                    }
                    let dest = shaders_dir.join(path.file_name().unwrap());
                    fs::copy(&path, &dest).map_err(|e| e.to_string())?;

                    if outline_type == "default" {
                        // Check per-project overlay.json first, then global settings
                        let default_color = serde_json::json!({"r":1.0,"g":1.0,"b":1.0,"a":1.0});
                        if let Some(co) = config.get("core_outline") {
                            if co.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false) {
                                let color = co.get("color").unwrap_or(&default_color);
                                let thickness = co.get("thickness").and_then(|v| v.as_f64()).unwrap_or(2.0);
                                crate::overlay::replace_outline_placeholders(&dest, color, thickness)?;
                                continue;
                            }
                        }
                        // Fallback to global settings
                        let global_cfg = read_overlay_config()?;
                        let global_settings = global_cfg.settings.unwrap_or_else(|| serde_json::json!({}));
                        if let Some(core_outline) = global_settings.get("core_outline") {
                            if core_outline.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false) {
                                let color = core_outline.get("color").unwrap_or(&default_color);
                                let thickness = core_outline.get("thickness").and_then(|v| v.as_f64()).unwrap_or(2.0);
                                crate::overlay::replace_outline_placeholders(&dest, color, thickness)?;
                            }
                        }
                    }
                }
            }
        }
    }

    if config.get("custom_glint").and_then(|v| v.as_bool()).unwrap_or(false) {
        let glint_dest = output_root.join("assets").join("minecraft").join("textures").join("misc");
        fs::create_dir_all(&glint_dest).map_err(|e| e.to_string())?;
        let src = overlay_dir.join("enchanted_item_glint").join("enchanted_item_glint.png");
        if src.exists() {
            fs::copy(&src, glint_dest.join("enchanted_item_glint.png")).map_err(|e| e.to_string())?;
        }
    }

    if is_workspace_mode {
        let mut overlay_data = config.clone();
        overlay_data["workspace"] = serde_json::json!({
            "path": output_root.to_string_lossy(),
            "processed": true
        });
        let content = serde_json::to_string_pretty(&overlay_data).map_err(|e| format!("Failed to serialize overlay.json: {}", e))?;
        fs::write(&overlay_json_path, content).map_err(|e| format!("Failed to write overlay.json: {}", e))?;
    }

    Ok(())
}

#[derive(serde::Deserialize)]
pub struct OverlaySettingsPatch {
    settings: serde_json::Value,
}

#[derive(serde::Deserialize)]
pub struct OverlayParentPatch {
    #[serde(alias = "projectId")]
    pub project_id: String,
    #[serde(alias = "parentPackPath")]
    pub parent_pack_path: Option<String>,
}

fn merge_json_settings(base: &mut serde_json::Value, patch: &serde_json::Value) {
    if let (serde_json::Value::Object(base_map), serde_json::Value::Object(patch_map)) = (base, patch) {
        for (k, v) in patch_map {
            base_map.insert(k.clone(), v.clone());
        }
    }
}

#[tauri::command]
pub fn get_overlay_settings() -> Result<serde_json::Value, String> {
    let cfg = read_overlay_config()?;
    Ok(cfg.settings.unwrap_or_else(|| serde_json::json!({})))
}

#[tauri::command]
pub fn save_overlay_settings(patch: OverlaySettingsPatch) -> Result<serde_json::Value, String> {
    let mut cfg = read_overlay_config()?;
    let mut base = cfg.settings.unwrap_or_else(|| serde_json::json!({}));
    merge_json_settings(&mut base, &patch.settings);
    cfg.settings = Some(base);
    write_overlay_config(&cfg)?;
    Ok(cfg.settings.unwrap_or_else(|| serde_json::json!({})))
}

#[tauri::command]
pub fn overlay_set_parent_pack(patch: OverlayParentPatch) -> Result<(), String> {
    let mut cfg = read_overlay_config()?;
    let project_id = patch.project_id;
    let parent_path = patch.parent_pack_path.unwrap_or_default();

    if let Some(project) = cfg.projects.iter_mut().find(|p| p.id == project_id) {
        project.parent_pack_path = parent_path;
        project.updated_at = chrono::Utc::now().timestamp_millis();
        write_overlay_config(&cfg)?;
        Ok(())
    } else {
        Err("Project does not exist".to_string())
    }
}
