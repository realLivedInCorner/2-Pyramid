    use super::*;
    use crate::arom::{write_zip, Pack, SafeLimits, SerializeOptions};
    use crate::converters::pack_diff::diff_containers;
    use std::io::Write as _;
    use std::path::{Path, PathBuf};

    fn png(size: (u32, u32)) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(size.0, size.1, image::Rgba([200, 30, 30, 255]));
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .expect("encode png");
        buf
    }

    /// 夹具刻意覆盖：被删的目录、被复制的两张旧贴图、待升级/已升级/非动画三种 mcmeta、无关文件。
    fn write_fixture(path: &Path) {
        let file = std::fs::File::create(path).expect("create fixture");
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::FileOptions::default();
        let mut add = |name: &str, body: Vec<u8>| {
            zip.start_file(name, opts).expect("start");
            zip.write_all(&body).expect("write");
        };

        add("pack.mcmeta", br#"{"pack":{"pack_format":34}}"#.to_vec());
        // 试点 1：整棵子树
        add("assets/minecraft/font/default.json", b"{}".to_vec());
        add("assets/minecraft/font/extra/deep.json", b"{}".to_vec());
        // 试点 1b–1e：另外四种「存在即删」（两目录 / 目录 / 目录 / 单文件）
        add("assets/minecraft/blockstates/oak.json", b"{}".to_vec());
        add("assets/minecraft/models/item/x.json", b"{}".to_vec());
        add(
            "assets/minecraft/textures/entity/horse/horse_brown.png",
            png((16, 16)),
        );
        add("assets/minecraft/shaders/core/x.fsh", b"void main(){}".to_vec());
        add(
            "assets/minecraft/textures/misc/enchanted_item_glint.png",
            png((16, 16)),
        );
        // 试点 1f：大改名任务（items/blocks 合并 + 两张重命名表 + 图像派生）
        //   这里刻意让 `item/` 与 `blocks/`+`block/` **同时存在**，以覆盖「合并」分支
        //   （真实包就是这种形态；只覆盖「纯改名」分支曾让一个真 bug 溜过去）。
        add("assets/minecraft/textures/items/gold_sword.png", png((16, 16)));
        add(
            "assets/minecraft/textures/items/gold_sword.png.mcmeta",
            br#"{"x":1}"#.to_vec(),
        );
        add("assets/minecraft/textures/item/gold_sword.png", png((8, 8)));
        add("assets/minecraft/textures/item/keep_me.png", png((8, 8)));
        add("assets/minecraft/textures/blocks/planks_oak.png", png((16, 16)));
        add("assets/minecraft/textures/block/keep_me_too.png", png((8, 8)));
        // 现代名的「目标名已存在」情形（真实包就是这样：`blocks/` 里全是现代名，
        // 重命名表的**目标**名因此天然已存在）——最小复现用。
        add("assets/minecraft/textures/blocks/dark_oak_planks.png", png((16, 16)));
        add("assets/minecraft/textures/blocks/farmland.png", png((16, 16)));
        add(
            "assets/minecraft/textures/blocks/redstone_dust_cross.png",
            png((16, 16)),
        );
        add("assets/minecraft/textures/blocks/coal_ore.png", png((16, 16)));
        add("assets/minecraft/textures/blocks/nether_quartz_ore.png", png((16, 16)));
        // 试点 2：旧贴图路径
        add("assets/minecraft/terrain.png", png((16, 16)));
        add("assets/minecraft/gui/items.png", png((16, 32)));
        // 试点 3：目录改名
        add("assets/minecraft/mcpatcher/cit/a.properties", b"a=1".to_vec());
        add("assets/minecraft/mcpatcher/cit/deep/b.properties", b"b=2".to_vec());
        // 试点 3：动画 mcmeta
        add("assets/minecraft/textures/item/water.png", png((32, 64)));
        add(
            "assets/minecraft/textures/item/water.png.mcmeta",
            br#"{"animation": {}}"#.to_vec(),
        );
        add("assets/minecraft/textures/item/stone.png", png((16, 16)));
        add(
            "assets/minecraft/textures/item/stone.png.mcmeta",
            br#"{"animation": {"frametime": 3}}"#.to_vec(),
        );
        add("assets/minecraft/textures/item/tool.png", png((16, 16)));
        add(
            "assets/minecraft/textures/item/tool.png.mcmeta",
            br#"{"texture": "x"}"#.to_vec(),
        );
        // 无关文件（验证透传）
        add("assets/minecraft/lang/zh_cn.json", br#"{"a":"b"}"#.to_vec());
        zip.finish().expect("finish");
    }

    /// 真实包上**暂不参与**双轨对照的迁移试点（附原因）。
    ///
    /// `rename_blocks`：本轮定位到两个原因——① 驱动把层写回 workdir 时**没有应用改名规则**
    /// （旧任务于是在 `items/`/`blocks/` 旧布局上工作）；② 补上之后试点本身在真实包上仍有分歧
    /// （`dark_oak_planks.png` / `farmland.png`）。因此它仍不派发、也不参与真实包对照，详见细则 §9.14。
    const REAL_PACK_SKIP: [&str; 0] = [];

    fn old_path_output(fixture: &Path, tmp: &Path, skip: &[&str]) -> PathBuf {
        use crate::converters::textures::{
            animated, drop_blockstates_models, drop_enchanted_glint, drop_font, drop_horse,
            drop_shaders, old_paths,
        };
        use crate::hurray::context::HurrayContext;

        let work = tmp.join("work");
        std::fs::create_dir_all(&work).expect("mkdir");
        crate::converters::zip::extract_resource_pack(
            fixture.to_str().expect("utf8"),
            work.to_str().expect("utf8"),
        )
        .expect("extract");

        // Eraser 阶段（与活注册表同序：四个删除任务 → 字体 → 大改名 → 改名；延迟清理一次执行）
        let ctx = HurrayContext::new(work.to_str().expect("utf8"));
        drop_blockstates_models::delete_blockstates_models(&ctx).expect("drop blockstates");
        drop_horse::delete_horse_folder(&ctx).expect("drop horse");
        drop_shaders::delete_shaders_folder(&ctx).expect("drop shaders");
        drop_enchanted_glint::delete_enchanted_item_glint(&ctx).expect("drop glint");
        drop_font::delete_font_folder(&ctx).expect("delete_font_folder");
        ctx.execute_cleanup().expect("cleanup");
        if !skip.contains(&"rename_blocks") {
            crate::converters::textures::rename_blocks::rename_blocks_items(&work)
                .expect("rename blocks/items");
        }
        old_paths::convert_old_texture_paths(&work).expect("old paths");
        crate::converters::textures::mcpatcher_to_optifine::rename_mcpatcher_to_optifine(&work)
            .expect("rename mcpatcher");

        // Surgeon 阶段
        animated::convert_animated_textures(&work).expect("animated");

        let out = tmp.join("old.zip");
        crate::converters::zip::repack_resource_pack(
            work.to_str().expect("utf8"),
            out.to_str().expect("utf8"),
        )
        .expect("repack");
        out
    }

    fn new_path_output(fixture: &Path, tmp: &Path, skip: &[&str]) -> (PathBuf, Vec<Outcome>) {
        let mut pack = Pack::open_zip(fixture, &SafeLimits::preserving_current(), None).expect("open");
        let mut outcomes = Vec::new();

        for (name, _decl, run) in all() {
            if skip.contains(&name) {
                // 占位保持下标稳定，便于断言按序对应
                outcomes.push(Outcome::default());
                continue;
            }
            let layer = {
                let mut tx = pack.tx(name);
                outcomes.push(run(&mut tx).expect("pilot run"));
                tx.into_layer()
            };
            pack.commit(layer);
        }
        let out = tmp.join("new.zip");
        let view = pack.view();
        write_zip(&pack, &view, &out, &SerializeOptions::default()).expect("serialize");
        (out, outcomes)
    }

    fn assert_equivalent(old: &Path, new: &Path) -> crate::converters::pack_diff::PackDiffReport {
        let report = diff_containers(old, new).expect("diff");
        assert_eq!(report.blocking, 0, "内容差异：{:?}", report.diffs);
        assert_eq!(
            report.container_entry_set_blocking, 0,
            "条目集合差异：{:?}",
            report.container
        );
        assert_eq!(
            report.container_byte_only, 0,
            "容器字节属性差异：{:?}",
            report.container
        );
        assert!(report.passed(true), "严格模式必须通过：{:?}", report.container);
        report
    }

    #[test]
    fn natives_match_the_old_implementations() {
        let skip: &[&str] = &[];
        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture = tmp.path().join("fixture.zip");
        write_fixture(&fixture);

        let old = old_path_output(&fixture, tmp.path(), skip);
        let (new, outcomes) = new_path_output(&fixture, tmp.path(), skip);

        assert_equivalent(&old, &new);
        // all() 的顺序：drop_font, drop_blockstates, drop_horse, drop_shaders, drop_glint,
        //                rename_blocks, old_paths, mcpatcher_optifine, animated
        assert_eq!(outcomes[0].changed, 1, "font 目录应被删除：{outcomes:?}");
        assert_eq!(outcomes[1].changed, 2, "blockstates 与 models 各删一个：{outcomes:?}");
        assert_eq!(outcomes[2].changed, 1, "horse 目录应被删除：{outcomes:?}");
        assert_eq!(outcomes[3].changed, 1, "shaders 目录应被删除：{outcomes:?}");
        assert_eq!(outcomes[4].changed, 1, "glint 单文件应被删除：{outcomes:?}");
        assert_eq!(
            outcomes[5].changed, 5,
            "items 合并 + blocks 合并 + 物品改名 + 方块改名 + nether_gold 派生：{outcomes:?}"
        );
        assert_eq!(outcomes[6].changed, 2, "两张旧贴图应被复制：{outcomes:?}");
        assert_eq!(outcomes[7].changed, 1, "mcpatcher 应被改名：{outcomes:?}");
        assert_eq!(outcomes[8].changed, 1, "只有 water 需要升级：{outcomes:?}");
        assert_eq!(
            outcomes[8].skipped, 3,
            "夹具共 4 个 mcmeta：water 升级，stone/tool/改名后的 golden_sword 各跳过一条：{outcomes:?}"
        );
    }

    #[test]
    fn pilot_effects_are_visible_in_the_new_output() {
        let skip: &[&str] = &[];
        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture = tmp.path().join("fixture.zip");
        write_fixture(&fixture);
        let (new, _) = new_path_output(&fixture, tmp.path(), skip);

        let pack = Pack::open_zip(&new, &SafeLimits::preserving_current(), None).expect("reopen");
        let view = pack.view();

        assert!(view.resolve("assets/minecraft/font").is_none(), "font 子树已删");
        assert!(
            view.resolve("assets/minecraft/blockstates").is_none(),
            "blockstates 子树已删"
        );
        assert!(view.resolve("assets/minecraft/models").is_none(), "models 子树已删");
        assert!(
            view.resolve("assets/minecraft/textures/entity/horse").is_none(),
            "horse 子树已删"
        );
        assert!(view.resolve("assets/minecraft/shaders").is_none(), "shaders 子树已删");
        assert!(
            view.resolve("assets/minecraft/textures/misc/enchanted_item_glint.png")
                .is_none(),
            "glint 文件已删"
        );
        // 大改名任务：目录合并 + 重命名 + 图像派生
        assert!(
            view.resolve("assets/minecraft/textures/items").is_none(),
            "items 已并入 item"
        );
        assert!(
            view.resolve("assets/minecraft/textures/item/golden_sword.png").is_some(),
            "重命名后的物品贴图"
        );
        assert!(
            view.resolve("assets/minecraft/textures/item/gold_sword.png").is_none(),
            "旧名消失"
        );
        assert!(
            view.resolve("assets/minecraft/textures/item/golden_sword.png.mcmeta").is_some(),
            "附属 mcmeta 跟着搬"
        );
        assert!(
            view.resolve("assets/minecraft/textures/block/deepslate_coal_ore.png").is_some(),
            "矿石派生图"
        );
        assert!(
            view.resolve("assets/minecraft/textures/block/red_dust_dot.png").is_some(),
            "红石粉十字派生"
        );
        assert!(
            view.resolve("assets/minecraft/textures/block/nether_gold_ore.png").is_some(),
            "白→黄派生"
        );
        assert!(
            view.resolve("assets/minecraft/textures/block/warped_planks.png").is_some(),
            "色相派生"
        );
        // 合并分支：源覆盖目标、目标独有者保留
        assert!(
            view.resolve("assets/minecraft/textures/item/keep_me.png").is_some(),
            "目标独有文件必须保留"
        );
        assert!(
            view.resolve("assets/minecraft/textures/block/keep_me_too.png").is_some(),
            "目标独有文件必须保留（block）"
        );
        assert!(view.resolve("assets/minecraft/block.png").is_some(), "复制产物存在");
        assert!(view.resolve("assets/minecraft/terrain.png").is_some(), "Copy 不动源文件");
        assert!(view.resolve("assets/minecraft/item.png").is_some());
        assert!(
            view.resolve("assets/minecraft/mcpatcher").is_none(),
            "Move：源目录消失"
        );
        assert!(
            view.resolve("assets/minecraft/optifine/cit/deep/b.properties").is_some(),
            "改名后子树跟着走"
        );

        let meta: serde_json::Value = view
            .json("assets/minecraft/textures/item/water.png.mcmeta")
            .expect("json");
        assert_eq!(meta["animation"]["frametime"], serde_json::json!(2));
        assert_eq!(meta["animation"]["interpolate"], serde_json::json!(true));

        // 现代名 + 目标名已存在：不得被删掉（真实包形态的最小复现点）
        assert!(
            view.resolve("assets/minecraft/textures/block/dark_oak_planks.png")
                .is_some(),
            "已存在的目标名必须保留：dark_oak_planks"
        );
        assert!(
            view.resolve("assets/minecraft/textures/block/farmland.png").is_some(),
            "已存在的目标名必须保留：farmland"
        );
    }

    #[test]
    /// ui 模块第一个迁移任务的专项双轨对照：旧 `process_chest_folder` vs A-ROM 原生实现。
    ///
    /// 不动 `all()` 的下标（那是别的用例的依赖），单独用胸口贴图夹具做闸门对照。
    #[test]
    fn chest_pilot_matches_the_old_implementation() {
        use crate::converters::pack_diff::diff_containers;

        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture = tmp.path().join("chest.zip");
        {
            let file = std::fs::File::create(&fixture).expect("create");
            let mut zip = zip::ZipWriter::new(file);
            let opts = zip::write::FileOptions::default();
            let mut add = |name: &str, body: Vec<u8>| {
                zip.start_file(name, opts).expect("start");
                zip.write_all(&body).expect("write");
            };
            add("pack.mcmeta", br#"{"pack":{"pack_format":34}}"#.to_vec());
            add(
                "assets/minecraft/textures/entity/chest/normal.png",
                png((64, 64)),
            );
            add(
                "assets/minecraft/textures/entity/chest/normal_double.png",
                png((128, 64)),
            );
            add(
                "assets/minecraft/textures/entity/chest/ender.png",
                png((32, 32)),
            );
            zip.finish().expect("finish");
        }

        let work = tmp.path().join("chest_legacy");
        std::fs::create_dir_all(&work).expect("mkdir");
        crate::converters::zip::extract_resource_pack(
            fixture.to_str().expect("utf8"),
            work.to_str().expect("utf8"),
        )
        .expect("extract");
        crate::converters::ui::process_chest_folder::process_chest_folder(&work).expect("legacy");
        let legacy_out = tmp.path().join("chest_legacy.zip");
        crate::converters::zip::repack_resource_pack(
            work.to_str().expect("utf8"),
            legacy_out.to_str().expect("utf8"),
        )
        .expect("repack");

        let mut pack =
            Pack::open_zip(&fixture, &SafeLimits::preserving_current(), None).expect("open");
        let outcome = {
            let mut tx = pack.tx("process_chest_folder");
            let outcome = chest::run(&mut tx).expect("pilot");
            let layer = tx.into_layer();
            pack.commit(layer);
            outcome
        };
        assert_eq!(outcome.changed, 2, "一张单胸 + 一张双胸：{outcome:?}");
        assert_eq!(outcome.skipped, 1, "32×32 不支持应跳过：{outcome:?}");

        let native_out = tmp.path().join("chest_native.zip");
        {
            let view = pack.view();
            write_zip(&pack, &view, &native_out, &SerializeOptions::default()).expect("write");
        }

        let report = diff_containers(&legacy_out, &native_out).expect("diff");
        assert_eq!(report.blocking, 0, "内容差异：{:?}", report.diffs);
        assert_eq!(
            report.container_entry_set_blocking, 0,
            "条目集合差异：{:?}",
            report.container
        );
        assert_eq!(
            report.container_byte_only, 0,
            "压缩方法/字节差异：{:?}",
            report.container
        );
    }

    /// 反向转换第一个任务的专项双轨对照：旧 `reverse_process_chest_folder` vs A-ROM 原生实现。
    #[test]
    fn reverse_chest_pilot_matches_the_old_implementation() {
        use crate::converters::pack_diff::diff_containers;

        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture = tmp.path().join("chest_rev.zip");
        {
            let file = std::fs::File::create(&fixture).expect("create");
            let mut zip = zip::ZipWriter::new(file);
            let opts = zip::write::FileOptions::default();
            let mut add = |name: &str, body: Vec<u8>| {
                zip.start_file(name, opts).expect("start");
                zip.write_all(&body).expect("write");
            };
            add("pack.mcmeta", br#"{"pack":{"pack_format":34}}"#.to_vec());
            add(
                "assets/minecraft/textures/entity/chest/normal.png",
                png((64, 64)),
            );
            add(
                "assets/minecraft/textures/entity/chest/ender.png",
                png((32, 32)),
            );
            zip.finish().expect("finish");
        }

        let work = tmp.path().join("chest_rev_legacy");
        std::fs::create_dir_all(&work).expect("mkdir");
        crate::converters::zip::extract_resource_pack(
            fixture.to_str().expect("utf8"),
            work.to_str().expect("utf8"),
        )
        .expect("extract");
        crate::converters::reverse::chest_folder::reverse_process_chest_folder(&work)
            .expect("legacy reverse");
        let legacy_out = tmp.path().join("chest_rev_legacy.zip");
        crate::converters::zip::repack_resource_pack(
            work.to_str().expect("utf8"),
            legacy_out.to_str().expect("utf8"),
        )
        .expect("repack");

        let mut pack =
            Pack::open_zip(&fixture, &SafeLimits::preserving_current(), None).expect("open");
        let outcome = {
            let mut tx = pack.tx("reverse_process_chest_folder");
            let outcome = chest_reverse::run(&mut tx).expect("pilot");
            let layer = tx.into_layer();
            pack.commit(layer);
            outcome
        };
        assert_eq!(outcome.changed, 1, "只有 64×64 的 normal 应被处理：{outcome:?}");
        assert_eq!(outcome.skipped, 1, "32×32 不支持应跳过：{outcome:?}");

        let native_out = tmp.path().join("chest_rev_native.zip");
        {
            let view = pack.view();
            write_zip(&pack, &view, &native_out, &SerializeOptions::default()).expect("write");
        }

        let report = diff_containers(&legacy_out, &native_out).expect("diff");
        assert_eq!(report.blocking, 0, "内容差异：{:?}", report.diffs);
        assert_eq!(
            report.container_entry_set_blocking, 0,
            "条目集合差异：{:?}",
            report.container
        );
        assert_eq!(
            report.container_byte_only, 0,
            "压缩方法/字节差异：{:?}",
            report.container
        );
    }

    fn frames_derivation_matches_the_old_rule() {
        assert_eq!(animated::frames_for(16, 16), Some(1), "正方形 1 帧");
        assert_eq!(animated::frames_for(32, 64), Some(2), "纵向条带");
        assert_eq!(animated::frames_for(64, 32), Some(2), "横向条带");
        assert_eq!(animated::frames_for(30, 20), None, "不能整除 → 跳过");
    }

    #[test]
    fn declarations_match_the_actual_effects() {
        // 声明的写范围必须真的覆盖各试点写下的路径（否则调试断言会当场报错）
        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture = tmp.path().join("fixture.zip");
        write_fixture(&fixture);

        let pack = Pack::open_zip(&fixture, &SafeLimits::preserving_current(), None).expect("open");
        for (name, decl, run) in all() {
            let mut tx = pack.tx(name);
            run(&mut tx).expect("run");
            for path in tx.layer().writes().keys() {
                decl.check_write(path).unwrap_or_else(|e| panic!("{e}"));
            }
        }
    }

    /// 真实包上的等价性（默认忽略）：
    /// `AROM_REAL_PACK=<包路径> cargo test --lib natives -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn natives_match_the_old_implementations_on_a_real_pack() {
        let skip = &REAL_PACK_SKIP;
        let Ok(src) = std::env::var("AROM_REAL_PACK") else {
            println!("AROM_REAL_PACK 未设置，跳过");
            return;
        };
        let fixture = std::path::PathBuf::from(&src);
        assert!(fixture.is_file(), "不是文件：{}", fixture.display());
        let tmp = tempfile::tempdir().expect("tempdir");

        let old = old_path_output(&fixture, tmp.path(), skip);
        let (new, outcomes) = new_path_output(&fixture, tmp.path(), skip);
        let report = assert_equivalent(&old, &new);
        println!("source = {}", fixture.display());
        println!("outcomes = {outcomes:?}");
        println!("report = {}", report.summary());
    }

    /// **夹具正题**（默认忽略）：四个任务的源在真实包里走的是「跳过」分支（§9.52/§9.56），
    /// 所以真实包证明不了它们**算得对**。这里自造 1.13+ 路径的源（部分贴图直接取自真实包），
    /// 对**同一份输入**分别跑旧转换器函数与原生实现，逐像素比对。
    ///
    /// 需要真实 `UImage`：解析不到就直接失败（**不许静默跳过**，否则这个用例会变成空跑）。
    #[test]
    #[ignore]
    fn uimage_tasks_match_the_old_implementations_on_a_fixture() {
        let Ok(src) = std::env::var("AROM_REAL_PACK") else {
            println!("AROM_REAL_PACK 未设置，跳过");
            return;
        };
        let real = std::path::PathBuf::from(&src);
        assert!(real.is_file(), "不是文件：{}", real.display());

        // ① UImage 必须可用，否则这四个任务在两边都会跳过，用例失去意义
        let uimage = crate::image_utils::get_uimage_path().expect("UImage 必须可解析");
        for probe in [
            "crossbow/crossbow_16.png",
            "crossbow/crossbow_firework_16.png",
            "tipped_arrow_head/tipped_arrow_head_16.png",
            "powder_snow_bucket/powder_snow_bucket_16.png",
            "water_bucket/cod_bucket_16.png",
        ] {
            assert!(
                uimage.join(probe).is_file(),
                "UImage 缺少 {probe}（{}）",
                uimage.display()
            );
        }

        // ② 夹具：源取自真实包（bow / bow_pulling_* / arrow / water_bucket / milk_bucket），
        //    路径换成新格式；目标是 16x16，与上面探测的覆盖图同名尺寸对齐。
        let item = "assets/minecraft/textures/item";
        let items_legacy = "assets/minecraft/textures/items";
        let sources: [(&str, &str); 9] = [
            (
                "assets/minecraft/textures/items/bow_standby.png",
                &format!("{item}/bow.png"),
            ),
            (
                "assets/minecraft/textures/items/bow_pulling_0.png",
                &format!("{item}/bow_pulling_0.png"),
            ),
            (
                "assets/minecraft/textures/items/bow_pulling_1.png",
                &format!("{item}/bow_pulling_1.png"),
            ),
            (
                "assets/minecraft/textures/items/bow_pulling_2.png",
                &format!("{item}/bow_pulling_2.png"),
            ),
            ("assets/minecraft/textures/items/arrow.png", &format!("{items_legacy}/arrow.png")),
            (
                "assets/minecraft/textures/items/bucket_water.png",
                &format!("{item}/water_bucket.png"),
            ),
            (
                "assets/minecraft/textures/items/bucket_milk.png",
                &format!("{item}/milk_bucket.png"),
            ),
            // 让 `snow_bucket` 的覆盖图分支也走到（不与 milk 同图，便于区分）
            (
                "assets/minecraft/textures/items/bucket_lava.png",
                &format!("{item}/milk_bucket_overlay_probe.png"),
            ),
            // `shulker_box_ui` 的输入：真实包里有 1.9 路径的 `gui/container/generic_54.png`
            (
                "assets/minecraft/textures/gui/container/generic_54.png",
                "assets/minecraft/textures/gui/container/generic_54.png",
            ),
        ];

        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture = tmp.path().join("uimage_fixture.zip");
        {
            use std::io::Write as _;
            let file = std::fs::File::create(&fixture).expect("create");
            let mut zip = zip::ZipWriter::new(file);
            let opts = zip::write::FileOptions::default();
            let mut add = |name: &str, body: Vec<u8>| {
                zip.start_file(name, opts).expect("start");
                zip.write_all(&body).expect("write");
            };
            add("pack.mcmeta", br#"{"pack":{"pack_format":34}}"#.to_vec());
            for (src_name, dst_name) in sources {
                let bytes = read_zip_entry(&real, src_name)
                    .unwrap_or_else(|| panic!("真实包里没有 {src_name}"));
                add(dst_name, bytes);
            }
            zip.finish().expect("finish");
        }

        // ③ 旧侧：解压后直接调旧转换器函数
        let legacy_dir = tmp.path().join("legacy");
        std::fs::create_dir_all(&legacy_dir).expect("mkdir");
        crate::converters::zip::extract_resource_pack(
            fixture.to_str().expect("utf8"),
            legacy_dir.to_str().expect("utf8"),
        )
        .expect("extract");

        // ④ 原生侧：同一个 zip 建 Pack，跑完各任务后物化到目录
        let native_dir = tmp.path().join("native");
        std::fs::create_dir_all(&native_dir).expect("mkdir");
        {
            let mut pack = Pack::open_zip(&fixture, &SafeLimits::preserving_current(), None)
                .expect("open fixture");
            for name in [
                "generate_crossbow",
                "generate_tipped_arrow_images",
                "generate_snow_bucket",
                "generate_fish_bucket",
                "generate_shulker_box_ui",
            ] {
                let (_, _, run) = crate::native_run::native_for_probe(name)
                    .unwrap_or_else(|| panic!("{name} 未在派发表里"));
                let mut tx = pack.tx(name);
                run(&mut tx).expect("native run");
                pack.commit(tx.into_layer());
            }
            let view = pack.view();
            crate::arom::pathview::materialize(&view, &native_dir).expect("materialize");
        }

        // ⑤ 跑旧函数并逐个输出对比
        let cases: [(&str, fn(&std::path::Path) -> Result<(), String>, Vec<String>); 5] = [
            (
                "generate_crossbow",
                crate::converters::textures::crossbow::generate_crossbow,
                [
                    "crossbow_standby",
                    "crossbow_pulling_0",
                    "crossbow_pulling_1",
                    "crossbow_pulling_2",
                    "crossbow_arrow",
                    "crossbow_firework",
                ]
                .iter()
                .map(|n| format!("{item}/{n}.png"))
                .collect(),
            ),
            (
                "generate_tipped_arrow_images",
                crate::converters::textures::tipped_arrows::generate_tipped_arrow_images,
                ["tipped_arrow_base", "tipped_arrow_head"]
                    .iter()
                    .map(|n| format!("{items_legacy}/{n}.png"))
                    .collect(),
            ),
            (
                "generate_snow_bucket",
                crate::converters::textures::snow_bucket::generate_snow_bucket,
                vec![format!("{item}/powder_snow_bucket.png")],
            ),
            (
                "generate_fish_bucket",
                crate::converters::textures::fish_bucket::generate_fish_bucket,
                ["axolotl", "cod", "pufferfish", "salmon", "tropical_fish", "tadpole"]
                    .iter()
                    .map(|n| format!("{item}/{n}_bucket.png"))
                    .collect(),
            ),
            (
                "generate_shulker_box_ui",
                crate::converters::ui::shulker_box::generate_shulker_box_ui,
                vec![format!(
                    "assets/minecraft/textures/gui/container/shulker_box.png"
                )],
            ),
        ];

        let mut problems: Vec<String> = Vec::new();
        for (name, legacy_fn, outputs) in cases {
            legacy_fn(&legacy_dir).unwrap_or_else(|e| panic!("{name} 旧实现失败：{e}"));
            let mut compared = 0usize;
            for rel in &outputs {
                let a = std::fs::read(legacy_dir.join(rel));
                let b = std::fs::read(native_dir.join(rel));
                match (a, b) {
                    (Ok(a), Ok(b)) => {
                        let ia = image::load_from_memory(&a)
                            .expect("decode legacy")
                            .to_rgba8();
                        let ib = image::load_from_memory(&b)
                            .expect("decode native")
                            .to_rgba8();
                        if ia.dimensions() != ib.dimensions() {
                            problems.push(format!(
                                "{name}: {rel} 尺寸不同 {:?} vs {:?}",
                                ia.dimensions(),
                                ib.dimensions()
                            ));
                            continue;
                        }
                        let mut diff = 0usize;
                        let mut worst = 0i32;
                        for (pa, pb) in ia.pixels().zip(ib.pixels()) {
                            if pa.0 != pb.0 {
                                diff += 1;
                                for c in 0..4 {
                                    worst = worst.max((pa.0[c] as i32 - pb.0[c] as i32).abs());
                                }
                            }
                        }
                        if diff > 0 {
                            problems.push(format!(
                                "{name}: {rel} 像素不同 {diff}/{} 最大通道差 {worst}",
                                ia.pixels().len()
                            ));
                        }
                        compared += 1;
                    }
                    (Err(_), Err(_)) => {
                        problems.push(format!("{name}: {rel} 两边都没生成"));
                    }
                    (a, b) => {
                        problems.push(format!(
                            "{name}: {rel} 一侧缺失（旧={} 原生={}）",
                            a.is_ok(),
                            b.is_ok()
                        ));
                    }
                }
            }
            println!("{name}: 比对 {compared}/{} 个产物", outputs.len());
        }

        assert!(problems.is_empty(), "夹具正题差异：{problems:#?}");
    }

    /// **`fix_tabs` 的正题**（默认忽略）。
    ///
    /// 为什么需要单独测：`fix_tabs` 在**本仓的验收路径上根本不出现**——`scheduler` 有一条规则
    /// `from==9 && to==12 && target_version>15` 时**跳过**它，而 1→97 的路径不含 9→12 段。
    /// 也就是说真实包闸门**覆盖不到**它，只能靠这里证明「算法与旧实现一致」。
    #[test]
    #[ignore]
    fn fix_tabs_matches_the_old_implementation_on_a_fixture() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture = tmp.path().join("tabs_fixture.zip");
        let tabs_rel = "assets/minecraft/textures/gui/container/creative_inventory/tabs.png";

        // 造一张 256×256 的 tabs.png：每个 8×8 区块用坐标派生出的确定性颜色，便于比对
        let make_tabs = |size: u32| -> Vec<u8> {
            let mut img = RgbaImage::new(size, size);
            for y in 0..size {
                for x in 0..size {
                    img.put_pixel(
                        x,
                        y,
                        image::Rgba([
                            (x % 251) as u8,
                            (y % 251) as u8,
                            ((x + y) % 251) as u8,
                            if (x + y) % 7 == 0 { 0 } else { 255 },
                        ]),
                    );
                }
            }
            let mut buf = Vec::new();
            image::DynamicImage::ImageRgba8(img)
                .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                .expect("encode");
            buf
        };

        for size in [256u32, 512u32] {
            let fixture = tmp.path().join(format!("tabs_{size}.zip"));
            {
                use std::io::Write as _;
                let file = std::fs::File::create(&fixture).expect("create");
                let mut zip = zip::ZipWriter::new(file);
                let opts = zip::write::FileOptions::default();
                zip.start_file("pack.mcmeta", opts).expect("start");
                zip.write_all(br#"{"pack":{"pack_format":34}}"#).expect("write");
                zip.start_file(tabs_rel, opts).expect("start");
                zip.write_all(&make_tabs(size)).expect("write");
                zip.finish().expect("finish");
            }

            // 旧侧
            let legacy_dir = tmp.path().join(format!("legacy_{size}"));
            std::fs::create_dir_all(&legacy_dir).expect("mkdir");
            crate::converters::zip::extract_resource_pack(
                fixture.to_str().expect("utf8"),
                legacy_dir.to_str().expect("utf8"),
            )
            .expect("extract");
            crate::converters::ui::tabs::fix_tabs(&legacy_dir).expect("legacy fix_tabs");

            // 原生侧
            let native_dir = tmp.path().join(format!("native_{size}"));
            std::fs::create_dir_all(&native_dir).expect("mkdir");
            {
                let mut pack = Pack::open_zip(&fixture, &SafeLimits::preserving_current(), None)
                    .expect("open fixture");
                let (_, _, run) = crate::native_run::native_for_probe("fix_tabs")
                    .expect("native fix_tabs");
                let mut tx = pack.tx("fix_tabs");
                run(&mut tx).expect("native run");
                pack.commit(tx.into_layer());
                crate::arom::pathview::materialize(&pack.view(), &native_dir)
                    .expect("materialize");
            }

            let a = image::open(legacy_dir.join(tabs_rel)).expect("legacy image").to_rgba8();
            let b = image::open(native_dir.join(tabs_rel)).expect("native image").to_rgba8();
            assert_eq!(a.dimensions(), b.dimensions(), "size {size}: 尺寸不同");
            let mut diff = 0usize;
            let mut worst = 0i32;
            for (pa, pb) in a.pixels().zip(b.pixels()) {
                if pa.0 != pb.0 {
                    diff += 1;
                    for c in 0..4 {
                        worst = worst.max((pa.0[c] as i32 - pb.0[c] as i32).abs());
                    }
                }
            }
            println!("fix_tabs size={size}: 差异 {diff} 像素（最大通道差 {worst}）");
            assert_eq!(diff, 0, "size {size}: fix_tabs 与旧实现不一致");
        }
    }

    /// **`fix_smithing2_villager2_ui` 的正题**（默认忽略，§9.87）。
    ///
    /// 两个子过程都比对：`smithing.png`（由 `anvil.png` 派生）与 `villager.png`（原位改写，
    /// 并产生 `villager_backup.png`）。夹具按 256 与 512 两档尺寸各造一份，
    /// 像素用坐标派生的确定性颜色（含 alpha 变体，以覆盖 paste/overlay 的 alpha 语义）。
    #[test]
    #[ignore]
    fn smithing2_villager2_matches_the_old_implementation_on_a_fixture() {
        let container = "assets/minecraft/textures/gui/container";
        let anvil_rel = format!("{container}/anvil.png");
        let smithing_rel = format!("{container}/smithing.png");
        let villager_rel = format!("{container}/villager.png");
        let backup_rel = format!("{container}/villager_backup.png");

        let make_png = |size: u32, seed: u32| -> Vec<u8> {
            let mut img = RgbaImage::new(size, size);
            for y in 0..size {
                for x in 0..size {
                    img.put_pixel(
                        x,
                        y,
                        image::Rgba([
                            ((x + seed) % 251) as u8,
                            ((y + seed * 3) % 251) as u8,
                            ((x + y + seed * 7) % 251) as u8,
                            if (x + y + seed) % 7 == 0 { 0 } else { 255 },
                        ]),
                    );
                }
            }
            let mut buf = Vec::new();
            image::DynamicImage::ImageRgba8(img)
                .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                .expect("encode");
            buf
        };

        let tmp = tempfile::tempdir().expect("tempdir");
        for size in [256u32, 512u32] {
            let fixture = tmp.path().join(format!("sv_{size}.zip"));
            {
                use std::io::Write as _;
                let file = std::fs::File::create(&fixture).expect("create");
                let mut zip = zip::ZipWriter::new(file);
                let opts = zip::write::FileOptions::default();
                zip.start_file("pack.mcmeta", opts).expect("start");
                zip.write_all(br#"{"pack":{"pack_format":34}}"#).expect("write");
                zip.start_file(&anvil_rel, opts).expect("start");
                zip.write_all(&make_png(size, 1)).expect("write");
                zip.start_file(&villager_rel, opts).expect("start");
                zip.write_all(&make_png(size, 2)).expect("write");
                zip.finish().expect("finish");
            }

            // 旧侧
            let legacy_dir = tmp.path().join(format!("legacy_{size}"));
            std::fs::create_dir_all(&legacy_dir).expect("mkdir");
            crate::converters::zip::extract_resource_pack(
                fixture.to_str().expect("utf8"),
                legacy_dir.to_str().expect("utf8"),
            )
            .expect("extract");
            crate::converters::ui::smithing_villager::fix_smithing2_villager2_ui(&legacy_dir)
                .expect("legacy fix_smithing2_villager2_ui");

            // 原生侧
            let native_dir = tmp.path().join(format!("native_{size}"));
            std::fs::create_dir_all(&native_dir).expect("mkdir");
            {
                let mut pack = Pack::open_zip(&fixture, &SafeLimits::preserving_current(), None)
                    .expect("open fixture");
                let (_, _, run) = crate::native_run::native_for_probe("fix_smithing2_villager2_ui")
                    .expect("native fix_smithing2_villager2_ui");
                let mut tx = pack.tx("fix_smithing2_villager2_ui");
                run(&mut tx).expect("native run");
                pack.commit(tx.into_layer());
                crate::arom::pathview::materialize(&pack.view(), &native_dir)
                    .expect("materialize");
            }

            for rel in [&smithing_rel, &villager_rel, &backup_rel] {
                let lp = legacy_dir.join(rel);
                let np = native_dir.join(rel);
                if lp.exists() != np.exists() {
                    panic!(
                        "size {size} {rel}: 存在性不同（旧 {} / 原生 {}）",
                        lp.exists(),
                        np.exists()
                    );
                }
                if !lp.exists() {
                    continue;
                }
                let a = image::open(&lp).expect("legacy image").to_rgba8();
                let b = image::open(&np).expect("native image").to_rgba8();
                assert_eq!(a.dimensions(), b.dimensions(), "size {size} {rel}: 尺寸不同");
                let mut diff = 0usize;
                let mut worst = 0i32;
                for (pa, pb) in a.pixels().zip(b.pixels()) {
                    if pa.0 != pb.0 {
                        diff += 1;
                        for c in 0..4 {
                            worst = worst.max((pa.0[c] as i32 - pb.0[c] as i32).abs());
                        }
                    }
                }
                println!("size={size} {rel}: 差异 {diff} 像素（最大通道差 {worst}）");
                assert_eq!(diff, 0, "size {size} {rel}: 与旧实现不一致");
            }
        }
    }

    /// **`gui_surgeon_tx` 阶段 1 的正题**（默认忽略，§9.105）。
    ///
    /// 只比对 **`SPRITE_MAP` 主循环**的产出（Late 那批 sprite）。
    /// 做法：造一份含 15 张 `gui/container/*.png` 的夹具（256×256，坐标派生色），
    /// **旧侧**跑完整的 `GuiSurgeon::execute_transformation`（它会产出主循环 + 7 个 `process_*`），
    /// **原生侧**只跑 `cut_sprite_map`。断言：**主循环的那 74 个 target 逐个逐字节相同**。
    ///
    /// 用"逐个比对原生产出"而不是"比对整个 sprites 树"，是因为原生侧**刻意只做了主循环**；
    /// 比对整棵树会把"尚未移植的 7 个过程"误报成差异。
    #[test]
    #[ignore]
    fn gui_surgeon_sprite_map_matches_the_old_implementation_on_a_fixture() {
        let container = "assets/minecraft/textures/gui/container";
        // 与 legacy `SPRITE_MAP` 的 source_name 去重结果一致（15 个）。
        const SOURCES: [&str; 15] = [
            "anvil", "beacon", "blast_furnace", "brewing_stand", "cartography_table",
            "enchanting_table", "furnace", "grindstone", "horse", "inventory", "loom",
            "smithing", "smoker", "stonecutter", "villager2",
        ];

        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture = tmp.path().join("gui_fixture.zip");
        {
            use std::io::Write as _;
            let f = std::fs::File::create(&fixture).expect("create");
            let mut zip = zip::ZipWriter::new(f);
            let opts = zip::write::FileOptions::default();
            zip.start_file("pack.mcmeta", opts).expect("start");
            zip.write_all(br#"{"pack":{"pack_format":34}}"#).expect("write");
            for name in SOURCES {
                // 256×256：与 base_width=256/512 的缩放路径都对得上（512 档会取 256/512=0.5）
                let mut img = RgbaImage::new(256, 256);
                for y in 0..256u32 {
                    for x in 0..256u32 {
                        img.put_pixel(
                            x,
                            y,
                            image::Rgba([
                                (x % 251) as u8,
                                (y % 251) as u8,
                                ((x * 3 + y * 7) % 251) as u8,
                                if (x + y) % 11 == 0 { 0 } else { 255 },
                            ]),
                        );
                    }
                }
                let mut buf = Vec::new();
                image::DynamicImage::ImageRgba8(img)
                    .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                    .expect("encode");
                zip.start_file(format!("{container}/{name}.png"), opts).expect("start");
                zip.write_all(&buf).expect("write");
            }
            // `save_slices` 的两个直接源图（不在 container/ 下，而在 gui/ 下）
            for name in ["resource_packs", "server_selection"] {
                let mut img = RgbaImage::new(256, 256);
                for y in 0..256u32 {
                    for x in 0..256u32 {
                        img.put_pixel(
                            x,
                            y,
                            image::Rgba([
                                ((x * 5 + y) % 251) as u8,
                                ((y * 3 + 11) % 251) as u8,
                                ((x + y * 2) % 251) as u8,
                                if (x * y) % 13 == 0 { 0 } else { 255 },
                            ]),
                        );
                    }
                }
                let mut buf = Vec::new();
                image::DynamicImage::ImageRgba8(img)
                    .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                    .expect("encode");
                zip.start_file(format!("assets/minecraft/textures/gui/{name}.png"), opts)
                    .expect("start");
                zip.write_all(&buf).expect("write");
            }
            // `process_slider` / `process_title` / `process_widgets` 的直接源图
            for rel in [
                "assets/minecraft/textures/gui/slider.png",
                "assets/minecraft/textures/gui/title/minecraft.png",
                "assets/minecraft/textures/gui/widgets.png",
                "assets/minecraft/textures/gui/container/creative_inventory/tabs.png",
                "assets/minecraft/textures/gui/icons.png",
            ] {
                let mut img = RgbaImage::new(256, 256);
                for y in 0..256u32 {
                    for x in 0..256u32 {
                        img.put_pixel(
                            x,
                            y,
                            image::Rgba([
                                ((x * 7 + y * 2) % 251) as u8,
                                ((y * 5 + 3) % 251) as u8,
                                ((x * 2 + y * 9) % 251) as u8,
                                if (x + y * 3) % 17 == 0 { 0 } else { 255 },
                            ]),
                        );
                    }
                }
                let mut buf = Vec::new();
                image::DynamicImage::ImageRgba8(img)
                    .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                    .expect("encode");
                zip.start_file(rel, opts).expect("start");
                zip.write_all(&buf).expect("write");
            }
            zip.finish().expect("finish");
        }

        // 旧侧：解压 → 完整 GuiSurgeon（它磁盘读写）
        let legacy_dir = tmp.path().join("legacy");
        std::fs::create_dir_all(&legacy_dir).expect("mkdir");
        crate::converters::zip::extract_resource_pack(
            fixture.to_str().expect("utf8"),
            legacy_dir.to_str().expect("utf8"),
        )
        .expect("extract");
        {
            let ctx = crate::hurray::context::HurrayContext::new(
                legacy_dir.to_str().expect("utf8"),
            );
            let mut pool = crate::hurray::texture::TexturePool::new();
            let mut res = crate::hurray::resolution::ResolutionTransducer::new();
            let _ = res.detect_resolution(&legacy_dir);
            crate::converters::ui::gui_surgeon::GuiSurgeon::execute_transformation(
                &ctx, &mut pool, &res,
            )
            .expect("legacy GuiSurgeon");
        }

        // 原生侧：Pack → cut_sprite_map → 物化
        let native_dir = tmp.path().join("native");
        std::fs::create_dir_all(&native_dir).expect("mkdir");
        let written = {
            let mut pack = Pack::open_zip(&fixture, &SafeLimits::preserving_current(), None)
                .expect("open fixture");
            let mut tx = pack.tx("gui_surgeon_tx");
            let mut n = crate::natives::gui_surgeon_tx::cut_sprite_map(&mut tx).expect("native cut");
            // §9.105 阶段 2：`save_slices` 的两个直接调用者。
            n += crate::natives::gui_surgeon_tx::process_resource_packs(&mut tx).expect("resource_packs");
            n += crate::natives::gui_surgeon_tx::process_server_selection(&mut tx).expect("server_selection");
            n += crate::natives::gui_surgeon_tx::process_slider(&mut tx).expect("slider");
            n += crate::natives::gui_surgeon_tx::process_title(&mut tx).expect("title");
            n += crate::natives::gui_surgeon_tx::process_widgets(&mut tx).expect("widgets");
            n += crate::natives::gui_surgeon_tx::process_tabs(&mut tx).expect("tabs");
            n += crate::natives::gui_surgeon_tx::process_icons(&mut tx).expect("icons");
            pack.commit(tx.into_layer());
            crate::arom::pathview::materialize(&pack.view(), &native_dir).expect("materialize");
            n
        };
        println!("原生已移植部分写出 {written} 个 sprite");

        // 非空转：必须真的写出一批（源图齐全时不至于是 0）
        assert!(written > 0, "原生侧没有写出任何 sprite —— 夹具或实现有问题");

        // **清理清单的两条硬约束**（§9.96/§9.111）：
        // ① 20 项；② **绝不含 `container/inventory.png`**——1.21 客户端仍需它渲染背包背景，
        //    删掉会让生存/创造背包 GUI 消失（旧注释专门写了这段）。
        let cl = crate::natives::gui_surgeon_tx::cleanup_list();
        assert_eq!(cl.len(), 20, "清理清单应为 20 项");
        assert!(
            !cl.iter().any(|p| p.ends_with("container/inventory.png")),
            "清理清单**不得**包含 container/inventory.png（会让背包 GUI 消失）"
        );

        // 逐条比对：原生**写出的每个路径**都必须与旧侧同路径文件逐像素相同。
        //
        // 比对范围 = `sprites/` 整棵子树 **+ `process_title` 会就地回写的源文件**
        // （`gui/title/minecraft.png`）。后者不在 `sprites/` 下，因此必须单独列出——
        // 否则会出现「写出 96、比对 95」，而那 1 个差异恰恰是最需要验证的（就地写语义）。
        let mut targets: Vec<std::path::PathBuf> = Vec::new();
        let sprites_root = native_dir.join("assets/minecraft/textures/gui/sprites");
        let mut stack = vec![sprites_root.clone()];
        while let Some(dir) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    targets.push(p);
                }
            }
        }
        targets.push(native_dir.join("assets/minecraft/textures/gui/title/minecraft.png"));

        let mut checked = 0usize;
        let mut problems: Vec<String> = Vec::new();
        for p in &targets {
            let rel = p.strip_prefix(&native_dir).expect("rel").to_string_lossy().replace('\\', "/");
            let lp = legacy_dir.join(&rel);
            if !lp.exists() {
                problems.push(format!("{rel}: 原生有、旧侧没有"));
                continue;
            }
            let a = std::fs::read(&lp).expect("read legacy");
            let b = std::fs::read(p).expect("read native");
            if a != b {
                // 逐像素给出差异量级（PNG 字节可能因编码不同而不同，故再看像素）
                let ia = image::load_from_memory(&a).map(|i| i.to_rgba8());
                let ib = image::load_from_memory(&b).map(|i| i.to_rgba8());
                match (ia, ib) {
                    (Ok(ia), Ok(ib)) if ia.dimensions() == ib.dimensions() => {
                        let diff = ia.pixels().zip(ib.pixels()).filter(|(x, y)| x.0 != y.0).count();
                        if diff > 0 {
                            problems.push(format!("{rel}: 像素不同 {diff} 个"));
                        }
                    }
                    _ => problems.push(format!("{rel}: 尺寸不同或解码失败")),
                }
            }
            checked += 1;
        }
        println!("逐条比对 {checked} 个路径（含就地回写的源文件）");
        assert!(problems.is_empty(), "已移植部分与旧实现不一致：{problems:#?}");
        // **比对数量 vs 写出数量**：`written` 数的是**写操作次数**，`checked` 数的是**磁盘上的文件**。
        // 旧实现里有**幂等的重复写**（`process_widgets` 对 `language.png` 写了两次），
        // 因此 `checked` 可以**小于** `written`，但**绝不能**小太多——
        // 用 `written - 少量重复` 兜底，既能接受已知的幂等重复，又能抓住"大片段没写出去"。
        assert!(
            checked <= written && checked + 8 >= written,
            "比对数量与写出数量差距过大：checked={checked} written={written}（预期只差少量幂等重复写）"
        );
        assert!(checked > 100, "比对的文件太少（{checked}），疑似大片未写出");

        // **§9.111：顶层 `run` 的资源守恒自检**。
        //
        // `run` = 8 个步骤 + 把清理清单里**此刻仍存在**的文件登记为延迟删除。
        // 夹具里那些文件都在，故 `run` 之后：
        //   写操作数 = written + 清理项数（`Tx` 里删除算写：`Slot::Tombstone`）
        //   文件数   = 上面比对的 `checked`（删除不产生文件）
        // 这条自检能在**不依赖旧实现**的前提下抓住"run 漏调了某一步或漏登记了删除"。
        {
            let mut pack = Pack::open_zip(&fixture, &SafeLimits::preserving_current(), None)
                .expect("open fixture");
            let outcome = {
                let mut tx = pack.tx("gui_surgeon_tx_run");
                let o = crate::natives::gui_surgeon_tx::run(&mut tx).expect("native run");
                pack.commit(tx.into_layer());
                o
            };
            println!(
                "顶层 run：changed={} deferred_removals={}",
                outcome.changed,
                outcome.deferred_removals.len()
            );
            assert_eq!(
                outcome.changed, written,
                "run 的写操作数应与逐步调用之和一致（漏调了某一步？）"
            );
            assert_eq!(
                outcome.deferred_removals.len(),
                20,
                "夹具里清理清单的 20 个文件都在，应全部登记为延迟删除"
            );
        }
    }

    /// 从 zip 里读一个条目的字节（读不到返回 None）。
    fn read_zip_entry(zip_path: &std::path::Path, name: &str) -> Option<Vec<u8>> {
        use std::io::Read as _;
        let f = std::fs::File::open(zip_path).ok()?;
        let mut archive = zip::ZipArchive::new(f).ok()?;
        let mut file = archive.by_name(name).ok()?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).ok()?;
        Some(buf)
    }

    /// **`fix_clock_compass` 的正题**（默认忽略）。
    ///
    /// 为什么需要单独测：在**生产计划里它是空操作**——旧实现读 `textures/items/{clock,compass}.png`，
    /// 而阶段 3–4 的 `rename_blocks_items` 已经把这两个文件改名到 `item/`，所以源"不存在"→ 整任务跳过。
    /// 真实包闸门因此只证明「我也跳过了」，证明不了**抽帧算法**（这也解释了它为何必须放在后阶段，见 §9.59）。
    ///
    /// 这里自造纵向条带图，对同一份输入分别跑旧函数与原生实现，逐像素 + 逐条目比对。
    #[test]
    #[ignore]
    fn clock_compass_split_matches_the_old_implementation_on_a_fixture() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture = tmp.path().join("split_fixture.zip");

        // clock：8x8 的宽度、高 8*8=64（8 帧）→ retain 64 → 全部保留，无抽取
        // compass：8x8 的宽度、高 8*32=256（32 帧）→ retain 32 → 全部保留
        // 另造一张「帧数多于 retain」的：高度 8*128=1024，retain 64 → 走抽帧分支
        let make_strip = |w: u32, frames: u32| -> Vec<u8> {
            let mut img = RgbaImage::new(w, w * frames);
            for y in 0..img.height() {
                for x in 0..w {
                    let frame = y / w;
                    img.put_pixel(
                        x,
                        y,
                        image::Rgba([(frame % 251) as u8, (x * 7 % 251) as u8, (y % 251) as u8, 255]),
                    );
                }
            }
            let mut buf = Vec::new();
            image::DynamicImage::ImageRgba8(img)
                .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                .expect("encode");
            buf
        };
        {
            use std::io::Write as _;
            let file = std::fs::File::create(&fixture).expect("create");
            let mut zip = zip::ZipWriter::new(file);
            let opts = zip::write::FileOptions::default();
            let mut add = |name: &str, body: Vec<u8>| {
                zip.start_file(name, opts).expect("start");
                zip.write_all(&body).expect("write");
            };
            add("pack.mcmeta", br#"{"pack":{"pack_format":34}}"#.to_vec());
            let items = "assets/minecraft/textures/items";
            add(&format!("{items}/clock.png"), make_strip(8, 8));
            add(&format!("{items}/compass.png"), make_strip(8, 128));
            add(
                &format!("{items}/compass.png.mcmeta"),
                br#"{"animation":{"frametime":1}}"#.to_vec(),
            );
            zip.finish().expect("finish");
        }

        // 旧侧
        let legacy_dir = tmp.path().join("legacy");
        std::fs::create_dir_all(&legacy_dir).expect("mkdir");
        crate::converters::zip::extract_resource_pack(
            fixture.to_str().expect("utf8"),
            legacy_dir.to_str().expect("utf8"),
        )
        .expect("extract");
        let ctx = crate::hurray::context::HurrayContext::new(&legacy_dir.to_string_lossy());
        crate::converters::ui::clock_compass::fix_clock_compass(&ctx).expect("legacy split");

        // 原生侧
        let native_dir = tmp.path().join("native");
        std::fs::create_dir_all(&native_dir).expect("mkdir");
        {
            let mut pack = Pack::open_zip(&fixture, &SafeLimits::preserving_current(), None)
                .expect("open fixture");
            let (_, _, run) = crate::native_run::native_for_probe("fix_clock_compass")
                .expect("native fix_clock_compass");
            let mut tx = pack.tx("fix_clock_compass");
            run(&mut tx).expect("native run");
            pack.commit(tx.into_layer());
            crate::arom::pathview::materialize(&pack.view(), &native_dir).expect("materialize");
        }

        // 逐条目比对（含「原图应被删除」这一条）
        let items = "assets/minecraft/textures/items";
        let mut problems = Vec::new();
        let mut checked = 0usize;
        for name in ["clock.png", "clock.png.mcmeta", "compass.png", "compass.png.mcmeta"] {
            let rel = format!("{items}/{name}");
            let a = legacy_dir.join(&rel);
            let b = native_dir.join(&rel);
            if a.exists() != b.exists() {
                problems.push(format!("{rel}: 存在性不同（旧={} 原生={}）", a.exists(), b.exists()));
            }
            checked += 1;
        }
        for (prefix, count) in [("clock", 8usize), ("compass", 64usize)] {
            for j in 0..count {
                let rel = format!("{items}/{prefix}_{j:02}.png");
                let a = std::fs::read(legacy_dir.join(&rel));
                let b = std::fs::read(native_dir.join(&rel));
                match (a, b) {
                    (Ok(a), Ok(b)) => {
                        let ia = image::load_from_memory(&a).expect("decode legacy").to_rgba8();
                        let ib = image::load_from_memory(&b).expect("decode native").to_rgba8();
                        if ia.dimensions() != ib.dimensions() {
                            problems.push(format!("{rel}: 尺寸不同"));
                        } else if ia.pixels().zip(ib.pixels()).any(|(x, y)| x.0 != y.0) {
                            problems.push(format!("{rel}: 像素不同"));
                        }
                        checked += 1;
                    }
                    (Err(_), Err(_)) => {}
                    (a, b) => problems.push(format!(
                        "{rel}: 一侧缺失（旧={} 原生={}）",
                        a.is_ok(),
                        b.is_ok()
                    )),
                }
            }
        }
        println!("fix_clock_compass: 比对 {checked} 个条目");
        assert!(problems.is_empty(), "抽帧夹具差异：{problems:#?}");
    }
