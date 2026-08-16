#[tauri::command]
pub fn get_logs(lines: Option<usize>) -> String {
    use crate::logger::GLOBAL_LOGGER;

    let logs = GLOBAL_LOGGER.get_logs();
    if let Some(line_count) = lines {
        let start_index = logs.len().saturating_sub(line_count);
        logs[start_index..].join("\n")
    } else {
        logs.join("\n")
    }
}

#[tauri::command]
pub fn set_dev_mode(enabled: bool) -> Result<bool, String> {
    use crate::logger::GLOBAL_LOGGER;
    GLOBAL_LOGGER.set_dev_mode(enabled);
    Ok(GLOBAL_LOGGER.is_dev_mode())
}

#[tauri::command]
pub fn get_dev_mode() -> bool {
    use crate::logger::GLOBAL_LOGGER;
    GLOBAL_LOGGER.is_dev_mode()
}

#[tauri::command]
pub fn log_notification(notification_type: String, title: String, body: String) {
    use crate::logger::GLOBAL_LOGGER;
    GLOBAL_LOGGER.info(&format!("[Notif] [{}] {}: {}", notification_type, title, body));
}

/// Export the in-memory log buffer to a file at `dest` path.
#[tauri::command]
pub fn export_logs(dest: String) -> Result<String, String> {
    use crate::logger::GLOBAL_LOGGER;
    GLOBAL_LOGGER.export_logs(&dest)
}

/// Return the path to today's log file.
#[tauri::command]
pub fn get_log_path() -> Option<String> {
    use crate::logger::GLOBAL_LOGGER;
    GLOBAL_LOGGER.log_file_path_str()
}

#[tauri::command]
pub fn open_folder(path: String) -> Result<(), String> {
    let path = std::path::Path::new(&path);
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        use std::os::windows::process::CommandExt;
        match Command::new("explorer")
            .arg(path)
            .creation_flags(0x08000000)
            .status() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to open folder: {}", e)),
        }
    }

    #[cfg(target_os = "macos")]
    {
        match std::process::Command::new("open").arg(path).status() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to open folder: {}", e)),
        }
    }

    #[cfg(target_os = "linux")]
    {
        match std::process::Command::new("xdg-open").arg(path).status() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to open folder: {}", e)),
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err("Opening folders not supported on current OS".to_string())
    }
}

#[tauri::command]
pub fn write_file(path: String, content: String) -> Result<(), String> {
    use std::fs;

    let path = std::path::Path::new(&path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    fs::write(path, content).map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn create_dir(path: String) -> Result<(), String> {
    use std::fs;
    let path = std::path::Path::new(&path);
    if path.exists() {
        return Ok(());
    }
    fs::create_dir_all(path).map_err(|e| format!("Failed to create directory: {}", e))?;
    Ok(())
}

/// Recursively delete one or more paths. Used by the conversion
/// page's "post-conversion source pack handling" flow when the user
/// picks `delete` or confirms the `ask` prompt.
///
/// Each path is deleted independently: a failure on one path does not
/// abort the rest, and we collect every per-path outcome so the
/// frontend can show a useful summary. The frontend only treats the
/// operation as failed if *all* paths failed.
#[tauri::command]
pub fn delete_paths(paths: Vec<String>) -> Result<Vec<DeleteResult>, String> {
    use std::fs;

    let mut out = Vec::with_capacity(paths.len());
    for p in &paths {
        let path = std::path::Path::new(p);
        if !path.exists() {
            // Treat "already gone" as success so the user doesn't get
            // a confusing error when a previous batch already removed
            // the file.
            out.push(DeleteResult {
                path: p.clone(),
                ok: true,
                error: None,
            });
            continue;
        }
        let res = if path.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        };
        match res {
            Ok(()) => out.push(DeleteResult {
                path: p.clone(),
                ok: true,
                error: None,
            }),
            Err(e) => out.push(DeleteResult {
                path: p.clone(),
                ok: false,
                error: Some(e.to_string()),
            }),
        }
    }
    Ok(out)
}

#[derive(serde::Serialize)]
pub struct DeleteResult {
    pub path: String,
    pub ok: bool,
    pub error: Option<String>,
}

// (Context-menu commands removed — see src-tauri/src/registry.rs deletion
// note in commit message. The right-click entry-point is gone by design.)

// ── App version / build info ──────────────────────────────────────

/// Aggregate struct returned to the frontend so the UI can show a
/// single canonical `version + build` stamp without having to know
/// about Cargo or BUILD files.
#[derive(serde::Serialize)]
pub struct AppInfo {
    /// Semantic version from `Cargo.toml` (e.g. "2.0.0").
    pub version: String,
    /// Raw build number injected by `build.rs` from the repo-root
    /// `BUILD` file. May be "dev", "dev.5", or a plain integer string.
    pub build: String,
    /// Convenience field: `"{version}+build.{build}"` (or just
    /// `"{version}+{build}"` when build is empty).
    pub full: String,
    /// True when built without `--release`.
    pub is_dev: bool,
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    let version = env!("CARGO_PKG_VERSION").to_string();
    let build_raw = env!("BUILD_NUMBER").to_string();
    // `build.rs` prefixes debug builds with "dev.", strip it for display.
    let build = build_raw
        .strip_prefix("dev.")
        .map(str::to_string)
        .unwrap_or_else(|| build_raw.clone());

    let full = if build.is_empty() || build == "0" {
        version.clone()
    } else if build == "dev" {
        // No BUILD file present — emit `version+dev` so the marker still shows.
        format!("{}+dev", version)
    } else {
        format!("{}+build.{}", version, build)
    };

    let is_dev = cfg!(debug_assertions);

    AppInfo {
        version,
        build,
        full,
        is_dev,
    }
}

// ── Process lifecycle ─────────────────────────────────────────────
//
// There is no tray / background-resident mode anymore: closing the
// main window exits the process (see `lib.rs`). These two commands
// are what remains of the old lifecycle surface:
//
//   * `force_quit` — hard exit, used by the frontend's Ctrl+Shift+Q
//     shortcut. `app.exit(0)` tears down webviews + the Tauri runtime
//     cleanly so in-flight commands get a chance to abort via normal
//     destructors.
//   * `ping` — trivial IPC liveness probe, kept as a general
//     diagnostic for the webview↔Rust bridge.

#[tauri::command]
pub fn force_quit(app: tauri::AppHandle) {
    crate::log_info!("force_quit: user requested hard exit (Ctrl+Shift+Q)");
    app.exit(0);
}

/// Trivial liveness probe so the frontend can tell whether the
/// webview↔Rust IPC bridge is still alive.
#[tauri::command]
pub fn ping() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    crate::log_warn!("ping: IPC alive at {}", ms);
    ms
}
