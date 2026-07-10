use image::{imageops, GenericImage, RgbaImage};

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
