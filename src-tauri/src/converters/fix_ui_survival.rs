// fix_ui_survival编码.rs
use std::fs;
use std::path::{Path, PathBuf};
use std::env;
use image::{imageops, RgbaImage};
use crate::converters::get_uimage_path;
use crate::hurray::context::HurrayContext;
use crate::image_utils::paste_region;

/// 修复生存模式背包界面
/// 遵循 Hurray Engine + RS 标准
/// - 坐标定义: 基于 16x 的基准坐标
/// - IO 处理: 使用 HurrayContext 的像素Buffer
/// - 并发控制: 注册为 Exclusive 任务
/// - 跨版本支持: 支持版本 6..34
pub fn fix_ui_survival(context: &HurrayContext) -> Result<(), String> {
    let temp_dir = context.temp_dir();
    let gui_path = temp_dir.join("assets/minecraft/textures/gui/container");
    let inventory_path = gui_path.join("inventory.png");

    if !inventory_path.exists() { return Ok(()); }

    // 从 HurrayContext 获取纹理，或从文件加载
    let mut img = if context.is_texture_cached(&inventory_path) {
        context.get_cached_texture(&inventory_path).ok_or_else(|| "texture cache inconsistency for inventory.png".to_string())?
    } else {
        image::open(&inventory_path).map_err(|e| e.to_string())?.to_rgba8()
    };

    let (width, height) = img.dimensions();
    
    // Based on pack.py: scale factor derived from inventory.png size
    let scale_factor = match determine_scale_factor(width, height) {
        Ok(scale) => scale,
        Err(reason) => {
            crate::log_info!("fix_ui_survival skipped: {}", reason);
            return Ok(());
        }
    };
    let scaled = |coord: u32| (coord as f32 * scale_factor) as u32;

    // --- 步骤 1: 提取 Mob Effects (基于 16x 坐标) ---
    let mob_effect_path = temp_dir.join("assets/minecraft/textures/mob_effect");
    fs::create_dir_all(&mob_effect_path).map_err(|e| e.to_string())?;

    let mob_effect_names = vec![
        vec!["speed.png", "slowness.png", "haste.png", "mining_fatigue.png", "strength.png", "weakness.png", "poison.png", "regeneration.png"],
        vec!["invisibility.png", "hunger.png", "jump_boost.png", "nausea.png", "night_vision.png", "blindness.png", "resistance.png", "fire_resistance.png"],
        vec!["water_breathing.png", "wither.png", "absorption.png"]
    ];

    // 基于 16x 的图标大小
    let base_icon_size = 18;
    let icon_size = scaled(base_icon_size);
    
    for (row_idx, row) in mob_effect_names.iter().enumerate() {
        for (col_idx, name) in row.iter().enumerate() {
            // 基于 16x 的坐标计算
            let base_x = col_idx as u32 * base_icon_size;
            let base_y = 198 + (row_idx as u32 * base_icon_size);
            
            let x = scaled(base_x);
            let y = scaled(base_y);
            
            // 裁剪图标
            let icon = imageops::crop_imm(&img, x, y, icon_size, icon_size).to_image();
            icon.save(mob_effect_path.join(name)).map_err(|e| e.to_string())?;
        }
    }

    // --- 步骤 2: 像素平移与填充 (基于 16x 坐标) ---
    // 对应 Python: move_region(img, 86, 24, 162, 62, 10, -8)
    move_region(&mut img, 86, 24, 162, 62, 10, -8, scale_factor);

    // 对应 Python: color_fill_region (填充背景)
    fill_region(&mut img, 75, 6, 96, 80, 90, 10, scale_factor);
    fill_region(&mut img, 96, 54, 162, 62, 90, 10, scale_factor);

    // 对应 Python: copy_and_paste_region
    copy_paste(&mut img, 152, 26, 172, 46, 75, 60, scale_factor);

    // --- 步骤 3: 外部 Inventory 模板覆盖 (核心兼容逻辑) ---
    let exe_path = env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    let exe_folder = exe_path.parent().unwrap_or(Path::new("."));

    let inventory_template_name = match width {
        256 => "inventory_256.png",
        512 => "inventory_512.png",
        1024 => "inventory_1024.png",
        2048 => "inventory_2048.png",
        _ => {
            crate::log_info!("fix_ui_survival unsupported inventory size: {}x{}", width, height);
            return Ok(());
        }
    };

    let inventory_folder = get_uimage_path()
        .map(|path| path.join("inventory"))
        .unwrap_or_else(|_| exe_folder.join("inventory"));
    let template_path = inventory_folder.join(inventory_template_name);

    if template_path.exists() {
        let overlay = image::open(&template_path).map_err(|e| e.to_string())?.to_rgba8();
        let overlay_resized = if overlay.dimensions() != img.dimensions() {
            imageops::resize(&overlay, width, height, imageops::FilterType::Lanczos3)
        } else {
            overlay
        };

        // 使用 imageops 的内置 overlay，它处理 Alpha 混合比手动循环稳定得多
        imageops::overlay(&mut img, &overlay_resized, 0, 0);
    }

    // --- 步骤 4: 额外新增 - 1.21 生成碎图 (Sprites) ---
    save_inventory_sprites(temp_dir, &img, scale_factor)?;

    // 缓存修改后的纹理到 HurrayContext
    context.cache_texture(&inventory_path, img.clone());

    // 保存最终修复的大图
    img.save(&inventory_path).map_err(|e| e.to_string())?;

    Ok(())
}

/// 兼容层：支持旧的函数签名
/// 用于与现有代码集成
pub fn fix_ui_survival_compat(temp_dir: &str) -> Result<(), String> {
    use crate::hurray::context::HurrayContext;
    let context = HurrayContext::new(temp_dir);
    fix_ui_survival(&context)
}

// --- 辅助工具函数 (基于 16x 坐标) ---

fn move_region(img: &mut RgbaImage, x1: u32, y1: u32, x2: u32, y2: u32, dx: i32, dy: i32, scale_factor: f32) {
    let scaled = |coord: u32| (coord as f32 * scale_factor) as u32;
    
    // 1. 裁剪出要移动的像素块
    let src_x = scaled(x1);
    let src_y = scaled(y1);
    let src_w = scaled(x2 - x1);
    let src_h = scaled(y2 - y1);
    let region = imageops::crop_imm(img, src_x, src_y, src_w, src_h).to_image();
    
    // 2. 计算目标位置
    let target_x = ((x1 as i32 + dx) as f32 * scale_factor).max(0.0) as u32;
    let target_y = ((y1 as i32 + dy) as f32 * scale_factor).max(0.0) as u32;
    
    // 3. 把**源位置**擦成**目标位置**的颜色(目标位置此时还是原图背景,src 已被 crop 完
    //    不会受影响),然后把 region 覆盖到目标位置。
    //    之前用 imageops::overlay 只搬不擦 → 留下原位置 + 目标位置两套贴图(用户报的 bug)。
    //    之前我的 fix 是从 src_x 取 bg_color,但 src_x 本身就是被搬的色块,等于"用红色当背景
    //    擦红色",所以是 no-op。正确做法:目标位置(尚未被 paste)处的颜色 = 此处应有的背景。
    let bg_color = *img.get_pixel(target_x, target_y);
    fill_rect_solid(img, src_x, src_y, src_w, src_h, bg_color);
    paste_region(img, &region, target_x, target_y).map_err(|e| format!("paste_region: {}", e)).ok();
}

/// Fill a solid-color rectangle — used to clear the destination of a
/// move_region operation before pasting the moved block, otherwise the
/// source row stays behind and the move looks like a copy.
fn fill_rect_solid(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, color: image::Rgba<u8>) {
    let (img_w, img_h) = img.dimensions();
    let x2 = (x + w).min(img_w);
    let y2 = (y + h).min(img_h);
    for yy in y..y2 {
        for xx in x..x2 {
            img.put_pixel(xx, yy, color);
        }
    }
}

fn fill_region(img: &mut RgbaImage, x1: u32, y1: u32, x2: u32, y2: u32, cx: u32, cy: u32, scale_factor: f32) {
    let scaled = |coord: u32| (coord as f32 * scale_factor) as u32;
    
    let color = *img.get_pixel(scaled(cx), scaled(cy));
    for y in scaled(y1)..scaled(y2) {
        for x in scaled(x1)..scaled(x2) {
            img.put_pixel(x, y, color);
        }
    }
}

fn copy_paste(img: &mut RgbaImage, x1: u32, y1: u32, x2: u32, y2: u32, tx: u32, ty: u32, scale_factor: f32) {
    let scaled = |coord: u32| (coord as f32 * scale_factor) as u32;
    
    let region = imageops::crop_imm(
        img, 
        scaled(x1), 
        scaled(y1), 
        scaled(x2 - x1), 
        scaled(y2 - y1)
    ).to_image();
    
    // Use paste (raw overwrite) to match Pillow's `Image.paste(region, box)`,
    // not alpha-blend overlay — inventory's "slot" pixels must be replaced
    // bit-for-bit, otherwise the underlying background bleeds through.
    paste_region(img, &region, scaled(tx), scaled(ty)).map_err(|e| format!("paste_region: {}", e)).ok();
}

fn save_inventory_sprites(temp_dir: &Path, img: &RgbaImage, scale_factor: f32) -> Result<(), String> {
    let scaled = |coord: u32| (coord as f32 * scale_factor) as u32;
    
    let sprite_path = temp_dir.join("assets/minecraft/textures/gui/sprites/container/inventory");
    fs::create_dir_all(&sprite_path).map_err(|e| e.to_string())?;

    // 裁剪 1.21+ 药水背景 (基于 16x 坐标)
    let regions = [
        ((0, 166, 120, 198), "effect_background_large.png"),
        ((0, 198, 32, 230), "effect_background_small.png"),
    ];

    for ((x1, y1, x2, y2), name) in regions {
        let cropped = imageops::crop_imm(
            img, 
            scaled(x1), 
            scaled(y1), 
            scaled(x2 - x1), 
            scaled(y2 - y1)
        ).to_image();
        cropped.save(sprite_path.join(name)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn determine_scale_factor(width: u32, height: u32) -> Result<f32, String> {
    if width == 256 && height == 256 {
        Ok(1.0)
    } else if width == 512 && height == 512 {
        Ok(2.0)
    } else if width == 1024 && height == 1024 {
        Ok(4.0)
    } else if width == 2048 && height == 2048 {
        Ok(8.0)
    } else {
        Err(format!("unsupported inventory.png size: {}x{}", width, height))
    }
}

/// 注册生存模式背包界面修复任务
///
/// # 参数
/// - `engine`: Hurray 引擎
pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    use crate::hurray::scheduler::{TaskType, TaskTier};
    engine.register_task(
        "fix_ui_survival",
        TaskType::Exclusive,
        TaskTier::Surgeon,
        fix_ui_survival
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    /// 256x256 inventory.png 在 (86, 24) 起 76x38 的 hotbar 块被 +10/-8 移到 (96, 16)。
    /// Bug 报告: 旧实现用 imageops::overlay(只搬不擦),导致原位置(86, 24)的红色块
    /// 残留在原处,新位置(96, 16)又出现红块 — 出现两套贴图。
    /// 正确行为: 原位置 (86, 24) 应被擦成该处的背景色(这里用 BG),新位置 (96, 16)
    /// 必须是红色块,无任何原位残留。
    #[test]
    fn move_region_clears_source_and_pastes_at_destination() {
        // 背景: 半透明青(这样肉眼能区分"被擦成背景"和"原位置贴图残留")
        let bg = Rgba([40, 80, 200, 255]);
        // 移动块: 不透明亮红
        let block = Rgba([220, 30, 30, 255]);

        let mut img = RgbaImage::from_pixel(256, 256, bg);

        // 在 (86, 24) 起 76x38 区域涂满红色
        for y in 24..24 + 38 {
            for x in 86..86 + 76 {
                img.put_pixel(x, y, block);
            }
        }

        // 拍快照:原位置(86, 24) 必须是红色(确认初始状态)
        assert_eq!(*img.get_pixel(86, 24), block, "precondition: source is red");
        assert_eq!(*img.get_pixel(96, 16), bg, "precondition: destination is background");

        // 行动:把 (86, 24, 162, 62) 区域 +10/-8 移到 (96, 16)
        move_region(&mut img, 86, 24, 162, 62, 10, -8, 1.0);

        // 验证 1:新位置 (96, 16) 起 76x38 应是红色块
        for y in 16..16 + 38 {
            for x in 96..96 + 76 {
                assert_eq!(
                    *img.get_pixel(x, y),
                    block,
                    "destination ({},{}) should be the moved red block, got {:?}",
                    x, y, img.get_pixel(x, y)
                );
            }
        }

        // 验证 2:原位置 (86, 24) 起 76x38 必须是背景色(被擦掉了)
        // 注意:原位置(86, 24)起 76x38 = (86..162, 24..62),新位置是 (96..172, 16..54),
        // 两块在左上角(96..162, 24..54)有 66x30 的重叠,所以我们只检查不重叠的部分:
        //   原位置特有区域: (86..96, 24..62) — 左侧 10 像素
        //   原位置特有区域: (96..162, 54..62) — 底 8 像素
        for y in 24..62 {
            for x in 86..96 {
                assert_eq!(
                    *img.get_pixel(x, y),
                    bg,
                    "source ({},{}) must be erased to background, got {:?}",
                    x, y, img.get_pixel(x, y)
                );
            }
        }
        for y in 54..62 {
            for x in 96..162 {
                assert_eq!(
                    *img.get_pixel(x, y),
                    bg,
                    "source bottom strip ({},{}) must be erased, got {:?}",
                    x, y, img.get_pixel(x, y)
                );
            }
        }
    }

    /// copy_paste 同样应该用 paste(覆盖)而不是 overlay(混合)。
    /// 测试:把不透明红色块从 (152, 26) 拷到 (75, 60),目标位置之前是
    /// 半透明绿色,拷贝后必须完全覆盖成红色,不允许绿色渗出。
    #[test]
    fn copy_paste_overwrites_destination() {
        let bg = Rgba([40, 80, 200, 255]);
        let block = Rgba([220, 30, 30, 255]);

        let mut img = RgbaImage::from_pixel(256, 256, bg);

        // 在 (152, 26) 起 20x20 涂红(源)
        for y in 26..26 + 20 {
            for x in 152..152 + 20 {
                img.put_pixel(x, y, block);
            }
        }

        copy_paste(&mut img, 152, 26, 172, 46, 75, 60, 1.0);

        // 目标位置 (75, 60) 起 20x20 必须是红色
        for y in 60..60 + 20 {
            for x in 75..75 + 20 {
                assert_eq!(*img.get_pixel(x, y), block);
            }
        }
    }
}