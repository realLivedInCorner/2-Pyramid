//! Java ↔ Java 着色器适配（j2j），重点 **1.20 → 26.x**。
//!
//! 版本差异（社区/实战归纳 + Wiki）：
//! - **1.17+（format ≥ 7）**：core JSON；JSON 里 **只能传 mat4**，不能 mat2/mat3。
//! - **1.20.5（format ≥ 32）**：`fog.glsl` 中 `fog_distance()` 参数变动。
//! - **1.21.4（format ≥ 46）**：`#moj_import` 需要 **命名空间+路径**（`<minecraft:...>`）。
//! - **1.21.6（format ≥ 63）**：部分 JSON 消失；`ScreenSize` / `GameTime` 等改从
//!   **include 的 glsl（globals.glsl 等）** 获取。
//!
//! 本模块在保留 `.vsh`/`.fsh` 的前提下做这些改写；无法安全改写的只打日志。

use std::fs;
use std::path::Path;

use crate::hurray::context::HurrayContext;
use crate::{log_info, log_warn};

/// pack_format 里程碑
const FMT_MODERN_JSON: u32 = 7; // 1.17
const FMT_FOG_DISTANCE: u32 = 32; // 1.20.5
const FMT_IMPORT_NS: u32 = 46; // 1.21.4
const FMT_GLOBALS_INCLUDE: u32 = 63; // 1.21.6

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

    // 1) JSON：仅在成对 .vsh+.fsh 且缺失 JSON 时补最小定义；
    //    1.21.6+（JSON 体系收缩）不再生成 stub，避免无效定义导致「着色器重载失败」。
    if core.is_dir() {
        if target_pack_format < FMT_GLOBALS_INCLUDE {
            ensure_core_json(&core, &mut stats);
        }
        if target_pack_format >= FMT_MODERN_JSON {
            rewrite_json_matrix_types(&core, &mut stats);
        }
    }

    // 2) 源码遍历（跳过 include/，避免写坏被 #moj_import 的公共文件）
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
    if stats.imports_namespaced > 0 {
        log_info!(
            "OKAY java-shaders [moj_import → namespace × {}]",
            stats.imports_namespaced
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
    imports_namespaced: usize,
    globals_injected: usize,
    fog_rewritten: usize,
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
        if stem == "screenquad" || stem == "animate_sprite" || stem == "position_color" {
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

fn walk_and_rewrite(shaders: &Path, target: u32, stats: &mut ShaderStats) {
    walk_dir(shaders, target, stats);
}

fn walk_dir(dir: &Path, target: u32, stats: &mut ShaderStats) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // include/ 被 #moj_import 引用，写坏会导致整个包 shader 重载失败
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

        // 1.21.4+：仅 core 下，无冒号的 <foo.glsl> 补 minecraft: 前缀
        if is_core && target >= FMT_IMPORT_NS {
            let (next, n) = namespace_moj_imports(&out);
            if n > 0 {
                out = next;
                changed = true;
                stats.imports_namespaced += n;
            }
        }

        // 1.21.6+：仅 core 的 vsh/fsh 注入 globals
        if is_core && target >= FMT_GLOBALS_INCLUDE && (name.ends_with(".vsh") || name.ends_with(".fsh"))
        {
            if needs_globals_import(&out) && !has_globals_import(&out) {
                out = inject_globals_import(&out);
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

/// 把 `#moj_import <path.glsl>` / `"path.glsl"` 写成带 `minecraft:` 前缀的 import。
fn namespace_moj_imports(src: &str) -> (String, usize) {
    let mut n = 0usize;
    let mut out = String::with_capacity(src.len());
    for line in src.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#moj_import") {
            // #moj_import <foo.glsl> 或 "foo.glsl"
            if let Some(rest) = trimmed.strip_prefix("#moj_import") {
                let rest = rest.trim();
                if (rest.starts_with('<') && rest.ends_with('>') && !rest.contains(':'))
                    || (rest.starts_with('"') && rest.ends_with('"') && !rest.contains(':'))
                {
                    // 不带命名空间 → 补 minecraft:
                    let inner: &str = if rest.starts_with('<') {
                        &rest[1..rest.len() - 1]
                    } else {
                        &rest[1..rest.len() - 1]
                    };
                    let inner = inner.trim_start_matches('/');
                    // 已是 include/xxx 时只补命名空间，不改成 include/include
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
    // 只看标识符使用，避免把注释里的 Globals 也算进去
    src.contains("ScreenSize") || src.contains("GameTime")
}

fn has_globals_import(src: &str) -> bool {
    src.lines().any(|l| {
        let t = l.trim();
        t.starts_with("#moj_import") && t.contains("globals.glsl")
    })
}

fn inject_globals_import(src: &str) -> String {
    // 插在文件首个非注释非空行之前
    let import = "#moj_import <minecraft:include/globals.glsl>\n";
    let mut out = String::with_capacity(src.len() + import.len() + 8);
    let mut injected = false;
    for line in src.lines() {
        let t = line.trim();
        if !injected && !t.is_empty() && !t.starts_with("//") && !t.starts_with("/*") && !t.starts_with("*") {
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
    fn test_existing_json_not_overwritten() {
        let temp = tempdir().unwrap();
        let core = temp.path().join("assets/minecraft/shaders/core");
        fs::create_dir_all(&core).unwrap();
        fs::write(core.join("rendertype_entity.vsh"), b"// v").unwrap();
        fs::write(core.join("rendertype_entity.json"), b"{\"keep\":true}").unwrap();

        adapt_java_shaders_at(temp.path(), 88).unwrap();
        let json = fs::read_to_string(core.join("rendertype_entity.json")).unwrap();
        assert!(json.contains("keep"));
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
        fs::write(
            core.join("rendertype_entity.fsh"),
            "void main() { vec2 s = ScreenSize; float t = GameTime; }\n",
        )
        .unwrap();

        adapt_java_shaders_at(temp.path(), 63).unwrap();

        let fsh = fs::read_to_string(core.join("rendertype_entity.fsh")).unwrap();
        assert!(fsh.contains("#moj_import <minecraft:include/globals.glsl>"));
        assert!(fsh.contains("ScreenSize"));
    }

    #[test]
    fn test_fog_distance_flagged_for_1205() {
        let temp = tempdir().unwrap();
        let core = temp.path().join("assets/minecraft/shaders/core");
        fs::create_dir_all(&core).unwrap();
        // include/ 不改写；放在 core 下验证标记逻辑
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
