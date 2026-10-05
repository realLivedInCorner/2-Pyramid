    use std::io::Write as _;

    #[test]
    #[ignore]
    fn legacy_armor_then_netherite_step_by_step() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture = tmp.path().join("armor.zip");
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
                "assets/minecraft/textures/entity/equipment/humanoid/netherite.png",
                b"humanoid-net".to_vec(),
            );
            add(
                "assets/minecraft/textures/models/armor/netherite_layer_1.png",
                b"preexisting-layer1".to_vec(),
            );
            zip.finish().expect("finish");
        }

        let work = tmp.path().join("legacy_armor_work");
        std::fs::create_dir_all(&work).expect("mkdir");
        crate::converters::zip::extract_resource_pack(
            fixture.to_str().expect("utf8"),
            work.to_str().expect("utf8"),
        )
        .expect("extract");

        let layer1 = work.join("assets/minecraft/textures/models/armor/netherite_layer_1.png");
        let humanoid = work.join("assets/minecraft/textures/entity/equipment/humanoid/netherite.png");
        println!("[初始] layer_1 存在={} / humanoid 存在={}", layer1.exists(), humanoid.exists());

        crate::converters::reverse::armor::reverse_fix_armor_models(&work).expect("armor");
        println!("[armor 改名后] layer_1 存在={} / humanoid 存在={}", layer1.exists(), humanoid.exists());

        let ctx = crate::hurray::context::HurrayContext::new(work.to_str().expect("utf8"));
        crate::converters::reverse::netherite::reverse_generate_netherite_armor_models(&ctx)
            .expect("netherite defer");
        println!("[netherite 登记后] layer_1 存在={}（延迟删除尚未执行）", layer1.exists());

        ctx.execute_cleanup().expect("cleanup");
        println!("[清理后] layer_1 存在={}", layer1.exists());
    }
