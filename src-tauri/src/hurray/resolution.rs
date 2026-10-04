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

/// 采样一批候选贴图的宽度，换算成相对 `base_px` 的倍率。
///
/// **§9.95：读尺寸的职责交给调用方**（`read_dims`）。这样同一套算法既能跑在
/// 「临时目录 + 磁盘」上（旧执行器），也能跑在「A-ROM 视图 + Tx」上（原生任务），
/// 而不必为后者复制一份采样/众数逻辑。
///
/// `read_dims` 返回 `None` = 该路径**不可用**（不存在或解码失败），本函数**静默跳过**。
/// 之所以不在函数内记日志：旧实现对「文件不存在」是**静默**的、只对「解码失败」记 warn，
/// 这个区分只能在**知道原因的那一层**（闭包内）表达。由闭包按需记日志，语义才逐字一致。
fn sample_scales_with<F>(relatives: &[&str], base_px: f32, mut read_dims: F) -> Vec<f32>
where
    F: FnMut(&str) -> Option<(u32, u32)>,
{
    let mut out = Vec::new();
    for rel in relatives {
        if let Some((w, _)) = read_dims(rel) {
            let w = w.max(1) as f32;
            let scale = w / base_px;
            if scale > 0.0 {
                out.push(scale);
            }
        }
    }
    out
}

/// 旧的磁盘版采样：`root.join(rel)` 是否存在 + `image::open` 能否解码。
fn sample_scales(root: &Path, relatives: &[&str], base_px: f32) -> Vec<f32> {
    sample_scales_with(relatives, base_px, |rel| {
        let full = root.join(rel);
        if !full.exists() {
            return None; // 旧实现：不存在 → 静默跳过（不记日志）
        }
        match image::open(&full) {
            Ok(img) => Some(img.dimensions()),
            Err(e) => {
                log_warn!("resolution probe decode failed {}: {}", full.display(), e);
                None
            }
        }
    })
}

/// 三路采样的**路径清单**（与旧实现逐字一致）。
const ITEM_PROBES: &[&str] = &[
    "assets/minecraft/textures/item/diamond.png",
    "assets/minecraft/textures/item/apple.png",
    "assets/minecraft/textures/item/stick.png",
    "assets/minecraft/textures/item/iron_ingot.png",
    "assets/minecraft/textures/item/gold_ingot.png",
    "assets/minecraft/textures/item/coal.png",
    "assets/minecraft/textures/items/diamond.png",
    "assets/minecraft/textures/items/apple.png",
    "assets/minecraft/textures/items/stick.png",
];
const BLOCK_PROBES: &[&str] = &[
    "assets/minecraft/textures/block/stone.png",
    "assets/minecraft/textures/block/dirt.png",
    "assets/minecraft/textures/block/oak_planks.png",
    "assets/minecraft/textures/block/cobblestone.png",
    "assets/minecraft/textures/block/bedrock.png",
    "assets/minecraft/textures/blocks/stone.png",
    "assets/minecraft/textures/blocks/dirt.png",
];
const GUI_PROBES: &[&str] = &[
    "assets/minecraft/textures/gui/container/inventory.png",
    "assets/minecraft/textures/gui/icons.png",
    "assets/minecraft/textures/gui/widgets.png",
    "assets/minecraft/textures/gui/container/generic_54.png",
    "assets/minecraft/textures/gui/container/crafting_table.png",
];

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

    /// 旧入口（磁盘）：从 `resource_pack_path` 下读三路候选贴图。
    pub fn detect_resolution(&mut self, resource_pack_path: &Path) -> EngineResult<()> {
        let item_samples = sample_scales(resource_pack_path, ITEM_PROBES, 16.0);
        let block_samples = sample_scales(resource_pack_path, BLOCK_PROBES, 16.0);
        let gui_samples = sample_scales(resource_pack_path, GUI_PROBES, 256.0);
        self.apply_samples(&item_samples, &block_samples, &gui_samples);
        Ok(())
    }

    /// **§9.95：不碰磁盘的入口**——读尺寸由调用方提供。
    ///
    /// 原生任务（`Tx` 侧）用这个：它只有 A-ROM 视图，没有可用的临时目录。
    /// 采样清单、众数、三路优先级、混合分辨率告警与兜底**全部复用同一套代码**，
    /// 因此两种入口在同一份输入上必然得到同一结果（由测试钉住）。
    pub fn detect_resolution_with<F>(&mut self, mut read_dims: F)
    where
        F: FnMut(&str) -> Option<(u32, u32)>,
    {
        let item_samples = sample_scales_with(ITEM_PROBES, 16.0, &mut read_dims);
        let block_samples = sample_scales_with(BLOCK_PROBES, 16.0, &mut read_dims);
        let gui_samples = sample_scales_with(GUI_PROBES, 256.0, &mut read_dims);
        self.apply_samples(&item_samples, &block_samples, &gui_samples);
    }

    /// 由三路采样得出倍率（**旧 `detect_resolution` 的后半段，逐字搬来**）。
    fn apply_samples(&mut self, item_samples: &[f32], block_samples: &[f32], gui_samples: &[f32]) {
        self.item_scale = mode_scale(item_samples).unwrap_or(0.0);
        self.block_scale = mode_scale(block_samples).unwrap_or(0.0);
        self.gui_scale = mode_scale(gui_samples).unwrap_or(0.0);

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

    /// **§9.95：注入式入口与读盘入口必须给出同一结果**。
    ///
    /// 这是把 `ResolutionTransducer` 从「只能读盘」改成「可注入读尺寸」的验收：
    /// 两者共用同一套采样/众数/优先级代码，因此在同一份输入上必然同值；
    /// 本用例把这个「必然」钉住——若将来有人给某一侧单独改行为，这里会红。
    #[test]
    fn injected_and_disk_entries_agree() {
        use std::collections::HashMap;
        use std::fs;
        use tempfile::tempdir;

        /// 造一份「32x 物品 + 32x 方块 + 512 GUI」的假包（三路都命中，且倍率一致）。
        /// 基准：item/block 以 16px 为基准 → 32px 即 2.0；GUI 以 256 为基准 → 512 即 2.0。
        fn make(dir: &Path) -> () {
            let put = |rel: &str, w: u32, h: u32| {
                let full = dir.join(rel);
                fs::create_dir_all(full.parent().unwrap()).unwrap();
                let img = image::RgbaImage::new(w, h);
                img.save(&full).unwrap();
            };
            put("assets/minecraft/textures/item/diamond.png", 32, 32);
            put("assets/minecraft/textures/item/apple.png", 32, 32);
            put("assets/minecraft/textures/block/stone.png", 32, 32);
            put("assets/minecraft/textures/block/dirt.png", 32, 32);
            put("assets/minecraft/textures/gui/container/inventory.png", 512, 512);
            put("assets/minecraft/textures/gui/icons.png", 512, 512);
        }

        let tmp = tempdir().expect("tempdir");
        make(tmp.path());

        // ① 读盘入口
        let mut disk = ResolutionTransducer::new();
        disk.detect_resolution(tmp.path()).expect("disk detect");

        // ② 注入式入口：从同一目录按需读尺寸（模拟 A-ROM 视图侧）
        let mut injected = ResolutionTransducer::new();
        injected.detect_resolution_with(|rel| {
            let full = tmp.path().join(rel);
            image::open(&full).ok().map(|i| i.dimensions())
        });

        assert!(
            (disk.get_item_scale() - injected.get_item_scale()).abs() < 1e-6,
            "item 倍率不同：disk={} injected={}",
            disk.get_item_scale(),
            injected.get_item_scale()
        );
        assert!(
            (disk.get_block_scale() - injected.get_block_scale()).abs() < 1e-6,
            "block 倍率不同"
        );
        assert!(
            (disk.get_gui_scale() - injected.get_gui_scale()).abs() < 1e-6,
            "gui 倍率不同"
        );
        assert!(
            (disk.get_scale_factor() - injected.get_scale_factor()).abs() < 1e-6,
            "主倍率不同"
        );
        // 非空转：夹具确实被采到了（否则两侧都是 1.0 兜底，"相等"毫无意义）
        assert!(
            (injected.get_item_scale() - 2.0).abs() < 1e-6,
            "夹具应采到 2.0，实际 {}",
            injected.get_item_scale()
        );

        // ③ 另一份「三路都缺」的输入：两侧都应落到 1.0 兜底
        let empty = tempdir().expect("tempdir");
        let mut d2 = ResolutionTransducer::new();
        d2.detect_resolution(empty.path()).expect("disk detect empty");
        let mut i2 = ResolutionTransducer::new();
        i2.detect_resolution_with(|_| None);
        assert!((d2.get_scale_factor() - 1.0).abs() < 1e-6);
        assert!((i2.get_scale_factor() - 1.0).abs() < 1e-6);
        let _ = HashMap::<u8, u8>::new();
    }
}
