#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
pub struct AppConfig {
    pub output_mode: Option<String>,
    pub output_path: Option<String>,
    pub palette: Option<serde_json::Value>,
    pub overlay_history: Option<serde_json::Value>,
    pub update_channel: Option<String>,
    pub initialized: Option<bool>,
    pub user_name: Option<String>,
    pub notification_enabled: Option<bool>,
    pub notification_mode: Option<String>,
    /// Post-conversion source-pack handling policy:
    ///   * "ask"    — show an in-app confirmation after each batch
    ///   * "delete" — always delete the original
    ///   * "keep"   — always keep the original
    pub source_handling: Option<String>,
    /// Whether to pop the OS file explorer open on the output folder
    /// after a successful conversion.
    pub open_output_after_convert: Option<bool>,
    /// Desktop toast auto-dismiss time in milliseconds (4000–15000).
    pub toast_duration_ms: Option<u64>,
    /// Desktop toast corner: "top-right" (default) / "top-left" /
    /// "bottom-right" / "bottom-left".
    pub toast_position: Option<String>,
    /// How many resource packs a batch converts in parallel (1–4).
    pub conversion_threads: Option<u32>,
    /// Output file naming template with `[Name]` / `[Ver]` / `[Time]` /
    /// `[Date]` placeholders, e.g. `[Name] [Ver]`.
    pub output_naming: Option<String>,
    /// Custom background image path (managed via set_background).
    pub background_image: Option<String>,
    /// Background display fit: "cover" / "contain" / "stretch" / "tile".
    pub background_fit: Option<String>,
    /// Background layer opacity (0.1–1.0).
    pub background_opacity: Option<f64>,
    /// UI surface style: "glass" (translucent + blur) or "frosted"
    /// (opaque matte).
    pub ui_style: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct ConfigPatch {
    #[serde(alias = "outputMode")]
    pub output_mode: Option<String>,
    #[serde(alias = "outputPath")]
    pub output_path: Option<String>,
    pub palette: Option<serde_json::Value>,
    #[serde(alias = "overlayHistory")]
    pub overlay_history: Option<serde_json::Value>,
    #[serde(alias = "updateChannel")]
    pub update_channel: Option<String>,
    pub initialized: Option<bool>,
    #[serde(alias = "userName")]
    pub user_name: Option<String>,
    #[serde(alias = "notificationEnabled")]
    pub notification_enabled: Option<bool>,
    #[serde(alias = "notificationMode")]
    pub notification_mode: Option<String>,
    #[serde(alias = "sourceHandling")]
    pub source_handling: Option<String>,
    #[serde(alias = "openOutputAfterConvert")]
    pub open_output_after_convert: Option<bool>,
    #[serde(alias = "toastDurationMs")]
    pub toast_duration_ms: Option<u64>,
    #[serde(alias = "toastPosition")]
    pub toast_position: Option<String>,
    #[serde(alias = "conversionThreads")]
    pub conversion_threads: Option<u32>,
    #[serde(alias = "outputNaming")]
    pub output_naming: Option<String>,
    #[serde(alias = "uiStyle")]
    pub ui_style: Option<String>,
}

pub(crate) fn config_path() -> Result<std::path::PathBuf, String> {
    let base = dirs::home_dir().ok_or_else(|| "Failed to get user home directory".to_string())?;
    Ok(base.join(".2pyr").join("configs").join("settings.json"))
}

/// Directory that holds everything user-specific on disk:
///   * `configs/settings.json`  — main config
///   * `backups/`                — auto-created by `factory_reset*` so
///                                  the next launch / OOBE can offer to
///                                  restore
///   * `last_backup.json`        — pointer to the most recent backup
pub(crate) fn app_data_dir() -> Result<std::path::PathBuf, String> {
    let base = dirs::home_dir().ok_or_else(|| "Failed to get user home directory".to_string())?;
    Ok(base.join(".2pyr"))
}

/// Directory containing settings backups created by `factory_reset`.
pub(crate) fn backup_dir() -> Result<std::path::PathBuf, String> {
    Ok(app_data_dir()?.join("backups"))
}

/// Path to the small "last backup pointer" file. Always points at the
/// most recent backup so the OOBE "import previous settings" flow
/// only needs to read one JSON file regardless of how many backups
/// exist on disk.
pub(crate) fn last_backup_pointer_path() -> Result<std::path::PathBuf, String> {
    Ok(app_data_dir()?.join("last_backup.json"))
}

pub fn read_config_file() -> Result<AppConfig, String> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    let cfg: AppConfig = serde_json::from_str(&content)
        .unwrap_or_default();
    Ok(cfg)
}

pub fn write_config_file(cfg: &AppConfig) -> Result<(), String> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    let content = serde_json::to_string_pretty(cfg)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    std::fs::write(&path, content)
        .map_err(|e| format!("Failed to write config: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn get_config() -> Result<serde_json::Value, String> {
    let cfg = read_config_file()?;
    serde_json::to_value(cfg).map_err(|e| format!("Failed to read config: {}", e))
}

/// Dev-only: delete the on-disk `settings.json` so the next read returns
/// the defaults. Guarded by `is_dev_mode()` so this cannot be triggered from
/// the production UI by accident — it exists to make QA reset cycles fast.
#[tauri::command]
pub fn clear_config() -> Result<String, String> {
    use crate::logger::GLOBAL_LOGGER;
    if !GLOBAL_LOGGER.is_dev_mode() {
        return Err("clear_config is only available in dev mode".to_string());
    }
    let path = config_path()?;
    let path_str = path.to_string_lossy().to_string();
    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| format!("Failed to remove config: {}", e))?;
    }
    crate::log_info!("dev: config file removed: {}", path_str);
    Ok(path_str)
}

/// User-facing "factory reset": permanently deletes the on-disk
/// `settings.json` so the next launch falls back to defaults and the
/// OOBE flow runs again. Unlike `clear_config`, this one is *not*
/// gated behind dev mode — it's the entry point for the Settings
/// page's "Delete user profile" button, which is an explicit user
/// action.
///
/// Before deleting, we copy the existing config (if any) to
/// `~/.2pyr/backups/settings-YYYYMMDD-HHMMSS.json` and update
/// `~/.2pyr/last_backup.json` so the OOBE flow on the next launch
/// can offer to "import previous settings".
#[tauri::command]
pub fn factory_reset() -> Result<String, String> {
    factory_reset_impl(false)
}

/// User-facing "deep factory reset": same as `factory_reset` but also
/// wipes overlay-history data and the on-disk log directory. We keep
/// the actual overlay-pack projects (the user-built packs) intact —
/// wiping those would feel spooky and the user can do it by hand.
#[tauri::command]
pub fn factory_reset_deep() -> Result<FactoryResetReport, String> {
    let deleted_config = factory_reset_impl(true)?;

    // Wipe log directory (`<local_data>/2-Pyramid/logs/...`).
    let mut logs_deleted = 0u64;
    if let Some(local) = dirs::data_local_dir() {
        let logs_dir = local.join("2-Pyramid").join("logs");
        if logs_dir.exists() {
            match std::fs::remove_dir_all(&logs_dir) {
                Ok(()) => {
                    crate::log_info!("factory_reset_deep: removed logs dir {:?}", logs_dir);
                    logs_deleted = 1;
                }
                Err(e) => crate::log_warn!("factory_reset_deep: failed to remove logs dir: {}", e),
            }
        }
    }

    // overlay_history lives inside settings.json, so it's already gone
    // once `factory_reset_impl` deletes the file. We do nothing extra
    // here — the report field exists for symmetry / future expansion
    // (e.g. if overlay_history ever moves to its own file).

    Ok(FactoryResetReport {
        config_path: deleted_config,
        logs_deleted,
        overlay_history_cleared: true,
    })
}

#[derive(serde::Serialize)]
pub struct FactoryResetReport {
    pub config_path: String,
    /// Number of log directories removed (0 or 1 today — the global
    /// `logs/` dir under `<local_data>/2-Pyramid/`).
    pub logs_deleted: u64,
    /// Always true after a reset since `overlay_history` is part of
    /// `settings.json`.
    pub overlay_history_cleared: bool,
}

/// Shared implementation of the two factory-reset variants. Performs
/// the backup dance + config delete; the `deep` flag is currently a
/// hint for logging but the heavy lifting for the deep path happens
/// in `factory_reset_deep` after this returns.
fn factory_reset_impl(deep: bool) -> Result<String, String> {
    let path = config_path()?;
    let path_str = path.to_string_lossy().to_string();

    // 1) Try to back up the existing config. If it doesn't exist
    //    (first launch), skip silently — nothing to back up.
    if path.exists() {
        if let Err(e) = backup_current_config(&path) {
            crate::log_warn!(
                "factory_reset: backup step failed (continuing with delete): {}",
                e
            );
        }
    } else {
        crate::log_info!(
            "factory_reset: no existing config at {}; skipping backup",
            path_str
        );
    }

    // 2) Delete the live config.
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| {
            crate::log_error!("factory_reset: failed to delete {}: {}", path_str, e);
            format!("Failed to remove config: {}", e)
        })?;
    }
    crate::log_info!(
        "factory_reset(deep={}): config deleted; next launch will show OOBE",
        deep
    );
    Ok(path_str)
}

/// Copy the live settings.json into `backups/settings-YYYYMMDD-HHMMSS.json`
/// and refresh `last_backup.json` so the OOBE import flow can find it.
fn backup_current_config(path: &std::path::Path) -> Result<(), String> {
    let backup_dir = backup_dir()?;
    std::fs::create_dir_all(&backup_dir).map_err(|e| {
        format!("Failed to create backup dir {}: {}", backup_dir.display(), e)
    })?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let backup_file = backup_dir.join(format!("settings-{}.json", stamp));
    std::fs::copy(path, &backup_file).map_err(|e| {
        format!("Failed to copy {} -> {}: {}", path.display(), backup_file.display(), e)
    })?;
    crate::log_info!("factory_reset: backup created at {:?}", backup_file);

    // Update the small pointer file the OOBE flow reads.
    let pointer = last_backup_pointer_path()?;
    let body = serde_json::json!({
        "path": backup_file.to_string_lossy(),
        "created_at": chrono::Local::now().to_rfc3339(),
        "original_path": path.to_string_lossy(),
    });
    std::fs::write(&pointer, serde_json::to_string_pretty(&body).unwrap_or_default())
        .map_err(|e| format!("Failed to write last_backup.json: {}", e))?;
    Ok(())
}

/// Metadata returned to the frontend when OOBE asks "do you want to
/// restore the previous session?". The frontend uses `summary` to
/// render the preview card so the user knows what they're about to
/// restore before committing.
#[derive(serde::Serialize)]
pub struct BackupInfo {
    pub exists: bool,
    pub path: Option<String>,
    pub created_at: Option<String>,
    pub summary: Option<BackupSummary>,
}

#[derive(serde::Serialize)]
pub struct BackupSummary {
    pub user_name: Option<String>,
    pub output_mode: Option<String>,
    pub notification_mode: Option<String>,
    pub source_handling: Option<String>,
    pub open_output_after_convert: Option<bool>,
    pub theme_color: Option<String>,
    pub language: Option<String>,
}

/// Inspect `last_backup.json` (if present) and return a summary the
/// OOBE page can show in the "import previous settings" preview card.
/// We deliberately do NOT include the full settings blob here — only
/// the fields the user can sanity-check at a glance.
#[tauri::command]
pub fn get_last_backup_info() -> Result<BackupInfo, String> {
    let pointer = last_backup_pointer_path()?;
    if !pointer.exists() {
        return Ok(BackupInfo {
            exists: false,
            path: None,
            created_at: None,
            summary: None,
        });
    }
    let raw = std::fs::read_to_string(&pointer)
        .map_err(|e| format!("Failed to read last_backup.json: {}", e))?;
    let v: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| format!("Failed to parse last_backup.json: {}", e))?;
    let path = v.get("path").and_then(|x| x.as_str()).map(String::from);
    let created_at = v.get("created_at").and_then(|x| x.as_str()).map(String::from);

    let summary = path
        .as_deref()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .map(|cfg| BackupSummary {
            user_name: cfg.get("user_name").and_then(|x| x.as_str()).map(String::from),
            output_mode: cfg.get("output_mode").and_then(|x| x.as_str()).map(String::from),
            notification_mode: cfg
                .get("notification_mode")
                .and_then(|x| x.as_str())
                .map(String::from),
            source_handling: cfg.get("source_handling").and_then(|x| x.as_str()).map(String::from),
            open_output_after_convert: cfg
                .get("open_output_after_convert")
                .and_then(|x| x.as_bool()),
            theme_color: cfg
                .get("palette")
                .and_then(|p| p.get("theme_color"))
                .and_then(|x| x.as_str())
                .map(String::from),
            language: None, // language lives in localStorage, not the file
        });

    Ok(BackupInfo {
        exists: true,
        path,
        created_at,
        summary,
    })
}

/// Restore the most recent backup as the live settings.json. Used by
/// the OOBE flow when the user picks "import previous settings". We
/// refuse to proceed if no backup pointer exists (caller should have
/// already gated this behind `get_last_backup_info`).
#[tauri::command]
pub fn import_last_backup() -> Result<String, String> {
    let pointer = last_backup_pointer_path()?;
    let raw = std::fs::read_to_string(&pointer)
        .map_err(|e| format!("Failed to read last_backup.json: {}", e))?;
    let v: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| format!("Failed to parse last_backup.json: {}", e))?;
    let backup_path = v
        .get("path")
        .and_then(|x| x.as_str())
        .ok_or_else(|| "last_backup.json missing 'path' field".to_string())?;

    let src = std::path::Path::new(backup_path);
    if !src.exists() {
        return Err(format!(
            "Backup file no longer exists at {}; cannot import",
            src.display()
        ));
    }
    let dest = config_path()?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config dir: {}", e))?;
    }
    std::fs::copy(src, &dest).map_err(|e| {
        format!("Failed to copy backup {} -> {}: {}", src.display(), dest.display(), e)
    })?;
    crate::log_info!("import_last_backup: restored {} -> {}", src.display(), dest.display());

    // The pointer has now served its purpose — delete it so the next
    // OOBE launch doesn't keep offering to re-import the same
    // backup. (The on-disk backup file stays put in `backups/` so the
    // user could still reach it manually.)
    let _ = std::fs::remove_file(&pointer);

    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub fn update_config(patch: ConfigPatch) -> Result<serde_json::Value, String> {
    // 收敛为一条 OKAY 日志（字段值不值得逐条刷屏；错误路径照常记 Error）
    let changed: Vec<&'static str> = [
        patch.output_mode.as_ref().map(|_| "output_mode"),
        patch.output_path.as_ref().map(|_| "output_path"),
        patch.palette.as_ref().map(|_| "palette"),
        patch.overlay_history.as_ref().map(|_| "overlay_history"),
        patch.update_channel.as_ref().map(|_| "update_channel"),
        patch.initialized.as_ref().map(|_| "initialized"),
        patch.user_name.as_ref().map(|_| "user_name"),
        patch.notification_enabled.as_ref().map(|_| "notification_enabled"),
        patch.notification_mode.as_ref().map(|_| "notification_mode"),
        patch.source_handling.as_ref().map(|_| "source_handling"),
        patch.open_output_after_convert.as_ref().map(|_| "open_output_after_convert"),
        patch.toast_duration_ms.as_ref().map(|_| "toast_duration_ms"),
        patch.toast_position.as_ref().map(|_| "toast_position"),
        patch.conversion_threads.as_ref().map(|_| "conversion_threads"),
        patch.output_naming.as_ref().map(|_| "output_naming"),
        patch.ui_style.as_ref().map(|_| "ui_style"),
    ]
    .into_iter()
    .flatten()
    .collect();

    let mut cfg = read_config_file()?;
    if let Some(v) = patch.output_mode {
        cfg.output_mode = Some(v);
    }
    if let Some(v) = patch.output_path {
        cfg.output_path = Some(v);
    }
    if let Some(v) = patch.palette {
        cfg.palette = Some(v);
    }
    if let Some(v) = patch.overlay_history {
        cfg.overlay_history = Some(v);
    }
    if let Some(v) = patch.update_channel {
        cfg.update_channel = Some(v);
    }
    if let Some(v) = patch.initialized {
        cfg.initialized = Some(v);
    }
    if let Some(v) = patch.user_name {
        cfg.user_name = Some(v);
    }
    if let Some(v) = patch.notification_enabled {
        cfg.notification_enabled = Some(v);
    }
    if let Some(v) = patch.notification_mode {
        cfg.notification_mode = Some(v);
    }
    if let Some(v) = patch.source_handling {
        cfg.source_handling = Some(v);
    }
    if let Some(v) = patch.open_output_after_convert {
        cfg.open_output_after_convert = Some(v);
    }
    if let Some(v) = patch.toast_duration_ms {
        cfg.toast_duration_ms = Some(v.clamp(4000, 15000));
    }
    if let Some(v) = patch.toast_position {
        cfg.toast_position = Some(v);
    }
    if let Some(v) = patch.conversion_threads {
        cfg.conversion_threads = Some(v.clamp(1, 4));
    }
    if let Some(v) = patch.output_naming {
        // Free-form template with [Name]/[Ver]/[Time]/[Date] placeholders;
        // just cap the length so a runaway paste can't bloat the file.
        let capped: String = v.chars().take(200).collect();
        cfg.output_naming = Some(capped);
    }
    if let Some(v) = patch.ui_style {
        let normalized = match v.as_str() {
            "frosted" => "frosted".to_string(),
            _ => "glass".to_string(),
        };
        cfg.ui_style = Some(normalized);
    }
    write_config_file(&cfg)?;
    crate::log_info!("OKAY update_config [{}]", changed.join(", "));
    serde_json::to_value(cfg).map_err(|e| format!("Failed to read config: {}", e))
}
