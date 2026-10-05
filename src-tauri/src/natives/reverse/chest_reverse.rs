    use super::*;

    pub const CHEST: &str = "assets/minecraft/textures/entity/chest";
    const SINGLE: [&str; 4] = ["ender.png", "normal.png", "trapped.png", "christmas.png"];

    /// 反向侧的「交换 + 180° 翻转」——**与 `crate::chest_region::swap_and_mirror` 不是同一个函数**。
    ///
    /// §9.126（实测）：两者在**半透明像素**上结果不同，因此**不能合并**：
    ///
    /// | 版本 | 交换用 | 翻转用哪张图 | 收尾 |
    /// |---|---|---|---|
    /// | `chest_region::swap_and_mirror`（**正向** `process_chest_folder` 用） | `paste_region`（原样覆写） | **最初裁下的** r1/r2 | `overlay`（**按 alpha 混合**） |
    /// | 本函数（**反向** `reverse_process_chest_folder` 用，逐句照抄 `converters/reverse/chest_folder.rs`） | `paste_region` | **重裁后**的图 | `paste_region`（原样覆写） |
    ///
    /// `chest_region` 的模块注释早就警告过「两个区域重叠时三者结果不同」；本轮实测补齐了
    /// 另一半事实：**即使区域不相交**，只要图里有半透明像素，`overlay` 与 `paste_region`
    /// 就会给出不同结果（实测：真实 chest 图 128×128 中有 9776 个非不透明像素，
    /// 单次交换即分叉 **968** 像素）。
    ///
    /// 因此这里**照抄反向模块的语义**，而不是复用正向的共享助手。
    fn swap_and_mirror_reverse(
        img: &mut RgbaImage,
        b1: (u32, u32, u32, u32),
        b2: (u32, u32, u32, u32),
    ) -> Result<(), String> {
        use image::imageops;

        let w1 = b1.2 - b1.0;
        let h1 = b1.3 - b1.1;
        let w2 = b2.2 - b2.0;
        let h2 = b2.3 - b2.1;

        let region1 = imageops::crop_imm(img, b1.0, b1.1, w1, h1).to_image();
        let region2 = imageops::crop_imm(img, b2.0, b2.1, w2, h2).to_image();

        // 交换：r2→b1，r1→b2（原样覆写）
        crate::image_utils::paste_region(img, &region2, b1.0, b1.1)?;
        crate::image_utils::paste_region(img, &region1, b2.0, b2.1)?;

        // 各自就地翻转：**重裁**刚写入的内容
        let r1f = image::imageops::flip_horizontal(&image::imageops::flip_vertical(
            &imageops::crop_imm(img, b1.0, b1.1, w1, h1).to_image(),
        ));
        let r2f = image::imageops::flip_horizontal(&image::imageops::flip_vertical(
            &imageops::crop_imm(img, b2.0, b2.1, w2, h2).to_image(),
        ));
        // **注意**：旧反向实现这两句用的是 `overlay`（**不是** `paste_region`）——
        // 它自己两个阶段就不一致（交换用覆写、翻转用混合）。**照抄必须连这个一起照抄**：
        // 本模块首版这里误用了 `paste_region`，单次交换就分叉 522 像素，
        // 正是本模块的回归用例 `reverse_swap_matches_the_legacy_...` 抓到的。
        imageops::overlay(img, &r1f, b1.0 as i64, b1.1 as i64);
        imageops::overlay(img, &r2f, b2.0 as i64, b2.1 as i64);
        Ok(())
    }

    pub fn decl() -> TaskDecl {
        TaskDecl::new("reverse_process_chest_folder", Tier::Eraser)
            .reads(ScopeSet::prefix(CHEST))
            .writes(ScopeSet::prefix(CHEST))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        

        if !tx.has_prefix(CHEST)? {
            return Ok(Outcome::default());
        }
        let mut outcome = Outcome::default();
        for name in SINGLE {
            let path = format!("{CHEST}/{name}");
            if !tx.exists(&path) {
                continue;
            }
            let mut img: RgbaImage = (*tx.image(&path)?).clone();
            let Some(s) = super::chest::scale_for_single(img.width()) else {
                outcome.skipped += 1;
                continue;
            };
            let sb = |x1: u32, y1: u32, x2: u32, y2: u32| (x1 * s, y1 * s, x2 * s, y2 * s);
            for (a, b) in [
                (sb(14, 0, 28, 14), sb(28, 0, 42, 14)),
                (sb(14, 14, 28, 19), sb(42, 14, 56, 19)),
                (sb(14, 19, 28, 33), sb(28, 19, 42, 33)),
                (sb(14, 33, 28, 43), sb(42, 33, 56, 43)),
            ] {
                swap_and_mirror_reverse(&mut img, a, b).map_err(AromError::internal)?;
            }
            for b in [
                sb(14, 0, 28, 14),
                sb(28, 0, 42, 14),
                sb(0, 14, 14, 19),
                sb(28, 14, 42, 19),
                sb(14, 19, 28, 33),
                sb(28, 19, 42, 33),
                sb(0, 33, 14, 43),
                sb(28, 33, 42, 43),
            ] {
                crate::chest_region::mirror_region(&mut img, b);
            }
            tx.put_image(&path, &img)?;
            outcome.changed += 1;
        }
        Ok(outcome)
    }

    /// **回归守卫（§9.126）**：反向的 `swap_and_mirror` **必须逐句照抄**旧反向实现，
    /// 不能"顺手换成看起来等价的写法"。
    ///
    /// 旧反向实现（`converters/reverse/chest_folder.rs`）的两个阶段**本身就不一致**：
    /// **交换**用 `paste_region`（原样覆写），**两处翻转**用 `overlay`（按 alpha 混合）。
    /// 本模块首版把翻转也写成了 `paste_region`——那看起来"更一致"、也更容易过测试，
    /// 但它是**错的**：真实素材上单次交换即分叉 **522** 像素（本用例抓到的）。
    ///
    /// 为什么"看着等价"却不等价：`overlay` 按 alpha 混合，而真实 chest 图正是
    /// 16384 像素里 **9776 个 `alpha=0`、其中 9491 个 RGB 非零**。
    ///
    /// **本用例只比"交换"这一步**：镜像那一步（8 组）两侧都是 `overlay`，与要守的差异无关。
    /// 夹具的 `(3x+7y) mod 251` / `(11x+5y) mod 241` 保证**颜色两两不同**，
    /// 否则「像素相等」可能只是巧合。
    ///
    /// **判据边界（如实标注）**：本条只覆盖两个区域**不相交**时的交换语义
    /// （真实任务用的 4 组区域都是不相交的）。区域重叠时两版仍有差异
    /// （"重裁后的图" vs "最初裁下的 r1/r2"），**本用例覆盖不到**——
    /// 那一路由整包对照 `reverse_whole_pack_matches_the_old_pipeline` 兜底（3952 个条目）。
    #[cfg(all(test, feature = "legacy-oracle"))]
    #[test]
    fn reverse_swap_matches_the_legacy_pixel_for_pixel() {
        // 128×128、s=2 的夹具：alpha 只有 0 / 255，且 alpha=0 处 RGB 非零
        let mut orig = image::RgbaImage::new(128, 128);
        for y in 0..128u32 {
            for x in 0..128u32 {
                let r = ((3 * x + 7 * y) % 251) as u8;
                let g = ((11 * x + 5 * y) % 241) as u8;
                let a = if (x + y) % 3 == 0 { 0 } else { 255 };
                orig.put_pixel(x, y, image::Rgba([r, g, 7, a]));
            }
        }
        {
            let mut seen = std::collections::HashSet::new();
            for p in orig.pixels() {
                assert!(
                    seen.insert((p.0[0], p.0[1], p.0[3])),
                    "夹具颜色重复 ⇒ 断言会失真，请更换生成式"
                );
            }
        }
        let s = 2u32;
        let sb = |x1: u32, y1: u32, x2: u32, y2: u32| (x1 * s, y1 * s, x2 * s, y2 * s);
        let p1 = sb(14, 0, 28, 14);
        let p2 = sb(28, 0, 42, 14);

        let mut legacy = orig.clone();
        crate::converters::reverse::chest_folder::swap_and_mirror(&mut legacy, p1, p2)
            .expect("legacy swap");
        let mut native = orig.clone();
        swap_and_mirror_reverse(&mut native, p1, p2).expect("native swap");
        let mut shared = orig.clone();
        crate::chest_region::swap_and_mirror(&mut shared, p1, p2).expect("shared swap");

        let count = |x: &image::RgbaImage, y: &image::RgbaImage| {
            let mut n = 0;
            for py in 0..x.height() {
                for px in 0..x.width() {
                    if x.get_pixel(px, py) != y.get_pixel(px, py) {
                        n += 1;
                    }
                }
            }
            n
        };
        let vs_legacy = count(&legacy, &native);
        let shared_vs_legacy = count(&legacy, &shared);

        // ① 本模块实现必须与旧实现逐像素一致（**这是唯一判据**）
        assert_eq!(
            vs_legacy, 0,
            "反向 swap 与旧实现不一致：分叉 {vs_legacy} 像素（应当为 0）"
        );

        // ② 诊断读数（**刻意不断言**）：两个区域不相交时，"重裁后的图"与"最初裁下的 r1/r2"
        //    数学上等价，因此共享助手在这里**也可能**与旧实现一致（实测：本项目当前为
        //    {shared_vs_legacy} 像素）。把这条写成断言会是**错的判据**——它会把一次
        //    "恰好等价"误判成回归，或反过来给出虚假的安全感。真正能分开两版的是
        //    **区域重叠**与整包对照，见本用例文档的"判据边界"。
        println!(
            "反向 swap 对照：本模块 vs 旧 = {vs_legacy} 像素；共享助手 vs 旧 = {shared_vs_legacy} 像素"
        );
    }
