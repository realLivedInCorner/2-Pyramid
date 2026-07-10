use std::path::Path;

use image::GenericImageView;

use crate::hurray::error::{EngineError, EngineResult};
use crate::{log_info, log_warn};

/// Detects texture-pack resolution and provides coordinate scaling helpers.
pub struct ResolutionTransducer {
    scale_factor: f32,
    base_resolution: u32,
}

impl ResolutionTransducer {
    pub fn new() -> Self {
        Self {
            scale_factor: 1.0,
            base_resolution: 16,
        }
    }

    pub fn detect_resolution(&mut self, resource_pack_path: &Path) -> EngineResult<()> {
        let probe_paths = [
            "assets/minecraft/textures/item/diamond.png",
            "assets/minecraft/textures/items/diamond.png",
            "assets/minecraft/textures/block/stone.png",
            "assets/minecraft/textures/blocks/stone.png",
            "assets/minecraft/textures/gui/container/inventory.png",
        ];

        for relative in probe_paths {
            let full_path = resource_pack_path.join(relative);
            if !full_path.exists() {
                continue;
            }

            let image = image::open(&full_path)
                .map_err(|e| EngineError::image("detect_resolution", &full_path, e))?;
            let (width, _) = image.dimensions();
            self.scale_factor = width as f32 / self.base_resolution as f32;
            log_info!(
                "detected resource-pack resolution {}px from {} (scale={:.2})",
                width,
                full_path.display(),
                self.scale_factor
            );
            return Ok(());
        }

        self.scale_factor = 1.0;
        log_warn!("resolution probe texture not found, fallback scale=1.0");
        Ok(())
    }

    pub fn get_scale_factor(&self) -> f32 {
        self.scale_factor
    }

    pub fn scale_coordinate(&self, coord: u32) -> u32 {
        (coord as f32 * self.scale_factor).round() as u32
    }

    pub fn scale_rect(&self, x: u32, y: u32, width: u32, height: u32) -> (u32, u32, u32, u32) {
        (
            self.scale_coordinate(x),
            self.scale_coordinate(y),
            self.scale_coordinate(width),
            self.scale_coordinate(height),
        )
    }

    pub fn unscale_coordinate(&self, coord: u32) -> u32 {
        if self.scale_factor <= f32::EPSILON {
            return coord;
        }
        (coord as f32 / self.scale_factor).round() as u32
    }
}