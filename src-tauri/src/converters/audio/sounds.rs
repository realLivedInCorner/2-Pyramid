use std::fs;
use std::path::Path;

use walkdir::WalkDir;

pub fn convert_sound_files(temp_dir: &str) -> Result<(), String> {
    crate::log_info!("processing sound file conversion");

    let old_sounds_path = Path::new(temp_dir).join("sound3");
    let new_sounds_path = Path::new(temp_dir)
        .join("assets")
        .join("minecraft")
        .join("sounds");

    if !old_sounds_path.exists() || !old_sounds_path.is_dir() {
        crate::log_info!("sound3 folder not found, nothing to convert");
        return Ok(());
    }

    fs::create_dir_all(&new_sounds_path)
        .map_err(|e| format!("failed to create sounds target directory: {}", e))?;

    let mut copied_files = 0;
    for entry in WalkDir::new(&old_sounds_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let relative_path = match entry.path().strip_prefix(&old_sounds_path) {
            Ok(path) => path,
            Err(_) => continue,
        };

        let target_path = new_sounds_path.join(relative_path);
        if let Some(parent) = target_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                crate::log_info!("warning: failed to create target parent: {}", e);
                continue;
            }
        }

        match fs::copy(entry.path(), &target_path) {
            Ok(_) => {
                copied_files += 1;
                if copied_files % 10 == 0 {
                    crate::log_info!("copied {} sound files", copied_files);
                }
            }
            Err(e) => {
                crate::log_info!(
                    "warning: failed to copy {}: {}",
                    entry.path().display(),
                    e
                );
            }
        }
    }

    if copied_files > 0 {
        if let Err(e) = fs::remove_dir_all(&old_sounds_path) {
            crate::log_info!("warning: failed to remove old sound3 folder: {}", e);
        } else {
            crate::log_info!("old sound3 folder removed");
        }
    }

    crate::log_info!("sound file conversion done, copied {} files", copied_files);
    Ok(())
}

fn update_sound_json(temp_dir: &str) -> Result<(), String> {
    let sounds_json_path = Path::new(temp_dir)
        .join("assets")
        .join("minecraft")
        .join("sounds.json");

    if !sounds_json_path.exists() || !sounds_json_path.is_file() {
        crate::log_info!("sounds.json not found, skip json update");
        return Ok(());
    }

    crate::log_info!("sounds.json found, update is currently a no-op");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "convert_sound_files",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Surgeon,
        |context| {
            let temp_dir_str = context.temp_dir().to_string_lossy().to_string();
            convert_sound_files(&temp_dir_str)?;
            update_sound_json(&temp_dir_str)?;
            Ok(())
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, write};
    use tempfile::tempdir;

    #[test]
    fn test_convert_sound_files() {
        let temp_dir = tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_str().unwrap();

        let old_sounds_dir = Path::new(temp_dir_path).join("sound3");
        let sub_dir = old_sounds_dir.join("entity").join("player");
        create_dir_all(&sub_dir).unwrap();

        let test_file1 = old_sounds_dir.join("ambient1.ogg");
        let test_file2 = sub_dir.join("hurt.ogg");
        write(&test_file1, "content1").unwrap();
        write(&test_file2, "content2").unwrap();

        let result = convert_sound_files(temp_dir_path);
        assert!(result.is_ok());
        assert!(!old_sounds_dir.exists());

        let new_sounds_dir = Path::new(temp_dir_path)
            .join("assets")
            .join("minecraft")
            .join("sounds");
        let new_file1 = new_sounds_dir.join("ambient1.ogg");
        let new_file2 = new_sounds_dir.join("entity").join("player").join("hurt.ogg");

        assert!(new_file1.exists());
        assert!(new_file2.exists());
        assert_eq!(fs::read_to_string(new_file1).unwrap(), "content1");
        assert_eq!(fs::read_to_string(new_file2).unwrap(), "content2");
    }

    #[test]
    fn test_convert_sound_files_nonexistent() {
        let temp_dir = tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_str().unwrap();

        let old_sounds_dir = Path::new(temp_dir_path).join("sound3");
        assert!(!old_sounds_dir.exists());

        let result = convert_sound_files(temp_dir_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_sound_json_nonexistent() {
        let temp_dir = tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_str().unwrap();

        let sounds_json_path = Path::new(temp_dir_path)
            .join("assets")
            .join("minecraft")
            .join("sounds.json");
        assert!(!sounds_json_path.exists());

        let result = update_sound_json(temp_dir_path);
        assert!(result.is_ok());
    }
}