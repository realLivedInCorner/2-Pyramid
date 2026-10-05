    use super::*;
    use image::imageops;

    /// 一条 sprite 的切图定义（与 `converters/ui/gui_surgeon.rs::GuiSpriteDef` 字段一致）。
    struct SpriteDef {
        source_name: &'static str,
        target_path: &'static str,
        rect: (u32, u32, u32, u32),
        base_width: u32,
    }

    /// 与旧 `SPRITE_MAP` **逐条一致**（§9.97 记过：一个字节错就会改变产物）。
    /// 顺序即语义（同名 target 会被后者覆盖），故照抄原顺序。
    const SPRITE_MAP: &[SpriteDef] = &[
        SpriteDef { source_name: "anvil", target_path: "container/anvil/error", rect: (176, 0, 204, 21), base_width: 256 },
        SpriteDef { source_name: "anvil", target_path: "container/anvil/text_field", rect: (0, 166, 110, 182), base_width: 256 },
        SpriteDef { source_name: "anvil", target_path: "container/anvil/text_field_disabled", rect: (0, 182, 110, 198), base_width: 256 },
        SpriteDef { source_name: "beacon", target_path: "container/beacon/button", rect: (0, 219, 22, 241), base_width: 256 },
        SpriteDef { source_name: "beacon", target_path: "container/beacon/button_selected", rect: (22, 219, 44, 241), base_width: 256 },
        SpriteDef { source_name: "beacon", target_path: "container/beacon/button_disabled", rect: (44, 219, 66, 241), base_width: 256 },
        SpriteDef { source_name: "beacon", target_path: "container/beacon/button_highlighted", rect: (66, 219, 88, 241), base_width: 256 },
        SpriteDef { source_name: "beacon", target_path: "container/beacon/confirm", rect: (90, 220, 108, 238), base_width: 256 },
        SpriteDef { source_name: "beacon", target_path: "container/beacon/cancel", rect: (112, 220, 130, 238), base_width: 256 },
        SpriteDef { source_name: "furnace", target_path: "container/furnace/lit_progress", rect: (176, 0, 190, 14), base_width: 256 },
        SpriteDef { source_name: "furnace", target_path: "container/furnace/burn_progress", rect: (176, 14, 200, 31), base_width: 256 },
        SpriteDef { source_name: "blast_furnace", target_path: "container/blast_furnace/lit_progress", rect: (176, 0, 190, 14), base_width: 256 },
        SpriteDef { source_name: "blast_furnace", target_path: "container/blast_furnace/burn_progress", rect: (176, 14, 200, 31), base_width: 256 },
        SpriteDef { source_name: "smoker", target_path: "container/smoker/lit_progress", rect: (176, 0, 190, 14), base_width: 256 },
        SpriteDef { source_name: "smoker", target_path: "container/smoker/burn_progress", rect: (176, 14, 200, 31), base_width: 256 },
        SpriteDef { source_name: "brewing_stand", target_path: "container/brewing_stand/brew_progress", rect: (176, 0, 185, 28), base_width: 256 },
        SpriteDef { source_name: "brewing_stand", target_path: "container/brewing_stand/bubbles", rect: (185, 14, 197, 29), base_width: 256 },
        SpriteDef { source_name: "brewing_stand", target_path: "container/brewing_stand/fuel_length", rect: (176, 29, 194, 33), base_width: 256 },
        SpriteDef { source_name: "inventory", target_path: "container/inventory/effect_background_large", rect: (0, 166, 120, 198), base_width: 256 },
        SpriteDef { source_name: "inventory", target_path: "container/inventory/effect_background_small", rect: (0, 198, 32, 230), base_width: 256 },
        SpriteDef { source_name: "horse", target_path: "container/horse/armor_slot", rect: (0, 220, 18, 238), base_width: 256 },
        SpriteDef { source_name: "horse", target_path: "container/horse/saddle_slot", rect: (18, 220, 36, 238), base_width: 256 },
        SpriteDef { source_name: "horse", target_path: "container/horse/llama_armor_slot", rect: (36, 220, 54, 238), base_width: 256 },
        SpriteDef { source_name: "horse", target_path: "container/horse/chest_slots", rect: (0, 166, 90, 220), base_width: 256 },
        SpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/enchantment_slot", rect: (0, 166, 108, 185), base_width: 256 },
        SpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/enchantment_slot_disabled", rect: (0, 185, 108, 204), base_width: 256 },
        SpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/enchantment_slot_highlighted", rect: (0, 204, 108, 223), base_width: 256 },
        SpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/level_1", rect: (0, 223, 16, 239), base_width: 256 },
        SpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/level_2", rect: (16, 223, 32, 239), base_width: 256 },
        SpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/level_3", rect: (32, 223, 48, 239), base_width: 256 },
        SpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/level_1_disabled", rect: (0, 239, 16, 255), base_width: 256 },
        SpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/level_2_disabled", rect: (16, 239, 32, 255), base_width: 256 },
        SpriteDef { source_name: "enchanting_table", target_path: "container/enchanting_table/level_3_disabled", rect: (32, 239, 48, 255), base_width: 256 },
        SpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/recipe", rect: (0, 166, 16, 184), base_width: 256 },
        SpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/recipe_selected", rect: (0, 184, 16, 202), base_width: 256 },
        SpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/recipe_highlighted", rect: (0, 202, 16, 220), base_width: 256 },
        SpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/scroller", rect: (176, 0, 188, 15), base_width: 256 },
        SpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/scroller_disabled", rect: (188, 0, 200, 15), base_width: 256 },
        SpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/input_slot", rect: (176, 0, 192, 16), base_width: 256 },
        SpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/output_slot", rect: (192, 0, 208, 16), base_width: 256 },
        SpriteDef { source_name: "stonecutter", target_path: "container/stonecutter/result_slot", rect: (192, 0, 208, 16), base_width: 256 },
        SpriteDef { source_name: "loom", target_path: "container/loom/banner_slot", rect: (176, 0, 192, 16), base_width: 256 },
        SpriteDef { source_name: "loom", target_path: "container/loom/dye_slot", rect: (192, 0, 208, 16), base_width: 256 },
        SpriteDef { source_name: "loom", target_path: "container/loom/pattern_slot", rect: (208, 0, 224, 16), base_width: 256 },
        SpriteDef { source_name: "loom", target_path: "container/loom/pattern", rect: (0, 166, 14, 180), base_width: 256 },
        SpriteDef { source_name: "loom", target_path: "container/loom/pattern_selceted", rect: (0, 180, 14, 194), base_width: 256 },
        SpriteDef { source_name: "loom", target_path: "container/loom/pattern_selected", rect: (0, 180, 14, 194), base_width: 256 },
        SpriteDef { source_name: "loom", target_path: "container/loom/pattern_highlighted", rect: (0, 194, 14, 208), base_width: 256 },
        SpriteDef { source_name: "loom", target_path: "container/loom/scroller", rect: (232, 0, 244, 15), base_width: 256 },
        SpriteDef { source_name: "loom", target_path: "container/loom/scroller_disabled", rect: (244, 0, 256, 15), base_width: 256 },
        SpriteDef { source_name: "smithing", target_path: "container/smithing/error", rect: (176, 0, 204, 21), base_width: 256 },
        SpriteDef { source_name: "smithing", target_path: "container/smithing/template_slot", rect: (16, 0, 32, 16), base_width: 256 },
        SpriteDef { source_name: "smithing", target_path: "container/smithing/base_slot", rect: (32, 0, 48, 16), base_width: 256 },
        SpriteDef { source_name: "smithing", target_path: "container/smithing/addition_slot", rect: (16, 16, 32, 32), base_width: 256 },
        SpriteDef { source_name: "smithing", target_path: "container/smithing/result_slot", rect: (32, 16, 48, 32), base_width: 256 },
        SpriteDef { source_name: "villager2", target_path: "container/villager/discount_strikethrough", rect: (0, 176, 9, 178), base_width: 512 },
        SpriteDef { source_name: "villager2", target_path: "container/villager/experience_bar_result", rect: (0, 181, 102, 186), base_width: 512 },
        SpriteDef { source_name: "villager2", target_path: "container/villager/experience_bar_background", rect: (0, 186, 102, 191), base_width: 512 },
        SpriteDef { source_name: "villager2", target_path: "container/villager/experience_bar_current", rect: (0, 191, 102, 196), base_width: 512 },
        SpriteDef { source_name: "villager2", target_path: "container/villager/trade_arrow", rect: (15, 171, 25, 180), base_width: 512 },
        SpriteDef { source_name: "villager2", target_path: "container/villager/out_of_stuck", rect: (25, 171, 35, 180), base_width: 512 },
        SpriteDef { source_name: "villager2", target_path: "container/villager/out_of_stock", rect: (25, 171, 35, 180), base_width: 512 },
        SpriteDef { source_name: "villager2", target_path: "container/villager/scroller", rect: (0, 199, 6, 226), base_width: 512 },
        SpriteDef { source_name: "villager2", target_path: "container/villager/scroller_disabled", rect: (6, 199, 12, 226), base_width: 512 },
        SpriteDef { source_name: "cartography_table", target_path: "container/cartography_table/duplicated_map", rect: (176, 132, 226, 198), base_width: 256 },
        SpriteDef { source_name: "cartography_table", target_path: "container/cartography_table/scaled_map", rect: (176, 66, 242, 132), base_width: 256 },
        SpriteDef { source_name: "cartography_table", target_path: "container/cartography_table/map", rect: (176, 0, 242, 66), base_width: 256 },
        SpriteDef { source_name: "cartography_table", target_path: "container/cartography_table/locked", rect: (52, 214, 62, 228), base_width: 256 },
        SpriteDef { source_name: "cartography_table", target_path: "container/cartography_table/error", rect: (226, 132, 254, 153), base_width: 256 },
        SpriteDef { source_name: "grindstone", target_path: "container/grindstone/error", rect: (176, 0, 204, 21), base_width: 256 },
        SpriteDef { source_name: "grindstone", target_path: "container/grindstone/input_slot", rect: (30, 53, 48, 71), base_width: 256 },
        SpriteDef { source_name: "grindstone", target_path: "container/grindstone/additional_slot", rect: (66, 53, 84, 71), base_width: 256 },
        SpriteDef { source_name: "grindstone", target_path: "container/grindstone/output_slot", rect: (66, 53, 84, 71), base_width: 256 },
        SpriteDef { source_name: "grindstone", target_path: "container/grindstone/result_slot", rect: (66, 53, 84, 71), base_width: 256 },
    ];

    const CONTAINER: &str = "assets/minecraft/textures/gui/container";
    const SPRITES: &str = "assets/minecraft/textures/gui/sprites";

    /// 旧 `scale_from_image_base`。
    fn scale_from_image_base(img: &RgbaImage, base_width: u32) -> f32 {
        let width = img.width().max(1);
        let base = base_width.max(1);
        width as f32 / base as f32
    }

    /// **裁剪守卫**（§9.122）：`crop_imm` 越界时返回**空图**，而 PNG 编码器拒绝 0 宽/0 高。
    /// 旧实现里这个错误发生在并行 `commit_all` 内部并被吞掉；`Tx` 形态若直接 `?`
    /// 会把**整次转换**打断。返回 `None` 表示"这次裁剪无意义，跳过"，与旧实现的净效果一致。
    fn crop_checked(
        img: &RgbaImage,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        what: &str,
    ) -> Option<RgbaImage> {
        if w == 0 || h == 0 || x >= img.width() || y >= img.height() {
            crate::log_warn!(
                "{}: 裁剪越界（源 {}x{}，rect {x},{y},{w},{h}）——跳过",
                what, img.width(), img.height()
            );
            return None;
        }
        Some(imageops::crop_imm(img, x, y, w, h).to_image())
    }
    /// 旧 `scale_coordinate`。
    fn scale_coordinate(scale: f32, coord: u32) -> u32 {
        (coord as f32 * scale).round() as u32
    }

    /// 旧 `scale_rect`（注意第 3/4 个返回值是**宽高**，不是右/下坐标）。
    fn scale_rect(scale: f32, x1: u32, y1: u32, x2: u32, y2: u32) -> (u32, u32, u32, u32) {
        (
            scale_coordinate(scale, x1),
            scale_coordinate(scale, y1),
            scale_coordinate(scale, x2 - x1),
            scale_coordinate(scale, y2 - y1),
        )
    }

    /// **主循环**（旧 `execute_transformation` 的第一个 `for def in SPRITE_MAP` 段）。
    ///
    /// 语义与旧实现逐条对应：源图缺失**静默跳过**（`tx.image(..).ok()`）；
    /// 缩放按 `def.base_width`；裁剪后写 `sprites/<target_path>.png`。
    pub fn cut_sprite_map(tx: &mut Tx<'_>) -> Result<usize, AromError> {
        let mut written = 0usize;
        for def in SPRITE_MAP {
            let src = format!("{CONTAINER}/{}.png", def.source_name);
            let Ok(img) = tx.image(&src) else {
                continue; // 与旧实现一致：源图缺失 → 跳过（旧侧是 load_texture 返回 Err）
            };
            let (x1, y1, x2, y2) = def.rect;
            let scale = scale_from_image_base(&img, def.base_width);
            let (rx1, ry1, rw, rh) = scale_rect(scale, x1, y1, x2, y2);
            // `tx.image` 返回 `Arc<RgbaImage>`；`crop_imm` 要 `&RgbaImage`，故解引用（不克隆整图）。
            let sprite = imageops::crop_imm(&*img, rx1, ry1, rw, rh).to_image();
            let target = format!("{SPRITES}/{}.png", def.target_path);
            tx.put_image(&target, &sprite)?;
            written += 1;
        }
        Ok(written)
    }

    /// 旧 `SplitMode`（`save_slices` 的切分方式）。
    #[derive(Clone, Copy)]
    enum SplitMode {
        None,
        Horizontal,
        Vertical,
    }

    /// 旧 `GuiSurgeon::sprite_dir`。
    fn sprite_dir(subdir: &str) -> String {
        format!("{SPRITES}/{subdir}")
    }

    /// 旧 `save_slices`：裁一块 → 按 `split` 切成 `names.len()` 片 → 逐片写 `sprites/<target_dir>/<name>`。
    ///
    /// 与旧实现的对应关系：`base_path` 参数**删除**（`x` 直接是包内相对路径）；
    /// `pool` 换成 `tx`；旧签名里的 `_res` 本来就**未被使用**（旧代码就带下划线），故不保留。
    /// `names` 里的元素**自带 `.png` 后缀**（与旧表一致）。
    fn save_slices(
        tx: &mut Tx<'_>,
        img: &RgbaImage,
        crop: (u32, u32, u32, u32),
        split: SplitMode,
        slice_size: (u32, u32),
        names: &[&str],
        target_dir: &str,
    ) -> Result<usize, AromError> {
        let scale = scale_from_image_base(img, 256);
        let (x1, y1, x2, y2) = crop;
        let (rx, ry, rw, rh) = scale_rect(scale, x1, y1, x2, y2);
        // **零尺寸守卫**（§9.122）：`crop_imm` 在越界时返回**空图**，而 PNG 编码器
        // **拒绝 0 宽/0 高**。旧实现里这个错误发生在并行的 `commit_all` 内部并被吞掉
        // （转换照常完成，只是少一个 sprite）；`Tx` 形态若直接 `?` 会把**整次转换**打断。
        // 因此这里显式跳过并告警——与旧实现的净效果一致，且不再静默。
        if rw == 0 || rh == 0 {
            crate::log_warn!(
                "save_slices: 裁剪越界（源 {}x{}，scale {:.3}，rect {rx},{ry},{rw},{rh}）——跳过",
                img.width(), img.height(), scale
            );
            return Ok(0);
        }
        let cropped = imageops::crop_imm(img, rx, ry, rw, rh).to_image();

        let slice_w = scale_coordinate(scale, slice_size.0);
        let slice_h = scale_coordinate(scale, slice_size.1);

        let mut slices: Vec<RgbaImage> = Vec::new();
        match split {
            SplitMode::None => slices.push(cropped),
            SplitMode::Horizontal => {
                for i in 0..names.len() {
                    let sx = i as u32 * slice_w;
                    slices.push(imageops::crop_imm(&cropped, sx, 0, slice_w, slice_h).to_image());
                }
            }
            SplitMode::Vertical => {
                for i in 0..names.len() {
                    let sy = i as u32 * slice_h;
                    slices.push(imageops::crop_imm(&cropped, 0, sy, slice_w, slice_h).to_image());
                }
            }
        }

        let root = sprite_dir(target_dir);
        let mut written = 0usize;
        for (idx, name) in names.iter().enumerate() {
            if let Some(slice) = slices.get(idx) {
                tx.put_image(&format!("{root}/{name}"), slice)?;
                written += 1;
            }
        }
        Ok(written)
    }

    /// 旧 `process_resource_packs`。源图缺失 → 静默跳过（旧侧先 `exists()` 再 `load_texture`）。
    pub fn process_resource_packs(tx: &mut Tx<'_>) -> Result<usize, AromError> {
        let src = "assets/minecraft/textures/gui/resource_packs.png";
        let Ok(img) = tx.image(src) else {
            crate::log_info!("resource_packs.png not found, skip");
            return Ok(0);
        };
        let mut n = 0usize;
        n += save_slices(
            tx, &*img, (0, 0, 128, 32), SplitMode::Horizontal, (32, 32),
            &["select.png", "unselect.png", "move_down.png", "move_up.png"],
            "transferable_list",
        )?;
        n += save_slices(
            tx, &*img, (0, 32, 128, 64), SplitMode::Horizontal, (32, 32),
            &["select_highlighted.png", "unselect_highlighted.png",
              "move_down_highlighted.png", "move_up_highlighted.png"],
            "transferable_list",
        )?;
        Ok(n)
    }

    /// 旧 `process_server_selection`。源图缺失 → 静默跳过。
    pub fn process_server_selection(tx: &mut Tx<'_>) -> Result<usize, AromError> {
        let src = "assets/minecraft/textures/gui/server_selection.png";
        let Ok(img) = tx.image(src) else {
            crate::log_info!("server_selection.png not found, skip");
            return Ok(0);
        };
        let mut n = 0usize;
        n += save_slices(
            tx, &*img, (0, 0, 128, 32), SplitMode::Horizontal, (32, 32),
            &["join.png", "emm.png", "move_down.png", "move_up.png"],
            "server_list",
        )?;
        n += save_slices(
            tx, &*img, (0, 32, 128, 64), SplitMode::Horizontal, (32, 32),
            &["join_highlighted.png", "emmm.png",
              "move_down_highlighted.png", "move_up_highlighted.png"],
            "server_list",
        )?;
        Ok(n)
    }
    /// 旧 `process_slider`：由 `gui/slider.png`（vanilla 基准 200 宽）产出 3 个 widget sprite。
    ///
    /// 注意 `scale_from_image`（基准 **256**）与 slider 自身的 200 宽基准**不是一回事**——
    /// 与旧实现一致，照抄。
    pub fn process_slider(tx: &mut Tx<'_>) -> Result<usize, AromError> {
        let src = "assets/minecraft/textures/gui/slider.png";
        let Ok(img) = tx.image(src) else {
            crate::log_info!("slider.png not found, skip");
            return Ok(0);
        };
        let scale = scale_from_image_base(&img, 256);
        let mut n = 0usize;

        // 滑块主体
        let (x1, y1, w, h) = scale_rect(scale, 0, 0, 200, 20);
        let Some(slider) = crop_checked(&img, x1, y1, w, h, "slider") else { return Ok(0) };
        tx.put_image("assets/minecraft/textures/gui/sprites/widget/slider.png", &slider)?;
        n += 1;

        // 手柄 = 左片(0,40)-(4,60) 与 右片(196,40)-(200,60) 横向拼接
        let (lx, ly, lw, lh) = scale_rect(scale, 0, 40, 4, 60);
        let (rx, ry, rw, rh) = scale_rect(scale, 196, 40, 200, 60);
        let (Some(left), Some(right)) = (crop_checked(&img, lx, ly, lw, lh, "slider_handle_l"), crop_checked(&img, rx, ry, rw, rh, "slider_handle_r")) else {
            return Ok(n);
        };
        let mut handle = RgbaImage::new(left.width() + right.width(), left.height());
        for y in 0..left.height() {
            for x in 0..left.width() {
                handle.put_pixel(x, y, *left.get_pixel(x, y));
            }
            for x in 0..right.width() {
                handle.put_pixel(x + left.width(), y, *right.get_pixel(x, y));
            }
        }
        tx.put_image(
            "assets/minecraft/textures/gui/sprites/widget/slider_handle.png",
            &handle,
        )?;
        n += 1;

        // 高亮手柄 = 同样的拼法，取 y=60..80 那一行
        let (hx, hy, hw, hh) = scale_rect(scale, 0, 60, 4, 80);
        let (hrx, hry, hrw, hrh) = scale_rect(scale, 196, 60, 200, 80);
        let (Some(hl), Some(hr)) = (crop_checked(&img, hx, hy, hw, hh, "slider_hl_l"), crop_checked(&img, hrx, hry, hrw, hrh, "slider_hl_r")) else {
            return Ok(n);
        };
        let mut h_handle = RgbaImage::new(hl.width() + hr.width(), hl.height());
        for y in 0..hl.height() {
            for x in 0..hl.width() {
                h_handle.put_pixel(x, y, *hl.get_pixel(x, y));
            }
            for x in 0..hr.width() {
                h_handle.put_pixel(x + hl.width(), y, *hr.get_pixel(x, y));
            }
        }
        tx.put_image(
            "assets/minecraft/textures/gui/sprites/widget/slider_handle_highlighted.png",
            &h_handle,
        )?;
        n += 1;

        Ok(n)
    }

    /// 旧 `process_title`：由 `gui/title/minecraft.png` 产出 `sprites/title/{realms,minecraft}.png`，
    /// **并且把拼接结果回写到源文件 `gui/title/minecraft.png` 本身**。
    ///
    /// 那处"回写源文件"是旧实现的**刻意行为**（1.21 的 title 用法变了），照抄——
    /// 在 `Tx` 形态下它表现为"同一路径既读又写"，层会以最后一次写为准。
    pub fn process_title(tx: &mut Tx<'_>) -> Result<usize, AromError> {
        let src = "assets/minecraft/textures/gui/title/minecraft.png";
        let Ok(img) = tx.image(src) else {
            crate::log_info!("title/minecraft.png not found, skip");
            return Ok(0);
        };
        let scale = scale_from_image_base(&img, 256);
        let mut n = 0usize;

        // realms：直接裁一块
        let (rx, ry, rw, rh) = scale_rect(scale, 0, 94, 200, 194);
        let Some(realms) = crop_checked(&img, rx, ry, rw, rh, "title/realms") else { return Ok(0) };
        tx.put_image("assets/minecraft/textures/gui/sprites/title/realms.png", &realms)?;
        n += 1;

        // 两片横向拼接，再补一段透明底
        let (x1, y1, w1, h1) = scale_rect(scale, 0, 0, 155, 44);
        let Some(part1) = crop_checked(&img, x1, y1, w1, h1, "title/part1") else { return Ok(n) };
        let (x2, y2, w2, h2) = scale_rect(scale, 0, 45, 119, 89);
        let Some(part2) = crop_checked(&img, x2, y2, w2, h2, "title/part2") else { return Ok(n) };

        let concat_w = part1.width() + part2.width();
        let concat_h = part1.height().max(part2.height());
        let mut concatenated = RgbaImage::new(concat_w, concat_h);
        imageops::overlay(&mut concatenated, &part1, 0, 0);
        imageops::overlay(&mut concatenated, &part2, part1.width() as i64, 0);

        let tw = scale_coordinate(scale, 274);
        let th = scale_coordinate(scale, 25);
        let final_w = tw.max(concatenated.width());
        let final_h = concatenated.height() + th;
        let mut final_img = RgbaImage::new(final_w, final_h);
        imageops::overlay(&mut final_img, &concatenated, 0, 0);

        tx.put_image("assets/minecraft/textures/gui/sprites/title/minecraft.png", &final_img)?;
        n += 1;
        // 旧实现：把同一张图**回写源文件**
        tx.put_image(src, &final_img)?;
        n += 1;

        Ok(n)
    }
    /// 旧 `equalize_nine_slice_frame`：把 sprite 的上下左右边框统一为 `border` 像素，
    /// 避免 1.21 九宫格拆开时某侧变薄/缺边导致 UI 错位。
    ///
    /// 逐条照抄：①中心原样；②上下边条用**最外 1 行**沿边方向拉伸；③左右边条用**最外 1 列**拉伸；
    /// ④四角用原图角像素块（保留立体倒角）。`border` 被 `clamp(1, min(w,h)/2)`。
    fn equalize_nine_slice_frame(img: &RgbaImage, border: u32) -> RgbaImage {
        let (w, h) = img.dimensions();
        let b = border.clamp(1, w.min(h) / 2);
        if w < 2 || h < 2 {
            return img.clone();
        }
        let mut out = RgbaImage::new(w, h);

        // 1) 中心
        for y in b..(h - b) {
            for x in b..(w - b) {
                out.put_pixel(x, y, *img.get_pixel(x, y));
            }
        }
        // 2) 上下边条
        for x in 0..w {
            let top_src = *img.get_pixel(x.min(w - 1), 0);
            let bot_src = *img.get_pixel(x.min(w - 1), h - 1);
            for t in 0..b {
                out.put_pixel(x, t, top_src);
                out.put_pixel(x, h - 1 - t, bot_src);
            }
        }
        // 3) 左右边条
        for y in 0..h {
            let left_src = *img.get_pixel(0, y.min(h - 1));
            let right_src = *img.get_pixel(w - 1, y.min(h - 1));
            for t in 0..b {
                out.put_pixel(t, y, left_src);
                out.put_pixel(w - 1 - t, y, right_src);
            }
        }
        // 4) 四角
        for ty in 0..b {
            for tx in 0..b {
                let src_tx = tx.min(b - 1);
                let src_ty = ty.min(b - 1);
                out.put_pixel(tx, ty, *img.get_pixel(src_tx, src_ty));
                out.put_pixel(w - 1 - tx, ty, *img.get_pixel(w - 1 - src_tx, src_ty));
                out.put_pixel(tx, h - 1 - ty, *img.get_pixel(src_tx, h - 1 - src_ty));
                out.put_pixel(w - 1 - tx, h - 1 - ty, *img.get_pixel(w - 1 - src_tx, h - 1 - src_ty));
            }
        }
        out
    }

    /// 旧 `process_widgets`：由 `gui/widgets.png` 产出 HUD / icon / widget 三组 sprite。
    ///
    /// **照抄未改的两处**：
    /// 1. `language.png` 被**写两次**（旧代码里是两段完全相同的 `save_slices`）—— 幂等，但照抄；
    /// 2. 按钮族用 `equalize_nine_slice_frame` 统一边厚，`border = (2*scale).round().max(2)`。
    pub fn process_widgets(tx: &mut Tx<'_>) -> Result<usize, AromError> {
        let src = "assets/minecraft/textures/gui/widgets.png";
        let Ok(img) = tx.image(src) else {
            crate::log_info!("widgets.png not found, skip");
            return Ok(0);
        };
        let mut n = 0usize;

        n += save_slices(tx, &*img, (0, 0, 182, 22), SplitMode::None, (182, 22), &["hotbar.png"], "hud")?;
        n += save_slices(tx, &*img, (0, 22, 24, 45), SplitMode::None, (24, 23), &["hotbar_selection.png"], "hud")?;
        n += save_slices(tx, &*img, (24, 22, 53, 46), SplitMode::None, (29, 24), &["hotbar_offhand_left.png"], "hud")?;
        n += save_slices(tx, &*img, (53, 22, 82, 46), SplitMode::None, (29, 24), &["hotbar_offhand_right.png"], "hud")?;

        // 按钮族：切出后统一 `2*scale` 边厚
        {
            let bscale = scale_from_image_base(&img, 256);
            let border = ((2.0 * bscale).round() as u32).max(2);
            for (crop_y, name) in [
                (46u32, "button_disabled.png"),
                (66, "button.png"),
                (86, "button_highlighted.png"),
            ] {
                let (x1, y1, w, h) = scale_rect(bscale, 0, crop_y, 200, crop_y + 20);
                let Some(raw) = crop_checked(&img, x1, y1, w, h, "widgets/button") else { continue };
                let fixed = equalize_nine_slice_frame(&raw, border);
                tx.put_image(&format!("{SPRITES}/widget/{name}"), &fixed)?;
                n += 1;
            }
        }

        // language.png：旧实现写两次（两段相同调用），照抄
        n += save_slices(tx, &*img, (3, 109, 18, 124), SplitMode::None, (15, 15), &["language.png"], "icon")?;
        n += save_slices(tx, &*img, (3, 109, 18, 124), SplitMode::None, (15, 15), &["language.png"], "icon")?;

        // locked / unlocked 按钮族：竖直切成 3 片，各做边厚统一
        {
            let bscale = scale_from_image_base(&img, 256);
            let border = ((2.0 * bscale).round() as u32).max(2);
            for (x1, x2, names) in [
                (0u32, 20u32, ["locked_button.png", "locked_button_highlighted.png", "locked_button_disabled.png"]),
                (20, 40, ["unlocked_button.png", "unlocked_button_highlighted.png", "unlocked_button_disabled.png"]),
            ] {
                let (rx1, ry1, rw, rh) = scale_rect(bscale, x1, 146, x2, 206);
                let Some(strip) = crop_checked(&img, rx1, ry1, rw, rh, "widgets/strip") else { continue };
                let slice_h = scale_coordinate(bscale, 20);
                let slice_w = scale_coordinate(bscale, 20);
                for (i, name) in names.iter().enumerate() {
                    let sy = i as u32 * slice_h;
                    let Some(raw) = crop_checked(&strip, 0, sy, slice_w, slice_h, "widgets/locked") else { continue };
                    let fixed = equalize_nine_slice_frame(&raw, border);
                    tx.put_image(&format!("{SPRITES}/widget/{name}"), &fixed)?;
                    n += 1;
                }
            }
        }

        Ok(n)
    }
    /// 旧 `process_tabs`：由 `gui/container/creative_inventory/tabs.png` 产出 4 行 × 7 个 tab sprite。
    ///
    /// **照抄未改的三处语义**：
    /// 1. **只接受 256/512/1024/2048 的精确方形**，其它尺寸整体跳过（`(w.max(h), w == h)` 匹配）；
    /// 2. **始终裁 168 宽（6 个 tab），第 7 个复制第 6 个**——旧注释明确写了不去探测 168–196，
    ///    因为 1.8 图集该区域可能有残留像素会导致误裁；
    /// 3. 每片的 `slice_height == ch`（即"切片高"与"行高"同值），故竖直方向上正好切满。
    pub fn process_tabs(tx: &mut Tx<'_>) -> Result<usize, AromError> {
        let src = "assets/minecraft/textures/gui/container/creative_inventory/tabs.png";
        let Ok(img) = tx.image(src) else {
            crate::log_info!("tabs.png not found, skip");
            return Ok(0);
        };

        let (w, h) = img.dimensions();
        let scale = match (w.max(h), w == h) {
            (256, true) => 1u32,
            (512, true) => 2,
            (1024, true) => 4,
            (2048, true) => 8,
            _ => {
                crate::log_info!("unsupported tabs.png size {}x{}, skip", w, h);
                return Ok(0);
            }
        };

        const OUT: &str = "assets/minecraft/textures/gui/sprites/container/creative_inventory";
        let mut n = 0usize;

        // 一行 = 裁 (0, cy*scale) 起 168*scale × ch*scale，再横切 6 片，最后复制第 6 片为第 7 片。
        let mut store_tabs =
            |tx: &mut Tx<'_>, cy: u32, ch: u32, names: [&str; 7]| -> Result<usize, AromError> {
                let cropped = imageops::crop_imm(
                    &*img,
                    0,
                    cy * scale,
                    168 * scale,
                    ch * scale,
                )
                .to_image();
                let slice_w = 28 * scale;
                let slice_h = ch * scale; // 旧的 slice_h 参数与 ch 同值，故此处等价

                let mut last: Option<RgbaImage> = None;
                let mut k = 0usize;
                for i in 0..6u32 {
                    let tab = imageops::crop_imm(&cropped, i * slice_w, 0, slice_w, slice_h).to_image();
                    tx.put_image(&format!("{OUT}/{}", names[i as usize]), &tab)?;
                    last = Some(tab);
                    k += 1;
                }
                if let Some(tab7) = last {
                    tx.put_image(&format!("{OUT}/{}", names[6]), &tab7)?;
                    k += 1;
                }
                Ok(k)
            };

        // y 区间与旧实现逐条一致
        n += store_tabs(tx, 2, 30, [
            "tab_top_unselected_1.png", "tab_top_unselected_2.png", "tab_top_unselected_3.png",
            "tab_top_unselected_4.png", "tab_top_unselected_5.png", "tab_top_unselected_6.png",
            "tab_top_unselected_7.png",
        ])?;
        n += store_tabs(tx, 32, 32, [
            "tab_top_selected_1.png", "tab_top_selected_2.png", "tab_top_selected_3.png",
            "tab_top_selected_4.png", "tab_top_selected_5.png", "tab_top_selected_6.png",
            "tab_top_selected_7.png",
        ])?;
        n += store_tabs(tx, 64, 30, [
            "tab_bottom_unselected_1.png", "tab_bottom_unselected_2.png", "tab_bottom_unselected_3.png",
            "tab_bottom_unselected_4.png", "tab_bottom_unselected_5.png", "tab_bottom_unselected_6.png",
            "tab_bottom_unselected_7.png",
        ])?;
        n += store_tabs(tx, 96, 32, [
            "tab_bottom_selected_1.png", "tab_bottom_selected_2.png", "tab_bottom_selected_3.png",
            "tab_bottom_selected_4.png", "tab_bottom_selected_5.png", "tab_bottom_selected_6.png",
            "tab_bottom_selected_7.png",
        ])?;

        Ok(n)
    }
    /// 旧 `process_icons`：由 `gui/icons.png` 产出 HUD / icon / server_list 三组 sprite。
    ///
    /// **本函数在旧实现里是纯声明式的**——17 次 `save_slices` 调用，没有任何直接的裁剪/写入，
    /// 因此移植是"逐条誊抄参数"，风险集中在**抄错数字或漏抄一条**。
    /// 夹具对照会逐文件比对，且数量断言能抓出"整段漏抄"。
    ///
    /// 注意 `names` 里有一批 `wtf*.png`：那是旧实现里**刻意保留**的槽位名（对应 1.20 图集中
    /// 1.21 已不用的小格），照抄——改名会改变产物。
    pub fn process_icons(tx: &mut Tx<'_>) -> Result<usize, AromError> {
        let src = "assets/minecraft/textures/gui/icons.png";
        let Ok(img) = tx.image(src) else {
            crate::log_info!("icons.png not found, skip");
            return Ok(0);
        };
        let img = &*img;
        let mut n = 0usize;

        n += save_slices(tx, img, (0, 0, 15, 15), SplitMode::None, (15, 15),
            &["crosshair.png"], "hud")?;

        n += save_slices(tx, img, (16, 0, 196, 9), SplitMode::Horizontal, (9, 9),
            &["container.png", "container_blinking.png", "wtf.png", "wtf2.png",
              "full.png", "half.png", "full_blinking.png", "half_blinking.png",
              "poisoned_full.png", "poisoned_half.png", "poisoned_full_blinking.png",
              "poisoned_half_blinking.png", "withered_full.png", "withered_half.png",
              "withered_full_blinking.png", "withered_half_blinking.png",
              "absorbing_full.png", "absorbing_half.png", "frozen_full.png",
              "frozen_half.png"], "hud/heart")?;

        n += save_slices(tx, img, (16, 9, 124, 18), SplitMode::Horizontal, (9, 9),
            &["armor_empty.png", "armor_half.png", "armor_full.png", "wtf3.png"], "hud")?;

        n += save_slices(tx, img, (52, 9, 124, 18), SplitMode::Horizontal, (9, 9),
            &["vehicle_container.png", "wtf4.png", "wtf5.png", "wtf6.png",
              "vehicle_full.png", "vehicle_half.png", "wtf7.png", "wtf8.png"], "hud/heart")?;

        n += save_slices(tx, img, (16, 18, 52, 27), SplitMode::Horizontal, (9, 9),
            &["air.png", "air_bursting.png", "wtf9.png", "wtf10.png"], "hud")?;

        n += save_slices(tx, img, (16, 27, 142, 36), SplitMode::Horizontal, (9, 9),
            &["food_empty.png", "wtf11.png", "wtf123.png", "wtf13.png", "food_full.png",
              "food_half.png", "wtf14.png", "wtf15.png", "food_full_hunger.png",
              "food_half_hunger.png", "wtf16.png", "wtf17.png", "wtf18.png",
              "food_empty_hunger.png"], "hud")?;

        n += save_slices(tx, img, (16, 45, 196, 54), SplitMode::Horizontal, (9, 9),
            &["container_hardcore.png", "container_hardcore_blinking.png", "wtf19.png",
              "wtf20.png", "hardcore_full.png", "hardcore_half.png",
              "hardcore_full_blinking.png", "hardcore_half_blinking.png",
              "poisoned_hardcore_full.png", "poisoned_hardcore_half.png",
              "poisoned_hardcore_full_blinking.png", "poisoned_hardcore_half_blinking.png",
              "withered_hardcore_full.png", "withered_hardcore_half.png",
              "withered_hardcore_full_blinking.png", "withered_hardcore_half_blinking.png",
              "absorbing_hardcore_full.png", "absorbing_hardcore_half.png",
              "frozen_hardcore_full.png", "frozen_hardcore_half.png"], "hud/heart")?;

        n += save_slices(tx, img, (0, 15, 10, 63), SplitMode::Vertical, (10, 8),
            &["ping_5.png", "ping_4.png", "ping_3.png", "ping_2.png", "ping_1.png",
              "ping_unknown.png"], "icon")?;

        n += save_slices(tx, img, (0, 64, 182, 94), SplitMode::Vertical, (182, 5),
            &["experience_bar_background.png", "experience_bar_progress.png",
              "jump_bar_cooldown.png", "wtf21.png", "jump_bar_background.png",
              "jump_bar_progress.png"], "hud")?;

        n += save_slices(tx, img, (0, 94, 18, 112), SplitMode::None, (18, 18),
            &["hotbar_attack_indicator_background.png"], "hud")?;
        n += save_slices(tx, img, (18, 94, 36, 112), SplitMode::None, (18, 18),
            &["hotbar_attack_indicator_progress.png"], "hud")?;
        n += save_slices(tx, img, (36, 94, 52, 98), SplitMode::None, (16, 4),
            &["crosshair_attack_indicator_background.png"], "hud")?;
        n += save_slices(tx, img, (52, 94, 68, 98), SplitMode::None, (16, 4),
            &["crosshair_attack_indicator_progress.png"], "hud")?;
        n += save_slices(tx, img, (68, 94, 84, 110), SplitMode::None, (16, 16),
            &["crosshair_attack_indicator_full.png"], "hud")?;

        n += save_slices(tx, img, (0, 176, 10, 224), SplitMode::Vertical, (10, 8),
            &["ping_5.png", "ping_4.png", "ping_3.png", "ping_2.png", "ping_1.png",
              "unreachable.png"], "server_list")?;
        n += save_slices(tx, img, (10, 176, 20, 216), SplitMode::Vertical, (10, 8),
            &["pinging_5.png", "pinging_4.png", "pinging_3.png", "pinging_2.png",
              "pinging_1.png"], "server_list")?;

        Ok(n)
    }
    /// **`GuiSurgeon` 的完整 `Tx` 形态入口**（§9.111）。
    ///
    /// 顺序与旧 `execute_transformation` **完全一致**：主循环 → 7 个 `process_*` → 登记延迟删除。
    /// （旧实现在 `commit_all()` 之后才登记删除，此处等价：`Tx` 形态下删除**必须延迟**，
    /// 早删会让更晚的任务看不到文件——§9.23 的坑。）
    ///
    /// **清理清单逐条照抄，尤其两点**：
    /// 1. **不含 `container/inventory.png`**——1.21 客户端仍需要它渲染生存/创造背包背景，
    ///    删掉会让背包 GUI 消失（旧注释专门写了这段，§9.96 也记过）；
    /// 2. 用 `tx.has_prefix` 判断"此刻是否存在"（对应旧实现的 `path.exists()`）。
    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        let mut outcome = Outcome::default();

        outcome.changed += cut_sprite_map(tx)?;
        outcome.changed += process_slider(tx)?;
        outcome.changed += process_icons(tx)?;
        outcome.changed += process_widgets(tx)?;
        outcome.changed += process_tabs(tx)?;
        outcome.changed += process_resource_packs(tx)?;
        outcome.changed += process_server_selection(tx)?;
        outcome.changed += process_title(tx)?;

        // 旧 atlas 文件在 1.21+ 已不再使用；**延迟**到收尾时机统一删除。
        for path in cleanup_list().iter().copied() {
            if tx.has_prefix(path)? {
                outcome.deferred_removals.push(path.to_string());
                outcome.notes.push(format!("defer removal of {path}"));
            }
        }

        Ok(outcome)
    }

    /// 清理清单（20 项，供测试断言"清单没被漏抄/没混入 inventory.png"）。
    pub fn cleanup_list() -> &'static [&'static str] {
        const CLEANUP: [&str; 20] = [
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
        &CLEANUP
    }
    /// `SPRITE_MAP` 的条目数（供测试断言"表没被漏抄"）。
    pub fn sprite_map_len() -> usize {
        SPRITE_MAP.len()
    }
