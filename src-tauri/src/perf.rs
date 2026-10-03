//! 性能档位：把「平衡 / 性能」两档解析成具体的线程预算与并发包数。
//!
//! 设计要点
//! ─────────
//! * 引擎的并行是**单一总预算**：批处理用 `conversion_threads` 建一个 rayon 池，
//!   包内 `TaskType::Parallel` 任务与 PNG 并行编码都在同一个池里跑。所以这里
//!   返回的 `threads` 是「总活跃线程上限」，`packs` 只是在此基础上再限制
//!   「同时展开几个包」（主要受内存约束）。
//! * **平衡**：留出交互余量，用一半核心（下限 2、上限 8），同时最多 2 个包。
//! * **性能**：吃满核心（上限 32），同时最多 6 个包——包数上限是内存保护，
//!   不是 CPU 保护（每个包解压 + 图片解码有峰值内存）。
//! * **内存保护**：可用物理内存不足时自动把包数压到 1 并给出提示，避免 OOM。
//! * 旧配置 `conversion_threads`（1–4）作为迁移来源：≤2 → 平衡，≥3 → 性能。

use serde::Serialize;

/// 并发档位。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PerfMode {
    Balanced,
    Performance,
}

impl PerfMode {
    pub fn as_str(self) -> &'static str {
        match self {
            PerfMode::Balanced => "balanced",
            PerfMode::Performance => "performance",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "balanced" | "balance" => Some(PerfMode::Balanced),
            "performance" | "perf" => Some(PerfMode::Performance),
            _ => None,
        }
    }
}

/// 解析结果（同时给前端展示用）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerfPlan {
    pub mode: String,
    /// 本机逻辑核心数。
    pub cores: usize,
    /// 总活跃线程上限（rayon 池大小）。
    pub threads: usize,
    /// 同时转换的包数上限。
    pub packs: usize,
    /// 可用物理内存（MB），拿不到时为 None。
    pub available_memory_mb: Option<u64>,
    /// 内存保护等原因造成的降级说明。
    pub note: Option<String>,
}

/// 逻辑核心数（拿不到时保守返回 2）。
pub fn detect_cores() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
        .max(1)
}

/// 可用物理内存（MB）。Windows 用 `GlobalMemoryStatusEx`，其他平台返回 None。
#[cfg(windows)]
pub fn available_memory_mb() -> Option<u64> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
    if ok == 0 {
        None
    } else {
        Some(status.ullAvailPhys / (1024 * 1024))
    }
}

#[cfg(not(windows))]
pub fn available_memory_mb() -> Option<u64> {
    None
}

/// 平衡档的线程预算：一半核心，下限 2、上限 8。
fn balanced_threads(cores: usize) -> usize {
    (cores / 2).clamp(2, 8)
}

/// 性能档的线程预算：吃满核心，上限 32。
fn performance_threads(cores: usize) -> usize {
    cores.clamp(2, 32)
}

/// 包数上限：平衡 2；性能 min(核心数, 6)（内存保护）。
fn packs_for(mode: PerfMode, cores: usize) -> usize {
    match mode {
        PerfMode::Balanced => 2.min(cores.max(1)),
        PerfMode::Performance => cores.clamp(1, 6),
    }
}

/// 低于该可用内存（MB）就把并发包数压到 1。
const MEMORY_GUARD_MB: u64 = 1500;

/// 解析档位 → 具体预算。
///
/// `mode_opt` 为空时按 `legacy_threads` 迁移：≤2 → 平衡，≥3 → 性能，
/// 都没有则默认**平衡**。
pub fn resolve(mode_opt: Option<&str>, legacy_threads: Option<u32>) -> PerfPlan {
    let mode = mode_opt
        .and_then(PerfMode::from_str)
        .unwrap_or_else(|| match legacy_threads {
            Some(n) if n >= 3 => PerfMode::Performance,
            Some(_) => PerfMode::Balanced,
            None => PerfMode::Balanced,
        });

    let cores = detect_cores();
    let threads = match mode {
        PerfMode::Balanced => balanced_threads(cores),
        PerfMode::Performance => performance_threads(cores),
    };
    let mut packs = packs_for(mode, cores).min(threads).max(1);

    let available = available_memory_mb();
    let mut note = None;
    if let Some(mb) = available {
        if mb < MEMORY_GUARD_MB && packs > 1 {
            note = Some(format!(
                "可用内存偏低（{} MB），并发包数已降为 1",
                mb
            ));
            packs = 1;
        }
    }

    PerfPlan {
        mode: mode.as_str().to_string(),
        cores,
        threads,
        packs,
        available_memory_mb: available,
        note,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balanced_uses_half_cores_with_bounds() {
        assert_eq!(balanced_threads(2), 2);
        assert_eq!(balanced_threads(4), 2);
        assert_eq!(balanced_threads(8), 4);
        assert_eq!(balanced_threads(16), 8);
        assert_eq!(balanced_threads(64), 8, "上限 8");
    }

    #[test]
    fn performance_uses_all_cores_capped() {
        assert_eq!(performance_threads(4), 4);
        assert_eq!(performance_threads(12), 12);
        assert_eq!(performance_threads(64), 32, "上限 32");
        assert_eq!(performance_threads(1), 2, "下限 2");
    }

    #[test]
    fn packs_are_memory_bounded() {
        assert_eq!(packs_for(PerfMode::Balanced, 16), 2);
        assert_eq!(packs_for(PerfMode::Performance, 4), 4);
        assert_eq!(packs_for(PerfMode::Performance, 16), 6, "包数上限 6");
    }

    #[test]
    fn legacy_threads_migrate_to_modes() {
        assert_eq!(resolve(Some("balanced"), Some(4)).mode, "balanced");
        assert_eq!(resolve(Some("performance"), None).mode, "performance");
        assert_eq!(resolve(None, Some(1)).mode, "balanced");
        assert_eq!(resolve(None, Some(4)).mode, "performance");
        assert_eq!(resolve(None, None).mode, "balanced", "默认平衡");
        assert_eq!(resolve(Some("garbage"), None).mode, "balanced");
    }

    #[test]
    fn plan_is_self_consistent() {
        let plan = resolve(Some("performance"), None);
        assert!(plan.threads >= 2);
        assert!(plan.packs >= 1 && plan.packs <= 6);
        assert!(
            plan.packs <= plan.threads,
            "packs ({}) 不应超过线程预算 ({})",
            plan.packs,
            plan.threads
        );
        assert!(plan.cores >= 1);
    }
}
