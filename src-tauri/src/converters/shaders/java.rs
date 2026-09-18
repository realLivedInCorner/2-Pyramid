//! Java ↔ Java 着色器适配（j2j），重点 **1.20 → 26.x**。
//!
//! 版本差异（Minecraft Wiki + 社区实战）：
//! - **1.17+（format ≥ 7）**：core JSON；JSON 里 **只能传 mat4**，不能 mat2/mat3。
//! - **1.20.5（format ≥ 32）**：`fog.glsl` 中 `fog_distance()` 参数变动。
//! - **1.21.4（format ≥ 46）**：`#moj_import` 需要 **命名空间+路径**（`<minecraft:...>`）。
//! - **1.21.6（format ≥ 63）**：Uniform block 化；大量 `rendertype_*` 合并为
//!   `terrain` / `entity` / `text` 等；`FogShape/FogStart/FogEnd` 移除；
//!   `ScreenSize`/`GameTime` 走 include 的 `globals.glsl`。
//! - **26.1（format ≥ 84）**：`rendertype_entity_alpha/decal` 删除 → `entity`；
//!   `rendertype_translucent_moving_block` → `block`。
//! - **26.3（format ≥ 97）**：`#moj_import` → `#include`（ShaderC）；
//!   `rendertype_clouds` → `clouds`；`rendertype_world_border` → `world_border`；
//!   `text_background*` 删除。加载出错会 **禁用整包**。
//!
//! 本模块：改写仍有效的着色器源码，并按 Wiki 清理 **目标版本已移除** 的核心程序，
//! 避免转换后材质/资源包重载失败。

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::hurray::context::HurrayContext;
use crate::{log_info, log_warn};

/// pack_format 里程碑
const FMT_MODERN_JSON: u32 = 7; // 1.17
const FMT_FOG_DISTANCE: u32 = 32; // 1.20.5
const FMT_IMPORT_NS: u32 = 46; // 1.21.4
const FMT_GLOBALS_INCLUDE: u32 = 63; // 1.21.6 — uniform block / 核心着色器合并
const FMT_ENTITY_BLOCK: u32 = 84; // 26.1 — entity/block 进一步收敛
const FMT_INCLUDE_DIRECTIVE: u32 = 97; // 26.3 — #moj_import → #include

/// 共享顶点程序（不得按 stem 补 JSON；可能无同名 json）
const SHARED_VERTEX_STEMS: &[&str] = &["screenquad", "animate_sprite"];

fn is_modern_shader_api(pack_format: u32) -> bool {
    pack_format >= FMT_MODERN_JSON
}

/// 挂到调度器的入口：按目标 pack_format 适配 shaders/。
pub fn adapt_java_shaders(ctx: &HurrayContext) -> Result<(), String> {
    let root = Path::new(ctx.temp_dir());
    let target = ctx
        .get_data("target_pack_format")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(88);
    adapt_java_shaders_at(root, target)
}

/// 对工作目录中的 `assets/minecraft/shaders` 做目标版本适配。
pub fn adapt_java_shaders_at(pack_root: &Path, target_pack_format: u32) -> Result<(), String> {
    let shaders = pack_root.join("assets").join("minecraft").join("shaders");
    if !shaders.is_dir() {
        return Ok(());
    }

    // 路径清理：post 定义位置随版本迁移；错误路径/过期定义会导致整包重载失败
    adapt_post_paths(pack_root, &shaders, target_pack_format);

    if !is_modern_shader_api(target_pack_format) {
        remove_dir_quiet(&shaders.join("post_effect"));
        let mut n = 0usize;
        strip_json_in(&shaders, &mut n);
        if n > 0 {
            log_info!(
                "OKAY java-shaders [strip JSON × {} for legacy target {}]",
                n,
                target_pack_format
            );
        }
        log_warn!(
            "目标 pack_format {} 使用旧着色器 API；uniform block 无法自动改写",
            target_pack_format
        );
        return Ok(());
    }

    let core = shaders.join("core");
    let mut stats = ShaderStats::default();

    // 0) 按 Wiki 处理目标版本已移除/已改名的核心着色器（避免重载失败）
    if core.is_dir() {
        prune_and_rename_core(&core, target_pack_format, &mut stats);
        // include 末尾必须空行，否则着色器不加载（Wiki）
        ensure_include_trailing_newline(&shaders.join("include"), &mut stats);
    }

    // 1) JSON：仅在成对 .vsh+.fsh 且缺失 JSON 时补最小定义；
    //    1.21.6+（JSON 体系收缩）不再生成 stub，避免无效定义导致「着色器重载失败」。
    if core.is_dir() {
        if target_pack_format < FMT_GLOBALS_INCLUDE {
            ensure_core_json(&core, &mut stats);
        }
        if target_pack_format >= FMT_MODERN_JSON {
            rewrite_json_matrix_types(&core, &mut stats);
        }
        if target_pack_format >= FMT_GLOBALS_INCLUDE {
            strip_json_uniforms_for_ubo(&core, &mut stats);
        }
    }

    // 2) 源码遍历（跳过 include/，避免写坏被 #moj_import/#include 的公共文件）
    walk_and_rewrite(&shaders, target_pack_format, &mut stats);

    if stats.ensured_json > 0 {
        log_info!("OKAY java-shaders [ensured core JSON × {}]", stats.ensured_json);
    }
    if stats.mat_upgraded > 0 {
        log_info!(
            "OKAY java-shaders [JSON mat2/mat3→mat4 × {}]",
            stats.mat_upgraded
        );
    }
    if stats.uniforms_stripped > 0 {
        log_info!(
            "OKAY java-shaders [strip obsolete JSON uniforms × {}]",
            stats.uniforms_stripped
        );
    }
    if stats.core_renamed > 0 {
        log_info!(
            "OKAY java-shaders [core rename → target names × {}]",
            stats.core_renamed
        );
    }
    if stats.core_removed > 0 {
        log_info!(
            "OKAY java-shaders [removed obsolete core programs × {}]",
            stats.core_removed
        );
    }
    if stats.imports_namespaced > 0 {
        log_info!(
            "OKAY java-shaders [moj_import → namespace × {}]",
            stats.imports_namespaced
        );
    }
    if stats.imports_to_include > 0 {
        log_info!(
            "OKAY java-shaders [moj_import → #include × {}]",
            stats.imports_to_include
        );
    }
    if stats.globals_injected > 0 {
        log_info!(
            "OKAY java-shaders [injected globals.glsl import × {}]",
            stats.globals_injected
        );
    }
    if stats.fog_rewritten > 0 {
        log_info!(
            "OKAY java-shaders [fog_distance notes × {}]",
            stats.fog_rewritten
        );
    }
    if stats.include_newline_fixed > 0 {
        log_info!(
            "OKAY java-shaders [include trailing newline × {}]",
            stats.include_newline_fixed
        );
    }
    if target_pack_format >= FMT_FOG_DISTANCE {
        log_warn!(
            "目标 ≥1.20.5：fog_distance() 参数已变；若雾效异常请对照 vanilla fog.glsl 手调"
        );
    }
    if target_pack_format >= FMT_GLOBALS_INCLUDE {
        log_warn!(
            "目标 ≥1.21.6：请确认 ScreenSize/GameTime 走 include/globals.glsl，而非 core JSON"
        );
    }
    Ok(())
}

#[derive(Default)]
struct ShaderStats {
    ensured_json: usize,
    mat_upgraded: usize,
    uniforms_stripped: usize,
    core_renamed: usize,
    core_removed: usize,
    imports_namespaced: usize,
    imports_to_include: usize,
    globals_injected: usize,
    fog_rewritten: usize,
    include_newline_fixed: usize,
}

/// 目标版本仍存在的核心程序 stem（Wiki「List of core shaders」）。
/// Wiki 现行列表仍同时出现 `text` 与部分 `rendertype_text*`，故宽表保留；
/// 仅在明确移除的版本节点再剔除（如 26.3 删除 `text_background*`）。
fn modern_core_allowlist(target: u32) -> HashSet<&'static str> {
    let mut set: HashSet<&'static str> = HashSet::from([
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
        // Wiki 仍列出的 text 族（含 intensity / see_through）
        "rendertype_text",
        "rendertype_text_intensity",
        "rendertype_text_see_through",
        "rendertype_text_intensity_see_through",
        "rendertype_water_mask",
        "sky",
        "stars",
        "terrain",
        "text",
    ]);
    for s in SHARED_VERTEX_STEMS {
        set.insert(s);
    }
    if target >= FMT_ENTITY_BLOCK {
        set.insert("block");
    }
    if target >= FMT_INCLUDE_DIRECTIVE {
        set.insert("clouds");
        set.insert("world_border");
        // 26.3-snap8 删除 text_background* → 不进白名单
    } else {
        set.insert("rendertype_clouds");
        set.insert("rendertype_world_border");
        set.insert("rendertype_text_background");
        set.insert("rendertype_text_background_see_through");
    }
    set
}

/// 旧版目标仍存在的核心名（1.17–1.21.5 常见 `rendertype_*`；不含 1.21.6+ 合并名）。
fn legacy_core_allowlist() -> HashSet<&'static str> {
    HashSet::from([
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
        // 资源包里已合并命名的自定义/半现代程序：旧目标下保留，避免误删
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
    ])
}

/// 目标版本下：旧名 → 新名（仅在 **≥1.21.6 / format 63** 后才应用合并改名）。
fn core_rename_table(target: u32) -> Vec<(&'static str, &'static str)> {
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
        // Wiki：text 族合并到 `text`（IS_* defines）；旧专用名一并归并
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

/// 目标版本下应直接删除、无可靠改名目标的 stem。
fn core_removed_stems(target: u32) -> Vec<&'static str> {
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

/// 仅对 core 目录：改名仍有效的旧程序 + 删除目标版本已移除的程序文件。
fn prune_and_rename_core(core: &Path, target: u32, stats: &mut ShaderStats) {
    let modern = target >= FMT_GLOBALS_INCLUDE;
    let allow = if modern {
        modern_core_allowlist(target)
    } else {
        legacy_core_allowlist()
    };
    let renames = core_rename_table(target);
    let removed = core_removed_stems(target);

    // 1) 旧名 → 新名（json/vsh/fsh 成组；仅 modern 目标）
    for (old, new) in &renames {
        if *old == *new {
            continue;
        }
        for ext in ["json", "vsh", "fsh"] {
            let src = core.join(format!("{}.{}", old, ext));
            if !src.is_file() {
                continue;
            }
            let dst = core.join(format!("{}.{}", new, ext));
            if dst.is_file() {
                if fs::remove_file(&src).is_ok() {
                    stats.core_removed += 1;
                }
            } else if fs::rename(&src, &dst).is_ok() {
                stats.core_renamed += 1;
            }
        }
    }

    // 2) 明确移除名单（目标版本已删除的程序）
    for stem in &removed {
        // 旧目标不删 legacy 白名单里仍存在的名字
        if !modern && allow.contains(stem) {
            continue;
        }
        for ext in ["json", "vsh", "fsh"] {
            let p = core.join(format!("{}.{}", stem, ext));
            if p.is_file() && fs::remove_file(&p).is_ok() {
                stats.core_removed += 1;
            }
        }
    }

    // 3) 白名单外的核心程序文件删除（已移除/未知；.glsl 与共享 vsh 保留）
    let Ok(entries) = fs::read_dir(core) else { return };
    let files: Vec<std::path::PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    for path in &files {
        let name = match path.file_name().and_then(|s| s.to_str()) {
            Some(n) => n.to_ascii_lowercase(),
            None => continue,
        };
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
        if allow.contains(stem.as_str()) {
            continue;
        }
        if SHARED_VERTEX_STEMS.contains(&stem.as_str()) {
            continue;
        }
        // 旧目标：不要误删仍在使用的 legacy 名（上面 allow 已覆盖）；modern：删未知/已移除
        if fs::remove_file(path).is_ok() {
            stats.core_removed += 1;
        }
    }
}

/// Wiki：include 文件末尾必须空行，否则着色器加载失败。
fn ensure_include_trailing_newline(include_dir: &Path, stats: &mut ShaderStats) {
    if !include_dir.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(include_dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext, "glsl" | "vsh" | "fsh") {
            continue;
        }
        let Ok(mut raw) = fs::read_to_string(&path) else { continue };
        if raw.is_empty() {
            continue;
        }
        if raw.ends_with('\n') {
            // 保证至少有一个空行（两个换行结尾）更符合 Wiki「ends with an empty line」
            if !raw.ends_with("\n\n") {
                raw.push('\n');
                if fs::write(&path, &raw).is_ok() {
                    stats.include_newline_fixed += 1;
                }
            }
        } else {
            raw.push_str("\n\n");
            if fs::write(&path, &raw).is_ok() {
                stats.include_newline_fixed += 1;
            }
        }
    }
}

/// post 路径：现代版本在 `assets/<ns>/post_effect/`；旧版在 `shaders/post` / `shaders/post_effect`。
/// 错误路径或过期 JSON 会导致资源包重载失败——目标侧只保留合法位置，并去掉另一侧。
fn adapt_post_paths(pack_root: &Path, shaders: &Path, target: u32) {
    let ns_root = pack_root.join("assets");
    if target >= FMT_GLOBALS_INCLUDE {
        // 删除旧位置（这些 JSON 在新版本不会被正确加载，且可能干扰）
        let old_post = shaders.join("post");
        let old_pe = shaders.join("post_effect");
        if old_post.is_dir() {
            remove_dir_quiet(&old_post);
        }
        if old_pe.is_dir() {
            remove_dir_quiet(&old_pe);
        }
        // 扫描各 namespace：若只有旧式 shaders/post 源而无 post_effect，不自动伪造 JSON（格式已变）
    } else {
        // 旧目标：删除现代路径 post_effect，避免旧版本无法解析
        if let Ok(ns_entries) = fs::read_dir(&ns_root) {
            for ns in ns_entries.flatten() {
                let pe = ns.path().join("post_effect");
                if pe.is_dir() {
                    remove_dir_quiet(&pe);
                }
            }
        }
        // 旧版可保留 shaders/ 下的 post 文件夹；post_effect 近代短名也删
        remove_dir_quiet(&shaders.join("post_effect"));
    }
}

fn ensure_core_json(core: &Path, stats: &mut ShaderStats) {
    let Ok(entries) = fs::read_dir(core) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !(name.ends_with(".vsh") || name.ends_with(".fsh")) {
            continue;
        }
        let stem = name
            .trim_end_matches(".vsh")
            .trim_end_matches(".fsh")
            .to_string();
        // 共享顶点程序（screenquad / animate_sprite）不能按 stem 补 JSON
        if SHARED_VERTEX_STEMS.contains(&stem.as_str()) || stem == "position_color" {
            continue;
        }
        // 必须成对存在，避免 vertex/fragment 指到不存在的文件
        if !core.join(format!("{}.vsh", stem)).is_file() || !core.join(format!("{}.fsh", stem)).is_file()
        {
            continue;
        }
        let json_path = core.join(format!("{}.json", stem));
        if json_path.is_file() {
            continue;
        }
        let body = format!(
            "{{\n  \"vertex\": \"{}\",\n  \"fragment\": \"{}\"\n}}\n",
            stem, stem
        );
        if fs::write(&json_path, body).is_ok() {
            stats.ensured_json += 1;
        }
    }
}

/// JSON 里 mat2 / mat3 → mat4（1.17 只支持 mat4 传递）。
fn rewrite_json_matrix_types(core: &Path, stats: &mut ShaderStats) {
    let Ok(entries) = fs::read_dir(core) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(raw) = fs::read_to_string(&path) else { continue };
        let mut out = raw.clone();
        // 只替换 type 字段里的 mat2/mat3，避免误伤其它字符串
        let mut changed = false;
        for from in ["\"type\": \"mat2\"", "\"type\": \"mat3\""] {
            if out.contains(from) {
                out = out.replace(from, "\"type\": \"mat4\"");
                changed = true;
            }
        }
        if changed && out != raw {
            if fs::write(&path, out).is_ok() {
                stats.mat_upgraded += 1;
            }
        }
    }
}

/// 1.21.6+：core JSON 的 uniforms 数组已被 uniform block 取代；
/// 残留旧 uniforms 可能导致程序加载失败 → 去掉 `uniforms` 键（保留 vertex/fragment/samplers/defines）。
fn strip_json_uniforms_for_ubo(core: &Path, stats: &mut ShaderStats) {
    let Ok(entries) = fs::read_dir(core) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(raw) = fs::read_to_string(&path) else { continue };
        if !raw.contains("\"uniforms\"") {
            continue;
        }
        let stripped = remove_json_key(&raw, "uniforms");
        if stripped != raw && fs::write(&path, &stripped).is_ok() {
            stats.uniforms_stripped += 1;
        }
    }
}

/// 从 JSON 对象文本中删除顶层 `"key": …`（粗粒度；足够处理 core shader json）。
fn remove_json_key(src: &str, key: &str) -> String {
    let pattern = format!("\"{}\"", key);
    let Some(start) = src.find(&pattern) else {
        return src.to_string();
    };
    // 找到 key 后的冒号
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
            let open = bytes[i];
            let _close = if open == b'{' { b'}' } else { b']' };
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
    // 吃掉后随空白与逗号
    let mut end = value_end;
    while end < bytes.len() && (bytes[end] as char).is_whitespace() {
        end += 1;
    }
    if end < bytes.len() && bytes[end] == b',' {
        end += 1;
    } else {
        // 删前导逗号
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

fn walk_and_rewrite(shaders: &Path, target: u32, stats: &mut ShaderStats) {
    walk_dir(shaders, target, stats);
}

fn walk_dir(dir: &Path, target: u32, stats: &mut ShaderStats) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // include/ 被 import 引用，写坏会导致整个包 shader 重载失败
            let folder = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if folder.eq_ignore_ascii_case("include") {
                continue;
            }
            walk_dir(&path, target, stats);
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if !(name.ends_with(".vsh") || name.ends_with(".fsh") || name.ends_with(".glsl")) {
            continue;
        }
        let is_core = dir.file_name().and_then(|s| s.to_str()) == Some("core");
        let Ok(raw) = fs::read_to_string(&path) else { continue };
        let mut out = raw.clone();
        let mut changed = false;

        // 导入指令：1.21.4+ 补命名空间；26.3+ `#moj_import` → `#include`
        if is_core && target >= FMT_IMPORT_NS {
            if target >= FMT_INCLUDE_DIRECTIVE {
                let (next, n) = convert_moj_import_to_include(&out);
                if n > 0 {
                    out = next;
                    changed = true;
                    stats.imports_to_include += n;
                }
            } else {
                let (next, n) = namespace_moj_imports(&out);
                if n > 0 {
                    out = next;
                    changed = true;
                    stats.imports_namespaced += n;
                }
            }
        }

        // 1.21.6+：仅 core 的 vsh/fsh 注入 globals
        if is_core
            && target >= FMT_GLOBALS_INCLUDE
            && (name.ends_with(".vsh") || name.ends_with(".fsh"))
        {
            if needs_globals_import(&out) && !has_globals_import(&out) {
                out = inject_globals_import(&out, target >= FMT_INCLUDE_DIRECTIVE);
                changed = true;
                stats.globals_injected += 1;
            }
        }

        // 1.20.5+：仅标记，不改函数体
        if target >= FMT_FOG_DISTANCE && out.contains("fog_distance(") {
            if count_args_likely_three(&out, "fog_distance") && !out.contains("2PYR: fog_distance") {
                out = format!(
                    "// 2PYR: fog_distance() 1.20.5+ 签名变更，请对照 vanilla fog.glsl\n{}",
                    out
                );
                changed = true;
                stats.fog_rewritten += 1;
            }
        }

        if changed {
            let _ = fs::write(&path, out);
        }
    }
}

/// 26.3+：`#moj_import` → `#include`，并按 Wiki 校正 include 路径形态：
/// - `<ns:include/x.glsl>` → `<ns:x.glsl>`（include 下路径，不重复 include/）
/// - `<include/x.glsl>` → `<x.glsl>`
/// - `"x.glsl"` 保持双引号（core 相对路径）
fn convert_moj_import_to_include(src: &str) -> (String, usize) {
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
            // 无法解析则仍替换指令，保留原路径
            out.push_str(&format!("#include {}\n", rest));
            n += 1;
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    (out, n)
}

fn rewrite_import_path(rest: &str) -> Option<String> {
    let quoted = if rest.starts_with('<') && rest.ends_with('>') {
        false
    } else if rest.starts_with('"') && rest.ends_with('"') {
        true
    } else {
        return None;
    };
    let inner = rest[1..rest.len() - 1].trim().trim_start_matches('/');
    if quoted {
        // core 相对："foo.glsl"
        return Some(format!("\"{}\"", inner));
    }
    if let Some((ns, path)) = inner.split_once(':') {
        let path = path.strip_prefix("include/").unwrap_or(path);
        return Some(format!("<{}:{}>", ns, path));
    }
    let path = inner.strip_prefix("include/").unwrap_or(inner);
    Some(format!("<{}>", path))
}

/// 把 `#moj_import <path.glsl>` / `"path.glsl"` 写成带 `minecraft:` 前缀的 import。
fn namespace_moj_imports(src: &str) -> (String, usize) {
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

fn needs_globals_import(src: &str) -> bool {
    src.contains("ScreenSize") || src.contains("GameTime")
}

fn has_globals_import(src: &str) -> bool {
    src.lines().any(|l| {
        let t = l.trim();
        (t.starts_with("#moj_import") || t.starts_with("#include")) && t.contains("globals.glsl")
    })
}

fn inject_globals_import(src: &str, use_include_directive: bool) -> String {
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

/// 启发式：fog_distance(...) 是否像三参调用。
fn count_args_likely_three(src: &str, fn_name: &str) -> bool {
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

fn strip_json_in(dir: &Path, n: &mut usize) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            strip_json_in(&path, n);
        } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if fs::remove_file(&path).is_ok() {
                *n += 1;
            }
        }
    }
}

fn remove_dir_quiet(p: &Path) {
    if p.is_dir() {
        let _ = fs::remove_dir_all(p);
    }
}

pub fn register_scheduler_task(scheduler: &mut crate::hurray::scheduler::Scheduler) {
    scheduler.register_task(
        "adapt_java_shaders",
        crate::hurray::scheduler::TaskType::Exclusive,
        crate::hurray::scheduler::TaskTier::Surgeon,
        |ctx| adapt_java_shaders(ctx),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_ensure_missing_core_json() {
        let temp = tempdir().unwrap();
        let core = temp.path().join("assets/minecraft/shaders/core");
        fs::create_dir_all(&core).unwrap();
        fs::write(core.join("rendertype_entity.vsh"), b"// v").unwrap();
        fs::write(core.join("rendertype_entity.fsh"), b"// f").unwrap();

        adapt_java_shaders_at(temp.path(), 34).unwrap();

        // format 34 < 63：仍用旧名，不合并改名
        let json = fs::read_to_string(core.join("rendertype_entity.json")).unwrap();
        assert!(json.contains("\"vertex\""));
        assert!(json.contains("rendertype_entity"));
    }

    #[test]
    fn test_legacy_target_strips_json() {
        let temp = tempdir().unwrap();
        let core = temp.path().join("assets/minecraft/shaders/core");
        let post = temp.path().join("assets/minecraft/shaders/post_effect");
        fs::create_dir_all(&core).unwrap();
        fs::create_dir_all(&post).unwrap();
        fs::write(core.join("rendertype_entity.vsh"), b"// v").unwrap();
        fs::write(core.join("rendertype_entity.json"), b"{}").unwrap();
        fs::write(post.join("blur.json"), b"{}").unwrap();

        adapt_java_shaders_at(temp.path(), 1).unwrap();

        assert!(core.join("rendertype_entity.vsh").exists());
        assert!(!core.join("rendertype_entity.json").exists());
        assert!(!post.exists());
    }

    #[test]
    fn test_existing_json_not_overwritten_pre_merge() {
        let temp = tempdir().unwrap();
        let core = temp.path().join("assets/minecraft/shaders/core");
        fs::create_dir_all(&core).unwrap();
        fs::write(core.join("rendertype_entity.vsh"), b"// v").unwrap();
        fs::write(core.join("rendertype_entity.json"), b"{\"keep\":true}").unwrap();

        // 55 < 63：不改名、不覆盖已有 json
        adapt_java_shaders_at(temp.path(), 55).unwrap();
        let json = fs::read_to_string(core.join("rendertype_entity.json")).unwrap();
        assert!(json.contains("keep"));
    }

    #[test]
    fn test_rename_obsolete_core_on_1216() {
        let temp = tempdir().unwrap();
        let core = temp.path().join("assets/minecraft/shaders/core");
        fs::create_dir_all(&core).unwrap();
        fs::write(core.join("rendertype_solid.vsh"), b"// v").unwrap();
        fs::write(core.join("rendertype_solid.fsh"), b"// f").unwrap();
        fs::write(core.join("rendertype_solid.json"), b"{\"vertex\":\"rendertype_solid\"}")
            .unwrap();
        // 目标版本已删除的程序（无可靠改名）
        fs::write(core.join("position_color_normal.vsh"), b"// gone").unwrap();
        fs::write(core.join("position_color_normal.json"), b"{}").unwrap();

        adapt_java_shaders_at(temp.path(), 63).unwrap();

        assert!(!core.join("rendertype_solid.vsh").exists());
        assert!(core.join("terrain.vsh").is_file());
        assert!(core.join("terrain.fsh").is_file());
        assert!(core.join("terrain.json").is_file());
        assert!(!core.join("position_color_normal.vsh").exists());
        assert!(!core.join("position_color_normal.json").exists());
    }

    #[test]
    fn test_remove_unknown_core_program_on_263() {
        let temp = tempdir().unwrap();
        let core = temp.path().join("assets/minecraft/shaders/core");
        let post = temp.path().join("assets/minecraft/shaders/post");
        let ns_post = temp.path().join("assets/minecraft/post_effect");
        fs::create_dir_all(&core).unwrap();
        fs::create_dir_all(&post).unwrap();
        fs::create_dir_all(&ns_post).unwrap();
        fs::write(core.join("entity.vsh"), b"// ok").unwrap();
        fs::write(core.join("entity.fsh"), b"// ok").unwrap();
        fs::write(core.join("rendertype_text_background.vsh"), b"// removed in 26.3").unwrap();
        fs::write(core.join("rendertype_text_background.json"), b"{}").unwrap();
        fs::write(core.join("rendertype_clouds.vsh"), b"// old name").unwrap();
        fs::write(core.join("rendertype_clouds.fsh"), b"// old name").unwrap();
        fs::write(post.join("blur.json"), b"{}").unwrap();
        fs::write(ns_post.join("blur.json"), b"{}").unwrap();

        adapt_java_shaders_at(temp.path(), 97).unwrap();

        assert!(core.join("entity.vsh").exists());
        assert!(!core.join("rendertype_text_background.vsh").exists());
        assert!(!core.join("rendertype_text_background.json").exists());
        assert!(!core.join("rendertype_clouds.vsh").exists());
        assert!(core.join("clouds.vsh").is_file());
        assert!(core.join("clouds.fsh").is_file());
        // 旧 shaders/post 删除；现代 post_effect 保留
        assert!(!post.exists());
        assert!(ns_post.join("blur.json").is_file());
    }

    #[test]
    fn test_moj_import_becomes_include_on_263() {
        let temp = tempdir().unwrap();
        let core = temp.path().join("assets/minecraft/shaders/core");
        fs::create_dir_all(&core).unwrap();
        fs::write(
            core.join("entity.vsh"),
            "#moj_import <minecraft:include/light.glsl>\n#moj_import <include/globals.glsl>\nvoid main(){}\n",
        )
        .unwrap();

        adapt_java_shaders_at(temp.path(), 97).unwrap();

        let vsh = fs::read_to_string(core.join("entity.vsh")).unwrap();
        assert!(vsh.contains("#include <minecraft:light.glsl>"));
        assert!(vsh.contains("#include <globals.glsl>"));
        assert!(!vsh.contains("#moj_import"));
    }

    #[test]
    fn test_json_uniforms_stripped_on_modern() {
        let temp = tempdir().unwrap();
        let core = temp.path().join("assets/minecraft/shaders/core");
        fs::create_dir_all(&core).unwrap();
        fs::write(core.join("entity.vsh"), b"// v").unwrap();
        fs::write(core.join("entity.fsh"), b"// f").unwrap();
        fs::write(
            core.join("entity.json"),
            r#"{ "vertex": "entity", "fragment": "entity", "uniforms": [ { "name": "ProjMat", "type": "mat4", "values": [1] } ] }"#,
        )
        .unwrap();

        adapt_java_shaders_at(temp.path(), 63).unwrap();

        let json = fs::read_to_string(core.join("entity.json")).unwrap();
        assert!(json.contains("\"vertex\""));
        assert!(!json.contains("uniforms"));
    }

    #[test]
    fn test_json_mat2_mat3_upgraded_to_mat4() {
        let temp = tempdir().unwrap();
        let core = temp.path().join("assets/minecraft/shaders/core");
        fs::create_dir_all(&core).unwrap();
        fs::write(core.join("rendertype_entity.vsh"), b"// v").unwrap();
        fs::write(
            core.join("rendertype_entity.json"),
            r#"{ "uniforms": [ { "type": "mat3" }, { "type": "mat2" }, { "type": "mat4" } ] }"#,
        )
        .unwrap();

        adapt_java_shaders_at(temp.path(), 34).unwrap();

        let json = fs::read_to_string(core.join("rendertype_entity.json")).unwrap();
        assert!(!json.contains("mat3"));
        assert!(!json.contains("mat2"));
        assert!(json.contains("mat4"));
    }

    #[test]
    fn test_moj_import_gets_namespace_on_1214() {
        let temp = tempdir().unwrap();
        let core = temp.path().join("assets/minecraft/shaders/core");
        fs::create_dir_all(&core).unwrap();
        fs::write(
            core.join("rendertype_entity.vsh"),
            "#moj_import <fog.glsl>\n#moj_import <minecraft:include/light.glsl>\nvoid main(){}\n",
        )
        .unwrap();

        adapt_java_shaders_at(temp.path(), 46).unwrap();

        let vsh = fs::read_to_string(core.join("rendertype_entity.vsh")).unwrap();
        assert!(vsh.contains("#moj_import <minecraft:fog.glsl>"));
        assert!(vsh.contains("#moj_import <minecraft:include/light.glsl>"));
    }

    #[test]
    fn test_inject_globals_on_1216() {
        let temp = tempdir().unwrap();
        let core = temp.path().join("assets/minecraft/shaders/core");
        fs::create_dir_all(&core).unwrap();
        // ≥63 会把 rendertype_entity 合并为 entity
        fs::write(core.join("rendertype_entity.fsh"), b"// f").unwrap();
        fs::write(core.join("rendertype_entity.vsh"), b"// v").unwrap();
        fs::write(
            core.join("entity_content.fsh"),
            "void main() { vec2 s = ScreenSize; float t = GameTime; }\n",
        )
        .unwrap();
        // 直接写合并后的名字，验证 inject
        fs::write(
            core.join("particle.fsh"),
            "void main() { vec2 s = ScreenSize; float t = GameTime; }\n",
        )
        .unwrap();

        adapt_java_shaders_at(temp.path(), 63).unwrap();

        let fsh = fs::read_to_string(core.join("particle.fsh")).unwrap();
        assert!(fsh.contains("#moj_import <minecraft:include/globals.glsl>"));
        assert!(fsh.contains("ScreenSize"));
    }

    #[test]
    fn test_include_directive_globals_on_263() {
        let temp = tempdir().unwrap();
        let core = temp.path().join("assets/minecraft/shaders/core");
        fs::create_dir_all(&core).unwrap();
        fs::write(
            core.join("particle.fsh"),
            "void main() { float t = GameTime; }\n",
        )
        .unwrap();

        adapt_java_shaders_at(temp.path(), 97).unwrap();

        let fsh = fs::read_to_string(core.join("particle.fsh")).unwrap();
        assert!(fsh.contains("#include <minecraft:globals.glsl>"));
    }

    #[test]
    fn test_include_trailing_newline() {
        let temp = tempdir().unwrap();
        let include = temp.path().join("assets/minecraft/shaders/include");
        fs::create_dir_all(&include).unwrap();
        fs::write(include.join("light.glsl"), "float x;").unwrap();
        // core 触发 adapt 分支
        let core = temp.path().join("assets/minecraft/shaders/core");
        fs::create_dir_all(&core).unwrap();
        fs::write(core.join("entity.vsh"), b"// v").unwrap();

        adapt_java_shaders_at(temp.path(), 97).unwrap();

        let raw = fs::read_to_string(include.join("light.glsl")).unwrap();
        assert!(raw.ends_with("\n\n"));
    }

    #[test]
    fn test_fog_distance_flagged_for_1205() {
        let temp = tempdir().unwrap();
        let core = temp.path().join("assets/minecraft/shaders/core");
        fs::create_dir_all(&core).unwrap();
        fs::write(
            core.join("fog_helper.glsl"),
            "vec4 fog_distance(vec4 a, float b, float c) { return a; }\n",
        )
        .unwrap();

        adapt_java_shaders_at(temp.path(), 32).unwrap();

        let fog = fs::read_to_string(core.join("fog_helper.glsl")).unwrap();
        assert!(fog.contains("2PYR"));
        assert!(fog.contains("fog_distance"));
    }
}
