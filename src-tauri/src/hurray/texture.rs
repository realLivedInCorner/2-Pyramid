use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use image::RgbaImage;

use crate::hurray::context::HurrayContext;
use crate::hurray::error::{EngineError, EngineResult};
use crate::{log_error, log_info, log_warn};

#[derive(Debug)]
pub enum BusEvent {
    Write { path: PathBuf, texture: RgbaImage },
    Shutdown,
}

pub struct TexturePool {
    context: Option<Arc<HurrayContext>>,
    textures: HashMap<PathBuf, RgbaImage>,
    dirty_paths: HashSet<PathBuf>,
    bus_sender: Option<mpsc::Sender<BusEvent>>,
    io_thread_handle: Option<thread::JoinHandle<()>>,
}

impl TexturePool {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel::<BusEvent>();
        let io_thread = thread::spawn(move || {
            log_info!("texture io worker started");
            while let Ok(event) = receiver.recv() {
                match event {
                    BusEvent::Write { path, texture } => {
                        if let Some(parent) = path.parent() {
                            if let Err(e) = std::fs::create_dir_all(parent) {
                                log_error!(
                                    "failed to create output dir for {}: {}",
                                    path.display(),
                                    e
                                );
                                continue;
                            }
                        }

                        if let Err(e) = texture.save(&path) {
                            log_error!("failed to save texture {}: {}", path.display(), e);
                        } else {
                            log_info!("texture written: {}", path.display());
                        }
                    }
                    BusEvent::Shutdown => {
                        log_info!("texture io worker shutdown");
                        break;
                    }
                }
            }
        });

        Self {
            context: None,
            textures: HashMap::new(),
            dirty_paths: HashSet::new(),
            bus_sender: Some(sender),
            io_thread_handle: Some(io_thread),
        }
    }

    pub fn initialize(&mut self, context: Arc<HurrayContext>) {
        self.context = Some(context);
    }

    pub fn load_texture(&mut self, path: &Path) -> EngineResult<RgbaImage> {
        if let Some(texture) = self.textures.get(path) {
            return Ok(texture.clone());
        }

        if let Some(context) = &self.context {
            if let Some(texture) = context.get_cached_texture(path) {
                self.textures.insert(path.to_path_buf(), texture.clone());
                return Ok(texture);
            }
        }

        let texture = image::open(path)
            .map_err(|e| EngineError::image("load_texture", path, e))?
            .to_rgba8();

        self.textures.insert(path.to_path_buf(), texture.clone());
        if let Some(context) = &self.context {
            context.cache_texture(path, texture.clone());
        }

        Ok(texture)
    }

    pub fn get_texture(&self, path: &Path) -> Option<&RgbaImage> {
        self.textures.get(path)
    }

    pub fn get_texture_mut(&mut self, path: &Path) -> Option<&mut RgbaImage> {
        self.textures.get_mut(path)
    }

    pub fn store_texture(&mut self, path: &Path, texture: RgbaImage) {
        self.textures.insert(path.to_path_buf(), texture);
        self.dirty_paths.insert(path.to_path_buf());
    }

    pub fn commit_and_release(&mut self, path: &Path) -> EngineResult<()> {
        let texture = match self.textures.remove(path) {
            Some(texture) => texture,
            None => return Ok(()),
        };

        self.dirty_paths.remove(path);

        let sender = self
            .bus_sender
            .as_ref()
            .ok_or(EngineError::ChannelClosed("texture_pool.bus_sender"))?;

        sender
            .send(BusEvent::Write {
                path: path.to_path_buf(),
                texture,
            })
            .map_err(|_| EngineError::ChannelClosed("texture_pool.bus_sender"))
    }

    pub fn commit_all(&mut self) -> EngineResult<()> {
        if self.dirty_paths.is_empty() {
            return Ok(());
        }

        // Collect and drain before doing parallel work
        let mut pending_items = Vec::new();
        for path in self.dirty_paths.drain() {
            if let Some(texture) = self.textures.remove(&path) {
                pending_items.push((path, texture));
            }
        }

        // Shutdown IO-thread worker gracefully
        self.shutdown_io_thread();

        let total = pending_items.len();
        log_info!("Parallel committing {} modified textures...", total);

        let processed = std::sync::atomic::AtomicUsize::new(0);

        // Use Rayon for parallel PNG encoding and saving
        let save_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pending_items.par_iter().for_each(|(path, texture)| {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Err(e) = texture.save(path) {
                    log_error!("failed to save texture {}: {}", path.display(), e);
                }

                let current = processed.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                if current % 50 == 0 || current == total {
                    let percent = (current * 100) / total;
                    log_info!("Progress: {}/{} ({}%) - Committing textures", current, total, percent);
                }
            });
        }));

        log_info!("Parallel texture commit complete.");

        // Always restart the IO thread, even if panic occurred above
        self.restart_io_thread();

        match save_result {
            Ok(()) => Ok(()),
            Err(_) => {
                log_error!("texture commit_all panicked during parallel write");
                Err(EngineError::Message("texture commit_all panicked".to_string()))
            }
        }
    }

    fn restart_io_thread(&mut self) {
        let (sender, receiver) = mpsc::channel::<BusEvent>();
        let io_thread = thread::spawn(move || {
            while let Ok(event) = receiver.recv() {
                match event {
                    BusEvent::Write { path, texture } => {
                        if let Some(parent) = path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let _ = texture.save(&path);
                    }
                    BusEvent::Shutdown => break,
                }
            }
        });
        self.bus_sender = Some(sender);
        self.io_thread_handle = Some(io_thread);
    }

    pub fn clear(&mut self) {
        self.shutdown_io_thread();
        self.textures.clear();
        self.dirty_paths.clear();
    }

    pub fn clear_unused(&mut self) {
        if self.textures.is_empty() {
            return;
        }

        let before = self.textures.len();
        self.textures.retain(|path, _| self.dirty_paths.contains(path));
        let after = self.textures.len();
        log_warn!(
            "texture pool cleanup: dropped {} inactive textures, kept {} dirty textures",
            before.saturating_sub(after),
            after
        );
    }

    fn shutdown_io_thread(&mut self) {
        if let Some(sender) = &self.bus_sender {
            let _ = sender.send(BusEvent::Shutdown);
        }

        if let Some(handle) = self.io_thread_handle.take() {
            if handle.join().is_err() {
                log_error!("texture io worker join failed");
            }
        }

        self.bus_sender = None;
    }
}

impl Drop for TexturePool {
    fn drop(&mut self) {
        self.shutdown_io_thread();
    }
}