//! L3：结构层（只读）——把「包结构」当一等公民。
//!
//! 设计依据 `astray-arom-model.md` §7 与 D18：L3 在 M0 的交付范围是**多根 + overlays 只读**，
//! 写路径留 M1。因此本模块**只回答结构问题**，不产生层、不写字节、不改变任何输入。
//!
//! 为什么存在：现状 [`crate::pack::analysis`] 已经把这些问题答对了一遍，
//! 但它建立在「自己遍历一遍目录/zip」之上（`pack_analysis.rs:165` `analyze_zip` /
//! `pack_analysis.rs:197` `analyze_dir`），与 A-ROM 的容器视图**互为第二真相**。
//! 本模块改为只吃 [`PackView`]：条目集合来自 `BasePack` + 已提交层（`view.entries()`），
//! mcmeta 正文走 L2 视图 `mcmeta_at()`（`view.rs:156`，带缓存）。于是：
//!
//! * **单一读取路径**：不再重复实现解压/遍历/路径归一化（那些已在 L0/L1 定义一次）；
//! * **可交叉对照**：同一夹具两边各跑一遍，结论必须逐项一致（本文件末尾有该用例）——
//!   这是 D18 的验收方式，也保证 L3 不会悄悄与现状语义分叉。
//!
//! 与本模块对齐的现状证据（实现时逐条核对过 `pack_analysis.rs`）：
//!
//! | 结论 | 现状位置 |
//! |---|---|
//! | 包根 = `pack.mcmeta` 所在目录；主根 = 层级最浅者 | `pack_analysis.rs:217–238` |
//! | 覆盖层的 `overrides` = 覆盖层文件里与基础层同路径者 | `pack_analysis.rs:295–308` |
//! | 折叠目录 = 路径段（非末段）形如版本号 | `pack_analysis.rs:328–340` |
//! | 版本目录名的排除规则（≥3 段 / 段值 > 99 一律不认） | `pack_analysis.rs:144–160` |
//! | `pack_format` ≥ 69 只写 min/max | `converters/version_converter.rs:363–390`，已在 [`PackMeta::wants_min_max_format`] |
//!
//! 容错契约：解析失败（非 UTF-8 / 非 JSON / 结构不对）**不 panic、不返回 `Err`**，
//! 而是记进 [`PackStructure::warnings`] 并把对应字段留空——只读分析的价值在于
//! 「脏包也能给出结论」，让调用方自己决定降级。失败原因按 `view.rs:9` 的既有规定
//! 以 `AromError::View` 表达（`mcmeta_at` 的解析失败原样透传其 `Display` 文本），
//! 因此告警文本与其它模块的错误文本同源，不会出现两套措辞。

use std::collections::{BTreeMap, BTreeSet};

use super::layer::PackView;
use super::view::PackMeta;

/// 一个包根：`pack.mcmeta`（Java）或 `manifest.json`（Bedrock）所在目录。
///
/// `path` 是**相对视图根**的目录路径（视图根本身为空串），因为一包多根时
/// 上层需要它来拼子路径（`{path}/assets/...`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackRoot {
    /// 相对视图根的目录（根目录为 `""`）。
    pub path: String,
    /// 元数据文件路径（`pack.mcmeta` / `manifest.json`）。
    pub mcmeta_path: String,
    /// Bedrock 根（`manifest.json`）而非 Java 根。
    pub bedrock: bool,
    /// `pack_format`（缺省时回落到 `min_format`，见 `view.rs:78`）。
    pub format: Option<u32>,
    /// `pack.description`（Java）；Bedrock 或缺失时为空串。
    pub description: String,
    /// 该根之下的文件数（不含目录条目；嵌套根的文件也计入其祖先根）。
    pub file_count: usize,
}

/// 官方 overlays 覆盖层（`overlays.entries[]` 的一项）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayLayer {
    /// 声明中的 `directory`（原样保留，形如 `overlay_34`）。
    pub name: String,
    /// 声明适用的格式区间；`formats` 缺失或为空时为 `None`。
    pub formats: Option<(u32, u32)>,
    /// 该目录下的文件数。
    pub file_count: usize,
    /// 其中与**基础层**（覆盖层之外）同路径的文件数 —— 即会覆盖基础层的数量。
    ///
    /// 现状用同一口径算 `LayerInfo::override_count`（`pack_analysis.rs:301–307`），
    /// 因为它决定「压平」时的信息损失。口径细节：基础层集合含各根本身的
    /// `pack.mcmeta`，但比对时排除 `.mcmeta` 结尾者（同 `pack_analysis.rs:274`）。
    pub overrides: usize,
}

/// 版本折叠目录（非标准多版本做法，如 `textures/item/1.20/…`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldedDir {
    /// 目录路径（相对视图根）。
    pub path: String,
    /// 归一化后的版本串（`1_20_4` → `1.20.4`，`mc1.21` → `1.21`）。
    pub version: String,
    /// 其下文件数。
    pub file_count: usize,
}

/// 只读结构分析结果。字段顺序与 [`PackStructure::analyze`] 的计算顺序一致。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackStructure {
    /// 全部包根（Java + Bedrock，按路径排序；顺序确定）。
    pub roots: Vec<PackRoot>,
    /// 声明覆盖层（来自**每个** Java 根；现状只看主根，多根包会漏）。
    pub overlays: Vec<OverlayLayer>,
    /// 版本折叠目录（按路径排序）。
    pub folded_dirs: Vec<FoldedDir>,
    /// 任一根的 `supported_formats`（首个可解析者；`[a,b]` / `{min_inclusive,max_inclusive}` / 单数）。
    pub supported_formats: Option<(u32, u32)>,
    /// 需要人工注意的问题（**不 panic、不返回 Err** 的代价就是这些要有人看）。
    pub warnings: Vec<String>,
}

impl PackStructure {
    /// 只读分析：条目集合来自视图，mcmeta 正文走 L2 视图（带缓存）。
    pub fn analyze(view: &PackView<'_>) -> Self {
        let mut warnings: Vec<String> = Vec::new();

        // ── 条目清单（本模块唯一的数据来源；`entries()` 已物化层效果） ──────
        let entries = match view.entries() {
            Ok(entries) => entries,
            Err(e) => {
                // 只读分析必须容错：连条目都枚举不出来时给出空结论 + 告警，而不是 Err。
                warnings.push(format!("枚举包条目失败，结构分析降级为空：{e}"));
                return Self {
                    roots: Vec::new(),
                    overlays: Vec::new(),
                    folded_dirs: Vec::new(),
                    supported_formats: None,
                    warnings,
                };
            }
        };

        // ── 1. 包根：任何目录（含根）下有 pack.mcmeta / manifest.json ───────
        // Java 与 Bedrock 的判定并集：同一目录两者都有时算两个根（现状只认 pack.mcmeta）。
        let mut root_dirs: BTreeSet<(String, bool)> = BTreeSet::new();
        for res in &entries {
            if res.is_dir {
                continue;
            }
            if res.path.ends_with(PackMeta::PATH) {
                let dir = res.path.trim_end_matches(PackMeta::PATH).trim_end_matches('/');
                root_dirs.insert((dir.to_string(), false));
            } else if res.path.ends_with(BEDROCK_MANIFEST) {
                let dir = res
                    .path
                    .trim_end_matches(BEDROCK_MANIFEST)
                    .trim_end_matches('/');
                root_dirs.insert((dir.to_string(), true));
            }
        }
        if root_dirs.is_empty() {
            warnings.push("未找到 pack.mcmeta / manifest.json，无法定位包根".to_string());
        }

        let mut roots: Vec<PackRoot> = Vec::with_capacity(root_dirs.len());
        // 声明了 supported_formats 的根（主根优先，见下）
        let mut declared_formats: Vec<(String, Option<(u32, u32)>)> = Vec::new();
        // overlays 条目按根分组：(根目录, 覆盖层目录, 声明格式)
        let mut overlay_decls: Vec<(String, String, Option<(u32, u32)>)> = Vec::new();

        for (dir, bedrock) in &root_dirs {
            let mcmeta_path = join_path(dir, if *bedrock { BEDROCK_MANIFEST } else { PackMeta::PATH });
            let mut root = PackRoot {
                path: dir.clone(),
                mcmeta_path: mcmeta_path.clone(),
                bedrock: *bedrock,
                format: None,
                description: String::new(),
                file_count: 0,
            };

            if *bedrock {
                // Bedrock 的格式不是数字，`min_engine_version` 与 pack_format 不可换算；
                // 这里只定位根，不猜格式（对齐现状：`pack_analysis.rs` 根本不看 manifest.json）。
                warnings.push(format!(
                    "发现 Bedrock 根（{mcmeta_path}）：其格式语义与 Java pack_format 不同，本分析只定位不换算"
                ));
            } else {
                match view.mcmeta_at(&mcmeta_path) {
                    Ok(meta) => {
                        root.format = meta.effective_format();
                        root.description = meta.description.clone();
                        if let Some(warn) = &meta.warn {
                            warnings.push(format!("{mcmeta_path}: {warn}"));
                        }
                        // supported_formats 由 PackMeta::raw 再解析一次：PackMeta 只暴露
                        // 「主版本」（`view.rs:89–100`），区间需要原文。用 raw 而不是重读字节，
                        // 保证与 mcmeta() 看到的是同一份内容。
                        declared_formats.push((
                            dir.clone(),
                            parse_supported_formats(&meta.raw, &mcmeta_path, &mut warnings),
                        ));
                        overlay_decls.extend(
                            parse_overlay_entries(&meta.raw, &mcmeta_path, &mut warnings)
                                .into_iter()
                                .map(|(name, formats)| (dir.clone(), name, formats)),
                        );
                    }
                    Err(e) => {
                        // 损坏/不可读的 mcmeta 只记录，不中断：其余根与覆盖层仍要能分析。
                        warnings.push(format!("无法解析 {mcmeta_path}：{e}"));
                    }
                }
            }

            roots.push(root);
        }

        // ── 2. 每个根的文件数 ────────────────────────────────────────────
        for res in &entries {
            if res.is_dir {
                continue;
            }
            for root in roots.iter_mut() {
                if is_under(&res.path, &root.path) {
                    root.file_count += 1;
                }
            }
        }

        // ── 3. overlays：基础层 = 各根之下、覆盖层之外的**文件** ──────────
        // 覆盖层目录相对其所在根解析（多根时同一个相对名可在多个根下各有一份）。
        let overlay_dirs: BTreeSet<String> = overlay_decls
            .iter()
            .map(|(root, dir, _)| join_path(root, &normalize_dir(dir)))
            .collect();

        // 基础层文件：**按根分别**取其相对路径后的集合，覆盖层比对也用自己的根。
        //
        // 为什么必须分根：现状把包根前缀剥掉后再比对（`pack_analysis.rs:252–256` 的
        // `strip_prefix(&root)` 与 `:272–276` 的 `base_files`），因此 `PackB/overlay_34`
        // 里的 `assets/a.png` 只会与 `PackB/assets/a.png` 比较。若全局只留一份「视图根相对」
        // 集合，嵌套根声明覆盖层时会把 `PackB/assets/a.png` 误判为不覆盖（或把
        // `PackC` 的同名文件误判为覆盖）——这条差异是被本文件的交叉对照用例抓出来的。
        let mut rel_overlay_dirs: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
        for (root, dir, _) in &overlay_decls {
            let rel = normalize_dir(dir);
            if rel.is_empty() {
                continue;
            }
            rel_overlay_dirs
                .entry(root.as_str())
                .or_default()
                .insert(rel);
        }

        let mut base_by_root: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
        for res in &entries {
            if res.is_dir {
                continue;
            }
            for root in &roots {
                let Some(rel) = strip_under(&res.path, &root.path) else {
                    continue;
                };
                if rel.is_empty() {
                    continue; // 目录条目已排除；这里只可能是「路径 == 根本身」
                }
                let overlays_of_root = rel_overlay_dirs.get(root.path.as_str());
                let in_overlay = overlays_of_root
                    .map(|dirs| dirs.iter().any(|d| is_under(rel, d)))
                    .unwrap_or(false);
                if in_overlay {
                    continue;
                }
                if rel.ends_with(PackMeta::PATH) || rel.ends_with(BEDROCK_MANIFEST) {
                    continue;
                }
                base_by_root
                    .entry(root.path.as_str())
                    .or_default()
                    .insert(rel.to_string());
            }
        }

        let mut overlays: Vec<OverlayLayer> = Vec::new();
        // 覆盖层 -> 其内文件（相对其声明根；用 BTreeMap/BTreeSet：顺序与去重都确定）
        let mut overlay_files: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for res in &entries {
            if res.is_dir {
                continue;
            }
            for (root, dir, _) in &overlay_decls {
                let dir_clean = normalize_dir(dir);
                let full = join_path(root, &dir_clean);
                let Some(inner) = strip_under(&res.path, &full) else {
                    continue;
                };
                if inner.is_empty() {
                    continue;
                }
                overlay_files
                    .entry(full)
                    .or_default()
                    .insert(inner.to_string());
            }
        }

        for (root, dir, formats) in &overlay_decls {
            let dir_clean = normalize_dir(dir);
            let full = join_path(root, &dir_clean);
            let empty_files: BTreeSet<String> = BTreeSet::new();
            let empty_base: BTreeSet<String> = BTreeSet::new();
            let files = overlay_files.get(&full).unwrap_or(&empty_files);
            let base_files = base_by_root.get(root.as_str()).unwrap_or(&empty_base);
            let file_count = files.len();
            let overrides = files.iter().filter(|inner| base_files.contains(*inner)).count();

            if file_count == 0 {
                warnings.push(format!(
                    "覆盖层目录不存在或为空：{full}（声明于 {root}）"
                ));
            }
            if overrides > 0 {
                warnings.push(format!(
                    "覆盖层 {full} 会覆盖基础层 {overrides} 个文件（压平将丢失基础层对应内容）"
                ));
            }
            if formats.is_none() {
                warnings.push(format!("覆盖层 {full} 未声明可解析的 formats"));
            }

            overlays.push(OverlayLayer {
                name: dir_clean,
                formats: *formats,
                file_count,
                overrides,
            });
        }

        // ── 4. supported_formats：主根优先，其次按根路径顺序 ─────────────
        // 「声明了但形状不认识」的告警由 `parse_supported_formats` 就地给出，这里不重复。
        let primary = primary_root(&roots);
        let supported_formats = primary
            .and_then(|p| {
                declared_formats
                    .iter()
                    .find(|(dir, _)| *dir == p)
                    .and_then(|(_, f)| *f)
            })
            .or_else(|| declared_formats.iter().find_map(|(_, f)| *f));
        if supported_formats.is_some() && overlays.is_empty() {
            warnings.push(
                "包声明 supported_formats 区间但只有一份内容：转换后需按目标版本改写/清除该声明"
                    .to_string(),
            );
        }

        // ── 5. 版本折叠目录 ─────────────────────────────────────────────
        // 覆盖层目录本身不参与折叠判定：`overlay_1_21` 这类名字会命中版本启发式，
        // 但它已经被 overlays 结构化表达了，重复计入只会让上层做重复决策。
        let mut folded: BTreeMap<String, (String, usize)> = BTreeMap::new();
        for res in &entries {
            if res.is_dir {
                continue;
            }
            if overlay_dirs.iter().any(|d| is_under(&res.path, d)) {
                continue;
            }
            let parts: Vec<&str> = res.path.split('/').collect();
            for (idx, seg) in parts.iter().enumerate().take(parts.len().saturating_sub(1)) {
                if let Some(version) = looks_like_version_dir(seg) {
                    let entry = folded
                        .entry(parts[..=idx].join("/"))
                        .or_insert_with(|| (version, 0));
                    entry.1 += 1;
                }
            }
        }
        let folded_dirs: Vec<FoldedDir> = folded
            .into_iter()
            .map(|(path, (version, file_count))| FoldedDir {
                path,
                version,
                file_count,
            })
            .collect();
        if folded_dirs.len() > 20 {
            warnings.push(format!(
                "疑似版本折叠目录过多（{} 个），可能是常规目录名被误判，请人工核对",
                folded_dirs.len()
            ));
        }

        if !overlays.is_empty() {
            warnings.push(
                "当前转换管线只处理基础层，覆盖层目录会原样保留（本分析仅为决策提供依据）"
                    .to_string(),
            );
        }

        Self {
            roots,
            overlays,
            folded_dirs,
            supported_formats,
            warnings,
        }
    }

    /// 主根：层级最浅的 Java 根（对齐 `pack_analysis.rs:227` 的 `root_prefix` 选择）。
    pub fn primary_root(&self) -> Option<&PackRoot> {
        java_roots(&self.roots)
            .into_iter()
            .min_by_key(|r| r.path.matches('/').count())
    }

    /// 一包多根（现状形态判定 `PackShape::MultiRootPack` 的对应物）。
    pub fn is_multi_root(&self) -> bool {
        self.roots.len() > 1
    }
}

/// 版本写路径规则（现状语义）：目标 ≥ 69 只写 `min_format`/`max_format`，否则写 `pack_format`。
///
/// **必须**经由 [`PackMeta::wants_min_max_format`]（`view.rs:84`）判断，不另写一份阈值：
/// 阈值改动在现状是单一事实（`converters/version_converter.rs:363–390`），复制一份就会分叉。
pub fn format_write_rule(target: u32) -> &'static str {
    if PackMeta::wants_min_max_format(target) {
        "min_max"
    } else {
        "pack_format"
    }
}

const BEDROCK_MANIFEST: &str = "manifest.json";

/// 归一化目录写法：去首尾斜杠与反斜杠（对齐 `layer.rs:732` 的 `normalize`）。
fn normalize_dir(dir: &str) -> String {
    dir.trim_matches('/').trim_matches('\\').replace('\\', "/")
}

/// 把相对根目录 `root` 与相对路径 `rel` 拼成完整路径（`root` 为空时即 `rel`）。
fn join_path(root: &str, rel: &str) -> String {
    if root.is_empty() {
        rel.to_string()
    } else if rel.is_empty() {
        root.to_string()
    } else {
        format!("{root}/{rel}")
    }
}

/// `path` 是否在 `dir` 之下（同路径算命中；`dir` 为空即视图根，对所有路径成立）。
fn is_under(path: &str, dir: &str) -> bool {
    if dir.is_empty() {
        return true;
    }
    path == dir
        || (path.len() > dir.len()
            && path.starts_with(dir)
            && path.as_bytes()[dir.len()] == b'/')
}

/// `path` 在 `dir` 之下时返回相对部分（同路径时返回空串）。
fn strip_under<'a>(path: &'a str, dir: &str) -> Option<&'a str> {
    if dir.is_empty() {
        return Some(path);
    }
    if path == dir {
        return Some("");
    }
    path.strip_prefix(dir).and_then(|rest| rest.strip_prefix('/'))
}

/// 候选主根：层级最浅的 **Java** 根，与 `pack_analysis.rs:227` 的选择口径一致。
fn primary_root(roots: &[PackRoot]) -> Option<String> {
    java_roots(roots)
        .into_iter()
        .min_by_key(|r| r.path.matches('/').count())
        .map(|r| r.path.clone())
}

fn java_roots(roots: &[PackRoot]) -> Vec<&PackRoot> {
    roots.iter().filter(|r| !r.bedrock).collect()
}

/// 版本样式目录名：`1.20` / `1.20.4` / `1_20_4` / `v1.21` / `mc1.21` / `overlay_1_21` /
/// `assets_1_20` / `1.21+` / `26.3` 等；命中时返回**归一化版本串**（`1_20_4` → `1.20.4`）。
///
/// 为什么在 `pack_analysis.rs:124–162` 之外另立一份而不是复用它：那份是 `converters`
/// 模块的私有函数，L3 不能反向依赖转换器（`astray-arom-model.md` 的模块边界：
/// L4 之下不得引用 converters）。这里在**同一判定**上做了两处必要的扩展：
///
/// * 返回归一化版本串（上层要拿它做目标版本匹配，原文里 `1_20_4` 与 `1.20.4` 必须等价）；
/// * 接受日历式版本 `26.3`（`major.minor`，major ≥ 20）——现状只认 `1.x`。
///   这条是**有意放宽**：放宽的收益是覆盖新式折叠目录，代价是 `21.1` 这类目录名会被
///   当成版本（现实中极少见）；`26.3` 这类在 26.3 的格式方案下确实会出现。
///
/// 反例（必须不命中）：`foo` / `item` / `textures` / `gui` / `blocks` 这类名字在
/// 「首段是 1 或 ≥20 的两位数、段数 2–3、段值 ≤ 99」的约束下天然被排除；
/// 之所以要写下阈值而不是「全数字即版本」，是因为 `1.2.3.4.5.6`（构建号）与
/// 段值 > 99（如 `2020.1`）都不是 MC 版本目录。
fn looks_like_version_dir(name: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    // 非版本形态的字符先排除（避免 `#1.20` 之类被 `parse` 悄悄接受）
    if !lower
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '+' || c == '-')
    {
        return None;
    }
    // 常见前缀剥离（与 `pack_analysis.rs:126–131` 同一组，顺序：长前缀优先）
    let core = lower
        .trim_start_matches("overlay_")
        .trim_start_matches("assets_")
        .trim_start_matches("mc")
        .trim_start_matches('v')
        .trim_end_matches('+')
        .trim_end_matches('_')
        .trim_start_matches('_');
    let dotted = core.replace('_', ".");
    let parts: Vec<&str> = dotted.split('.').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return None;
    }
    if parts
        .iter()
        .any(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    let nums: Vec<u32> = parts
        .iter()
        .map(|p| p.parse::<u32>().unwrap_or(u32::MAX))
        .collect();
    if nums.iter().any(|n| *n > 99) {
        return None;
    }
    let major = nums[0];
    let mc_style = major == 1; // 1.x / 1.x.y
    let calendar_style = major >= 20 && parts.len() == 2; // 26.3
    if !mc_style && !calendar_style {
        return None;
    }
    Some(parts.join("."))
}

/// 解析包根的 `supported_formats`：`[a,b]` / `{"min_inclusive":..,"max_inclusive":..}` / 单个数。
///
/// 解析失败不返回 `Err`：调用方是只读分析，需要的是「有没有结论 + 为什么没有」。
fn parse_supported_formats(
    raw: &str,
    path: &str,
    warnings: &mut Vec<String>,
) -> Option<(u32, u32)> {
    let value: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(e) => {
            warnings.push(format!("{path}: supported_formats 无法解析（JSON 错误）：{e}"));
            return None;
        }
    };
    let Some(declared) = value.get("pack").and_then(|p| p.get("supported_formats")) else {
        return None;
    };
    match format_range(declared) {
        Some(range) => Some(range),
        None => {
            warnings.push(format!(
                "{path}: supported_formats 形状无法识别：{declared}"
            ));
            None
        }
    }
}

/// 一个「格式区间」JSON 值的统一解析：数字 / `[a,b]` / `{min_inclusive,max_inclusive}`。
///
/// overlays 的 `formats` 与 `supported_formats` 共用它，避免两处判断分叉。
fn format_range(value: &serde_json::Value) -> Option<(u32, u32)> {
    match value {
        serde_json::Value::Number(_) => {
            let n = number_to_u32(value)?;
            Some((n, n))
        }
        serde_json::Value::Array(items) => {
            let first = items.first().and_then(number_to_u32)?;
            let last = items.last().and_then(number_to_u32).unwrap_or(first);
            if items.iter().any(|item| number_to_u32(item).is_none()) {
                return None;
            }
            Some((first.min(last), first.max(last)))
        }
        serde_json::Value::Object(map) => {
            let min = map.get("min_inclusive").and_then(number_to_u32);
            let max = map.get("max_inclusive").and_then(number_to_u32);
            match (min, max) {
                (Some(min), Some(max)) => Some((min.min(max), min.max(max))),
                (Some(min), None) => Some((min, min)),
                (None, Some(max)) => Some((max, max)),
                (None, None) => None,
            }
        }
        _ => None,
    }
}

/// `[major, minor]` / `{"major":n}` / 单个数 → 主版本号（与 `view.rs:89` 同构）。
fn number_to_u32(value: &serde_json::Value) -> Option<u32> {
    match value {
        serde_json::Value::Number(n) => n.as_u64().map(|x| x as u32),
        serde_json::Value::Array(items) => items.first().and_then(number_to_u32),
        serde_json::Value::Object(map) => map.get("major").and_then(number_to_u32),
        _ => None,
    }
}

/// 解析 `overlays.entries[]`：每项形如 `{"directory":"x","formats":[n] 或 n 或 [a,b]}`。
///
/// 结构不对（缺 `directory`）的项跳过并告警 —— 一个坏条目不该让整份分析失败。
fn parse_overlay_entries(
    raw: &str,
    path: &str,
    warnings: &mut Vec<String>,
) -> Vec<(String, Option<(u32, u32)>)> {
    let value: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(e) => {
            warnings.push(format!("{path}: overlays 无法解析（JSON 错误）：{e}"));
            return Vec::new();
        }
    };
    // overlays 可以写在顶层，也可以写在 `pack` 里（`view.rs:63` 两处都认）
    let entries = value
        .get("overlays")
        .or_else(|| value.get("pack").and_then(|p| p.get("overlays")))
        .and_then(|o| o.get("entries"));
    let Some(entries) = entries else {
        return Vec::new();
    };
    let Some(entries) = entries.as_array() else {
        warnings.push(format!("{path}: overlays.entries 不是数组，已忽略"));
        return Vec::new();
    };

    let mut out: Vec<(String, Option<(u32, u32)>)> = Vec::new();
    for (idx, entry) in entries.iter().enumerate() {
        let Some(dir) = entry.get("directory").and_then(|d| d.as_str()) else {
            warnings.push(format!("{path}: overlays.entries[{idx}] 缺少 directory 字段"));
            continue;
        };
        let dir_clean = normalize_dir(dir);
        if dir_clean.is_empty() {
            warnings.push(format!("{path}: overlays.entries[{idx}] 的 directory 为空"));
            continue;
        }
        let formats = entry.get("formats").and_then(format_range);
        if formats.is_none() && entry.get("formats").is_some() {
            warnings.push(format!(
                "{path}: 覆盖层 {dir_clean} 的 formats 形状无法识别，已按「未声明」处理"
            ));
        }
        out.push((dir_clean, formats));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arom::layer::Pack;
    use crate::arom::source::MemSource;
    use crate::pack::analysis::analyze_zip;
    use std::io::Write;
    use std::path::Path;

    /// 夹具：同一份文件清单同时喂给 `MemSource`（A-ROM 侧）与一个临时 zip（现状侧）。
    ///
    /// 交叉对照要求「同一个夹具」，所以两边必须由同一份清单构建（避免手抄差异）。
    fn pack_of(files: &[(&str, &str)]) -> Pack {
        let mem: Vec<(String, Vec<u8>)> = files
            .iter()
            .map(|(p, b)| ((*p).to_string(), b.as_bytes().to_vec()))
            .collect();
        let src = MemSource::new(mem).expect("mem source");
        Pack::from_source(Box::new(src), None).expect("pack")
    }

    fn write_zip(path: &Path, files: &[(&str, &str)]) {
        let file = std::fs::File::create(path).expect("create zip");
        let mut zip = zip::ZipWriter::new(file);
        let opts =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, body) in files {
            zip.start_file(*name, opts).expect("start_file");
            zip.write_all(body.as_bytes()).expect("write");
        }
        zip.finish().expect("finish");
    }

    /// A-ROM 侧结论 + 现状侧结论（同一清单），供交叉对照用。
    fn analyze_both(
        name: &str,
        files: &[(&str, &str)],
    ) -> (PackStructure, crate::pack::analysis::PackAnalysis, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let zip_path = dir.path().join(name);
        write_zip(&zip_path, files);

        let pack = pack_of(files);
        let structure = PackStructure::analyze(&pack.view());
        let analysis = analyze_zip(&zip_path).expect("analyze_zip");
        (structure, analysis, dir)
    }

    #[test]
    fn single_root_reads_format_and_description() {
        let pack = pack_of(&[
            (
                "pack.mcmeta",
                r#"{"pack":{"pack_format":34,"description":"hello"}}"#,
            ),
            ("assets/minecraft/textures/item/apple.png", "png"),
            ("assets/minecraft/lang/en_us.json", "{}"),
        ]);
        let s = PackStructure::analyze(&pack.view());

        assert_eq!(s.roots.len(), 1);
        let root = &s.roots[0];
        assert_eq!(root.path, "", "视图根本身的包根路径是空串");
        assert_eq!(root.mcmeta_path, "pack.mcmeta");
        assert!(!root.bedrock);
        assert_eq!(root.format, Some(34));
        assert_eq!(root.description, "hello");
        assert_eq!(root.file_count, 3, "mcmeta 自身也算该根的文件");
        assert!(!s.is_multi_root());
        assert_eq!(s.primary_root().map(|r| r.path.as_str()), Some(""));
        assert!(s.warnings.is_empty(), "干净包不该有告警：{:?}", s.warnings);
    }

    #[test]
    fn nested_roots_are_all_reported() {
        let pack = pack_of(&[
            ("PackA/pack.mcmeta", r#"{"pack":{"pack_format":15}}"#),
            ("PackA/assets/a.png", "a"),
            ("PackB/pack.mcmeta", r#"{"pack":{"pack_format":34}}"#),
            ("PackB/assets/b.png", "b"),
        ]);
        let s = PackStructure::analyze(&pack.view());

        assert_eq!(s.roots.len(), 2, "一包多根：两个目录各带 pack.mcmeta");
        assert!(s.is_multi_root());
        let paths: Vec<&str> = s.roots.iter().map(|r| r.path.as_str()).collect();
        assert_eq!(paths, vec!["PackA", "PackB"], "按路径排序，顺序确定");
        assert_eq!(s.roots[0].mcmeta_path, "PackA/pack.mcmeta");
        assert_eq!(s.roots[0].format, Some(15));
        assert_eq!(s.roots[1].format, Some(34));
        assert_eq!(s.roots[0].file_count, 2, "PackA 下的 mcmeta + assets/a.png");
        // 主根 = 层级最浅者：两者同级，取路径序第一个（与 pack_analysis.rs:227 的口径一致）
        assert_eq!(s.primary_root().map(|r| r.path.as_str()), Some("PackA"));
    }

    #[test]
    fn overlays_report_formats_and_override_counts() {
        let pack = pack_of(&[
            (
                "pack.mcmeta",
                r#"{"pack":{"pack_format":15,"description":"x"},
                    "overlays":{"entries":[
                      {"formats":[34],"directory":"overlay_34"},
                      {"formats":{"min_inclusive":46,"max_inclusive":47},"directory":"overlay_46"}]}}"#,
            ),
            ("assets/minecraft/textures/item/apple.png", "base"),
            (
                "overlay_34/assets/minecraft/textures/item/apple.png",
                "override",
            ),
            ("overlay_34/assets/minecraft/textures/item/extra.png", "new"),
            (
                "overlay_46/assets/minecraft/textures/item/apple.png",
                "override",
            ),
        ]);
        let s = PackStructure::analyze(&pack.view());

        assert_eq!(s.overlays.len(), 2);
        let o34 = &s.overlays[0];
        assert_eq!(o34.name, "overlay_34");
        assert_eq!(o34.formats, Some((34, 34)), "单元素数组退化为点区间");
        assert_eq!(o34.file_count, 2, "apple.png + extra.png");
        assert_eq!(
            o34.overrides, 1,
            "只有 apple.png 与基础层同路径（extra.png 是独有的）"
        );

        let o46 = &s.overlays[1];
        assert_eq!(o46.name, "overlay_46");
        assert_eq!(o46.formats, Some((46, 47)), "对象形式的 min/max");
        assert_eq!(o46.file_count, 1);
        assert_eq!(o46.overrides, 1);
        assert!(
            s.warnings.iter().any(|w| w.contains("覆盖基础层")),
            "{:?}",
            s.warnings
        );
    }

    #[test]
    fn overlay_already_overridden_path_does_not_count_twice() {
        // 两个覆盖层声明同一个目录：文件只应计入一次（Set 去重），override 不能翻倍。
        let pack = pack_of(&[
            (
                "pack.mcmeta",
                r#"{"pack":{"pack_format":15},
                    "overlays":{"entries":[{"directory":"overlay_34"},{"directory":"overlay_34"}]}}"#,
            ),
            ("assets/apple.png", "base"),
            ("overlay_34/assets/apple.png", "override"),
        ]);
        let s = PackStructure::analyze(&pack.view());

        assert_eq!(s.overlays.len(), 2, "声明几条就报几条（不静默去重声明）");
        for o in &s.overlays {
            assert_eq!(o.file_count, 1);
            assert_eq!(o.overrides, 1);
            assert_eq!(o.formats, None);
        }
        assert!(
            s.warnings.iter().any(|w| w.contains("未声明")),
            "{:?}",
            s.warnings
        );
    }

    #[test]
    fn folded_dirs_are_recognized_and_plain_dirs_are_not() {
        let pack = pack_of(&[
            (
                "pack.mcmeta",
                r#"{"pack":{"pack_format":34,"description":"x"}}"#,
            ),
            ("textures/item/1.20/x.png", "x"),
            ("textures/item/1.21.4/y.png", "y"),
            ("textures/item/1_20_4/z.png", "z"),
            ("textures/item/foo/w.png", "w"),
            ("assets/minecraft/textures/item/root.png", "r"),
        ]);
        let s = PackStructure::analyze(&pack.view());

        let dirs: Vec<(&str, &str, usize)> = s
            .folded_dirs
            .iter()
            .map(|f| (f.path.as_str(), f.version.as_str(), f.file_count))
            .collect();
        assert_eq!(
            dirs,
            vec![
                ("textures/item/1.20", "1.20", 1),
                ("textures/item/1.21.4", "1.21.4", 1),
                ("textures/item/1_20_4", "1.20.4", 1),
            ],
            "版本串要归一化（1_20_4 → 1.20.4），且按路径排序"
        );
        assert!(
            !s.folded_dirs.iter().any(|f| f.path.contains("foo")),
            "普通目录名不得被当作版本：{:?}",
            s.folded_dirs
        );
    }

    #[test]
    fn overlay_dirs_are_not_counted_as_folded() {
        // `overlay_1_21` 会命中版本启发式，但它已由 overlays 结构化表达，不重复计入折叠目录。
        let pack = pack_of(&[
            (
                "pack.mcmeta",
                r#"{"pack":{"pack_format":15},
                    "overlays":{"entries":[{"formats":[21],"directory":"overlay_1_21"}]}}"#,
            ),
            ("assets/apple.png", "base"),
            ("overlay_1_21/assets/apple.png", "override"),
            ("textures/item/1.20/x.png", "x"),
        ]);
        let s = PackStructure::analyze(&pack.view());

        let paths: Vec<&str> = s.folded_dirs.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, vec!["textures/item/1.20"], "只报真正的折叠目录");
        assert_eq!(s.overlays[0].overrides, 1);
    }

    #[test]
    fn bedrock_root_is_flagged() {
        let pack = pack_of(&[
            (
                "manifest.json",
                r#"{"format_version":2,"header":{"min_engine_version":[1,20,0]}}"#,
            ),
            ("textures/blocks/stone.png", "png"),
        ]);
        let s = PackStructure::analyze(&pack.view());

        assert_eq!(s.roots.len(), 1);
        assert!(s.roots[0].bedrock, "manifest.json → Bedrock 根");
        assert_eq!(s.roots[0].mcmeta_path, "manifest.json");
        assert_eq!(s.roots[0].file_count, 2);
        assert_eq!(
            s.primary_root().map(|r| r.path.as_str()),
            None,
            "Bedrock 根没有 Java 格式，不作为主根"
        );
        assert!(
            s.warnings.iter().any(|w| w.contains("Bedrock")),
            "{:?}",
            s.warnings
        );
    }

    #[test]
    fn broken_mcmeta_warns_instead_of_panicking() {
        let pack = pack_of(&[
            ("pack.mcmeta", "not json at all"),
            ("assets/a.png", "a"),
            ("nested/pack.mcmeta", r#"{"pack":{"pack_format":34}}"#),
        ]);
        let s = PackStructure::analyze(&pack.view());

        assert_eq!(s.roots.len(), 2, "损坏的 mcmeta 仍是一个根（文件在那儿）");
        assert_eq!(s.roots[0].format, None);
        assert_eq!(s.roots[0].description, "");
        assert_eq!(s.roots[1].format, Some(34), "另一个根不受影响");
        assert!(
            s.warnings.iter().any(|w| w.contains("无法解析")),
            "损坏的 mcmeta 必须进 warnings：{:?}",
            s.warnings
        );
        assert!(s.overlays.is_empty());
        assert!(s.supported_formats.is_none());
    }

    #[test]
    fn broken_overlay_entries_warn_without_failing() {
        let pack = pack_of(&[
            (
                "pack.mcmeta",
                r#"{"pack":{"pack_format":15},
                    "overlays":{"entries":[{"directory":"ok"},{"no_directory":1}]}}"#,
            ),
            ("assets/a.png", "a"),
        ]);
        let s = PackStructure::analyze(&pack.view());

        assert_eq!(s.overlays.len(), 1, "坏条目跳过，好条目保留");
        assert_eq!(s.overlays[0].name, "ok");
        assert!(
            s.warnings.iter().any(|w| w.contains("缺少 directory")),
            "{:?}",
            s.warnings
        );
        assert!(
            s.warnings.iter().any(|w| w.contains("不存在或为空")),
            "声明了但目录不存在也要告警：{:?}",
            s.warnings
        );
    }

    #[test]
    fn supported_formats_accepts_all_declared_shapes() {
        let cases = [
            (r#"[15,34]"#, Some((15, 34)), "数组区间"),
            (r#"{"min_inclusive":15,"max_inclusive":97}"#, Some((15, 97)), "对象区间"),
            (r#"34"#, Some((34, 34)), "单个数"),
        ];
        for (declared, expected, label) in cases {
            let json = format!(
                r#"{{"pack":{{"pack_format":15,"supported_formats":{declared}}}}}"#
            );
            let pack = pack_of(&[("pack.mcmeta", json.as_str()), ("assets/a.png", "a")]);
            let s = PackStructure::analyze(&pack.view());
            assert_eq!(s.supported_formats, expected, "{label}");
        }

        // 区间 + 只有一份内容 → 现状会提示需改写/清除该声明
        let pack = pack_of(&[
            (
                "pack.mcmeta",
                r#"{"pack":{"pack_format":15,"supported_formats":[15,34]}}"#,
            ),
            ("assets/a.png", "a"),
        ]);
        let s = PackStructure::analyze(&pack.view());
        assert!(
            s.warnings.iter().any(|w| w.contains("supported_formats")),
            "{:?}",
            s.warnings
        );
    }

    #[test]
    fn format_write_rule_reuses_the_view_rule() {
        assert_eq!(format_write_rule(68), "pack_format");
        assert_eq!(format_write_rule(69), "min_max", "阈值来自 PackMeta");
        assert_eq!(format_write_rule(97), "min_max");
        for target in [0, 15, 34, 68, 69, 97, u32::MAX] {
            let expected = if PackMeta::wants_min_max_format(target) {
                "min_max"
            } else {
                "pack_format"
            };
            assert_eq!(format_write_rule(target), expected, "target={target}");
        }
    }

    /// 交叉对照（D18 的验收方式）：同一夹具同时喂给 A-ROM（`PackStructure`）与现状
    /// （`analyze_zip`），根数量 / 覆盖层数量与逐层统计必须一致。
    #[test]
    fn structure_agrees_with_pack_analysis_on_the_same_fixture() {
        let fixture: &[(&str, &str)] = &[
            (
                "pack.mcmeta",
                r#"{"pack":{"pack_format":15,"description":"x","supported_formats":[15,34]},
                    "overlays":{"entries":[
                      {"formats":[34],"directory":"overlay_34"},
                      {"formats":[46],"directory":"overlay_46"}]}}"#,
            ),
            ("assets/minecraft/textures/item/apple.png", "base"),
            ("assets/minecraft/textures/item/1.20/apple.png", "folded"),
            ("overlay_34/assets/minecraft/textures/item/apple.png", "override"),
            ("overlay_34/assets/minecraft/textures/item/extra.png", "new"),
            ("overlay_46/assets/minecraft/textures/item/apple.png", "override"),
        ];
        let (mine, theirs, _dir) = analyze_both("cross.zip", fixture);

        // 根数量：现状只认 Java 根（`root_prefix` + `multi_roots`）
        assert_eq!(mine.roots.len(), 1);
        assert_eq!(
            mine.roots.len(),
            1 + theirs.multi_roots.len(),
            "根数量必须与现状一致：root_prefix={:?} multi_roots={:?}",
            theirs.root_prefix,
            theirs.multi_roots
        );
        assert_eq!(theirs.root_prefix, mine.roots[0].path);

        // 覆盖层数量与逐层统计
        assert_eq!(mine.overlays.len(), theirs.layers.len() - 1, "含基础层的现状列表");
        assert_eq!(mine.overlays.len(), 2);
        for layer in theirs.layers.iter().filter(|l| !l.directory.is_empty()) {
            let mine_layer = mine
                .overlays
                .iter()
                .find(|o| o.name == layer.directory)
                .unwrap_or_else(|| panic!("缺少覆盖层 {}", layer.directory));
            assert_eq!(mine_layer.file_count, layer.file_count, "{}", layer.directory);
            assert_eq!(
                mine_layer.overrides, layer.override_count,
                "{} 的覆盖计数",
                layer.directory
            );
            let expected_formats = match (layer.formats.first(), layer.formats.last()) {
                (Some(a), Some(b)) => Some((*a, *b)),
                _ => None,
            };
            assert_eq!(mine_layer.formats, expected_formats, "{}", layer.directory);
        }

        // 折叠目录：现状只报目录集合，这里逐项比对
        let theirs_folding: Vec<&str> = theirs
            .folding_dirs
            .iter()
            .map(|l| l.directory.as_str())
            .collect();
        let mine_folding: Vec<&str> = mine.folded_dirs.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(mine_folding, theirs_folding, "折叠目录集合必须一致");

        // supported_formats 与 pack_format
        assert_eq!(mine.supported_formats, Some((15, 34)));
        assert_eq!(theirs.supported_formats.as_deref(), Some("[15,34]"));
        assert_eq!(mine.roots[0].format, theirs.pack_format);
    }

    /// 交叉对照之二：多根 + 根下覆盖层。现状主根选择（层级最浅者）必须与
    /// `PackStructure::primary_root()` 一致，否则两边会各自算在不同根上。
    #[test]
    fn structure_agrees_with_pack_analysis_for_multi_root() {
        let fixture: &[(&str, &str)] = &[
            ("PackA/pack.mcmeta", r#"{"pack":{"pack_format":15}}"#),
            ("PackA/assets/a.png", "a"),
            (
                "PackB/pack.mcmeta",
                r#"{"pack":{"pack_format":34},
                    "overlays":{"entries":[{"formats":[34],"directory":"overlay_34"}]}}"#,
            ),
            ("PackB/assets/a.png", "base-b"),
            ("PackB/overlay_34/assets/a.png", "override-b"),
            ("PackB/overlay_34/assets/c.png", "new-b"),
        ];
        let (mine, theirs, _dir) = analyze_both("multi.zip", fixture);

        assert_eq!(mine.roots.len(), 2);
        assert_eq!(mine.roots.len(), 1 + theirs.multi_roots.len(), "根数量一致");
        assert!(
            theirs.shapes.contains(&crate::pack::analysis::PackShape::MultiRootPack)
        );

        // 主根一致：现状 PackA（层级最浅、路径序最小），L3 必须给出同一个
        assert_eq!(mine.primary_root().map(|r| r.path.as_str()), Some("PackA"));
        assert_eq!(theirs.root_prefix, "PackA");

        // PackB 的 overlays 在现状里**看不到**（它只读主根的 mcmeta）——L3 更全，
        // 这是有意的差异：多根包应当逐个根分析。这里断言 L3 确实看到了它。
        assert_eq!(mine.overlays.len(), 1);
        assert_eq!(mine.overlays[0].name, "overlay_34");
        assert_eq!(
            mine.overlays[0].file_count, 2,
            "PackA 里没有 overlay_34，只应数到 PackB 的两个文件"
        );
        assert_eq!(mine.overlays[0].overrides, 1, "PackB/assets/a.png 被覆盖");
        assert_eq!(theirs.layers.len(), 1, "现状只看主根 → 看不到任何覆盖层");
    }
}
