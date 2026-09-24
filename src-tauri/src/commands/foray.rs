//! Foray Tauri 命令层：分析工作台 / 编辑 / 导出 / AI。

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use tauri::State;

use crate::foray::ai::{self, AiConfig, TierPayload};
use crate::foray::export::{export_pack, ExportMode};
use crate::foray::paint::{BrushStamp, HsvAdjust, PaintSession};
use crate::foray::probe::{run_probes, ProbeReport};
use crate::foray::rom::Rom;
use crate::foray::zip_safe::{open_pack, SafeLimits};

#[derive(Default)]
pub struct ForayState {
    pub rom: Mutex<Option<Rom>>,
    pub paint: Mutex<Option<PaintSession>>,
    pub paint_path: Mutex<Option<String>>,
    pub ai: Mutex<AiConfigWrapper>,
}

#[derive(Debug, Clone)]
pub struct AiConfigWrapper {
    pub cfg: AiConfig,
}

impl Default for AiConfigWrapper {
    fn default() -> Self {
        Self {
            cfg: AiConfig::default(),
        }
    }
}

fn walk_collect(dir: &crate::foray::rom::RomDir, out: &mut Vec<(String, String)>) {
    for f in &dir.files {
        out.push((f.path.clone(), format!("{:?}", f.kind)));
    }
    for d in &dir.dirs {
        walk_collect(d, out);
    }
}

#[derive(Serialize)]
pub struct OpenForayResult {
    pub ok: bool,
    pub source: String,
    pub file_count: usize,
    pub bytes: u64,
    pub description: String,
    pub pack_format: Option<u32>,
    pub issues: usize,
    pub probe_error: Option<String>,
}

/// 打开 zip 并构建 ROM + 跑探针（EM 分流后的主入口）。
#[tauri::command]
pub async fn foray_open_pack(
    state: State<'_, ForayState>,
    path: String,
) -> Result<OpenForayResult, String> {
    let p = PathBuf::from(&path);
    let limits = SafeLimits::default();
    let archive = open_pack(&p, &limits)?;
    let rom = crate::foray::rom::build(&archive, &path);
    let report = run_probes(&rom);
    let issues = rom.issues.len() + report.issues.len();
    let probe_error = report.errors.first().cloned();

    let result = OpenForayResult {
        ok: true,
        source: path.clone(),
        file_count: rom.stats.files,
        bytes: rom.stats.bytes,
        description: rom.meta.description.clone(),
        pack_format: rom.meta.pack_format.or(rom.meta.min_format),
        issues,
        probe_error,
    };

    *state.rom.lock().unwrap() = Some(rom);
    *state.paint.lock().unwrap() = None;
    *state.paint_path.lock().unwrap() = None;
    Ok(result)
}

#[derive(Serialize)]
pub struct RomSnapshot {
    pub description: String,
    pub pack_format: Option<u32>,
    pub min_format: Option<u32>,
    pub max_format: Option<u32>,
    pub stats: crate::foray::rom::RomStats,
    pub tree_json: String,
    pub issues: Vec<crate::foray::rom::RomIssue>,
    pub files: Vec<(String, String)>,
}

#[tauri::command]
pub async fn foray_get_rom(state: State<'_, ForayState>) -> Result<RomSnapshot, String> {
    let guard = state.rom.lock().unwrap();
    let rom = guard.as_ref().ok_or("no pack opened")?;
    let mut files = Vec::new();
    walk_collect(&rom.root, &mut files);
    files.sort();
    let tree_json = serde_json::to_string(&rom.root).map_err(|e| e.to_string())?;
    Ok(RomSnapshot {
        description: rom.meta.description.clone(),
        pack_format: rom.meta.pack_format,
        min_format: rom.meta.min_format,
        max_format: rom.meta.max_format,
        stats: rom.stats.clone(),
        tree_json,
        issues: rom.issues.clone(),
        files,
    })
}

#[tauri::command]
pub async fn foray_run_probes(state: State<'_, ForayState>) -> Result<ProbeReport, String> {
    let guard = state.rom.lock().unwrap();
    let rom = guard.as_ref().ok_or("no pack opened")?;
    Ok(run_probes(rom))
}

#[derive(Serialize)]
pub struct FilePreview {
    pub path: String,
    pub size: u64,
    pub kind: String,
    pub paintable: bool,
    pub dirty: bool,
    pub utf8: Option<String>,
    pub png_base64: Option<String>,
}

fn b64(data: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(data)
}

#[tauri::command]
pub async fn foray_read_file(
    state: State<'_, ForayState>,
    path: String,
) -> Result<FilePreview, String> {
    let guard = state.rom.lock().unwrap();
    let rom = guard.as_ref().ok_or("no pack opened")?;
    let f = rom.find_file(&path).ok_or_else(|| format!("not found: {path}"))?;
    let is_png = path.to_ascii_lowercase().ends_with(".png");
    Ok(FilePreview {
        path: f.path.clone(),
        size: f.size,
        kind: format!("{:?}", f.kind),
        paintable: f.paintable,
        dirty: f.dirty,
        utf8: if is_png {
            None
        } else {
            Some(String::from_utf8_lossy(&f.data).to_string())
        },
        png_base64: if is_png {
            Some(b64(&f.data))
        } else {
            None
        },
    })
}

#[tauri::command]
pub async fn foray_paint_open(state: State<'_, ForayState>, path: String) -> Result<(), String> {
    let data = {
        let guard = state.rom.lock().unwrap();
        let rom = guard.as_ref().ok_or("no pack opened")?;
        let f = rom.find_file(&path).ok_or_else(|| format!("not found: {path}"))?;
        if !f.paintable {
            return Err("not paintable".into());
        }
        f.data.clone()
    };
    let session = PaintSession::open_png(&data)?;
    *state.paint.lock().unwrap() = Some(session);
    *state.paint_path.lock().unwrap() = Some(path);
    Ok(())
}

#[tauri::command]
pub async fn foray_paint_brush(state: State<'_, ForayState>, stamp: BrushStamp) -> Result<(), String> {
    let mut g = state.paint.lock().unwrap();
    let s = g.as_mut().ok_or("paint session not open")?;
    s.brush(&stamp);
    Ok(())
}

#[tauri::command]
pub async fn foray_paint_hsv(state: State<'_, ForayState>, adj: HsvAdjust) -> Result<(), String> {
    let mut g = state.paint.lock().unwrap();
    let s = g.as_mut().ok_or("paint session not open")?;
    s.apply_hsv(&adj);
    Ok(())
}

#[tauri::command]
pub async fn foray_paint_undo(state: State<'_, ForayState>) -> Result<bool, String> {
    let mut g = state.paint.lock().unwrap();
    let s = g.as_mut().ok_or("paint session not open")?;
    Ok(s.undo())
}

#[tauri::command]
pub async fn foray_paint_preview(state: State<'_, ForayState>) -> Result<String, String> {
    let g = state.paint.lock().unwrap();
    let s = g.as_ref().ok_or("paint session not open")?;
    Ok(b64(&s.to_png()?))
}

#[tauri::command]
pub async fn foray_paint_commit(state: State<'_, ForayState>) -> Result<(), String> {
    let png = {
        let g = state.paint.lock().unwrap();
        let s = g.as_ref().ok_or("paint session not open")?;
        s.to_png()?
    };
    let path = state.paint_path.lock().unwrap().clone().ok_or("no paint path")?;
    let mut guard = state.rom.lock().unwrap();
    let rom = guard.as_mut().ok_or("no pack opened")?;
    let f = rom
        .find_file_mut(&path)
        .ok_or_else(|| format!("not found: {path}"))?;
    f.data = png;
    f.dirty = true;
    Ok(())
}

#[derive(Serialize)]
pub struct ExportResult {
    pub path: String,
}

#[tauri::command]
pub async fn foray_export(
    state: State<'_, ForayState>,
    mode: String,
    dest_dir: Option<String>,
) -> Result<ExportResult, String> {
    let guard = state.rom.lock().unwrap();
    let rom = guard.as_ref().ok_or("no pack opened")?;
    let m = match mode.as_str() {
        "in_place" | "inplace" | "overwrite" => ExportMode::InPlace,
        _ => ExportMode::SaveAs,
    };
    // 原地覆盖必须由 UI 二次确认后调用；这里再次校验源路径存在
    if m == ExportMode::InPlace {
        let src = Path::new(&rom.source_path);
        if !src.exists() {
            return Err("source pack missing".into());
        }
    }
    let dest = dest_dir.map(PathBuf::from);
    let out = export_pack(rom, m, dest.as_deref())?;
    Ok(ExportResult {
        path: out.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn foray_ai_config_get(state: State<'_, ForayState>) -> Result<AiConfig, String> {
    {
        let g = state.ai.lock().unwrap();
        if !g.cfg.api_key.is_empty() || !g.cfg.model.is_empty() && g.cfg.model != "gpt-4o-mini" {
            return Ok(g.cfg.clone());
        }
    }
    // 首次：从磁盘加载（0600）
    let loaded = ai::load_ai_config();
    state.ai.lock().unwrap().cfg = loaded.clone();
    Ok(loaded)
}

#[tauri::command]
pub async fn foray_ai_config_set(
    state: State<'_, ForayState>,
    config: AiConfig,
) -> Result<(), String> {
    let mut g = state.ai.lock().unwrap();
    g.cfg = config;
    // 落盘（POSIX 0600）；失败不阻断内存生效
    if let Err(e) = ai::save_ai_config(&g.cfg) {
        crate::log_warn!("foray ai config save failed: {}", e);
    }
    Ok(())
}

#[derive(Serialize)]
pub struct AiPrepareResult {
    pub payload_text: String,
    pub preview: Vec<String>,
}

#[tauri::command]
pub async fn foray_ai_prepare(
    state: State<'_, ForayState>,
    tier: u8,
    selected: Vec<String>,
) -> Result<AiPrepareResult, String> {
    if tier > 5 {
        return Err("tier must be 0..=5".into());
    }
    let guard = state.rom.lock().unwrap();
    let rom = guard.as_ref().ok_or("no pack opened")?;
    let report = run_probes(rom);
    let probe_json = serde_json::to_string(&report).unwrap_or_default();
    let payload: TierPayload = ai::build_tier_payload(rom, tier, &selected, "", false);
    Ok(AiPrepareResult {
        payload_text: payload.text,
        preview: payload.preview,
    })
}

#[tauri::command]
pub async fn foray_ai_analyze(
    state: State<'_, ForayState>,
    tier: u8,
    selected: Vec<String>,
) -> Result<String, String> {
    if tier == 0 {
        return Err("tier 0 does not call external API".into());
    }
    let (cfg, text) = {
        let guard = state.rom.lock().unwrap();
        let rom = guard.as_ref().ok_or("no pack opened")?;
        let report = run_probes(rom);
        let probe_json = serde_json::to_string(&report).unwrap_or_default();
        let payload = ai::build_tier_payload(rom, tier, &selected, "", false);
        let cfg = state.ai.lock().unwrap().cfg.clone();
        (cfg, payload.text)
    };
    // 不在日志中打印 api_key
    crate::log_info!(
        "foray ai analyze tier={} user_chars={}",
        tier,
        text.len()
    );
    tauri::async_runtime::spawn_blocking(move || {
        ai::chat_completion_blocking(&cfg, &text, 60)
    })
    .await
    .map_err(|e| format!("join: {e}"))?
}

#[tauri::command]
pub async fn foray_ai_test(state: State<'_, ForayState>) -> Result<String, String> {
    let cfg = state.ai.lock().unwrap().cfg.clone();
    tauri::async_runtime::spawn_blocking(move || {
        ai::chat_completion_blocking(&cfg, "ping", 15)
    })
    .await
    .map_err(|e| format!("join: {e}"))?
}
