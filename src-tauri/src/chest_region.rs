//! 箱子贴图的区域变换辅助（**共享**）。
//!
//! 这些函数原先只住在 `converters/ui/process_chest_folder.rs` 里，但 A-ROM 原生实现
//! （`pilots::chest_folder_gen`）**需要同一份逻辑**。放在这里之后：
//!
//! - 原生实现不再依赖旧转换器模块（§9.124 的收口目标）；
//! - 旧转换器改为**复用**本模块，**实现逐字不变** —— 因此产物不变。
//!
//! ## 为什么是「搬运」而不是「重写」
//!
//! 这四个函数直接决定像素输出。重写会引入分叉风险，而我们可以整段搬过来、
//! 让两侧调用同一份代码 —— 这样「产物不变」是**构造上成立**的，而不是靠测试碰运气。
//!
//! ## 注意：`swap_and_mirror` 有三个语义不同的版本
//!
//! 本模块里的是 `converters/ui/process_chest_folder.rs` 的版本（翻转用**最初裁下的**区域）。
//! 另有两个不同实现：
//!
//! | 位置 | 翻转用哪张图 |
//! |---|---|
//! | `crate::image_utils::swap_and_mirror` | 翻转后 `copy_from` |
//! | `converters/reverse/chest_folder.rs` | **重裁后**的图 |
//!
//! **两个区域重叠时三者结果不同** —— 不要合并，除非逐像素验证。

use image::{imageops, RgbaImage};

/// Swap two regions, then flip each horizontally+vertically in place.
pub(crate) fn swap_and_mirror(
    img: &mut RgbaImage,
    b1: (u32, u32, u32, u32),
    b2: (u32, u32, u32, u32),
) -> Result<(), String> {
    let w1 = b1.2 - b1.0;
    let h1 = b1.3 - b1.1;
    let w2 = b2.2 - b2.0;
    let h2 = b2.3 - b2.1;

    let r1 = imageops::crop_imm(img, b1.0, b1.1, w1, h1).to_image();
    let r2 = imageops::crop_imm(img, b2.0, b2.1, w2, h2).to_image();

    crate::image_utils::paste_region(img, &r2, b1.0, b1.1)?;
    crate::image_utils::paste_region(img, &r1, b2.0, b2.1)?;

    let r1f = image::imageops::flip_horizontal(&image::imageops::flip_vertical(&r1));
    let r2f = image::imageops::flip_horizontal(&image::imageops::flip_vertical(&r2));
    imageops::overlay(img, &r1f, b1.0 as i64, b1.1 as i64);
    imageops::overlay(img, &r2f, b2.0 as i64, b2.1 as i64);
    Ok(())
}

/// Flip a region horizontally+vertically in place.
pub(crate) fn mirror_region(img: &mut RgbaImage, b: (u32, u32, u32, u32)) {
    let w = b.2 - b.0;
    let h = b.3 - b.1;
    let r = imageops::crop_imm(img, b.0, b.1, w, h).to_image();
    let f = image::imageops::flip_horizontal(&image::imageops::flip_vertical(&r));
    imageops::overlay(img, &f, b.0 as i64, b.1 as i64);
}

/// Flip a region vertically.
pub(crate) fn vflip_region(r: &RgbaImage) -> RgbaImage {
    image::imageops::flip_vertical(r)
}

/// Flip a region horizontally+vertically.
pub(crate) fn hvflip_region(r: &RgbaImage) -> RgbaImage {
    image::imageops::flip_horizontal(&image::imageops::flip_vertical(r))
}

/// Generate left and right chest images from a double chest texture.
pub(crate) fn generate_double_chest_images(
    left: &mut RgbaImage,
    right: &mut RgbaImage,
    img: &RgbaImage,
    s: u32,
) {
    let sb = |x1: u32, y1: u32, x2: u32, y2: u32| (x1 * s, y1 * s, x2 * s, y2 * s);

    // --- Left chest ---
    let crop = |b: (u32, u32, u32, u32)| imageops::crop_imm(img, b.0, b.1, b.2 - b.0, b.3 - b.1).to_image();

    imageops::overlay(left, &vflip_region(&crop(sb(29, 0, 44, 14))), (29 * s) as i64, 0);
    imageops::overlay(left, &vflip_region(&crop(sb(59, 0, 74, 14))), (14 * s) as i64, 0);
    imageops::overlay(left, &hvflip_region(&crop(sb(29, 14, 44, 19))), (43 * s) as i64, (14 * s) as i64);
    imageops::overlay(left, &hvflip_region(&crop(sb(44, 14, 58, 19))), (29 * s) as i64, (14 * s) as i64);
    imageops::overlay(left, &hvflip_region(&crop(sb(58, 14, 73, 19))), (14 * s) as i64, (14 * s) as i64);
    imageops::overlay(left, &vflip_region(&crop(sb(29, 19, 44, 33))), (29 * s) as i64, (19 * s) as i64);
    imageops::overlay(left, &vflip_region(&crop(sb(59, 19, 74, 33))), (14 * s) as i64, (19 * s) as i64);
    imageops::overlay(left, &hvflip_region(&crop(sb(29, 33, 44, 43))), (43 * s) as i64, (33 * s) as i64);
    imageops::overlay(left, &hvflip_region(&crop(sb(44, 33, 58, 43))), (29 * s) as i64, (33 * s) as i64);
    imageops::overlay(left, &hvflip_region(&crop(sb(58, 33, 73, 43))), (14 * s) as i64, (33 * s) as i64);

    // Additional left transforms
    imageops::overlay(left, &hvflip_region(&crop(sb(2, 1, 5, 5))), (1 * s) as i64, (1 * s) as i64);
    imageops::overlay(left, &crop(sb(2, 0, 3, 1)), (2 * s) as i64, 0);
    imageops::overlay(left, &crop(sb(4, 0, 5, 1)), (1 * s) as i64, 0);
    imageops::overlay(left, &vflip_region(&crop(sb(5, 1, 6, 5))), (1 * s) as i64, (1 * s) as i64);
    imageops::overlay(left, &crop(sb(1, 0, 2, 1)), (2 * s) as i64, 0);
    imageops::overlay(left, &crop(sb(3, 0, 4, 1)), (1 * s) as i64, 0);

    // --- Right chest ---
    imageops::overlay(right, &vflip_region(&crop(sb(44, 0, 59, 14))), (14 * s) as i64, 0);
    imageops::overlay(right, &vflip_region(&crop(sb(14, 0, 29, 14))), (29 * s) as i64, 0);
    imageops::overlay(right, &hvflip_region(&crop(sb(0, 14, 14, 19))), 0, (14 * s) as i64);
    imageops::overlay(right, &hvflip_region(&crop(sb(73, 14, 88, 19))), (14 * s) as i64, (14 * s) as i64);
    imageops::overlay(right, &hvflip_region(&crop(sb(14, 14, 29, 19))), (43 * s) as i64, (14 * s) as i64);
    imageops::overlay(right, &vflip_region(&crop(sb(14, 19, 29, 33))), (29 * s) as i64, (19 * s) as i64);
    imageops::overlay(right, &vflip_region(&crop(sb(44, 19, 59, 33))), (14 * s) as i64, (19 * s) as i64);
    imageops::overlay(right, &hvflip_region(&crop(sb(14, 33, 29, 43))), (43 * s) as i64, (33 * s) as i64);
    imageops::overlay(right, &hvflip_region(&crop(sb(0, 33, 14, 43))), 0, (33 * s) as i64);

    // Additional right transforms
    imageops::overlay(right, &hvflip_region(&crop(sb(10, 1, 13, 5))), (43 * s) as i64, (1 * s) as i64);
    imageops::overlay(right, &crop(sb(13, 0, 14, 1)), 0, 0);
    imageops::overlay(right, &crop(sb(11, 0, 12, 1)), 0, 0);
    imageops::overlay(right, &vflip_region(&crop(sb(9, 1, 10, 5))), (43 * s) as i64, (1 * s) as i64);
    imageops::overlay(right, &crop(sb(14, 0, 15, 1)), 0, 0);
    imageops::overlay(right, &crop(sb(12, 0, 13, 1)), 0, 0);
    imageops::overlay(right, &hvflip_region(&crop(sb(10, 0, 11, 1))), (43 * s) as i64, 0);
}
