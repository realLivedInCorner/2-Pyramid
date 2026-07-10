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

// (Context-menu commands removed — see src-tauri/src/registry.rs deletion
// note in commit message. The right-click entry-point is gone by design.)
