use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

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
    /// 输出包名（**只读**）。
    ///
    /// §9.93（M3）：它原本经 `shared_data` 这个「任务间可变共享表」传递，但**从未被任何任务改写**——
    /// 只有一个写入点（转换入口）与一个读取点（Bedrock 的 `convert_java_to_bedrock`，
    /// 用于决定输出的 `.mcpack` 文件名）。既是只读，就应当是**构造期字段**而不是可变侧信道。
    pack_name: String,
    /// Arc 共享贴图：并行读取只 clone 指针，不复制整图。
    texture_cache: RwLock<HashMap<PathBuf, Arc<RgbaImage>>>,
    cleanup: RwLock<CleanupList>,
}

impl HurrayContext {
    /// 带包名构造（转换入口用）。
    pub fn with_pack_name(temp_dir: &str, pack_name: &str) -> Self {
        Self {
            temp_dir: PathBuf::from(temp_dir),
            pack_name: pack_name.to_string(),
            texture_cache: RwLock::new(HashMap::new()),
            cleanup: RwLock::new(CleanupList::new()),
        }
    }

    /// 不带包名的构造（测试与不关心包名的调用点）。包名取旧实现的兜底值 `"resource_pack"`。
    pub fn new(temp_dir: &str) -> Self {
        Self::with_pack_name(temp_dir, "resource_pack")
    }

    /// 输出包名（只读）。
    pub fn pack_name(&self) -> &str {
        &self.pack_name
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

    /// **取出并清空**延迟删除清单（§9.99）。
    ///
    /// 给「workdir 形态的调停者」用：它代替旧的适配层调用某个任务，需要把该任务**登记的**
    /// 延迟删除**转交**给驱动（经 `Outcome.deferred_removals`）以便在收尾时机统一应用；
    /// 取出后立刻从本清单移除，避免与 `execute_cleanup()` **重复处理**。
    ///
    /// 返回工作目录下的**相对路径**（与 `Outcome.deferred_removals` 的约定一致）；
    /// 若登记的路径不在 `temp_dir` 之下，则原样返回其字符串形式。
    pub fn take_cleanup_paths(&self) -> Vec<String> {
        let mut cleanup = Self::write_unpoisoned(&self.cleanup, "context.cleanup");
        let old = std::mem::replace(&mut *cleanup, CleanupList::new());
        let mut out: Vec<String> = Vec::new();
        for p in old.files.iter().chain(old.dirs.iter()) {
            let rel = p
                .strip_prefix(&self.temp_dir)
                .map(|r| r.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| p.to_string_lossy().replace('\\', "/"));
            out.push(rel);
        }
        out
    }

    pub fn temp_dir(&self) -> &Path {
        &self.temp_dir
    }

    // §9.93（M3）：`set_data` / `get_data` / `shared_data` **已删除**。
    // 它原本只承载两个键：`pack_name`（纯进度显示标签 → 已改为显式传参给调度器）
    // 与 `target_pack_format`（旧 `adapt_java_shaders` 读它 → 原生实现改为直接读
    // `pack.mcmeta`，见 §9.85）。二者都不再需要"任务间共享可变状态"这一机制。

    pub fn cache_texture(&self, path: &Path, texture: RgbaImage) {
        let mut cache = Self::write_unpoisoned(&self.texture_cache, "context.texture_cache");
        cache.insert(path.to_path_buf(), Arc::new(texture));
    }

    pub fn get_cached_texture(&self, path: &Path) -> Option<Arc<RgbaImage>> {
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