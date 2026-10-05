use image::{imageops, GenericImage, RgbaImage};

// ── UImage 路径解析（§9.125：从 `converters/mod.rs` 迁来这里）─────────────
//
// 它**不是**旧转换器的一部分：旧转换器与原生实现的测试夹具都要用它，
// 而 `converters/mod.rs` 现在要保留成「生产模块的容器」，因此这个通用工具跟随
// 同样被两边共用的 `scale_factor` 一起移到 crate 根（§9.124 的同类处置）。

/// 通过 Tauri 资源 API 解析 UImage 路径并缓存（委托 resource_resolver）
pub fn set_uimage_path_from_app(app: &tauri::AppHandle) {
    crate::resource_resolver::cache_resource_from_app(app, "UImage");
}

/// 获取 UImage 资源目录（多策略查找，找不到则在用户文档创建默认目录）
pub fn get_uimage_path() -> Result<std::path::PathBuf, String> {
    // 1. 优先通过 resource_resolver 的多策略解析
    match crate::resource_resolver::resolve_resource_dir("UImage", crate::resource_resolver::uimage_validator()) {
        Ok(p) => return Ok(p),
        Err(e) => crate::log_info!("UImage resolve_resource_dir failed: {}", e),
    }

    // 2. 回退到用户文档目录
    if let Ok(user_dir) = crate::overlay::user_data_root_dir() {
        let fallback = user_dir.join("UImage");
        if fallback.exists() {
            crate::log_info!("UImage found in user data dir: {}", fallback.display());
            return Ok(fallback);
        }
    }

    // 3. 最后手段：在用户文档目录创建默认 UImage
    let default_path = crate::overlay::user_data_root_dir()
        .map(|dir| dir.join("UImage"))
        .unwrap_or_else(|_| {
            std::env::current_dir()
                .map(|dir| dir.join("UImage"))
                .unwrap_or_else(|_| std::path::PathBuf::from("UImage"))
        });

    crate::log_info!("creating default UImage in user data: {}", default_path.display());
    std::fs::create_dir_all(&default_path).map_err(|e| {
        format!("UImage directory is missing and default directory creation failed: {}", e)
    })?;
    let _ = std::fs::write(default_path.join("README.txt"),
        "UImage directory for Resource Pack Converter.\nPut required UI image assets here.\n");
    Ok(default_path)
}

/// Copy `src` into `dst` at the given top-left coordinate using **raw RGBA
/// overwrites** (no alpha blending). This matches Pillow's
/// `Image.paste(src, box)` without a mask — every channel of every pixel
/// of `src` replaces the destination byte-for-byte, including the source's
/// alpha values.
///
/// Contrast this with `image::imageops::overlay`, which blends by source
/// alpha. Use this helper when the converter is *moving* a region from one
/// spot to another on the same atlas (or onto a freshly cleared canvas);
/// keep `imageops::overlay` when the intent is to layer a semi-transparent
/// decal on top of an existing pixel (matches `Image.alpha_composite`).
pub fn paste_region(dst: &mut RgbaImage, src: &RgbaImage, dx: u32, dy: u32) -> Result<(), String> {
    dst.copy_from(src, dx, dy).map_err(|e| e.to_string())
}

/// 对应 Python 的 swap_and_mirror
/// 交换两个区域，并各自进行 180 度旋转（镜像翻转）
pub fn swap_and_mirror(
    img: &mut RgbaImage,
    x1: u32, y1: u32, x2: u32, y2: u32, // 区域1和2的起点
    w: u32, h: u32,                   // 区域的宽高
) -> image::ImageResult<()> {
    // 裁剪区域 1 和 2
    let region1 = imageops::crop_imm(img, x1, y1, w, h).to_image();
    let region2 = imageops::crop_imm(img, x2, y2, w, h).to_image();

    // 进行 180 度翻转(水平+垂直)
    let flipped1 = imageops::flip_vertical(&imageops::flip_horizontal(&region1));
    let flipped2 = imageops::flip_vertical(&imageops::flip_horizontal(&region2));

    // 交换粘贴
    img.copy_from(&flipped2, x1, y1)?;
    img.copy_from(&flipped1, x2, y2)?;

    Ok(())
}
