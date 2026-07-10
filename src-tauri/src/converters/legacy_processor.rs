use std::fs;
use std::path::Path;

use image::{imageops, GenericImage, GenericImageView, Rgba, RgbaImage};

use crate::color_utils::{hsva_to_rgba, rgba_to_hsva};
use crate::hurray::context::HurrayContext;
use crate::hurray::resolution::ResolutionTransducer;

pub struct LegacyProcessor;

impl LegacyProcessor {
    pub fn run_architect_tasks(ctx: &HurrayContext, res: &ResolutionTransducer) -> Result<(), String> {
        let temp_dir = ctx.temp_dir();

        Self::generate_all_planks(temp_dir)?;
        Self::generate_metal_series(temp_dir)?;
        Self::generate_consumables(temp_dir, res)?;
        Self::generate_boats(temp_dir)?;

        Ok(())
    }

    pub fn run_surgeon_tasks(ctx: &HurrayContext, res: &ResolutionTransducer) -> Result<(), String> {
        let temp_dir = ctx.temp_dir();

        Self::process_container_uis(temp_dir)?;
        Self::process_complex_uis(temp_dir, res)?;

        Ok(())
    }

    fn generate_all_planks(base: &Path) -> Result<(), String> {
        let block_path = base.join("assets/minecraft/textures/block");
        let oak = block_path.join("oak_planks.png");
        if !oak.exists() {
            return Ok(());
        }

        let img = image::open(&oak)
            .map_err(|e| format!("open {} failed: {}", oak.display(), e))?
            .to_rgba8();

        let variants = [
            ("pale_oak_planks.png", 0, 30, -100),
            ("mangrove_planks.png", -59, -15, 0),
            ("cherry_planks.png", -80, 40, 0),
            ("bamboo_planks.png", 25, 20, 0),
        ];

        for (name, h, b, s) in variants {
            let output = block_path.join(name);
            let processed = Self::adjust_color(&img, h, b, s);
            processed
                .save(&output)
                .map_err(|e| format!("save {} failed: {}", output.display(), e))?;
        }

        Ok(())
    }

    fn generate_metal_series(base: &Path) -> Result<(), String> {
        let item_path = base.join("assets/minecraft/textures/item");
        let block_path = base.join("assets/minecraft/textures/block");
        let armor_path = base.join("assets/minecraft/textures/models/armor");

        Self::generate_filtered(
            &item_path.join("gold_ingot.png"),
            &item_path.join("netherite_ingot.png"),
            Self::apply_netherite_filter,
        )?;
        Self::generate_filtered(
            &block_path.join("diamond_block.png"),
            &block_path.join("netherite_block.png"),
            Self::apply_netherite_filter,
        )?;
        Self::generate_filtered(
            &block_path.join("iron_block.png"),
            &block_path.join("copper_block.png"),
            Self::apply_copper_filter,
        )?;

        for tool in ["sword", "pickaxe", "axe", "shovel", "hoe"] {
            Self::generate_filtered(
                &item_path.join(format!("diamond_{tool}.png")),
                &item_path.join(format!("netherite_{tool}.png")),
                Self::apply_netherite_filter,
            )?;
        }

        for piece in ["helmet", "chestplate", "leggings", "boots"] {
            Self::generate_filtered(
                &item_path.join(format!("diamond_{piece}.png")),
                &item_path.join(format!("netherite_{piece}.png")),
                Self::apply_netherite_filter,
            )?;
        }

        Self::generate_filtered(
            &armor_path.join("diamond_layer_1.png"),
            &armor_path.join("netherite_layer_1.png"),
            Self::apply_netherite_filter,
        )?;
        Self::generate_filtered(
            &armor_path.join("diamond_layer_2.png"),
            &armor_path.join("netherite_layer_2.png"),
            Self::apply_netherite_filter,
        )?;

        Ok(())
    }

    fn generate_consumables(base: &Path, _res: &ResolutionTransducer) -> Result<(), String> {
        let item_path = base.join("assets/minecraft/textures/item");

        Self::copy_if_exists(
            &item_path.join("arrow.png"),
            &item_path.join("tipped_arrow_base.png"),
        )?;

        let potion_path = item_path.join("potion.png");
        if potion_path.exists() {
            let mut pot = image::open(&potion_path)
                .map_err(|e| format!("open {} failed: {}", potion_path.display(), e))?
                .to_rgba8();

            let (w, h) = pot.dimensions();
            if w > 0 {
                for i in 0..(h / w) {
                    let y_offset = i * w;
                    for px in 0..w {
                        for py in 0..(w / 3) {
                            pot.put_pixel(px, y_offset + py, Rgba([0, 0, 0, 0]));
                        }
                    }
                }
            }

            let out = item_path.join("lingering_potion.png");
            pot.save(&out)
                .map_err(|e| format!("save {} failed: {}", out.display(), e))?;
        }

        let water_bucket = item_path.join("water_bucket.png");
        if water_bucket.exists() {
            for fish in ["cod", "salmon", "pufferfish", "tropical_fish"] {
                Self::copy_if_exists(&water_bucket, &item_path.join(format!("{fish}_bucket.png")))?;
            }
        }

        Ok(())
    }

    fn process_container_uis(base: &Path) -> Result<(), String> {
        let container = base.join("assets/minecraft/textures/gui/container");
        let furnace = container.join("furnace.png");
        if !furnace.exists() {
            return Ok(());
        }

        Self::copy_if_exists(&furnace, &container.join("blast_furnace.png"))?;
        Self::copy_if_exists(&furnace, &container.join("smoker.png"))?;
        Ok(())
    }

    fn generate_boats(base: &Path) -> Result<(), String> {
        let item_path = base.join("assets/minecraft/textures/item");
        let boat = item_path.join("boat.png");
        if !boat.exists() {
            return Ok(());
        }

        for variant in [
            "oak_boat.png",
            "birch_boat.png",
            "acacia_boat.png",
            "dark_oak_boat.png",
            "jungle_boat.png",
        ] {
            Self::copy_if_exists(&boat, &item_path.join(variant))?;
        }

        let spruce = item_path.join("spruce_boat.png");
        if spruce.exists() {
            fs::remove_file(&spruce)
                .map_err(|e| format!("remove {} failed: {}", spruce.display(), e))?;
        }
        fs::rename(&boat, &spruce)
            .map_err(|e| format!("rename {} -> {} failed: {}", boat.display(), spruce.display(), e))?;

        Ok(())
    }

    fn process_complex_uis(base: &Path, res: &ResolutionTransducer) -> Result<(), String> {
        let container = base.join("assets/minecraft/textures/gui/container");
        let scale = res.get_scale_factor();

        let generic = container.join("generic_54.png");
        if generic.exists() {
            let mut gui = image::open(&generic)
                .map_err(|e| format!("open {} failed: {}", generic.display(), e))?
                .to_rgba8();

            let crop_y = (127.0 * scale).round() as u32;
            let crop_w = (176.0 * scale).round() as u32;
            let crop_h = (95.0 * scale).round() as u32;
            let paste_y = (71.0 * scale).round() as i64;

            if crop_y + crop_h <= gui.height() && crop_w <= gui.width() {
                let sub_image = imageops::crop_imm(&gui, 0, crop_y, crop_w, crop_h).to_image();
                imageops::replace(&mut gui, &sub_image, 0, paste_y);
                let out = container.join("shulker_box.png");
                gui.save(&out)
                    .map_err(|e| format!("save {} failed: {}", out.display(), e))?;
            }
        }

        let anvil = container.join("anvil.png");
        if anvil.exists() {
            let img = image::open(&anvil)
                .map_err(|e| format!("open {} failed: {}", anvil.display(), e))?
                .to_rgba8();
            let out = container.join("smithing.png");
            img.save(&out)
                .map_err(|e| format!("save {} failed: {}", out.display(), e))?;
        }

        Ok(())
    }

    fn generate_filtered(
        src: &Path,
        dst: &Path,
        filter: fn(&mut RgbaImage),
    ) -> Result<(), String> {
        if !src.exists() {
            return Ok(());
        }

        let mut img = image::open(src)
            .map_err(|e| format!("open {} failed: {}", src.display(), e))?
            .to_rgba8();
        filter(&mut img);
        img.save(dst)
            .map_err(|e| format!("save {} failed: {}", dst.display(), e))
    }

    fn copy_if_exists(src: &Path, dst: &Path) -> Result<(), String> {
        if !src.exists() {
            return Ok(());
        }

        fs::copy(src, dst)
            .map_err(|e| format!("copy {} -> {} failed: {}", src.display(), dst.display(), e))?;
        Ok(())
    }

    fn apply_netherite_filter(img: &mut RgbaImage) {
        for pixel in img.pixels_mut() {
            if pixel[3] == 0 {
                continue;
            }

            let (mut h, mut s, mut v, a) = rgba_to_hsva(pixel[0], pixel[1], pixel[2], pixel[3]);
            h = (h + 0.02).fract();
            s = (s * 0.65).clamp(0.0, 1.0);
            v = (v * 0.55).clamp(0.0, 1.0);
            let rgba = hsva_to_rgba(h, s, v, a);
            *pixel = Rgba(rgba);
        }
    }

    fn apply_copper_filter(img: &mut RgbaImage) {
        for pixel in img.pixels_mut() {
            if pixel[3] == 0 {
                continue;
            }

            let (mut h, mut s, mut v, a) = rgba_to_hsva(pixel[0], pixel[1], pixel[2], pixel[3]);
            h = (h + 0.07).fract();
            s = (s * 1.15).clamp(0.0, 1.0);
            v = (v * 0.92).clamp(0.0, 1.0);
            let rgba = hsva_to_rgba(h, s, v, a);
            *pixel = Rgba(rgba);
        }
    }

    fn adjust_color(img: &RgbaImage, h: i32, b: i32, s: i32) -> RgbaImage {
        let hue_shift = h as f32 / 360.0;
        let brightness = 1.0 + (b as f32 / 255.0);
        let saturation = 1.0 + (s as f32 / 255.0);

        let mut out = img.clone();
        for pixel in out.pixels_mut() {
            if pixel[3] == 0 {
                continue;
            }

            let (mut hh, mut ss, mut vv, aa) = rgba_to_hsva(pixel[0], pixel[1], pixel[2], pixel[3]);
            hh = (hh + hue_shift).rem_euclid(1.0);
            ss = (ss * saturation).clamp(0.0, 1.0);
            vv = (vv * brightness).clamp(0.0, 1.0);
            let rgba = hsva_to_rgba(hh, ss, vv, aa);
            *pixel = Rgba(rgba);
        }

        out
    }
}