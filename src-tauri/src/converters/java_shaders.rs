//! Java ↔ Java 着色器适配（j2j）。
//!
//! 版本差异（Minecraft Wiki / resource pack shaders）：
//! - **1.17+（pack_format ≥ 7）**：`shaders/core/*.json` 定义 + `.vsh`/`.fsh`；`#moj_import` include；uniform block。
//! - **1.17 前**：仅 `.vsh`/`.fsh`，无 JSON 定义，uniform 直接声明。
//! - **1.21（pack_format 34）起**：core shader JSON / uniform 布局又变，旧包整包 shader 容易导致加载失败。
//!
//! 策略：
//! - 跨过 1.21 边界时不再**整目录删除**，而是：
//!   - 保留 `.vsh`/`.fsh`/include
//!   - 为缺失的 core 着色器补最小 `.json`
//!   - 目标 &lt; 1.17 时去掉 `.json` 与 `post_effect`（旧客户端不识别）
//! - 无法自动改写的 GLSL 语义差异仅记日志。

use std::fs;
use std::path::Path;

use crate::hurray::context::HurrayContext;
use crate::{log_info, log_warn};

/// pack_format ≥ 7 视为「1.17+ 着色器体系」
fn is_modern_shader_api(pack_format: u32) -> bool {
    pack_format >= 7
}

/// 挂到调度器的入口：按目标版本适配 shaders/。
pub fn adapt_java_shaders(ctx: &HurrayContext) -> Result<(), String> {
    let root = Path::new(ctx.temp_dir());
    let target = ctx
        .get_data("target_pack_format")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(75);
    adapt_java_shaders_at(root, target)
}

/// 对工作目录中的 `assets/minecraft/shaders` 做目标版本适配。
pub fn adapt_java_shaders_at(pack_root: &Path, target_pack_format: u32) -> Result<(), String> {
    let shaders = pack_root.join("assets").join("minecraft").join("shaders");
    if !shaders.is_dir() {
        return Ok(());
    }

    if !is_modern_shader_api(target_pack_format) {
        // 旧版：去掉 JSON 定义与 post_effect
        remove_dir_quiet(&shaders.join("post_effect"));
        let mut n = 0usize;
        strip_json_in(&shaders, &mut n);
        if n > 0 {
            log_info!("OKAY java-shaders [strip JSON × {} for legacy target {}]", n, target_pack_format);
        }
        log_warn!(
            "目标 pack_format {} 使用旧着色器 API；无法自动改写 uniform block，若加载失败请手改",
            target_pack_format
        );
        return Ok(());
    }

    // 1.17+：补全缺失的 core JSON
    let core = shaders.join("core");
    if !core.is_dir() {
        return Ok(());
    }
    let mut ensured = 0usize;
    let Ok(entries) = fs::read_dir(&core) else {
        return Ok(());
    };
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
        let json_path = core.join(format!("{}.json", stem));
        if json_path.is_file() {
            continue;
        }
        // 最小定义：vertex/fragment 同名（现代客户端可解析）
        let body = format!(
            "{{\n  \"vertex\": \"{}\",\n  \"fragment\": \"{}\"\n}}\n",
            stem, stem
        );
        if fs::write(&json_path, body).is_ok() {
            ensured += 1;
        }
    }
    if ensured > 0 {
        log_info!("OKAY java-shaders [ensured core JSON × {}]", ensured);
    }
    if target_pack_format >= 34 {
        log_warn!(
            "跨 1.21（pack_format 34）着色器 API 有变；已保留源码并补 JSON，仍可能需手工调整 uniform"
        );
    }
    Ok(())
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
}
