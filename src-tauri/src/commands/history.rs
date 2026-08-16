//! Conversion history — a small on-disk journal of recent batch
//! conversion entries, shown on the Settings page.
//!
//! Stored separately from `settings.json` (which stays tiny): a plain
//! JSON file at `~/.2pyr/history.json`, capped at 100 entries with the
//! newest first. Recording is best-effort — a failed write never
//! affects the conversion result.

use serde::{Deserialize, Serialize};

/// One recorded conversion attempt (per input file in a batch).
#[derive(Serialize, Deserialize, Clone)]
pub struct HistoryEntry {
    pub input: String,
    pub output: Option<String>,
    /// "success" | "error" | "cancelled"
    pub status: String,
    pub error: Option<String>,
    /// Wall-clock timestamp when the entry was recorded.
    pub time: String,
    /// Conversion duration in seconds (0 if unknown).
    pub duration_s: f64,
}

/// Maximum number of entries kept on disk.
const MAX_ENTRIES: usize = 100;

fn history_path() -> Result<std::path::PathBuf, String> {
    let base = dirs::home_dir().ok_or_else(|| "Failed to get user home directory".to_string())?;
    Ok(base.join(".2pyr").join("history.json"))
}

/// Append one entry at the front of the journal. Best-effort: IO
/// failures are logged and swallowed so conversion results are never
/// affected by history bookkeeping.
pub fn record_entry(entry: HistoryEntry) {
    let path = match history_path() {
        Ok(p) => p,
        Err(e) => {
            crate::log_warn!("history: path resolution failed: {}", e);
            return;
        }
    };

    let mut entries: Vec<HistoryEntry> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|v| {
            v.get("entries")
                .cloned()
                .and_then(|e| serde_json::from_value::<Vec<HistoryEntry>>(e).ok())
        })
        .unwrap_or_default();

    entries.insert(0, entry);
    entries.truncate(MAX_ENTRIES);

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let body = serde_json::to_string_pretty(&serde_json::json!({ "entries": entries }))
        .unwrap_or_default();
    if let Err(e) = std::fs::write(&path, body) {
        crate::log_warn!("history: write failed: {}", e);
    }
}

/// Return the journal, newest first.
#[tauri::command]
pub fn get_conversion_history() -> Vec<HistoryEntry> {
    history_path()
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|v| {
            v.get("entries")
                .cloned()
                .and_then(|e| serde_json::from_value::<Vec<HistoryEntry>>(e).ok())
        })
        .unwrap_or_default()
}

/// Wipe the journal.
#[tauri::command]
pub fn clear_conversion_history() -> Result<(), String> {
    let path = history_path()?;
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("Failed to remove history file: {}", e))?;
    }
    crate::log_info!("OKAY clear_conversion_history");
    Ok(())
}
