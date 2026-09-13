use crate::hurray::{
    context::HurrayContext,
    texture::TexturePool,
    resolution::ResolutionTransducer,
};
use image::{imageops, RgbaImage};
use std::path::Path;

struct GuiSpriteDef {
    source_name: &'static str,
    target_path: &'static str,
    rect: (u32, u32, u32, u32),
    base_width: u32,
}

enum SplitMode {
    None,
    Horizontal,
    Vertical,
}

const SPRITE_MAP: &[GuiSpriteDef] = &[

    GuiSpriteDef { source_name: "anvil", target_path: "container/anvil/error", rect: (176, 0, 204, 21), base_width: 256 },
    GuiSpriteDef { source_name: "anvil", target_path: "container/anvil/text_field", rect: (0, 166, 110, 182), base_width: 256 },
    GuiSpriteDef { source_name: "anvil", target_path: "container/anvil/text_field_disabled", rect: (0, 182, 110, 198), base_width: 256 },

    GuiSpriteDef { source_name: "beacon", target_path: "container/beacon/button", rect: (0, 219, 22, 241), base_width: 256 },
    GuiSpriteDef { source_name: "beacon", target_path: "container/beacon/button_selected", rect: (22, 219, 44, 241), base_width: 256 },
    GuiSpriteDef { source_name: "beacon", target_path: "container/beacon/button_disabled", rect: (44, 219, 66, 241), base_width: 256 },
    GuiSpriteDef { source_name: "beacon", target_path: "container/beacon/button_highlighted", rect: (66, 219, 88, 241), base_width: 256 },
    GuiSpriteDef { source_name: "beacon", target_path: "container/beacon/confirm", rect: (90, 220, 108, 238), base_width: 256 },
    GuiSpriteDef { source_name: "beacon", target_path: "container/beacon/cancel", rect: (112, 220, 130, 238), base_width: 256 },

    GuiSpriteDef { source_name: "furnace", target_path: "container/furnace/lit_progress", rect: (176, 0, 190, 14), base_width: 256 },
    GuiSpriteDef { source_name: "furnace", target_path: "container/furnace/burn_progress", rect: (176, 14, 200, 31), base_width: 256 },
    GuiSpriteDef { source_name: "blast_furnace", target_path: "container/blast_furnace/lit_progress", rect: (176, 0, 190, 14), base_width: 256 },
    GuiSpriteDef { source_name: "blast_furnace", target_path: "container/blast_furnace/burn_progress", rect: (176, 14, 200, 31), base_width: 256 },
    GuiSpriteDef { source_name: "smoker", target_path: "container/smoker/lit_progress", rect: (176, 0, 190, 14), base_width: 256 },
    GuiSpriteDef { source_name: "smoker", target_path: "container/smoker/burn_progress", rect: (176, 14, 200, 31), base_width: 256 },

    GuiSpriteDef { source_name: "brewing_stand", target_path: "container/brewing_stand/brew_progress", rect: (176, 0, 185, 28), base_width: 256 },
    GuiSpriteDef { source_name: "brewing_stand", target_path: "container/brewing_stand/bubbles", rect: (185, 14, 197, 29), base_width: 256 },
    GuiSpriteDef { source_name: "brewing_stand", target_path: "container/brewing_stand/fuel_length", rect: (176, 29, 194, 33), base_width: 256 },

    GuiSpriteDef { source_name: "inventory", target_path: "container/inventory/effect_background_large", rect: (0, 166, 120, 198), base_width: 256 },
    GuiSpriteDef { source_name: "inventory", target_path: "container/inventory/effect_background_small", rect: (0, 198, 32, 230), base_width: 256 },

    GuiSpriteDef { source_name: "horse", target_path: "container/horse/armor_slot", rect: (0, 220, 18, 238), base_width: 256 },
    GuiSpriteDef { source_name: "horse", target_path: "container/horse/saddle_slot", rect: (18, 220, 36, 238), base_width: 256 },
    GuiSpriteDef { source_name: "horse", target_path: "container/horse/llama_armor_slot", rect: (36, 220, 54, 238), base_width: 256 },
    GuiSpriteDef { source_name: "horse", target_path: "container/horse/chest_slots", rect: (0, 166, 90, 220), base_width: 256 },

    GuiSpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/enchantment_slot", rect: (0, 166, 108, 185), base_width: 256 },
    GuiSpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/enchantment_slot_disabled", rect: (0, 185, 108, 204), base_width: 256 },
    GuiSpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/enchantment_slot_highlighted", rect: (0, 204, 108, 223), base_width: 256 },
    GuiSpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/level_1", rect: (0, 223, 16, 239), base_width: 256 },
    GuiSpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/level_2", rect: (16, 223, 32, 239), base_width: 256 },
    GuiSpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/level_3", rect: (32, 223, 48, 239), base_width: 256 },
    GuiSpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/level_1_disabled", rect: (0, 239, 16, 255), base_width: 256 },
    GuiSpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/level_2_disabled", rect: (16, 239, 32, 255), base_width: 256 },
    GuiSpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/level_3_disabled", rect: (32, 239, 48, 255), base_width: 256 },

    GuiSpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/recipe", rect: (0, 166, 16, 184), base_width: 256 },
    GuiSpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/recipe_selected", rect: (0, 184, 16, 202), base_width: 256 },
    GuiSpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/recipe_highlighted", rect: (0, 202, 16, 220), base_width: 256 },
    GuiSpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/scroller", rect: (176, 0, 188, 15), base_width: 256 },
    GuiSpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/scroller_disabled", rect: (188, 0, 200, 15), base_width: 256 },
    GuiSpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/input_slot", rect: (176, 0, 192, 16), base_width: 256 },
    GuiSpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/output_slot", rect: (192, 0, 208, 16), base_width: 256 },
    GuiSpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/result_slot", rect: (192, 0, 208, 16), base_width: 256 },

    GuiSpriteDef { source_name: "loom", target_path: "container/loom/banner_slot", rect: (176, 0, 192, 16), base_width: 256 },
    GuiSpriteDef { source_name: "loom", target_path: "container/loom/dye_slot", rect: (192, 0, 208, 16), base_width: 256 },
    GuiSpriteDef { source_name: "loom", target_path: "container/loom/pattern_slot", rect: (208, 0, 224, 16), base_width: 256 },
    GuiSpriteDef { source_name: "loom", target_path: "container/loom/pattern", rect: (0, 166, 14, 180), base_width: 256 },
    GuiSpriteDef { source_name: "loom", target_path: "container/loom/pattern_selceted", rect: (0, 180, 14, 194), base_width: 256 },
    GuiSpriteDef { source_name: "loom", target_path: "container/loom/pattern_selected", rect: (0, 180, 14, 194), base_width: 256 },
    GuiSpriteDef { source_name: "loom", target_path: "container/loom/pattern_highlighted", rect: (0, 194, 14, 208), base_width: 256 },
    GuiSpriteDef { source_name: "loom", target_path: "container/loom/scroller", rect: (232, 0, 244, 15), base_width: 256 },
    GuiSpriteDef { source_name: "loom", target_path: "container/loom/scroller_disabled", rect: (244, 0, 256, 15), base_width: 256 },

    GuiSpriteDef { source_name: "smithing", target_path: "container/smithing/error", rect: (176, 0, 204, 21), base_width: 256 },
    GuiSpriteDef { source_name: "smithing", target_path: "container/smithing/template_slot", rect: (16, 0, 32, 16), base_width: 256 },
    GuiSpriteDef { source_name: "smithing", target_path: "container/smithing/base_slot", rect: (32, 0, 48, 16), base_width: 256 },
    GuiSpriteDef { source_name: "smithing", target_path: "container/smithing/addition_slot", rect: (16, 16, 32, 32), base_width: 256 },
    GuiSpriteDef { source_name: "smithing", target_path: "container/smithing/result_slot", rect: (32, 16, 48, 32), base_width: 256 },

    GuiSpriteDef { source_name: "villager2", target_path: "container/villager/discount_strikethrough", rect: (0, 176, 9, 178), base_width: 512 },
    GuiSpriteDef { source_name: "villager2", target_path: "container/villager/experience_bar_result", rect: (0, 181, 102, 186), base_width: 512 },
    GuiSpriteDef { source_name: "villager2", target_path: "container/villager/experience_bar_background", rect: (0, 186, 102, 191), base_width: 512 },
    GuiSpriteDef { source_name: "villager2", target_path: "container/villager/experience_bar_current", rect: (0, 191, 102, 196), base_width: 512 },
    GuiSpriteDef { source_name: "villager2", target_path: "container/villager/trade_arrow", rect: (15, 171, 25, 180), base_width: 512 },
    GuiSpriteDef { source_name: "villager2", target_path: "container/villager/out_of_stuck", rect: (25, 171, 35, 180), base_width: 512 },
    GuiSpriteDef { source_name: "villager2", target_path: "container/villager/out_of_stock", rect: (25, 171, 35, 180), base_width: 512 },
    GuiSpriteDef { source_name: "villager2", target_path: "container/villager/scroller", rect: (0, 199, 6, 226), base_width: 512 },
    GuiSpriteDef { source_name: "villager2", target_path: "container/villager/scroller_disabled", rect: (6, 199, 12, 226), base_width: 512 },

    GuiSpriteDef { source_name: "cartography_table", target_path: "container/cartography_table/duplicated_map", rect: (176, 132, 226, 198), base_width: 256 },
    GuiSpriteDef { source_name: "cartography_table", target_path: "container/cartography_table/scaled_map", rect: (176, 66, 242, 132), base_width: 256 },
    GuiSpriteDef { source_name: "cartography_table", target_path: "container/cartography_table/map", rect: (176, 0, 242, 66), base_width: 256 },
    GuiSpriteDef { source_name: "cartography_table", target_path: "container/cartography_table/locked", rect: (52, 214, 62, 228), base_width: 256 },
    GuiSpriteDef { source_name: "cartography_table", target_path: "container/cartography_table/error", rect: (226, 132, 254, 153), base_width: 256 },

    GuiSpriteDef { source_name: "grindstone", target_path: "container/grindstone/error", rect: (176, 0, 204, 21), base_width: 256 },
    GuiSpriteDef { source_name: "grindstone", target_path: "container/grindstone/input_slot", rect: (30, 53, 48, 71), base_width: 256 },
    GuiSpriteDef { source_name: "grindstone", target_path: "container/grindstone/additional_slot", rect: (66, 53, 84, 71), base_width: 256 },
    GuiSpriteDef { source_name: "grindstone", target_path: "container/grindstone/output_slot", rect: (66, 53, 84, 71), base_width: 256 },
    GuiSpriteDef { source_name: "grindstone", target_path: "container/grindstone/result_slot", rect: (66, 53, 84, 71), base_width: 256 },
];

pub struct GuiSurgeon;

impl GuiSurgeon {
    fn sprite_dir(base_path: &Path, subdir: &str) -> std::path::PathBuf {
        base_path
            .join("assets/minecraft/textures/gui/sprites")
            .join(subdir)
    }

    fn scale_from_image_base(img: &RgbaImage, base_width: u32) -> f32 {
        let width = img.width().max(1);
        let base = base_width.max(1);
        width as f32 / base as f32
    }

    fn scale_from_image(img: &RgbaImage) -> f32 {
        Self::scale_from_image_base(img, 256)
    }

    fn scale_coordinate(scale: f32, coord: u32) -> u32 {
        (coord as f32 * scale).round() as u32
    }

    fn scale_rect(scale: f32, x1: u32, y1: u32, x2: u32, y2: u32) -> (u32, u32, u32, u32) {
        (
            Self::scale_coordinate(scale, x1),
            Self::scale_coordinate(scale, y1),
            Self::scale_coordinate(scale, x2 - x1),
            Self::scale_coordinate(scale, y2 - y1),
        )
    }

    fn save_slices(
        img: &RgbaImage,
        base_path: &Path,
        pool: &mut TexturePool,
        _res: &ResolutionTransducer,
        crop: (u32, u32, u32, u32),
        split: SplitMode,
        slice_size: (u32, u32),
        names: &[&str],
        target_dir: &str,
    ) -> Result<(), String> {
        let scale = Self::scale_from_image(img);
        let (x1, y1, x2, y2) = crop;
        let (rx, ry, rw, rh) = Self::scale_rect(scale, x1, y1, x2, y2);
        let cropped = imageops::crop_imm(img, rx, ry, rw, rh).to_image();

        let slice_w = Self::scale_coordinate(scale, slice_size.0);
        let slice_h = Self::scale_coordinate(scale, slice_size.1);

        let mut slices: Vec<RgbaImage> = Vec::new();

        match split {
            SplitMode::None => {
                slices.push(cropped);
            }
            SplitMode::Horizontal => {
                for i in 0..names.len() {
                    let sx = i as u32 * slice_w;
                    let slice = imageops::crop_imm(&cropped, sx, 0, slice_w, slice_h).to_image();
                    slices.push(slice);
                }
            }
            SplitMode::Vertical => {
                for i in 0..names.len() {
                    let sy = i as u32 * slice_h;
                    let slice = imageops::crop_imm(&cropped, 0, sy, slice_w, slice_h).to_image();
                    slices.push(slice);
                }
            }
        }

        let target_root = Self::sprite_dir(base_path, target_dir);
        for (idx, name) in names.iter().enumerate() {
            if let Some(slice) = slices.get(idx) {
                let target = target_root.join(name);
                pool.store_texture(&target, slice.clone());
            }
        }

        Ok(())
    }

    pub fn execute_transformation(
        ctx: &HurrayContext,
        pool: &mut TexturePool,
        res: &ResolutionTransducer,
    ) -> Result<(), String> {
        crate::log_info!("2-Pyramid: GuiSurgeon cutting UI sprites...");

        let base_path = ctx.temp_dir();

        for def in SPRITE_MAP {
            let src_file = format!("assets/minecraft/textures/gui/container/{}.png", def.source_name);
            let src_path = base_path.join(&src_file);

            if let Ok(img) = pool.load_texture(&src_path) {
                let (x1, y1, x2, y2) = def.rect;
                let scale = Self::scale_from_image_base(&img, def.base_width);
                let (rx1, ry1, rw, rh) = Self::scale_rect(scale, x1, y1, x2, y2);

                let sprite = imageops::crop_imm(&img, rx1, ry1, rw, rh).to_image();

                let target_file = format!("assets/minecraft/textures/gui/sprites/{}.png", def.target_path);
                let target_path = base_path.join(&target_file);
                pool.store_texture(&target_path, sprite);
            }
        }

        Self::process_slider(&base_path, pool, res)?;
        Self::process_icons(&base_path, pool, res)?;
        Self::process_widgets(&base_path, pool, res)?;
        Self::process_tabs(&base_path, pool, res)?;
        Self::process_resource_packs(&base_path, pool, res)?;
        Self::process_server_selection(&base_path, pool, res)?;
        Self::process_title(&base_path, pool, res)?;

        pool.commit_all().map_err(|e| e.to_string())?;

        // Defer cleanup of old container atlas files — they are no longer needed
        // in 1.21+ sprite-based UI. All cleanup is deferred to the end of the
        // conversion pipeline so that no file is deleted before every task has
        // had a chance to read it (e.g. fix_ui_survival needs inventory.png).
        //
        // IMPORTANT: do NOT include `container/inventory.png` here. The 1.21
        // vanilla resource pack still ships an `inventory.png` (a 256x256
        // simplified panel that the client uses as a backwards-compatible
        // rendering surface for the survival inventory background). If we
        // delete it, resource packs that previously had a 1.20.1-style
        // `inventory.png` will be missing it post-conversion and the
        // survival/creative inventory GUI fails to render.
        let cleanup_files: &[&str] = &[
            "assets/minecraft/textures/gui/container/anvil.png",
            "assets/minecraft/textures/gui/container/beacon.png",
            "assets/minecraft/textures/gui/container/furnace.png",
            "assets/minecraft/textures/gui/container/blast_furnace.png",
            "assets/minecraft/textures/gui/container/smoker.png",
            "assets/minecraft/textures/gui/container/brewing_stand.png",
            "assets/minecraft/textures/gui/container/horse.png",
            "assets/minecraft/textures/gui/container/enchanting_table.png",
            "assets/minecraft/textures/gui/container/stonecutter.png",
            "assets/minecraft/textures/gui/container/loom.png",
            "assets/minecraft/textures/gui/container/smithing.png",
            "assets/minecraft/textures/gui/container/villager2.png",
            "assets/minecraft/textures/gui/container/cartography_table.png",
            "assets/minecraft/textures/gui/container/grindstone.png",
            "assets/minecraft/textures/gui/icons.png",
            "assets/minecraft/textures/gui/widgets.png",
            "assets/minecraft/textures/gui/slider.png",
            "assets/minecraft/textures/gui/container/creative_inventory/tabs.png",
            "assets/minecraft/textures/gui/resource_packs.png",
            "assets/minecraft/textures/gui/server_selection.png",
        ];
        for file in cleanup_files {
            let path = base_path.join(file);
            if path.exists() {
                ctx.defer_remove_file(&path);
            }
        }

        Ok(())
    }

    fn process_slider(base_path: &std::path::Path,
        pool: &mut TexturePool,
        _res: &ResolutionTransducer,
    ) -> Result<(), String> {
        let slider_path = base_path.join("assets/minecraft/textures/gui/slider.png");
        
        if !slider_path.exists() {
            crate::log_info!("slider.png not found, skip");
            return Ok(());
        }

        if let Ok(img) = pool.load_texture(&slider_path) {
            let scale = Self::scale_from_image(&img);

            let (x1, y1, w, h) = Self::scale_rect(scale, 0, 0, 200, 20);
            let slider = imageops::crop_imm(&img, x1, y1, w, h).to_image();
            let slider_target = base_path.join("assets/minecraft/textures/gui/sprites/widget/slider.png");
            pool.store_texture(&slider_target, slider);

            let (left_x1, left_y1, left_w, left_h) = Self::scale_rect(scale, 0, 40, 4, 60);
            let (right_x1, right_y1, right_w, right_h) = Self::scale_rect(scale, 196, 40, 200, 60);
            
            let left = imageops::crop_imm(&img, left_x1, left_y1, left_w, left_h).to_image();
            let right = imageops::crop_imm(&img, right_x1, right_y1, right_w, right_h).to_image();
            
            let handle_width = left.width() + right.width();
            let handle_height = left.height();
            let mut handle = RgbaImage::new(handle_width, handle_height);
            
            for y in 0..left.height() {
                for x in 0..left.width() {
                    handle.put_pixel(x, y, *left.get_pixel(x, y));
                }
                for x in 0..right.width() {
                    handle.put_pixel(x + left.width(), y, *right.get_pixel(x, y));
                }
            }
            
            let handle_target = base_path.join("assets/minecraft/textures/gui/sprites/widget/slider_handle.png");
            pool.store_texture(&handle_target, handle);

            let (h_left_x1, h_left_y1, h_left_w, h_left_h) = Self::scale_rect(scale, 0, 60, 4, 80);
            let (h_right_x1, h_right_y1, h_right_w, h_right_h) = Self::scale_rect(scale, 196, 60, 200, 80);
            
            let h_left = imageops::crop_imm(&img, h_left_x1, h_left_y1, h_left_w, h_left_h).to_image();
            let h_right = imageops::crop_imm(&img, h_right_x1, h_right_y1, h_right_w, h_right_h).to_image();
            
            let h_handle_width = h_left.width() + h_right.width();
            let h_handle_height = h_left.height();
            let mut h_handle = RgbaImage::new(h_handle_width, h_handle_height);
            
            for y in 0..h_left.height() {
                for x in 0..h_left.width() {
                    h_handle.put_pixel(x, y, *h_left.get_pixel(x, y));
                }
                for x in 0..h_right.width() {
                    h_handle.put_pixel(x + h_left.width(), y, *h_right.get_pixel(x, y));
                }
            }
            
            let h_handle_target = base_path.join("assets/minecraft/textures/gui/sprites/widget/slider_handle_highlighted.png");
            pool.store_texture(&h_handle_target, h_handle);
        }

        Ok(())
    }

    fn process_icons(
        base_path: &std::path::Path,
        pool: &mut TexturePool,
        res: &ResolutionTransducer,
    ) -> Result<(), String> {
        let icons_path = base_path.join("assets/minecraft/textures/gui/icons.png");
        
        if !icons_path.exists() {
            crate::log_info!("icons.png not found, skip");
            return Ok(());
        }

        if let Ok(img) = pool.load_texture(&icons_path) {
            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (0, 0, 15, 15),
                SplitMode::None,
                (15, 15),
                &["crosshair.png"],
                "hud",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (16, 0, 196, 9),
                SplitMode::Horizontal,
                (9, 9),
                &[
                    "container.png",
                    "container_blinking.png",
                    "wtf.png",
                    "wtf2.png",
                    "full.png",
                    "half.png",
                    "full_blinking.png",
                    "half_blinking.png",
                    "poisoned_full.png",
                    "poisoned_half.png",
                    "poisoned_full_blinking.png",
                    "poisoned_half_blinking.png",
                    "withered_full.png",
                    "withered_half.png",
                    "withered_full_blinking.png",
                    "withered_half_blinking.png",
                    "absorbing_full.png",
                    "absorbing_half.png",
                    "frozen_full.png",
                    "frozen_half.png",
                ],
                "hud/heart",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (16, 9, 124, 18),
                SplitMode::Horizontal,
                (9, 9),
                &["armor_empty.png", "armor_half.png", "armor_full.png", "wtf3.png"],
                "hud",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (52, 9, 124, 18),
                SplitMode::Horizontal,
                (9, 9),
                &[
                    "vehicle_container.png",
                    "wtf4.png",
                    "wtf5.png",
                    "wtf6.png",
                    "vehicle_full.png",
                    "vehicle_half.png",
                    "wtf7.png",
                    "wtf8.png",
                ],
                "hud/heart",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (16, 18, 52, 27),
                SplitMode::Horizontal,
                (9, 9),
                &["air.png", "air_bursting.png", "wtf9.png", "wtf10.png"],
                "hud",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (16, 27, 142, 36),
                SplitMode::Horizontal,
                (9, 9),
                &[
                    "food_empty.png",
                    "wtf11.png",
                    "wtf123.png",
                    "wtf13.png",
                    "food_full.png",
                    "food_half.png",
                    "wtf14.png",
                    "wtf15.png",
                    "food_full_hunger.png",
                    "food_half_hunger.png",
                    "wtf16.png",
                    "wtf17.png",
                    "wtf18.png",
                    "food_empty_hunger.png",
                ],
                "hud",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (16, 45, 196, 54),
                SplitMode::Horizontal,
                (9, 9),
                &[
                    "container_hardcore.png",
                    "container_hardcore_blinking.png",
                    "wtf19.png",
                    "wtf20.png",
                    "hardcore_full.png",
                    "hardcore_half.png",
                    "hardcore_full_blinking.png",
                    "hardcore_half_blinking.png",
                    "poisoned_hardcore_full.png",
                    "poisoned_hardcore_half.png",
                    "poisoned_hardcore_full_blinking.png",
                    "poisoned_hardcore_half_blinking.png",
                    "withered_hardcore_full.png",
                    "withered_hardcore_half.png",
                    "withered_hardcore_full_blinking.png",
                    "withered_hardcore_half_blinking.png",
                    "absorbing_hardcore_full.png",
                    "absorbing_hardcore_half.png",
                    "frozen_hardcore_full.png",
                    "frozen_hardcore_half.png",
                ],
                "hud/heart",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (0, 15, 10, 63),
                SplitMode::Vertical,
                (10, 8),
                &[
                    "ping_5.png",
                    "ping_4.png",
                    "ping_3.png",
                    "ping_2.png",
                    "ping_1.png",
                    "ping_unknown.png",
                ],
                "icon",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (0, 64, 182, 94),
                SplitMode::Vertical,
                (182, 5),
                &[
                    "experience_bar_background.png",
                    "experience_bar_progress.png",
                    "jump_bar_cooldown.png",
                    "wtf21.png",
                    "jump_bar_background.png",
                    "jump_bar_progress.png",
                ],
                "hud",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (0, 94, 18, 112),
                SplitMode::None,
                (18, 18),
                &["hotbar_attack_indicator_background.png"],
                "hud",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (18, 94, 36, 112),
                SplitMode::None,
                (18, 18),
                &["hotbar_attack_indicator_progress.png"],
                "hud",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (36, 94, 52, 98),
                SplitMode::None,
                (16, 4),
                &["crosshair_attack_indicator_background.png"],
                "hud",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (52, 94, 68, 98),
                SplitMode::None,
                (16, 4),
                &["crosshair_attack_indicator_progress.png"],
                "hud",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (68, 94, 84, 110),
                SplitMode::None,
                (16, 16),
                &["crosshair_attack_indicator_full.png"],
                "hud",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (0, 176, 10, 224),
                SplitMode::Vertical,
                (10, 8),
                &[
                    "ping_5.png",
                    "ping_4.png",
                    "ping_3.png",
                    "ping_2.png",
                    "ping_1.png",
                    "unreachable.png",
                ],
                "server_list",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (10, 176, 20, 216),
                SplitMode::Vertical,
                (10, 8),
                &[
                    "pinging_5.png",
                    "pinging_4.png",
                    "pinging_3.png",
                    "pinging_2.png",
                    "pinging_1.png",
                ],
                "server_list",
            )?;
        }

        Ok(())
    }

    fn process_widgets(
        base_path: &std::path::Path,
        pool: &mut TexturePool,
        res: &ResolutionTransducer,
    ) -> Result<(), String> {
        let widgets_path = base_path.join("assets/minecraft/textures/gui/widgets.png");
        
        if !widgets_path.exists() {
            crate::log_info!("widgets.png not found, skip");
            return Ok(());
        }

        if let Ok(img) = pool.load_texture(&widgets_path) {
            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (0, 0, 182, 22),
                SplitMode::None,
                (182, 22),
                &["hotbar.png"],
                "hud",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (0, 22, 24, 45),
                SplitMode::None,
                (24, 23),
                &["hotbar_selection.png"],
                "hud",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (24, 22, 53, 46),
                SplitMode::None,
                (29, 24),
                &["hotbar_offhand_left.png"],
                "hud",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (53, 22, 82, 46),
                SplitMode::None,
                (29, 24),
                &["hotbar_offhand_right.png"],
                "hud",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (0, 46, 200, 66),
                SplitMode::None,
                (200, 20),
                &["button_disabled.png"],
                "widget",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (0, 66, 200, 86),
                SplitMode::None,
                (200, 20),
                &["button.png"],
                "widget",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (0, 86, 200, 106),
                SplitMode::None,
                (200, 20),
                &["button_highlighted.png"],
                "widget",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (3, 109, 18, 124),
                SplitMode::None,
                (15, 15),
                &["language.png"],
                "icon",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (0, 146, 20, 206),
                SplitMode::Vertical,
                (20, 20),
                &[
                    "locked_button.png",
                    "locked_button_highlighted.png",
                    "locked_button_disabled.png",
                ],
                "widget",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (20, 146, 40, 206),
                SplitMode::Vertical,
                (20, 20),
                &[
                    "unlocked_button.png",
                    "unlocked_button_highlighted.png",
                    "unlocked_button_disabled.png",
                ],
                "widget",
            )?;
        }

        Ok(())
    }

    fn process_tabs(
        base_path: &std::path::Path,
        pool: &mut TexturePool,
        _res: &ResolutionTransducer,
    ) -> Result<(), String> {
        let tabs_path = base_path.join("assets/minecraft/textures/gui/container/creative_inventory/tabs.png");
        
        if !tabs_path.exists() {
            crate::log_info!("tabs.png not found, skip");
            return Ok(());
        }

        if let Ok(img) = pool.load_texture(&tabs_path) {
            let scale = Self::scale_from_image(&img);
            let has_seventh = |y: u32, height: u32| -> bool {
                let (x1, y1, w, h) = Self::scale_rect(scale, 168, y, 196, y + height);
                for yy in y1..(y1 + h) {
                    for xx in x1..(x1 + w) {
                        if img.get_pixel(xx, yy).0[3] > 0 {
                            return true;
                        }
                    }
                }
                false
            };

            let mut store_tabs = |crop_rect: (u32, u32, u32, u32),
                                slice_size: (u32, u32),
                                names: [&str; 7],
                                has_seventh_row: bool|
             -> Result<(), String> {
                let (cx, cy, _cw, ch) = crop_rect;
                let base_width = if has_seventh_row { slice_size.0 * 7 } else { slice_size.0 * 6 };
                let (x1, y1, w, h) = Self::scale_rect(scale, cx, cy, cx + base_width, cy + ch);
                let cropped = imageops::crop_imm(&img, x1, y1, w, h).to_image();

                let slice_width = Self::scale_coordinate(scale, slice_size.0);
                let slice_height = Self::scale_coordinate(scale, slice_size.1);

                let mut last_slice = None;
                let count = if has_seventh_row { 7 } else { 6 };
                for i in 0..count {
                    let tab = imageops::crop_imm(
                        &cropped,
                        i * slice_width,
                        0,
                        slice_width,
                        slice_height,
                    )
                    .to_image();
                    let target = base_path.join(format!(
                        "assets/minecraft/textures/gui/sprites/container/creative_inventory/{}",
                        names[i as usize]
                    ));
                    pool.store_texture(&target, tab.clone());
                    if i + 1 == count {
                        last_slice = Some(tab);
                    }
                }

                if !has_seventh_row {
                    if let Some(tab7) = last_slice {
                        let target = base_path.join(format!(
                            "assets/minecraft/textures/gui/sprites/container/creative_inventory/{}",
                            names[6]
                        ));
                        pool.store_texture(&target, tab7);
                    }
                }

                Ok(())
            };

            store_tabs(
                (0, 2, 168, 30),
                (28, 30),
                [
                    "tab_top_unselected_1.png",
                    "tab_top_unselected_2.png",
                    "tab_top_unselected_3.png",
                    "tab_top_unselected_4.png",
                    "tab_top_unselected_5.png",
                    "tab_top_unselected_6.png",
                    "tab_top_unselected_7.png",
                ],
                has_seventh(2, 30),
            )?;

            store_tabs(
                (0, 32, 168, 32),
                (28, 32),
                [
                    "tab_top_selected_1.png",
                    "tab_top_selected_2.png",
                    "tab_top_selected_3.png",
                    "tab_top_selected_4.png",
                    "tab_top_selected_5.png",
                    "tab_top_selected_6.png",
                    "tab_top_selected_7.png",
                ],
                has_seventh(32, 32),
            )?;

            store_tabs(
                (0, 64, 168, 30),
                (28, 30),
                [
                    "tab_bottom_unselected_1.png",
                    "tab_bottom_unselected_2.png",
                    "tab_bottom_unselected_3.png",
                    "tab_bottom_unselected_4.png",
                    "tab_bottom_unselected_5.png",
                    "tab_bottom_unselected_6.png",
                    "tab_bottom_unselected_7.png",
                ],
                has_seventh(64, 30),
            )?;

            store_tabs(
                (0, 96, 168, 32),
                (28, 32),
                [
                    "tab_bottom_selected_1.png",
                    "tab_bottom_selected_2.png",
                    "tab_bottom_selected_3.png",
                    "tab_bottom_selected_4.png",
                    "tab_bottom_selected_5.png",
                    "tab_bottom_selected_6.png",
                    "tab_bottom_selected_7.png",
                ],
                has_seventh(96, 32),
            )?;
        }

        Ok(())
    }

    fn process_resource_packs(
        base_path: &std::path::Path,
        pool: &mut TexturePool,
        res: &ResolutionTransducer,
    ) -> Result<(), String> {
        let resource_packs_path = base_path.join("assets/minecraft/textures/gui/resource_packs.png");
        
        if !resource_packs_path.exists() {
            crate::log_info!("resource_packs.png not found, skip");
            return Ok(());
        }

        if let Ok(img) = pool.load_texture(&resource_packs_path) {
            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (0, 0, 128, 32),
                SplitMode::Horizontal,
                (32, 32),
                &["select.png", "unselect.png", "move_down.png", "move_up.png"],
                "transferable_list",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (0, 32, 128, 64),
                SplitMode::Horizontal,
                (32, 32),
                &[
                    "select_highlighted.png",
                    "unselect_highlighted.png",
                    "move_down_highlighted.png",
                    "move_up_highlighted.png",
                ],
                "transferable_list",
            )?;
        }

        Ok(())
    }

    fn process_server_selection(
        base_path: &std::path::Path,
        pool: &mut TexturePool,
        res: &ResolutionTransducer,
    ) -> Result<(), String> {
        let server_selection_path =
            base_path.join("assets/minecraft/textures/gui/server_selection.png");

        if !server_selection_path.exists() {
            crate::log_info!("server_selection.png not found, skip");
            return Ok(());
        }

        if let Ok(img) = pool.load_texture(&server_selection_path) {
            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (0, 0, 128, 32),
                SplitMode::Horizontal,
                (32, 32),
                &["join.png", "emm.png", "move_down.png", "move_up.png"],
                "server_list",
            )?;

            Self::save_slices(
                &img,
                base_path,
                pool,
                res,
                (0, 32, 128, 64),
                SplitMode::Horizontal,
                (32, 32),
                &[
                    "join_highlighted.png",
                    "emmm.png",
                    "move_down_highlighted.png",
                    "move_up_highlighted.png",
                ],
                "server_list",
            )?;
        }

        Ok(())
    }

    fn process_title(
        base_path: &std::path::Path,
        pool: &mut TexturePool,
        res: &ResolutionTransducer,
    ) -> Result<(), String> {
        let title_path = base_path.join("assets/minecraft/textures/gui/title/minecraft.png");

        if !title_path.exists() {
            crate::log_info!("title/minecraft.png not found, skip");
            return Ok(());
        }

        if let Ok(img) = pool.load_texture(&title_path) {
            let scale = Self::scale_from_image(&img);
            let (rx, ry, rw, rh) = Self::scale_rect(scale, 0, 94, 200, 194);
            let realms = imageops::crop_imm(&img, rx, ry, rw, rh).to_image();
            let realms_target = Self::sprite_dir(base_path, "title").join("realms.png");
            pool.store_texture(&realms_target, realms);

            let (x1, y1, w1, h1) = Self::scale_rect(scale, 0, 0, 155, 44);
            let part1 = imageops::crop_imm(&img, x1, y1, w1, h1).to_image();

            let (x2, y2, w2, h2) = Self::scale_rect(scale, 0, 45, 119, 89);
            let part2 = imageops::crop_imm(&img, x2, y2, w2, h2).to_image();

            let concat_width = part1.width() + part2.width();
            let concat_height = part1.height().max(part2.height());
            let mut concatenated = RgbaImage::new(concat_width, concat_height);
            imageops::overlay(&mut concatenated, &part1, 0, 0);
            imageops::overlay(&mut concatenated, &part2, part1.width() as i64, 0);

            let transparent_width = Self::scale_coordinate(scale, 274);
            let transparent_height = Self::scale_coordinate(scale, 25);
            let final_width = transparent_width.max(concatenated.width());
            let final_height = concatenated.height() + transparent_height;

            let mut final_img = RgbaImage::new(final_width, final_height);
            imageops::overlay(&mut final_img, &concatenated, 0, 0);

            let title_target = Self::sprite_dir(base_path, "title").join("minecraft.png");
            pool.store_texture(&title_target, final_img.clone());
            pool.store_texture(&title_path, final_img);
        }

        Ok(())
    }
}
