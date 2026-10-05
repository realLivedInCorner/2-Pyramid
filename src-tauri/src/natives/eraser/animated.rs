    use super::*;

    pub const DIRS: [&str; 2] = [
        "assets/minecraft/textures/item",
        "assets/minecraft/textures/items",
    ];

    pub fn decl() -> TaskDecl {
        let scope = ScopeSet::prefix(DIRS[0]).union(&ScopeSet::prefix(DIRS[1]));
        // 阶段与活注册表一致：`invoke_conversion.rs` 把 `convert_animated_textures`
        // 登记为 `TaskType::Exclusive` / `Tier::Eraser`。
        TaskDecl::new("convert_animated_textures", Tier::Eraser)
            .reads(scope.clone())
            .writes(scope)
            .exclusive(true)
    }

    /// 由贴图尺寸推导帧数；语义与旧实现逐条一致（正方形 1 帧、条带取整除、否则跳过）。
    pub fn frames_for(w: u32, h: u32) -> Option<u32> {
        if w == h {
            return Some(1);
        }
        if w > h && w % h == 0 {
            return Some(w / h);
        }
        if h > w && h % w == 0 {
            return Some(h / w);
        }
        None
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        // 先读后写：`list` / `exists` 返回 owned 数据，收集完再改
        let mut targets: Vec<(String, String)> = Vec::new();
        for dir in DIRS {
            for res in tx.list(dir)? {
                if res.is_dir {
                    continue;
                }
                let lower = res.path.to_ascii_lowercase();
                if !lower.ends_with(".png.mcmeta") {
                    continue;
                }
                let png = res.path[..res.path.len() - ".mcmeta".len()].to_string();
                if tx.exists(&png) {
                    targets.push((res.path.clone(), png));
                }
            }
        }

        let mut outcome = Outcome::default();
        for (mcmeta, png) in targets {
            match upgrade_one(tx, &mcmeta, &png) {
                Ok(true) => outcome.changed += 1,
                Ok(false) => outcome.skipped += 1,
                Err(e) => {
                    // 与旧实现一致：单条失败只记日志，不中断整个任务
                    outcome.skipped += 1;
                    outcome.notes.push(format!("skip {mcmeta}: {e}"));
                }
            }
        }
        Ok(outcome)
    }

    fn upgrade_one(tx: &mut Tx<'_>, mcmeta: &str, png: &str) -> Result<bool, AromError> {
        let text = tx.text(mcmeta)?;
        let trimmed = text.trim_start_matches('\u{feff}');
        let mut data: serde_json::Value = serde_json::from_str(trimmed)
            .map_err(|e| AromError::View(format!("{mcmeta}: invalid JSON: {e}")))?;

        let Some(anim) = data.get_mut("animation").and_then(|a| a.as_object_mut()) else {
            return Ok(false);
        };
        if anim.contains_key("frametime") {
            return Ok(false);
        }

        let (w, h) = {
            let img = tx.image(png)?;
            (img.width(), img.height())
        };
        let frames = frames_for(w, h).ok_or_else(|| {
            AromError::View(format!("{png}: 无法从尺寸推导帧数 ({w}x{h})"))
        })?;

        anim.insert("frametime".into(), serde_json::json!(frames));
        anim.insert("interpolate".into(), serde_json::json!(true));
        let pretty = serde_json::to_string_pretty(&data).map_err(AromError::internal)?;
        tx.put(mcmeta, pretty.into_bytes())?;
        Ok(true)
    }
