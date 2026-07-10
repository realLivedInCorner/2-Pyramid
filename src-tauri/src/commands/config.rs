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
    pub close_action: Option<String>,
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
    #[serde(alias = "closeAction")]
    pub close_action: Option<String>,
}

pub(crate) fn config_path() -> Result<std::path::PathBuf, String> {
    let base = dirs::home_dir().ok_or_else(|| "Failed to get user home directory".to_string())?;
    Ok(base.join(".2pyr").join("configs").join("settings.json"))
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

#[tauri::command]
pub fn update_config(patch: ConfigPatch) -> Result<serde_json::Value, String> {
    crate::log_debug!("update_config called");
    let mut cfg = read_config_file()?;
    if let Some(v) = patch.output_mode {
        crate::log_info!("config: output_mode = {}", v);
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
        crate::log_info!("config: initialized = {}", v);
        cfg.initialized = Some(v);
    }
    if let Some(v) = patch.user_name {
        crate::log_info!("config: user_name = {}", v);
        cfg.user_name = Some(v);
    }
    if let Some(v) = patch.notification_enabled {
        crate::log_info!("config: notification_enabled = {}", v);
        cfg.notification_enabled = Some(v);
    }
    if let Some(v) = patch.notification_mode {
        crate::log_info!("config: notification_mode = {}", v);
        cfg.notification_mode = Some(v);
    }
    if let Some(v) = patch.close_action {
        crate::log_info!("config: close_action = {}", v);
        cfg.close_action = Some(v);
    }
    write_config_file(&cfg)?;
    crate::log_info!("config: saved to {:?}", config_path());
    serde_json::to_value(cfg).map_err(|e| format!("Failed to read config: {}", e))
}
