use std::path::Path;

use image::GenericImageView;

use crate::hurray::error::{EngineError, EngineResult};
use crate::{log_info, log_warn};

/// Detects texture-pack resolution and provides coordinate scaling helpers.
///
/// 分 item / block / gui 三路采样，取众数倍率；主倍率优先 item。
/// 混合分辨率包（如 32x 物品 + 16x 方块 + 1x GUI）会分别记录并在日志中告警，
/// 避免用 `inventory.png` 宽度/16 之类的错误基准把 GUI 判成 16 倍。
pub struct ResolutionTransducer {
    /// 主倍率：item > gui > block > 1.0（兼容旧调用方）
    scale_factor: f32,
    item_scale: f32,
    block_scale: f32,
    gui_scale: f32,
    base_resolution: u32,
}

/// 从若干候选倍率中选众数（平票取较大，便于高分材质）。
fn mode_scale(samples: &[f32]) -> Option<f32> {
    if samples.is_empty() {
        return None;
    }
    let mut counts: Vec<(f32, usize)> = Vec::new();
    for &s in samples {
        match counts.iter_mut().find(|(v, _)| (*v - s).abs() < 0.01) {
            Some((_, c)) => *c += 1,
            None => counts.push((s, 1)),
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)));
    Some(counts[0].0)
}

fn sample_scales(root: &Path, relatives: &[&str], base_px: f32) -> Vec<f32> {
    let mut out = Vec::new();
    for rel in relatives {
        let full = root.join(rel);
        if !full.exists() {
            continue;
        }
        match image::open(&full) {
            Ok(img) => {
                let (w, _) = img.dimensions();
                let w = w.max(1) as f32;
                let scale = w / base_px;
                if scale > 0.0 {
                    out.push(scale);
                }
            }
            Err(e) => {
                log_warn!("resolution probe decode failed {}: {}", full.display(), e);
            }
        }
    }
    out
}

impl ResolutionTransducer {
    pub fn new() -> Self {
        Self {
            scale_factor: 1.0,
            item_scale: 1.0,
            block_scale: 1.0,
            gui_scale: 1.0,
            base_resolution: 16,
        }
    }

    pub fn detect_resolution(&mut self, resource_pack_path: &Path) -> EngineResult<()> {
        // 物品：传统 items/ 与现代 item/
        let item_samples = sample_scales(
            resource_pack_path,
            &[
                "assets/minecraft/textures/item/diamond.png",
                "assets/minecraft/textures/item/apple.png",
                "assets/minecraft/textures/item/stick.png",
                "assets/minecraft/textures/item/iron_ingot.png",
                "assets/minecraft/textures/item/gold_ingot.png",
                "assets/minecraft/textures/item/coal.png",
                "assets/minecraft/textures/items/diamond.png",
                "assets/minecraft/textures/items/apple.png",
                "assets/minecraft/textures/items/stick.png",
            ],
            16.0,
        );

        // 方块
        let block_samples = sample_scales(
            resource_pack_path,
            &[
                "assets/minecraft/textures/block/stone.png",
                "assets/minecraft/textures/block/dirt.png",
                "assets/minecraft/textures/block/oak_planks.png",
                "assets/minecraft/textures/block/cobblestone.png",
                "assets/minecraft/textures/block/bedrock.png",
                "assets/minecraft/textures/blocks/stone.png",
                "assets/minecraft/textures/blocks/dirt.png",
            ],
            16.0,
        );

        // GUI：vanilla 基准 256（inventory / icons / widgets）
        let gui_samples = sample_scales(
            resource_pack_path,
            &[
                "assets/minecraft/textures/gui/container/inventory.png",
                "assets/minecraft/textures/gui/icons.png",
                "assets/minecraft/textures/gui/widgets.png",
                "assets/minecraft/textures/gui/container/generic_54.png",
                "assets/minecraft/textures/gui/container/crafting_table.png",
            ],
            256.0,
        );

        self.item_scale = mode_scale(&item_samples).unwrap_or(0.0);
        self.block_scale = mode_scale(&block_samples).unwrap_or(0.0);
        self.gui_scale = mode_scale(&gui_samples).unwrap_or(0.0);

        // 主倍率：优先物品（与物品/坐标变换最相关），其次 GUI，再次方块
        self.scale_factor = if self.item_scale > 0.0 {
            self.item_scale
        } else if self.gui_scale > 0.0 {
            self.gui_scale
        } else if self.block_scale > 0.0 {
            self.block_scale
        } else {
            1.0
        };

        log_info!(
            "resolution probe: item={:.2} block={:.2} gui={:.2} primary={:.2} (base={}px)",
            self.item_scale,
            self.block_scale,
            self.gui_scale,
            self.scale_factor,
            self.base_resolution
        );

        let mixed = (self.item_scale > 0.0
            && self.block_scale > 0.0
            && (self.item_scale - self.block_scale).abs() > 0.05)
            || (self.item_scale > 0.0
                && self.gui_scale > 0.0
                && (self.item_scale - self.gui_scale).abs() > 0.05)
            || (self.block_scale > 0.0
                && self.gui_scale > 0.0
                && (self.block_scale - self.gui_scale).abs() > 0.05);
        if mixed {
            log_warn!(
                "mixed resource-pack resolutions (item={:.2}, block={:.2}, gui={:.2}); \
                 GUI / inventory alignment may need per-area handling",
                self.item_scale,
                self.block_scale,
                self.gui_scale
            );
        }

        if self.item_scale <= 0.0 && self.block_scale <= 0.0 && self.gui_scale <= 0.0 {
            self.scale_factor = 1.0;
            log_warn!("resolution probe texture not found, fallback scale=1.0");
        }

        Ok(())
    }

    pub fn get_scale_factor(&self) -> f32 {
        self.scale_factor
    }

    pub fn get_item_scale(&self) -> f32 {
        if self.item_scale > 0.0 {
            self.item_scale
        } else {
            self.scale_factor
        }
    }

    pub fn get_block_scale(&self) -> f32 {
        if self.block_scale > 0.0 {
            self.block_scale
        } else {
            self.scale_factor
        }
    }

    /// GUI / 图集倍率（相对 vanilla 256 宽）。
    pub fn get_gui_scale(&self) -> f32 {
        if self.gui_scale > 0.0 {
            self.gui_scale
        } else {
            self.scale_factor
        }
    }

    /// 混合分辨率时返回 (item, block, gui)；一致时仍返回各自值。
    pub fn scales(&self) -> (f32, f32, f32) {
        (self.get_item_scale(), self.get_block_scale(), self.get_gui_scale())
    }

    pub fn scale_coordinate(&self, coord: u32) -> u32 {
        (coord as f32 * self.scale_factor).round() as u32
    }

    pub fn scale_coordinate_with(&self, scale: f32, coord: u32) -> u32 {
        (coord as f32 * scale).round() as u32
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_picks_majority() {
        assert_eq!(mode_scale(&[2.0, 2.0, 1.0]), Some(2.0));
        assert_eq!(mode_scale(&[]), None);
    }

    #[test]
    fn default_scales_one() {
        let r = ResolutionTransducer::new();
        assert!((r.get_scale_factor() - 1.0).abs() < 1e-6);
        assert!((r.get_gui_scale() - 1.0).abs() < 1e-6);
    }
}
