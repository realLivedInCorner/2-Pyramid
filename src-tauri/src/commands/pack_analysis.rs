//! 资源包结构分析命令（只读）。
//!
//! 供界面 / 开发者选项直接指定一个 zip 或已解压目录，返回结构分析结果：
//! 是否分层（overlays）、是否声明 supported_formats 区间、是否存在非标准
//! 版本折叠目录、是否一包多根，以及每层的文件数与覆盖计数。
//!
//! 该命令**不修改任何文件、不参与转换**——先用真实包跑出结果，再决定
//! 「多版本包怎么转换」的语义。

use std::path::Path;

use crate::pack::analysis::{analyze_dir, analyze_zip, PackAnalysis};

/// 分析 zip 或目录（自动判断）。
#[tauri::command]
pub fn analyze_pack(path: String) -> Result<PackAnalysis, String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(format!("路径不存在：{}", path));
    }
    let result = if p.is_dir() {
        analyze_dir(p)
    } else {
        analyze_zip(p)
    }?;

    // 摘要进日志，便于转换/测试后回看
    crate::log_info!("OKAY analyze_pack [{}]", result.summary());
    for layer in &result.layers {
        if layer.directory.is_empty() {
            continue;
        }
        crate::log_info!(
            "  layer {}: formats={:?} files={} overrides={}",
            layer.directory,
            layer.formats,
            layer.file_count,
            layer.override_count
        );
    }
    for w in &result.warnings {
        crate::log_warn!("  analysis: {}", w);
    }

    Ok(result)
}
