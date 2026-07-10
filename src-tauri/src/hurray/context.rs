use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use image::RgbaImage;

use crate::{log_info, log_warn};

/// Deferred file/directory cleanup registry.
/// All cleanup operations register paths here during conversion and are
/// executed in one batch at the very end, ensuring no file is deleted
/// before all conversion tasks have had a chance to use it.
struct CleanupList {
    files: Vec<PathBuf>,
    dirs: Vec<PathBuf>,
}

impl CleanupList {
    fn new() -> Self {
        Self { files: Vec::new(), dirs: Vec::new() }
    }

    fn defer_file(&mut self, path: PathBuf) {
        self.files.push(path);
    }

    fn defer_dir(&mut self, path: PathBuf) {
        self.dirs.push(path);
    }

    fn execute(self) -> Result<(), String> {
        // Delete files first
        for path in &self.files {
            if path.exists() {
                if path.is_dir() {
                    fs::remove_dir_all(path).map_err(|e| {
                        format!("cleanup: failed to remove dir {}: {}", path.display(), e)
                    })?;
                } else {
                    fs::remove_file(path).map_err(|e| {
                        format!("cleanup: failed to remove file {}: {}", path.display(), e)
                    })?;
                }
                log_info!("cleanup: removed {}", path.display());
            }
        }
        // Then delete directories
        for path in &self.dirs {
            if path.exists() {
                fs::remove_dir_all(path).map_err(|e| {
                    format!("cleanup: failed to remove dir {}: {}", path.display(), e)
                })?;
                log_info!("cleanup: removed dir {}", path.display());
            }
        }
        Ok(())
    }
}

/// Shared runtime context for conversion tasks.
pub struct HurrayContext {
    temp_dir: PathBuf,
    shared_data: RwLock<HashMap<String, String>>,
    texture_cache: RwLock<HashMap<PathBuf, RgbaImage>>,
    cleanup: RwLock<CleanupList>,
}

impl HurrayContext {
    pub fn new(temp_dir: &str) -> Self {
        Self {
            temp_dir: PathBuf::from(temp_dir),
            shared_data: RwLock::new(HashMap::new()),
            texture_cache: RwLock::new(HashMap::new()),
            cleanup: RwLock::new(CleanupList::new()),
        }
    }

    /// Register a file for deferred deletion. The file will only be removed
    /// when `execute_cleanup()` is called at the end of conversion.
    pub fn defer_remove_file(&self, path: &Path) {
        let mut cleanup = Self::write_unpoisoned(&self.cleanup, "context.cleanup");
        cleanup.defer_file(path.to_path_buf());
    }

    /// Register a directory for deferred deletion. The directory will only be
    /// removed when `execute_cleanup()` is called at the end of conversion.
    pub fn defer_remove_dir(&self, path: &Path) {
        let mut cleanup = Self::write_unpoisoned(&self.cleanup, "context.cleanup");
        cleanup.defer_dir(path.to_path_buf());
    }

    /// Execute all deferred file/directory deletions. Call this once at the
    /// very end of the conversion pipeline, after all tasks are complete.
    pub fn execute_cleanup(&self) -> Result<(), String> {
        let mut cleanup = Self::write_unpoisoned(&self.cleanup, "context.cleanup");
        let replacement = CleanupList::new();
        let old = std::mem::replace(&mut *cleanup, replacement);
        old.execute()
    }

    pub fn temp_dir(&self) -> &Path {
        &self.temp_dir
    }

    pub fn set_data(&self, key: &str, value: &str) {
        let mut data = Self::write_unpoisoned(&self.shared_data, "context.shared_data");
        data.insert(key.to_string(), value.to_string());
    }

    pub fn get_data(&self, key: &str) -> Option<String> {
        let data = Self::read_unpoisoned(&self.shared_data, "context.shared_data");
        data.get(key).cloned()
    }

    pub fn cache_texture(&self, path: &Path, texture: RgbaImage) {
        let mut cache = Self::write_unpoisoned(&self.texture_cache, "context.texture_cache");
        cache.insert(path.to_path_buf(), texture);
    }

    pub fn get_cached_texture(&self, path: &Path) -> Option<RgbaImage> {
        let cache = Self::read_unpoisoned(&self.texture_cache, "context.texture_cache");
        cache.get(path).cloned()
    }

    pub fn is_texture_cached(&self, path: &Path) -> bool {
        let cache = Self::read_unpoisoned(&self.texture_cache, "context.texture_cache");
        cache.contains_key(path)
    }

    pub fn clear_texture_cache(&self) {
        let mut cache = Self::write_unpoisoned(&self.texture_cache, "context.texture_cache");
        cache.clear();
    }

    fn read_unpoisoned<'a, T>(lock: &'a RwLock<T>, name: &'static str) -> RwLockReadGuard<'a, T> {
        match lock.read() {
            Ok(guard) => guard,
            Err(poisoned) => {
                log_warn!("recovering from poisoned read lock: {}", name);
                poisoned.into_inner()
            }
        }
    }

    fn write_unpoisoned<'a, T>(lock: &'a RwLock<T>, name: &'static str) -> RwLockWriteGuard<'a, T> {
        match lock.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                log_warn!("recovering from poisoned write lock: {}", name);
                poisoned.into_inner()
            }
        }
    }
}