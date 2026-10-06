//! 任务契约与冲突感知调度。
//!
//! 要解决的问题：并行与否若只由「类型 + 注册顺序」隐含决定，就没人能回答
//! 「这两个任务能不能同时跑」。本模块把这件事变成**可计算**的：
//!
//! * 任务声明 [`TaskDecl`]：读范围 + 写范围 + 是否独占；
//! * [`plan`] 按声明算出**波次**（同一波次内可并发，波次之间是屏障）；
//! * 声明 [`ScopeSet::any`]（读写整包）的任务与任何非空范围都冲突 → **自动退化为串行**。
//!   这是保守默认：宁可串行，也不让两个任务在"说不清各自碰哪里"的情况下并发；
//! * [`TaskDecl::check_write`] 供调试模式断言「写了自己没声明的路径」。
//!
//! 阶段用 [`Tier`] 表达，它是全库唯一的阶段类型（`task_registry` / `native_run` /
//! `bedrock_convert` 都用它）。

use std::collections::BTreeSet;

use super::error::AromError;

/// 执行阶段。取值与原 `TaskTier`（`hurray/scheduler.rs`）一一对应。
///
/// **§9.131**：`TaskTier` 已删除，本类型成为唯一的阶段类型。为此补上了 `Hash`
/// （原 `TaskTier` 有而本类型漏了——那正是同一概念写两遍的代价）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tier {
    Eraser = 10,
    Architect = 20,
    Surgeon = 30,
    Closure = 40,
}

impl Tier {
    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Eraser => "eraser",
            Tier::Architect => "architect",
            Tier::Surgeon => "surgeon",
            Tier::Closure => "closure",
        }
    }

    /// 全部阶段，按执行顺序。
    pub fn all() -> [Tier; 4] {
        [Tier::Eraser, Tier::Architect, Tier::Surgeon, Tier::Closure]
    }
}

/// 路径范围声明：精确路径、前缀（按路径段）或整包。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScopeSet {
    any: bool,
    prefixes: BTreeSet<String>,
    exact: BTreeSet<String>,
}

impl ScopeSet {
    /// 什么都不碰。
    pub fn none() -> Self {
        Self::default()
    }

    /// 整包范围（读写任意路径）。与任何非空范围都冲突，因此声明它的任务会退化为串行。
    pub fn any() -> Self {
        Self {
            any: true,
            ..Self::default()
        }
    }

    /// 前缀（按路径段匹配：`assets` 命中 `assets/x.png`，不命中 `assets2/x.png`）。
    pub fn prefix(prefix: impl Into<String>) -> Self {
        Self::none().with_prefix(prefix)
    }

    /// 精确路径。
    pub fn exact(path: impl Into<String>) -> Self {
        Self::none().with_exact(path)
    }

    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        let p = normalize(prefix.into());
        if p.is_empty() {
            self.any = true;
        } else {
            self.prefixes.insert(p);
        }
        self
    }

    pub fn with_exact(mut self, path: impl Into<String>) -> Self {
        let p = normalize(path.into());
        if !p.is_empty() {
            self.exact.insert(p);
        }
        self
    }

    pub fn union(&self, other: &Self) -> Self {
        let mut out = self.clone();
        out.any |= other.any;
        out.prefixes.extend(other.prefixes.iter().cloned());
        out.exact.extend(other.exact.iter().cloned());
        out
    }

    pub fn is_any(&self) -> bool {
        self.any
    }

    pub fn is_empty(&self) -> bool {
        !self.any && self.prefixes.is_empty() && self.exact.is_empty()
    }

    pub fn contains(&self, path: &str) -> bool {
        if self.any {
            return true;
        }
        let p = normalize(path.to_string());
        if self.exact.contains(&p) {
            return true;
        }
        self.prefixes
            .iter()
            .any(|prefix| p == *prefix || is_under(&p, prefix))
    }

    /// 两个范围是否可能触碰同一路径。
    ///
    /// 保守判定：任意一方声明整包即冲突；前缀之间只有「互为前缀」才算冲突
    /// （`assets/ui` 与 `assets/textures` 不相交）。
    pub fn overlaps(&self, other: &Self) -> bool {
        if self.is_empty() || other.is_empty() {
            return false;
        }
        if self.any || other.any {
            return true;
        }
        if self.exact.iter().any(|p| other.contains(p)) {
            return true;
        }
        if other.exact.iter().any(|p| self.contains(p)) {
            return true;
        }
        for a in &self.prefixes {
            for b in &other.prefixes {
                if a == b || is_under(a, b) || is_under(b, a) {
                    return true;
                }
            }
        }
        false
    }

    /// 供日志与报告：一句话描述范围。
    pub fn describe(&self) -> String {
        if self.any {
            return "any".to_string();
        }
        if self.is_empty() {
            return "none".to_string();
        }
        let mut parts: Vec<String> = self.prefixes.iter().map(|p| format!("{p}/**")).collect();
        parts.extend(self.exact.iter().cloned());
        parts.join(", ")
    }
}

/// 任务声明。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDecl {
    pub name: String,
    pub tier: Tier,
    /// 独占任务：必须单独占一个波次（现状 39 个 Eraser 型任务属此类）。
    pub exclusive: bool,
    pub reads: ScopeSet,
    pub writes: ScopeSet,
}

impl TaskDecl {
    pub fn new(name: impl Into<String>, tier: Tier) -> Self {
        Self {
            name: name.into(),
            tier,
            exclusive: false,
            reads: ScopeSet::none(),
            writes: ScopeSet::none(),
        }
    }

    pub fn reads(mut self, scope: ScopeSet) -> Self {
        self.reads = scope;
        self
    }

    pub fn writes(mut self, scope: ScopeSet) -> Self {
        self.writes = scope;
        self
    }

    pub fn exclusive(mut self, yes: bool) -> Self {
        self.exclusive = yes;
        self
    }

    /// 调试断言：写入未声明的路径必须报错（把「忘记登记」从静默错误变成可测失败）。
    pub fn check_write(&self, path: &str) -> Result<(), AromError> {
        if self.writes.contains(path) {
            Ok(())
        } else {
            Err(AromError::internal(format!(
                "task `{}` wrote an undeclared path: {} (declared writes: {})",
                self.name,
                path,
                self.writes.describe()
            )))
        }
    }
}

/// 一个波次：其中所有任务可以并发执行；波次之间是屏障。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wave {
    pub tier: Tier,
    /// 任务在输入切片里的下标。
    pub tasks: Vec<usize>,
    /// 为什么这样切分（日志与调试用）。
    pub note: String,
}

impl Wave {
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    pub waves: Vec<Wave>,
}

impl Plan {
    pub fn task_count(&self) -> usize {
        self.waves.iter().map(|w| w.tasks.len()).sum()
    }

    /// 单波最大并发度。
    pub fn max_parallelism(&self) -> usize {
        self.waves.iter().map(|w| w.tasks.len()).max().unwrap_or(0)
    }

    pub fn is_serial(&self) -> bool {
        self.max_parallelism() <= 1
    }

    pub fn summary(&self) -> String {
        let mut out = format!(
            "{} waves, {} tasks, max parallelism {}",
            self.waves.len(),
            self.task_count(),
            self.max_parallelism()
        );
        for (i, wave) in self.waves.iter().enumerate() {
            let names: Vec<String> = wave.tasks.iter().map(|t| t.to_string()).collect();
            out.push_str(&format!(
                "\n  wave {i} [{}] tasks=[{}] {}",
                wave.tier.as_str(),
                names.join(","),
                wave.note
            ));
        }
        out
    }
}

/// 两个任务能否同波并发；不能则给出原因。
pub fn conflict_reason(a: &TaskDecl, b: &TaskDecl) -> Option<String> {
    if a.exclusive || b.exclusive {
        return Some(format!(
            "{} is exclusive",
            if a.exclusive { &a.name } else { &b.name }
        ));
    }
    if a.writes.overlaps(&b.writes) {
        return Some(format!("write/write overlap: {} vs {}", a.name, b.name));
    }
    if a.writes.overlaps(&b.reads) {
        return Some(format!("write/read overlap: {} writes, {} reads", a.name, b.name));
    }
    if a.reads.overlaps(&b.writes) {
        return Some(format!("read/write overlap: {} reads, {} writes", a.name, b.name));
    }
    None
}

/// 按声明算出执行计划：**同一阶段内**，写集不相交的任务同波并发，独占任务自成一波。
pub fn plan(decls: &[TaskDecl]) -> Plan {
    let mut waves: Vec<Wave> = Vec::new();

    for tier in Tier::all() {
        // 当前正在填充的波次（波次成员以 `waves[w].tasks` 为唯一真相，避免镜像状态）
        let mut current: Option<usize> = None;

        for (idx, decl) in decls.iter().enumerate() {
            if decl.tier != tier {
                continue;
            }

            if decl.exclusive {
                waves.push(Wave {
                    tier,
                    tasks: vec![idx],
                    note: "(exclusive barrier)".to_string(),
                });
                current = None;
                continue;
            }

            let blocking = current.and_then(|w| {
                waves[w].tasks.iter().find_map(|other| {
                    conflict_reason(&decls[*other], decl).map(|reason| (*other, reason))
                })
            });

            match (current, blocking) {
                (Some(w), None) => waves[w].tasks.push(idx),
                (_, blocking) => {
                    let note = match &blocking {
                        Some((other, reason)) => {
                            format!("(serialized after task {other}: {reason})")
                        }
                        None => "(disjoint scopes)".to_string(),
                    };
                    waves.push(Wave {
                        tier,
                        tasks: vec![idx],
                        note,
                    });
                    current = Some(waves.len() - 1);
                }
            }
        }
    }

    Plan { waves }
}

fn normalize(mut path: String) -> String {
    while path.ends_with('/') {
        path.pop();
    }
    path.trim_start_matches('/').replace('\\', "/")
}

fn is_under(path: &str, prefix: &str) -> bool {
    path.len() > prefix.len()
        && path.starts_with(prefix)
        && path.as_bytes()[prefix.len()] == b'/'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_matching_is_segment_aware() {
        let s = ScopeSet::prefix("assets/minecraft");
        assert!(s.contains("assets/minecraft"));
        assert!(s.contains("assets/minecraft/textures/x.png"));
        assert!(!s.contains("assets/minecraft2/x.png"), "段边界不能被前缀吞掉");
        assert!(!s.contains("assets/other/x.png"));

        let e = ScopeSet::exact("pack.mcmeta");
        assert!(e.contains("pack.mcmeta"));
        assert!(!e.contains("pack.mcmeta2"));

        assert!(ScopeSet::any().contains("anything/at/all"));
        assert!(!ScopeSet::none().contains("anything"));
        assert!(ScopeSet::none().is_empty());
        assert!(ScopeSet::prefix("").is_any(), "空前缀视为整包");
    }

    #[test]
    fn scope_overlap_is_conservative() {
        let a = ScopeSet::prefix("assets/ui");
        let b = ScopeSet::prefix("assets/textures");
        assert!(!a.overlaps(&b), "兄弟前缀不相交");

        let child = ScopeSet::prefix("assets/ui/tabs");
        assert!(a.overlaps(&child), "父子前缀相交");

        assert!(ScopeSet::any().overlaps(&b));
        assert!(!ScopeSet::none().overlaps(&b));

        let exact_inside = ScopeSet::exact("assets/ui/tabs/x.png");
        assert!(a.overlaps(&exact_inside), "精确路径落在前缀内");
    }

    #[test]
    fn disjoint_writes_share_one_wave() {
        let decls = vec![
            TaskDecl::new("tabs", Tier::Surgeon)
                .reads(ScopeSet::prefix("assets/ui"))
                .writes(ScopeSet::prefix("assets/ui/tabs")),
            TaskDecl::new("textures", Tier::Surgeon)
                .reads(ScopeSet::prefix("assets/textures"))
                .writes(ScopeSet::prefix("assets/textures")),
        ];
        let p = plan(&decls);
        assert_eq!(p.waves.len(), 1, "{}", p.summary());
        assert_eq!(p.max_parallelism(), 2);
        assert!(!p.is_serial());
    }

    #[test]
    fn write_overlap_serializes() {
        let decls = vec![
            TaskDecl::new("a", Tier::Surgeon).writes(ScopeSet::prefix("assets/ui")),
            TaskDecl::new("b", Tier::Surgeon).writes(ScopeSet::prefix("assets/ui/tabs")),
        ];
        let p = plan(&decls);
        assert_eq!(p.waves.len(), 2);
        assert!(p.is_serial());
        assert!(p.waves[1].note.contains("write/write"), "{}", p.summary());
    }

    #[test]
    fn read_write_overlap_serializes() {
        let decls = vec![
            TaskDecl::new("writer", Tier::Surgeon).writes(ScopeSet::prefix("assets/ui")),
            TaskDecl::new("reader", Tier::Surgeon).reads(ScopeSet::prefix("assets/ui/tabs")),
        ];
        let p = plan(&decls);
        assert_eq!(p.waves.len(), 2, "写者与读者不得同波：{}", p.summary());
        assert!(p.waves[1].note.contains("read/write") || p.waves[1].note.contains("write/read"));
    }

    #[test]
    fn exclusive_tasks_are_barriers() {
        let decls = vec![
            TaskDecl::new("a", Tier::Eraser).writes(ScopeSet::prefix("a")),
            TaskDecl::new("solo", Tier::Eraser)
                .exclusive(true)
                .writes(ScopeSet::prefix("b")),
            TaskDecl::new("c", Tier::Eraser).writes(ScopeSet::prefix("c")),
        ];
        let p = plan(&decls);
        assert_eq!(p.waves.len(), 3, "{}", p.summary());
        assert_eq!(p.waves[0].tasks, vec![0]);
        assert_eq!(p.waves[1].tasks, vec![1]);
        assert_eq!(p.waves[2].tasks, vec![2]);
        assert!(p.waves[1].note.contains("exclusive"));
    }

    #[test]
    fn tiers_never_share_a_wave() {
        let decls = vec![
            TaskDecl::new("arch", Tier::Architect).writes(ScopeSet::prefix("x")),
            TaskDecl::new("eraser", Tier::Eraser).writes(ScopeSet::prefix("y")),
        ];
        let p = plan(&decls);
        assert_eq!(p.waves.len(), 2);
        assert_eq!(p.waves[0].tier, Tier::Eraser, "阶段顺序必须先 Eraser 后 Architect");
        assert_eq!(p.waves[1].tier, Tier::Architect);
    }

    #[test]
    fn unmigrated_tasks_force_serialization() {
        let decls = vec![
            TaskDecl::new("legacy-a", Tier::Surgeon)
                .reads(ScopeSet::any())
                .writes(ScopeSet::any()),
            TaskDecl::new("legacy-b", Tier::Surgeon)
                .reads(ScopeSet::any())
                .writes(ScopeSet::any()),
        ];
        let p = plan(&decls);
        assert!(p.is_serial(), "未迁移任务声明整包 → 兼容期不得并行：{}", p.summary());
    }

    #[test]
    fn plan_is_deterministic() {
        let decls = vec![
            TaskDecl::new("a", Tier::Surgeon).writes(ScopeSet::prefix("a")),
            TaskDecl::new("b", Tier::Surgeon).writes(ScopeSet::prefix("b")),
            TaskDecl::new("c", Tier::Surgeon).writes(ScopeSet::prefix("a")),
        ];
        let p1 = plan(&decls);
        let p2 = plan(&decls);
        assert_eq!(p1, p2);
        assert_eq!(p1.waves.len(), 2, "c 与 a 冲突，必须往后排：{}", p1.summary());
        assert_eq!(p1.waves[0].tasks, vec![0, 1]);
        assert_eq!(p1.waves[1].tasks, vec![2]);
    }

    #[test]
    fn undeclared_writes_are_caught() {
        let decl = TaskDecl::new("tabs", Tier::Surgeon).writes(ScopeSet::prefix("assets/ui/tabs"));
        assert!(decl.check_write("assets/ui/tabs/x.png").is_ok());
        let err = decl.check_write("assets/textures/x.png").expect_err("must fail");
        assert_eq!(err.kind(), "internal");
        assert!(err.to_string().contains("undeclared"), "{err}");

        let permissive = TaskDecl::new("legacy", Tier::Surgeon).writes(ScopeSet::any());
        assert!(permissive.check_write("anything").is_ok());
    }

    #[test]
    fn scope_describe_is_readable() {
        let s = ScopeSet::prefix("assets/ui").with_exact("pack.mcmeta");
        let text = s.describe();
        assert!(text.contains("assets/ui/**"), "{text}");
        assert!(text.contains("pack.mcmeta"), "{text}");
        assert_eq!(ScopeSet::any().describe(), "any");
        assert_eq!(ScopeSet::none().describe(), "none");
    }
}
