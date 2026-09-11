//! j2b 着色器适配（尽力）。
//!
//! 参考 Bedrock Wiki《Shaders》：
//! - Bedrock 自定义 shader 位于 `shaders/hlsl` 与 `shaders/glsl`，并由 `materials/*.material` 引用。
//! - **Render Dragon 不兼容旧式自定义 shader**（Windows/主机 ≥1.16.200、其它平台 ≥1.18.30 无效）。
//!
//! 因此本模块只做「结构落盘」：
//! 1. 将 Java `assets/minecraft/shaders/core` 的 `.vsh`/`.fsh` 复制到 `shaders/glsl/` 与 `shaders/hlsl/`
//! 2. 按文件名生成最小 `materials/*.material`（引用 glsl 路径）
//! 3. 日志提示 Render Dragon 限制
//!
//! 不尝试把 GLSL 语义自动翻成 HLSL。

use std::fs;
use std::path::Path;

use crate::{log_info, log_warn};

use super::fsutil::{merge_dir, remove_dir_quiet};

/// 在 j2b 结构重组后调用。
pub fn adapt_java_shaders_for_bedrock(minecraft: &Path, pack_root: &Path) {
    let java_shaders = minecraft.join("shaders");
    if !java_shaders.is_dir() {
        return;
    }

    let glsl = pack_root.join("shaders").join("glsl");
    let hlsl = pack_root.join("shaders").join("hlsl");
    let materials = pack_root.join("materials");
    if fs::create_dir_all(&glsl).is_err() || fs::create_dir_all(&hlsl).is_err() {
        return;
    }

    // core：现代 Java 主着色器；post / include 一并收进 glsl
    let mut copied = 0usize;
    let mut names: Vec<String> = Vec::new();

    for sub in ["core", "include", "post", "program"] {
        let src = java_shaders.join(sub);
        if !src.is_dir() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&src) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let fname = entry.file_name().to_string_lossy().to_string();
                let lower = fname.to_ascii_lowercase();
                let is_shader = lower.ends_with(".vsh")
                    || lower.ends_with(".fsh")
                    || lower.ends_with(".glsl")
                    || lower.ends_with(".json");
                if !is_shader {
                    continue;
                }
                // json 仅用于推断 material 名，不进 shaders/
                if lower.ends_with(".json") {
                    let stem = fname.trim_end_matches(".json").to_string();
                    if stem.starts_with("rendertype_") || stem.starts_with("position_") {
                        names.push(stem);
                    }
                    continue;
                }
                let dst_glsl = glsl.join(&fname);
                let dst_hlsl = hlsl.join(&fname);
                if fs::copy(&path, &dst_glsl).is_ok() {
                    copied += 1;
                }
                let _ = fs::copy(&path, &dst_hlsl);
                // 从文件名推 material
                let stem = fname
                    .trim_end_matches(".vsh")
                    .trim_end_matches(".fsh")
                    .trim_end_matches(".glsl")
                    .to_string();
                if stem.starts_with("rendertype_") || stem.starts_with("position_") {
                    names.push(stem);
                }
            }
        }
        // 无子目录的散落 shader
        let _ = merge_dir(&src, &glsl);
    }

    names.sort();
    names.dedup();
    if !names.is_empty() {
        let _ = fs::create_dir_all(&materials);
        let n = write_materials_stub(&materials, &names);
        log_info!("OKAY bedrock [shaders materials × {}]", n);
    }

    remove_dir_quiet(&java_shaders);
    if copied > 0 {
        log_info!(
            "OKAY bedrock [Java shaders → shaders/glsl|hlsl × {}]（Render Dragon 下可能无效）",
            copied
        );
        log_warn!(
            "Bedrock Render Dragon 不兼容旧式自定义 shader（Win/主机≥1.16.200）；着色器仅作结构保留"
        );
    }
}

/// 为每个着色器名生成最小 material（parent 指向常见 vanilla 材质，避免半残 JSON）。
fn write_materials_stub(materials: &Path, names: &[String]) -> usize {
    let mut n = 0usize;
    for name in names {
        // json.material 文件名：rendertype_entity → entity.material 有时按后缀
        let file_stem = name.trim_start_matches("rendertype_");
        let path = materials.join(format!("{}.material", file_stem));
        if path.exists() {
            continue;
        }
        // 已有同名则跳过
        let body = format!(
            "{{\n  \"materials\": {{\n    \"version\": \"1.0.0\",\n    \"{}\": {{\n      \"vertexShader\": \"shaders/{}.vertex\",\n      \"fragmentShader\": \"shaders/{}.fragment\"\n    }}\n  }}\n}}\n",
            name, file_stem, file_stem
        );
        if fs::write(&path, body).is_ok() {
            n += 1;
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_copy_core_shaders_and_material_stub() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        let mc = root.join("assets/minecraft");
        let core = mc.join("shaders/core");
        fs::create_dir_all(&core).unwrap();
        fs::write(core.join("rendertype_entity.vsh"), b"// vs").unwrap();
        fs::write(core.join("rendertype_entity.fsh"), b"// fs").unwrap();
        fs::write(core.join("rendertype_entity.json"), b"{}").unwrap();

        adapt_java_shaders_for_bedrock(&mc, root);

        assert!(root.join("shaders/glsl/rendertype_entity.vsh").exists());
        assert!(root.join("shaders/hlsl/rendertype_entity.vsh").exists());
        assert!(root.join("materials/entity.material").exists());
        assert!(!root.join("assets/minecraft/shaders").exists());
        let mat = fs::read_to_string(root.join("materials/entity.material")).unwrap();
        assert!(mat.contains("rendertype_entity"));
        assert!(mat.contains("shaders/entity.vertex"));
    }

    #[test]
    fn test_no_java_shaders_noop() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        let mc = root.join("assets/minecraft");
        fs::create_dir_all(&mc).unwrap();
        adapt_java_shaders_for_bedrock(&mc, root);
        assert!(!root.join("shaders").exists());
    }
}
