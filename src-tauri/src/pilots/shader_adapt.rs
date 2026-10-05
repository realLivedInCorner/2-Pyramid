    use super::*;

    /// 旧 `rewrite_import_path`：`<a/b.glsl>` → `<a:b.glsl>`；`"x.glsl"` 保持引号形式。
    pub fn rewrite_import_path(rest: &str) -> Option<String> {
        let quoted = if rest.starts_with('<') && rest.ends_with('>') {
            false
        } else if rest.starts_with('"') && rest.ends_with('"') {
            true
        } else {
            return None;
        };
        let inner = rest[1..rest.len() - 1].trim().trim_start_matches('/');
        if quoted {
            return Some(format!("\"{}\"", inner));
        }
        if let Some((ns, path)) = inner.split_once(':') {
            let path = path.strip_prefix("include/").unwrap_or(path);
            return Some(format!("<{}:{}>", ns, path));
        }
        let path = inner.strip_prefix("include/").unwrap_or(inner);
        Some(format!("<{}>", path))
    }

    /// 旧 `convert_moj_import_to_include`（26.3+：`#moj_import` → `#include`）。
    ///
    /// 注意旧实现用 `lines()` 重组、**每行都补 `\n`**，因此会顺手把 CRLF 归一为 LF
    /// 并给末行补上换行——移植时照抄这一点（否则产物字节会不同）。
    pub fn convert_moj_import_to_include(src: &str) -> (String, usize) {
        let mut n = 0usize;
        let mut out = String::with_capacity(src.len());
        for line in src.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("#moj_import") {
                let rest = rest.trim();
                let converted = rewrite_import_path(rest);
                if let Some(new_rest) = converted {
                    out.push_str(&format!("#include {}\n", new_rest));
                    n += 1;
                    continue;
                }
                out.push_str(&format!("#include {}\n", rest));
                n += 1;
                continue;
            }
            out.push_str(line);
            out.push('\n');
        }
        (out, n)
    }

    /// 旧 `namespace_moj_imports`（1.21.4+：给无命名空间的 import 补 `minecraft:`）。
    pub fn namespace_moj_imports(src: &str) -> (String, usize) {
        let mut n = 0usize;
        let mut out = String::with_capacity(src.len());
        for line in src.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("#moj_import") {
                if let Some(rest) = trimmed.strip_prefix("#moj_import") {
                    let rest = rest.trim();
                    if (rest.starts_with('<') && rest.ends_with('>') && !rest.contains(':'))
                        || (rest.starts_with('"') && rest.ends_with('"') && !rest.contains(':'))
                    {
                        let inner: &str = &rest[1..rest.len() - 1];
                        let inner = inner.trim_start_matches('/');
                        if inner.starts_with("include/") || inner.contains(':') {
                            out.push_str(line);
                            out.push('\n');
                            continue;
                        }
                        out.push_str(&format!("#moj_import <minecraft:{}>\n", inner));
                        n += 1;
                        continue;
                    }
                }
            }
            out.push_str(line);
            out.push('\n');
        }
        (out, n)
    }

    /// 旧 `needs_globals_import`：源码是否用到 `ScreenSize` / `GameTime`。
    pub fn needs_globals_import(src: &str) -> bool {
        src.contains("ScreenSize") || src.contains("GameTime")
    }

    /// 旧 `has_globals_import`：**逐行**判断是否存在 import/include 了 globals 的行。
    ///
    /// 注意与 `needs_globals_import` 不同：这里**不是**子串搜索——`globals.glsl` 出现在注释里
    /// 不算数。§9.82 的逐函数对照正是靠这条把第一版的子串写法抓了出来。
    pub fn has_globals_import(src: &str) -> bool {
        src.lines().any(|l| {
            let t = l.trim();
            (t.starts_with("#moj_import") || t.starts_with("#include")) && t.contains("globals.glsl")
        })
    }

    /// 旧 `inject_globals_import`：在**第一个非空、非注释**行之前插入 import；
    /// 若通篇都是空行/注释，则追加到末尾。`use_include_directive` 决定用 `#include` 还是 `#moj_import`。
    pub fn inject_globals_import(src: &str, use_include_directive: bool) -> String {
        let import = if use_include_directive {
            "#include <minecraft:globals.glsl>\n"
        } else {
            "#moj_import <minecraft:include/globals.glsl>\n"
        };
        let mut out = String::with_capacity(src.len() + import.len() + 8);
        let mut injected = false;
        for line in src.lines() {
            let t = line.trim();
            if !injected
                && !t.is_empty()
                && !t.starts_with("//")
                && !t.starts_with("/*")
                && !t.starts_with('*')
            {
                out.push_str(import);
                injected = true;
            }
            out.push_str(line);
            out.push('\n');
        }
        if !injected {
            out.push_str(import);
        }
        out
    }

    /// 旧 `count_args_likely_three`：启发式判断 `fn_name(...)` 是否像**三参**调用
    /// （按括号深度数顶层逗号，遇到第一个 ≥2 个逗号的调用即返回真）。
    pub fn count_args_likely_three(src: &str, fn_name: &str) -> bool {
        let pat = format!("{}(", fn_name);
        let mut idx = 0usize;
        while let Some(pos) = src[idx..].find(&pat) {
            let start = idx + pos + pat.len();
            let bytes = src.as_bytes();
            let mut depth = 1usize;
            let mut commas = 0usize;
            let mut i = start;
            while i < bytes.len() && depth > 0 {
                match bytes[i] {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    b',' if depth == 1 => commas += 1,
                    _ => {}
                }
                i += 1;
            }
            if commas >= 2 {
                return true;
            }
            idx = start;
        }
        false
    }

    /// 旧 `walk_dir` 里的 fog 标记：`target >= 32` 且像三参调用且未标记时，**在文件开头**插入一行注释。
    pub fn fog_note_if_needed(src: &str, target: u32) -> (String, bool) {
        if target >= FMT_FOG_DISTANCE && src.contains("fog_distance(") {
            if count_args_likely_three(src, "fog_distance") && !src.contains("2PYR: fog_distance") {
                return (
                    format!(
                        "// 2PYR: fog_distance() 1.20.5+ 签名变更，请对照 vanilla fog.glsl\n{}",
                        src
                    ),
                    true,
                );
            }
        }
        (src.to_string(), false)
    }

    const SHADERS: &str = "assets/minecraft/shaders";

    pub fn decl() -> TaskDecl {
        TaskDecl::new("adapt_java_shaders", Tier::Surgeon)
            .reads(ScopeSet::prefix(SHADERS))
            .writes(ScopeSet::prefix(SHADERS))
            .exclusive(true)
    }

    /// 与旧实现同一套里程碑判定。
    pub fn is_modern_shader_api(target: u32) -> bool {
        target >= FMT_MODERN_JSON
    }

    /// 递归改写：**跳过 `include/`**（被 import 引用，写坏会导致整包重载失败）。
    ///
    /// 逐条照抄 `walk_dir` 的三段判定与**顺序**：导入指令 → globals 注入 → fog 标记。
    fn walk_dir(
        tx: &mut Tx<'_>,
        dir: &str,
        target: u32,
        changed: &mut usize,
    ) -> Result<(), AromError> {
        for entry in tx.list(dir)? {
            let path = entry.path.clone();
            if entry.is_dir {
                let folder = path.rsplit('/').next().unwrap_or("");
                if folder.eq_ignore_ascii_case("include") {
                    continue;
                }
                walk_dir(tx, &path, target, changed)?;
                continue;
            }
            let lower = path.to_ascii_lowercase();
            if !(lower.ends_with(".vsh") || lower.ends_with(".fsh") || lower.ends_with(".glsl")) {
                continue;
            }
            let is_core = dir.rsplit('/').next() == Some("core");
            let Some(bytes) = tx.read(&path)? else { continue };
            let Ok(raw) = String::from_utf8(bytes) else {
                continue;
            };
            let mut out = raw.clone();
            let mut file_changed = false;

            if is_core && target >= FMT_IMPORT_NS {
                if target >= FMT_INCLUDE_DIRECTIVE {
                    let (next, n) = convert_moj_import_to_include(&out);
                    if n > 0 {
                        out = next;
                        file_changed = true;
                    }
                } else {
                    let (next, n) = namespace_moj_imports(&out);
                    if n > 0 {
                        out = next;
                        file_changed = true;
                    }
                }
            }

            if is_core
                && target >= FMT_GLOBALS_INCLUDE
                && (lower.ends_with(".vsh") || lower.ends_with(".fsh"))
                && needs_globals_import(&out)
                && !has_globals_import(&out)
            {
                out = inject_globals_import(&out, target >= FMT_INCLUDE_DIRECTIVE);
                file_changed = true;
            }

            let (next, fog_changed) = fog_note_if_needed(&out, target);
            if fog_changed {
                out = next;
                file_changed = true;
            }

            if file_changed {
                tx.put(&path, out.into_bytes())?;
                *changed += 1;
            }
        }
        Ok(())
    }

    /// 旧 `prune_and_rename_core` 的**改名**步骤：`json/vsh/fsh` 成组；目标已存在则删源。
    fn rename_core_group(
        tx: &mut Tx<'_>,
        old: &str,
        new: &str,
        changed: &mut usize,
    ) -> Result<(), AromError> {
        for ext in ["json", "vsh", "fsh"] {
            let src = format!("{SHADERS}/core/{old}.{ext}");
            if !tx.exists(&src) {
                continue;
            }
            let dst = format!("{SHADERS}/core/{new}.{ext}");
            if tx.exists(&dst) {
                tx.remove(&src)?;
            } else {
                let bytes = tx.read(&src)?.unwrap_or_default();
                tx.put(&dst, bytes)?;
                tx.remove(&src)?;
            }
            *changed += 1;
        }
        Ok(())
    }

    /// 旧 `ensure_include_trailing_newline`：include 下的 `glsl/vsh/fsh` 必须以**空行**结尾
    /// （至少两个换行）；空文件跳过。
    fn ensure_include_trailing_newline(tx: &mut Tx<'_>, changed: &mut usize) -> Result<(), AromError> {
        let dir = format!("{SHADERS}/include");
        if !tx.has_prefix(&dir)? {
            return Ok(());
        }
        for entry in tx.list(&dir)? {
            if entry.is_dir {
                continue;
            }
            let ext = entry.path.rsplit('.').next().unwrap_or("");
            if !matches!(ext, "glsl" | "vsh" | "fsh") {
                continue;
            }
            let Some(bytes) = tx.read(&entry.path)? else {
                continue;
            };
            let Ok(mut raw) = String::from_utf8(bytes) else {
                continue;
            };
            if raw.is_empty() {
                continue;
            }
            if raw.ends_with('\n') {
                if !raw.ends_with("\n\n") {
                    raw.push('\n');
                    tx.put(&entry.path, raw.into_bytes())?;
                    *changed += 1;
                }
            } else {
                raw.push_str("\n\n");
                tx.put(&entry.path, raw.into_bytes())?;
                *changed += 1;
            }
        }
        Ok(())
    }

    /// **派发入口**：目标 `pack_format` 由任务**自行从包里的 `pack.mcmeta` 读出**。
    ///
    /// 为什么这样做（§9.85）：旧实现从 `ctx.get_data("target_pack_format")` 取，而原生任务只拿得到
    /// `&mut Tx`。两条可选路径：
    /// ① 改驱动，把 `MixedRunOptions::target_version` 传进执行体——但那要为**一个任务**改动
    ///    `native_for` 的返回类型与约 45 个 return 点（§9.84 两次脚本尝试都编译不过，已回退）；
    /// ②**让任务自己读包**——它是自描述的，且与生产**同源**：
    ///    生产里 `context.set_data("target_pack_format", target_version)`（`invoke_conversion.rs:158`），
    ///    而驱动的收尾步骤把同一个 `target_version` 写进 `pack.mcmeta`（`write_pack_format`），
    ///    所以从包里读出的值与传进来的那个是**同一个数**。
    ///
    /// 取不到 `pack.mcmeta` 或解析失败时按旧实现的缺省值 **88** 处理。
    pub fn run_from_pack(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        let target = tx
            .view()
            .mcmeta()
            .ok()
            .and_then(|m| m.effective_format())
            .unwrap_or(88);
        run(tx, target)
    }

    /// 总体编排（**与原实现逐项对照通过**，见 §9.83 的夹具快照对照）。
    ///
    /// 覆盖旧实现 `adapt_java_shaders_at` 的**全部步骤**：
    /// 0) **post 路径**（现代删 `shaders/post` 与 `post_effect`；旧目标删各 namespace 的 `post_effect`）；
    ///    旧着色器 API（`target < 7`）→ 删 `post_effect` + **递归删 JSON** 后返回；
    /// 1) core **改名** + **删除**（移除名单 / 白名单外）+ include 结尾空行；
    /// 2) core JSON 的**补齐**（`<63`）/ **mat 升级**（`≥7`）/ **剥 uniforms**（`≥63`）；
    /// 3) 源码遍历（导入指令 / globals 注入 / fog 标记，跳过 `include/`）。
    ///
    /// **目标 pack_format 由任务自行从包里读出**（见 [`run_from_pack`]），无需驱动传参。
    pub fn run(tx: &mut Tx<'_>, target_pack_format: u32) -> Result<Outcome, AromError> {
        if !tx.has_prefix(SHADERS)? {
            return Ok(Outcome::default());
        }
        let mut changed = 0usize;

        // 0) post 路径（现代删 shaders/post 与 post_effect；旧目标删各 namespace 的 post_effect）
        adapt_post_paths(tx, target_pack_format, &mut changed)?;

        // 旧着色器 API：删 post_effect + 递归删 JSON，然后返回
        if !is_modern_shader_api(target_pack_format) {
            if tx.has_prefix(&format!("{SHADERS}/post_effect"))? {
                tx.remove(&format!("{SHADERS}/post_effect"))?;
                changed += 1;
            }
            strip_json_in(tx, SHADERS, &mut changed)?;
            return Ok(Outcome {
                changed,
                notes: vec![format!("shader adapt (legacy target {target_pack_format})")],
                ..Outcome::default()
            });
        }

        // 1) core：改名 + 删除（两者都只动 core/）
        prune_and_rename_core(tx, target_pack_format, &mut changed)?;
        ensure_include_trailing_newline(tx, &mut changed)?;

        // 2) core JSON：补齐（<63）/ mat 升级（≥7）/ 剥 uniforms（≥63）
        if tx.has_prefix(&format!("{SHADERS}/core"))? {
            if target_pack_format < FMT_GLOBALS_INCLUDE {
                ensure_core_json(tx, &mut changed)?;
            }
            if target_pack_format >= FMT_MODERN_JSON {
                rewrite_json_matrix_types(tx, &mut changed)?;
            }
            if target_pack_format >= FMT_GLOBALS_INCLUDE {
                strip_json_uniforms_for_ubo(tx, &mut changed)?;
            }
        }

        // 3) 源码遍历（跳过 include/）
        walk_dir(tx, SHADERS, target_pack_format, &mut changed)?;

        Ok(Outcome {
            changed,
            notes: vec![format!("shader adapt target={target_pack_format}")],
            ..Outcome::default()
        })
    }

    /// 旧 `adapt_post_paths`。
    fn adapt_post_paths(tx: &mut Tx<'_>, target: u32, changed: &mut usize) -> Result<(), AromError> {
        if target >= FMT_GLOBALS_INCLUDE {
            // 现代：删掉旧位置（这些 JSON 在新版本不会被正确加载，且可能干扰）
            for dir in [format!("{SHADERS}/post"), format!("{SHADERS}/post_effect")] {
                if tx.has_prefix(&dir)? {
                    tx.remove(&dir)?;
                    *changed += 1;
                }
            }
            // 扫描各 namespace：若只有旧式 shaders/post 源而无 post_effect，**不**自动伪造 JSON
        } else {
            // 旧目标：删各 namespace 下的现代路径 post_effect，避免旧版本无法解析
            for entry in tx.list("assets")? {
                if !entry.is_dir {
                    continue;
                }
                let pe = format!("{}/post_effect", entry.path);
                if tx.has_prefix(&pe)? {
                    tx.remove(&pe)?;
                    *changed += 1;
                }
            }
            let pe = format!("{SHADERS}/post_effect");
            if tx.has_prefix(&pe)? {
                tx.remove(&pe)?;
                *changed += 1;
            }
        }
        Ok(())
    }

    /// 旧 `prune_and_rename_core`：改名组（仅 modern）+ 移除名单 + 白名单外清理。
    fn prune_and_rename_core(
        tx: &mut Tx<'_>,
        target: u32,
        changed: &mut usize,
    ) -> Result<(), AromError> {
        let core = format!("{SHADERS}/core");
        if !tx.has_prefix(&core)? {
            return Ok(());
        }
        let modern = target >= FMT_GLOBALS_INCLUDE;
        let allow: Vec<&str> = if modern {
            modern_core_allowlist(target)
        } else {
            legacy_core_allowlist()
        };
        let removed = core_removed_stems(target);

        // 1) 旧名 → 新名（json/vsh/fsh 成组；仅 modern 目标）
        if modern {
            for (old, new) in core_rename_table(target) {
                if old == new {
                    continue;
                }
                rename_core_group(tx, old, new, changed)?;
            }
        }

        // 2) 明确移除名单（旧目标不删 legacy 白名单里仍存在的名字）
        for stem in &removed {
            if !modern && allow.contains(stem) {
                continue;
            }
            for ext in ["json", "vsh", "fsh"] {
                let p = format!("{SHADERS}/core/{stem}.{ext}");
                if tx.exists(&p) {
                    tx.remove(&p)?;
                    *changed += 1;
                }
            }
        }

        // 3) 白名单外的核心程序文件删除（.glsl 与共享 vsh 保留）
        for entry in tx.list(&core)? {
            if entry.is_dir {
                continue;
            }
            let name = entry
                .path
                .rsplit('/')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            if name.ends_with(".glsl") {
                continue;
            }
            let stem = if let Some(s) = name.strip_suffix(".vsh") {
                s.to_string()
            } else if let Some(s) = name.strip_suffix(".fsh") {
                s.to_string()
            } else if let Some(s) = name.strip_suffix(".json") {
                s.to_string()
            } else {
                continue;
            };
            if allow.contains(&stem.as_str()) || SHARED_VERTEX_STEMS.contains(&stem.as_str()) {
                continue;
            }
            tx.remove(&entry.path)?;
            *changed += 1;
        }
        Ok(())
    }

    /// 旧 `ensure_core_json`：成对 `vsh+fsh` 且缺 JSON 时补最小定义（共享顶点程序与
    /// `position_color` 除外）。
    fn ensure_core_json(tx: &mut Tx<'_>, changed: &mut usize) -> Result<(), AromError> {
        let core = format!("{SHADERS}/core");
        for entry in tx.list(&core)? {
            if entry.is_dir {
                continue;
            }
            let name = entry.path.rsplit('/').next().unwrap_or("").to_string();
            if !(name.ends_with(".vsh") || name.ends_with(".fsh")) {
                continue;
            }
            let stem = name
                .trim_end_matches(".vsh")
                .trim_end_matches(".fsh")
                .to_string();
            if SHARED_VERTEX_STEMS.contains(&stem.as_str()) || stem == "position_color" {
                continue;
            }
            if !tx.exists(&format!("{core}/{stem}.vsh")) || !tx.exists(&format!("{core}/{stem}.fsh")) {
                continue;
            }
            let json = format!("{core}/{stem}.json");
            if tx.exists(&json) {
                continue;
            }
            tx.put(&json, minimal_core_json(&stem).into_bytes())?;
            *changed += 1;
        }
        Ok(())
    }

    /// 旧 `rewrite_json_matrix_types`：core 下每个 `.json` 做 mat2/mat3 → mat4。
    fn rewrite_json_matrix_types(tx: &mut Tx<'_>, changed: &mut usize) -> Result<(), AromError> {
        let core = format!("{SHADERS}/core");
        for entry in tx.list(&core)? {
            if entry.is_dir || !entry.path.ends_with(".json") {
                continue;
            }
            let Some(bytes) = tx.read(&entry.path)? else {
                continue;
            };
            let Ok(raw) = String::from_utf8(bytes) else {
                continue;
            };
            let (out, did) = rewrite_json_matrix_types_text(&raw);
            if did && out != raw {
                tx.put(&entry.path, out.into_bytes())?;
                *changed += 1;
            }
        }
        Ok(())
    }

    /// 旧 `strip_json_uniforms_for_ubo`：含 `"uniforms"` 的 core JSON 去掉该键。
    fn strip_json_uniforms_for_ubo(tx: &mut Tx<'_>, changed: &mut usize) -> Result<(), AromError> {
        let core = format!("{SHADERS}/core");
        for entry in tx.list(&core)? {
            if entry.is_dir || !entry.path.ends_with(".json") {
                continue;
            }
            let Some(bytes) = tx.read(&entry.path)? else {
                continue;
            };
            let Ok(raw) = String::from_utf8(bytes) else {
                continue;
            };
            if !json_has_uniforms(&raw) {
                continue;
            }
            let stripped = remove_json_key(&raw, "uniforms");
            if stripped != raw {
                tx.put(&entry.path, stripped.into_bytes())?;
                *changed += 1;
            }
        }
        Ok(())
    }

    /// 旧 `strip_json_in`：**递归**删掉目录下所有 `.json`。
    fn strip_json_in(tx: &mut Tx<'_>, dir: &str, changed: &mut usize) -> Result<(), AromError> {
        if !tx.has_prefix(dir)? {
            return Ok(());
        }
        for entry in tx.list(dir)? {
            if entry.is_dir {
                strip_json_in(tx, &entry.path, changed)?;
            } else if entry.path.ends_with(".json") {
                tx.remove(&entry.path)?;
                *changed += 1;
            }
        }
        Ok(())
    }

    // ───────────────────────── 表格（移植自 `converters/shaders/java.rs`）─────────────────────────
    //
    // 这些表是**纯数据**，但逐字错误会静默改变删/改名行为，所以移植时一并抄全，
    // 并用「表对照」测试（§9.80）在多个 target 上与旧实现比对（排序后逐项相等）。

    /// pack_format 里程碑（与旧实现同名同值）。
    pub const FMT_MODERN_JSON: u32 = 7;
    pub const FMT_FOG_DISTANCE: u32 = 32;
    pub const FMT_IMPORT_NS: u32 = 46;
    pub const FMT_GLOBALS_INCLUDE: u32 = 63;
    pub const FMT_ENTITY_BLOCK: u32 = 84;
    pub const FMT_INCLUDE_DIRECTIVE: u32 = 97;

    /// 共享顶点程序（不得按 stem 补 JSON；可能无同名 json）。
    pub const SHARED_VERTEX_STEMS: &[&str] = &["screenquad", "animate_sprite"];

    /// 旧 `modern_core_allowlist`：目标版本仍存在的核心程序 stem。
    pub fn modern_core_allowlist(target: u32) -> Vec<&'static str> {
        let mut set: Vec<&'static str> = vec![
            "animate_sprite_blit",
            "animate_sprite_interpolate",
            "blit_depth",
            "blit_screen",
            "block",
            "debug_point",
            "entity",
            "glint",
            "gui",
            "item",
            "lightmap",
            "oit_composite",
            "panorama",
            "particle",
            "position",
            "position_color",
            "position_tex",
            "position_tex_color",
            "rendertype_beacon_beam",
            "rendertype_crumbling",
            "rendertype_end_portal",
            "rendertype_entity_shadow",
            "rendertype_leash",
            "rendertype_lightning",
            "rendertype_lines",
            "rendertype_outline",
            "rendertype_text",
            "rendertype_text_intensity",
            "rendertype_text_see_through",
            "rendertype_text_intensity_see_through",
            "rendertype_water_mask",
            "sky",
            "stars",
            "terrain",
            "text",
        ];
        set.extend_from_slice(SHARED_VERTEX_STEMS);
        if target >= FMT_ENTITY_BLOCK {
            set.push("block");
        }
        if target >= FMT_INCLUDE_DIRECTIVE {
            set.push("clouds");
            set.push("world_border");
        } else {
            set.push("rendertype_clouds");
            set.push("rendertype_world_border");
            set.push("rendertype_text_background");
            set.push("rendertype_text_background_see_through");
        }
        set.sort_unstable();
        set
    }

    /// 旧 `legacy_core_allowlist`：旧版目标仍存在的核心名。
    pub fn legacy_core_allowlist() -> Vec<&'static str> {
        let mut set: Vec<&'static str> = vec![
            "blit_screen",
            "position",
            "position_color",
            "position_tex",
            "position_tex_color",
            "position_color_tex",
            "position_texture",
            "particle",
            "rendertype_armor_entity_glint",
            "rendertype_armor_entity_glint_direct",
            "rendertype_beacon_beam",
            "rendertype_block",
            "rendertype_breeze_spikes",
            "rendertype_breeze_wind",
            "rendertype_clouds",
            "rendertype_crumbling",
            "rendertype_cutout",
            "rendertype_cutout_mipped",
            "rendertype_cutout_mipped_aliased",
            "rendertype_end_portal",
            "rendertype_end_gateway",
            "rendertype_entity",
            "rendertype_entity_alpha",
            "rendertype_entity_cutout",
            "rendertype_entity_cutout_no_cull",
            "rendertype_entity_cutout_no_cull_z_offset",
            "rendertype_entity_decal",
            "rendertype_entity_glint",
            "rendertype_entity_glint_direct",
            "rendertype_entity_no_outline",
            "rendertype_entity_shadow",
            "rendertype_entity_smooth_cutout",
            "rendertype_entity_solid",
            "rendertype_entity_translucent",
            "rendertype_entity_translucent_cull",
            "rendertype_entity_translucent_emissive",
            "rendertype_entity_translucent_no_outline",
            "rendertype_energy_swirl",
            "rendertype_glint",
            "rendertype_glint_direct",
            "rendertype_glint_translucent",
            "rendertype_gui",
            "rendertype_gui_ghost_recipe_overlay",
            "rendertype_gui_overlay",
            "rendertype_gui_text_highlight",
            "rendertype_item",
            "rendertype_item_entity_translucent_cull",
            "rendertype_leash",
            "rendertype_lightning",
            "rendertype_lines",
            "rendertype_outline",
            "rendertype_solid",
            "rendertype_text",
            "rendertype_text_background",
            "rendertype_text_background_see_through",
            "rendertype_text_intensity",
            "rendertype_text_intensity_see_through",
            "rendertype_text_see_through",
            "rendertype_translucent",
            "rendertype_translucent_moving_block",
            "rendertype_translucent_no_crumbling",
            "rendertype_tripwire",
            "rendertype_water_mask",
            "rendertype_world_border",
            "rendertype_phantom",
            "rendertype_dragon_explosion_alpha",
            "rendertype_chain",
            "screenquad",
            "animate_sprite",
            "lightmap",
            "gui",
            "sky",
            "stars",
            "terrain",
            "entity",
            "text",
            "item",
            "glint",
            "clouds",
            "world_border",
            "block",
            "panorama",
            "oit_composite",
            "animate_sprite_blit",
            "animate_sprite_interpolate",
            "blit_depth",
            "debug_point",
        ];
        set.sort_unstable();
        set
    }

    /// 旧 `core_rename_table`：旧名 → 新名（仅 ≥63 之后应用合并改名）。
    pub fn core_rename_table(target: u32) -> Vec<(&'static str, &'static str)> {
        if target < FMT_GLOBALS_INCLUDE {
            return Vec::new();
        }
        let mut map: Vec<(&'static str, &'static str)> = vec![
            ("rendertype_solid", "terrain"),
            ("rendertype_cutout", "terrain"),
            ("rendertype_cutout_mipped", "terrain"),
            ("rendertype_cutout_mipped_aliased", "terrain"),
            ("rendertype_translucent", "terrain"),
            ("rendertype_translucent_no_crumbling", "terrain"),
            ("rendertype_tripwire", "terrain"),
            ("rendertype_entity_solid", "entity"),
            ("rendertype_entity_cutout", "entity"),
            ("rendertype_entity_cutout_no_cull", "entity"),
            ("rendertype_entity_cutout_no_cull_z_offset", "entity"),
            ("rendertype_entity_translucent", "entity"),
            ("rendertype_entity_translucent_cull", "entity"),
            ("rendertype_entity_translucent_emissive", "entity"),
            ("rendertype_entity_translucent_no_outline", "entity"),
            ("rendertype_entity_no_outline", "entity"),
            ("rendertype_entity_smooth_cutout", "entity"),
            ("rendertype_energy_swirl", "entity"),
            ("rendertype_breeze_spikes", "entity"),
            ("rendertype_breeze_wind", "entity"),
            ("rendertype_chain", "entity"),
            ("rendertype_armor_entity_glint", "glint"),
            ("rendertype_armor_entity_glint_direct", "glint"),
            ("rendertype_entity_glint", "glint"),
            ("rendertype_entity_glint_direct", "glint"),
            ("rendertype_glint", "glint"),
            ("rendertype_glint_direct", "glint"),
            ("rendertype_glint_translucent", "glint"),
            ("position_texture", "position_tex"),
            ("position_color_tex", "position_tex_color"),
            ("position_color_tex_lightmap", "position_tex_color"),
            ("rendertype_gui", "gui"),
            ("rendertype_gui_overlay", "position_tex_color"),
            ("rendertype_gui_text_highlight", "gui"),
            ("rendertype_gui_ghost_recipe_overlay", "gui"),
            ("rendertype_text", "text"),
            ("rendertype_text_intensity", "text"),
            ("rendertype_text_see_through", "text"),
            ("rendertype_text_intensity_see_through", "text"),
            ("text_see_through", "text"),
        ];
        if target >= FMT_ENTITY_BLOCK {
            map.push(("rendertype_entity_alpha", "entity"));
            map.push(("rendertype_entity_decal", "entity"));
            map.push(("rendertype_item_entity_translucent_cull", "entity"));
            map.push(("rendertype_translucent_moving_block", "block"));
        }
        if target >= FMT_INCLUDE_DIRECTIVE {
            map.push(("rendertype_clouds", "clouds"));
            map.push(("rendertype_world_border", "world_border"));
        }
        map
    }

    /// 旧 `core_removed_stems`：目标版本下应直接删除的 stem。
    pub fn core_removed_stems(target: u32) -> Vec<&'static str> {
        let mut gone = vec![
            "position_color_normal",
            "position_tex_lightmap_color",
            "position_color_lightmap",
            "rendertype_end_gateway",
            "rendertype_dragon_explosion_alpha",
            "rendertype_phantom",
            "rendertype_water_mask_offset",
            "position_tex_lightmap",
            "position_color_tex_lightmap_color",
        ];
        if target >= FMT_INCLUDE_DIRECTIVE {
            gone.push("text_background");
            gone.push("text_background_see_through");
            gone.push("rendertype_text_background");
            gone.push("rendertype_text_background_see_through");
        }
        gone
    }

    /// 旧 `rewrite_json_matrix_types` 的核心文本变换：`"type": "mat2"|"mat3"` → `"type": "mat4"`。
    ///
    /// 逐条照抄：只匹配**带空格的**字面量形式（§9.78 实测到过——写成无空格的 `"mat3"` 不会命中），
    /// 返回 `(新文本, 是否改变)`。
    pub fn rewrite_json_matrix_types_text(raw: &str) -> (String, bool) {
        let mut out = raw.to_string();
        let mut changed = false;
        for from in ["\"type\": \"mat2\"", "\"type\": \"mat3\""] {
            if out.contains(from) {
                out = out.replace(from, "\"type\": \"mat4\"");
                changed = true;
            }
        }
        (out, changed)
    }

    /// 旧 `ensure_core_json` 的 JSON 体（成对 vsh+fsh 且缺 JSON 时补的最小定义）。
    pub fn minimal_core_json(stem: &str) -> String {
        format!(
            "{{\n  \"vertex\": \"{}\",\n  \"fragment\": \"{}\"\n}}\n",
            stem, stem
        )
    }

    /// 旧 `strip_json_uniforms_for_ubo` 的判定：该 JSON 是否含 `"uniforms"`。
    pub fn json_has_uniforms(raw: &str) -> bool {
        raw.contains("\"uniforms\"")
    }

    /// 旧 `remove_json_key`：从 JSON 对象文本中删除顶层 `"key": …`（粗粒度，够用即可）。
    pub fn remove_json_key(src: &str, key: &str) -> String {
        let pattern = format!("\"{}\"", key);
        let Some(start) = src.find(&pattern) else {
            return src.to_string();
        };
        let after_key = &src[start + pattern.len()..];
        let Some(colon_rel) = after_key.find(':') else {
            return src.to_string();
        };
        let value_start = start + pattern.len() + colon_rel + 1;
        let bytes = src.as_bytes();
        let mut i = value_start;
        while i < bytes.len() && (bytes[i] as char).is_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            return src.to_string();
        }
        let value_end = match bytes[i] {
            b'{' | b'[' => {
                let mut depth = 0usize;
                let mut j = i;
                while j < bytes.len() {
                    match bytes[j] {
                        b'{' | b'[' => depth += 1,
                        b'}' | b']' => {
                            depth -= 1;
                            if depth == 0 {
                                j += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
                j
            }
            b'"' => {
                let mut j = i + 1;
                while j < bytes.len() {
                    if bytes[j] == b'"' && bytes[j - 1] != b'\\' {
                        j += 1;
                        break;
                    }
                    j += 1;
                }
                j
            }
            _ => {
                let mut j = i;
                while j < bytes.len() && bytes[j] != b',' && bytes[j] != b'}' && bytes[j] != b'\n' {
                    j += 1;
                }
                j
            }
        };
        let mut end = value_end;
        while end < bytes.len() && (bytes[end] as char).is_whitespace() {
            end += 1;
        }
        if end < bytes.len() && bytes[end] == b',' {
            end += 1;
        } else {
            let mut k = start;
            while k > 0 && (bytes[k - 1] as char).is_whitespace() {
                k -= 1;
            }
            if k > 0 && bytes[k - 1] == b',' {
                return format!("{}{}", &src[..k - 1], &src[end..]);
            }
        }
        format!("{}{}", &src[..start], &src[end..])
    }
