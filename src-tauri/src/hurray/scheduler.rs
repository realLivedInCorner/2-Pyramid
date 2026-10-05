use crate::arom::Tier;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;

use crate::hurray::error::{EngineError, EngineResult};
use crate::{log_error, log_info, log_warn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskType {
    Parallel,
    Exclusive,
    Hybrid,
}

type TaskFn = Arc<dyn Fn() -> Result<(), String> + Send + Sync>;

#[derive(Clone)]
struct Task {
    name: Arc<str>,
    task_type: TaskType,
    tier: Tier,
    task: TaskFn,
}

type VersionMap = HashMap<(u32, u32), Vec<String>>;

pub struct ConversionMaps {
    pub forward: VersionMap,
    pub reverse: VersionMap,
}

impl ConversionMaps {
    pub fn new() -> Self {
        let mut forward = HashMap::new();
        let mut reverse = HashMap::new();

        // §9.130：`convert_animated_textures` 从**首位**移到这里（`fix_clock_compass` 之后）。
        //
        // 它原本排在这个步骤的第一个。这一改动是让**计划顺序本身更贴近真实数据依赖**：
        // 它是**原位改写**（给 `item/*.png.mcmeta` 补 `frametime` / `interpolate`），
        // 不产生新文件；而 `fix_clock_compass` 要读 `item/clock.png` / `item/compass.png`
        // 的原尺寸来切帧。把"改写 mcmeta"排在"消费原始贴图"之后，顺序上不再有歧义。
        //
        // **如实记录：这一改动本身不足以取消驱动的前置阶段。** §9.129 的偏差
        // （多 105 条 `textures/item/*`、少 6 条原图）实测**逐个单独前置都不够**——
        // 10 个 `EARLY_NATIVES` 一个个试过，没有任何一个能单独保持冻结（只有"Eraser 级
        // 全部 + `EARLY_NATIVES` 全部"一起前置才正确）。因此驱动里的
        // `native_placements` / `Side::Early` 是**必要机制**，不是"批次边界的权宜之计"。
        forward.insert((1, 2), vec!["delete_blockstates_models".to_string(), "generate_tipped_arrow_images".to_string(), "fix_ui_survival".to_string(), "fix_ui_creative".to_string(), "fix_ui_sub_hand".to_string(), "generate_boat".to_string(), "generate_potion_lingering".to_string(), "generate_shulker_box_ui".to_string(), "fix_brewing_stand_ui".to_string(), "fix_clock_compass".to_string(), "convert_animated_textures".to_string(), "overlay_icons".to_string()]);
        forward.insert((2, 3), vec!["generate_shulker_box_ui".to_string(), "delete_horse_folder".to_string(), "fix_horse_ui".to_string()]);
        forward.insert((3, 4), vec!["rename_blocks_items".to_string(), "fix_sign".to_string(), "fix_sign_entities".to_string(), "generate_furnace".to_string(), "fix_machinery_ui".to_string(), "fix_particles".to_string(), "generate_fish_bucket".to_string(), "generate_crossbow".to_string()]);
        forward.insert((4, 5), vec!["process_chest_folder".to_string(), "generate_netherite_block".to_string(), "generate_netherite_ingot".to_string(), "delete_enchanted_item_glint".to_string(), "generate_netherite_tools".to_string(), "generate_netherite_armor_models".to_string(), "generate_smithing_ui".to_string()]);
        forward.insert((5, 6), vec!["delete_font_folder".to_string()]);
        forward.insert((7, 8), vec!["rename_mcpatcher_to_optifine".to_string()]);
        forward.insert((8, 9), vec![]);
        forward.insert((9, 12), vec!["fix_tabs".to_string(), "generate_redwood_cherry_bamboo_planks".to_string()]);
        forward.insert((12, 13), vec!["fix_smithing2_villager2_ui".to_string(), "fix_slider".to_string()]);
        forward.insert((13, 15), vec![]);
        forward.insert((15, 18), vec!["cut_gui".to_string()]);
        forward.insert((18, 22), vec![]);
        forward.insert((22, 32), vec!["adapt_java_shaders".to_string()]);
        forward.insert((32, 34), vec!["generate_tricky_trials_breeze".to_string(), "adapt_java_shaders".to_string()]);
        forward.insert((34, 42), vec!["adapt_java_shaders".to_string()]);
        // 1.17 着色器体系边界（format 7）：6→7 既要生成雪球贴图，也要换着色器体系，
        // 两件事必须写在同一条 insert 里 —— HashMap::insert 是覆盖语义，
        // 拆成两条会让先写的那条静默失效（此前的 generate_snow_bucket 就是这样丢的）。
        forward.insert((6, 7), vec!["generate_snow_bucket".to_string(), "adapt_java_shaders".to_string()]);
        reverse.insert((7, 6), vec!["adapt_java_shaders".to_string()]);
        forward.insert((42, 46), vec!["fix2_horse_ui".to_string(), "fix_armor_models".to_string(), "generate_pale_planks".to_string(), "adapt_java_shaders".to_string()]);
        forward.insert((46, 55), vec!["adapt_java_shaders".to_string()]);
        forward.insert((55, 63), vec![]);
        forward.insert((63, 64), vec![]);
        forward.insert((64, 69), vec!["generate_copper_ingot".to_string(), "generate_copper_block".to_string(), "generate_copper_tools".to_string(), "generate_copper_armor_models".to_string()]);
        forward.insert((69, 75), vec!["adapt_java_shaders".to_string()]);
        forward.insert((75, 84), vec!["adapt_java_shaders".to_string()]);
        forward.insert((84, 88), vec!["adapt_java_shaders".to_string()]);
        forward.insert((88, 97), vec!["generate_poplar_planks".to_string(), "adapt_java_shaders".to_string()]);
        // Bedrock：最新 Java 26.3（97）↔ 1000
        forward.insert((84, 1000), vec!["bedrock_java_to_bedrock".to_string()]);
        forward.insert((88, 1000), vec!["bedrock_java_to_bedrock".to_string()]);
        forward.insert((97, 1000), vec!["bedrock_java_to_bedrock".to_string()]);
        reverse.insert((1000, 97), vec!["bedrock_bedrock_to_java".to_string()]);
        reverse.insert((1000, 88), vec!["bedrock_bedrock_to_java".to_string()]);
        reverse.insert((1000, 84), vec!["bedrock_bedrock_to_java".to_string()]);

        reverse.insert((97, 88), vec!["reverse_generate_poplar_planks".to_string(), "adapt_java_shaders".to_string()]);
        reverse.insert((88, 84), vec!["adapt_java_shaders".to_string()]);
        reverse.insert((84, 75), vec!["adapt_java_shaders".to_string()]);
        reverse.insert((75, 69), vec!["adapt_java_shaders".to_string()]);
        // (69,64) 的铜材质逆变换在下方统一登记，此处不再写空表覆盖
        reverse.insert((64, 63), vec![]);
        reverse.insert((63, 55), vec![]);
        reverse.insert((55, 46), vec!["adapt_java_shaders".to_string()]);
        reverse.insert((46, 42), vec!["reverse_fix_armor_models".to_string(), "reverse_fix2_horse_ui".to_string(), "reverse_generate_pale_planks".to_string(), "adapt_java_shaders".to_string()]);
        reverse.insert((42, 34), vec!["reverse_fix2_horse_ui".to_string(), "adapt_java_shaders".to_string()]);
        reverse.insert((34, 32), vec!["reverse_generate_tricky_trials_breeze".to_string(), "adapt_java_shaders".to_string()]);
        reverse.insert((32, 22), vec!["adapt_java_shaders".to_string()]);
        reverse.insert((22, 18), vec![]);
        reverse.insert((18, 15), vec!["reverse_cut_gui".to_string()]);
        reverse.insert((15, 13), vec![]);
        reverse.insert((13, 12), vec!["reverse_fix_smithing2_villager2_ui".to_string(), "reverse_fix_slider".to_string()]);
        reverse.insert((12, 9), vec!["reverse_generate_redwood_cherry_bamboo_planks".to_string()]);
        reverse.insert((9, 8), vec![]);
        reverse.insert((8, 7), vec!["reverse_rename_mcpatcher_to_optifine".to_string()]);
        // (7,6) 已在着色器边界处登记 adapt_java_shaders，此处不可再 insert 空表覆盖
        reverse.insert((6, 5), vec!["reverse_generate_snow_bucket".to_string()]);
        reverse.insert((69, 64), vec!["reverse_generate_copper_ingot".to_string(), "reverse_generate_copper_block".to_string(), "reverse_generate_copper_tools".to_string(), "reverse_generate_copper_armor_models".to_string()]);
        reverse.insert((5, 4), vec!["reverse_process_chest_folder".to_string(), "reverse_generate_netherite_block".to_string(), "reverse_generate_netherite_ingot".to_string(), "reverse_generate_netherite_tools".to_string(), "reverse_generate_netherite_armor_models".to_string(), "reverse_generate_smithing_ui".to_string()]);
        reverse.insert((4, 3), vec!["reverse_rename_blocks_items".to_string(), "reverse_fix_sign".to_string(), "reverse_fix_sign_entities".to_string(), "reverse_generate_furnace".to_string(), "reverse_fix_machinery_ui".to_string(), "reverse_fix_particles".to_string(), "reverse_generate_fish_bucket".to_string(), "reverse_generate_crossbow".to_string()]);
        reverse.insert((3, 2), vec!["reverse_fix_horse_ui".to_string(), "delete_horse_folder".to_string()]);
        reverse.insert((2, 1), vec!["delete_blockstates_models".to_string(), "reverse_generate_tipped_arrow_images".to_string(), "reverse_fix_ui_survival".to_string(), "reverse_fix_ui_creative".to_string(), "reverse_fix_ui_sub_hand".to_string(), "reverse_generate_boat".to_string(), "reverse_generate_potion_lingering".to_string(), "reverse_generate_shulker_box_ui".to_string(), "reverse_fix_brewing_stand_ui".to_string(), "reverse_fix_clock_compass".to_string(), "reverse_overlay_icons".to_string()]);

        Self { forward, reverse }
    }
}

pub struct Scheduler {
    conversion_maps: ConversionMaps,
    /// 已注册任务的实现表。**按名字索引**——`execute_version_conversion` 只从这里取任务。
    ///
    /// §9.133：原先还有一个 `tasks: Vec<Task>` 与它并存（`register_task` 同时 push 两边），
    /// 但那个 `Vec` **没有任何读取点**；`clear()` 是唯一同时用到两者的方法，而它也没有调用者。
    task_registry: HashMap<String, Task>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            conversion_maps: ConversionMaps::new(),
            task_registry: HashMap::new(),
        }
    }

    pub fn register_task<F>(&mut self, name: &str, task_type: TaskType, tier: Tier, task: F)
    where
        F: Fn() -> Result<(), String> + Send + Sync + 'static,
    {
        let task = Task {
            name: Arc::from(name),
            task_type,
            tier,
            task: Arc::new(task),
        };

        // §9.133：不再往 `tasks` 里 push —— 那个 `Vec` 只被写入、从未被读取
        // （`execute_version_conversion` 只查 `task_registry`）。字段本身也已删除。
        self.task_registry.insert(name.to_string(), task);
    }

    /// 已注册任务名的**排序快照**（§9.89 的注册表↔segment 一致性检查用）。
    ///
    /// 注册表与 segment 是两份独立的清单，原本没有任何交叉检查——§9.88 的
    /// `convert_animated_textures` 就是「注册了却不在任何段里」，因此永不执行。
    // §9.132：`registered_task_names` **已删除**——全库 0 个调用者（含测试）。

    pub fn calculate_path(&self, source: u32, target: u32) -> EngineResult<Vec<(u32, u32)>> {
        let maps = if target >= source {
            &self.conversion_maps.forward
        } else {
            &self.conversion_maps.reverse
        };

        let mut graph: HashMap<u32, Vec<u32>> = HashMap::new();
        for &(from, to) in maps.keys() {
            graph.entry(from).or_default().push(to);
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut parent: HashMap<u32, u32> = HashMap::new();

        queue.push_back(source);
        visited.insert(source);

        while let Some(current) = queue.pop_front() {
            if current == target {
                let mut path = Vec::new();
                let mut node = target;
                while node != source {
                    let prev = match parent.get(&node) {
                        Some(prev) => *prev,
                        None => {
                            return Err(EngineError::PathNotFound { source, target });
                        }
                    };
                    path.push((prev, node));
                    node = prev;
                }
                path.reverse();
                return Ok(path);
            }

            if let Some(neighbors) = graph.get(&current) {
                for &neighbor in neighbors {
                    if visited.insert(neighbor) {
                        parent.insert(neighbor, current);
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        Err(EngineError::PathNotFound { source, target })
    }

    pub fn get_tasks_for_path(&self, path: &[(u32, u32)]) -> Vec<String> {
        let mut ordered = Vec::new();
        let mut seen = HashSet::new();

        for &(from, to) in path {
            if let Some(tasks) = self.conversion_maps.forward.get(&(from, to)) {
                for task in tasks {
                    if seen.insert(task.clone()) {
                        ordered.push(task.clone());
                    }
                }
            }
            if let Some(tasks) = self.conversion_maps.reverse.get(&(from, to)) {
                for task in tasks {
                    if seen.insert(task.clone()) {
                        ordered.push(task.clone());
                    }
                }
            }
        }

        ordered
    }

    fn get_tasks_for_path_with_rules(&self, path: &[(u32, u32)], target_version: u32) -> Vec<String> {
        let mut ordered = Vec::new();
        let mut seen = HashSet::new();

        for &(from, to) in path {
            if let Some(tasks) = self.conversion_maps.forward.get(&(from, to)) {
                for task in tasks {
                    if from == 9 && to == 12 && target_version > 15 && task == "fix_tabs" {
                        continue;
                    }
                    if seen.insert(task.clone()) {
                        ordered.push(task.clone());
                    }
                }
            }
            if let Some(tasks) = self.conversion_maps.reverse.get(&(from, to)) {
                for task in tasks {
                    if seen.insert(task.clone()) {
                        ordered.push(task.clone());
                    }
                }
            }
        }

        ordered
    }

    pub fn execute_version_conversion(
        &mut self,
        source_version: u32,
        target_version: u32,
        pack_name: &str,
    ) -> EngineResult<()> {
        log_info!(
            "start version conversion: {} -> {}",
            source_version,
            target_version
        );

        let path = self.calculate_path(source_version, target_version)?;
        log_info!("resolved conversion path: {:?}", path);

        let task_names = self.get_tasks_for_path_with_rules(&path, target_version);
        log_info!("tasks selected: {:?}", task_names);

        let filtered_tasks: Vec<Task> = task_names
            .iter()
            .filter_map(|task_name| self.task_registry.get(task_name).cloned())
            .collect();

        let total_tasks = filtered_tasks.len();
        // §9.93（M3）：`pack_name` 改为**显式参数**，不再走 `HurrayContext::shared_data`。
        // 它只是一个**进度显示用**的标签（`ProgressTracker::new`），没有任何任务读它。
        let progress = Arc::new(ProgressTracker::new(total_tasks, pack_name.to_string()));
        // 用引用执行，避免再 clone 一整份 Task 列表
        self.execute_tasks(&filtered_tasks, Some(progress))?;

        Ok(())
    }

    // §9.133：`clear()` **已删除**——全库 0 个调用者。它同时暴露了 `tasks` 字段的问题：
    // 那个 `Vec<Task>` 只被 `register_task` push、**从未被读取**，是纯粹的重复持有。

    /// 版本对 → **该跑哪些任务**（有序名字）。
    ///
    /// 与 [`Self::execute_version_conversion`] 的选择逻辑完全一致（同一对私有方法），
    /// 只是把「选哪些」与「怎么跑」分开——M2 的混合运行驱动据此按名字逐个执行：
    /// 迁移过的任务交给 A-ROM，未迁移的在这里按名字跑。
    pub fn plan(&self, source_version: u32, target_version: u32) -> EngineResult<Vec<String>> {
        let path = self.calculate_path(source_version, target_version)?;
        Ok(self.get_tasks_for_path_with_rules(&path, target_version))
    }

    // §9.131：`Scheduler::task_tier` **已删除**。
    //
    // 它返回活注册表里的阶段，查不到时回落到 `crate::task_registry`（元数据表）。但驱动
    // **从不往自己的 scheduler 注册任务**（§9.128 起旧闭包全没了），因此那条"活注册表"
    // 分支恒不命中——**每一次调用都走在回落分支上**，等价于直接读元数据表。
    // `TaskTier` 合并进 A-ROM 的 `Tier` 之后，驱动与测试都直接调
    // `crate::task_registry::tier_of`，这条间接层就只剩"看起来有两个数据源"的误导。

    // §9.132：`Scheduler::run_named` **已删除**。
    //
    // 它是"按名字跑一批任务"的入口，曾是驱动执行旧批次的唯一途径。§9.128 送走旧闭包后
    // 它**没有任何生产调用者**——驱动按 `plan` 顺序自己派发 `Tx`，Bedrock 边任务走
    // `execute_version_conversion`。它唯一的使用者是一个只验证自身的用例，而那个用例
    // 已改写成 [`Self::execution_buckets_by_tier_not_by_plan_or_registration_order`]
    // ——去钉住本引擎真正在生产里被依赖的语义（阶段分桶）。

    fn execute_tasks(
        &self,
        tasks: &[Task],
        progress: Option<Arc<ProgressTracker>>,
    ) -> EngineResult<()> {
        // 只按 tier 分桶保存引用；Arc<TaskFn>/Arc<str> 保证并行阶段廉价克隆
        let mut eraser: Vec<&Task> = Vec::new();
        let mut architect: Vec<&Task> = Vec::new();
        let mut surgeon: Vec<&Task> = Vec::new();
        let mut closure: Vec<&Task> = Vec::new();

        for task in tasks {
            match task.tier {
                Tier::Eraser => eraser.push(task),
                Tier::Architect => architect.push(task),
                Tier::Surgeon => surgeon.push(task),
                Tier::Closure => closure.push(task),
            }
        }

        self.execute_serial_tier("Eraser", &eraser, progress.clone())?;
        self.execute_parallel_capable_tier("Architect", &architect, false, progress.clone())?;
        self.execute_parallel_capable_tier("Surgeon", &surgeon, true, progress.clone())?;
        self.execute_serial_tier("Closure", &closure, progress.clone())?;

        Ok(())
    }

    fn execute_serial_tier(
        &self,
        tier_name: &'static str,
        tasks: &[&Task],
        progress: Option<Arc<ProgressTracker>>,
    ) -> EngineResult<()> {
        if tasks.is_empty() {
            return Ok(());
        }

        log_info!("tier start [{}], tasks={}", tier_name, tasks.len());
        let mut failures = Vec::new();

        for task in tasks {
            if let Some(progress) = &progress {
                progress.start_task(&task.name);
            }
            if let Err(reason) = (task.task)() {
                let wrapped = EngineError::Task {
                    task: task.name.to_string(),
                    reason,
                }
                .to_string();
                log_error!("{}", wrapped);
                failures.push(wrapped);
            }
            if let Some(progress) = &progress {
                progress.bump(&task.name);
            }
        }

        if failures.is_empty() {
            log_info!("tier done [{}]", tier_name);
            return Ok(());
        }

        Err(EngineError::Tier {
            tier: tier_name,
            failures,
        })
    }

    fn execute_parallel_capable_tier(
        &self,
        tier_name: &'static str,
        tasks: &[&Task],
        use_pool_guard: bool,
        progress: Option<Arc<ProgressTracker>>,
    ) -> EngineResult<()> {
        if tasks.is_empty() {
            return Ok(());
        }

        log_info!("tier start [{}], tasks={}", tier_name, tasks.len());

        // Hybrid 与 Exclusive 仍串行；仅 TaskType::Parallel 并行。
        // pool_guard：并行批持有读锁，随后串行任务持写锁 —— 保证同层
        // 内「先并行、后独占」的 happens-before，而不是保护贴图池本身。
        let (parallel, serial): (Vec<&Task>, Vec<&Task>) = tasks
            .iter()
            .copied()
            .partition(|task| matches!(task.task_type, TaskType::Parallel));

        let mut failures = Vec::new();

        let pool_guard = Arc::new(RwLock::new(()));

        let parallel_failures: Vec<String> = parallel
            .par_iter()
            .filter_map(|task| {
                let started = std::time::Instant::now();
                let run = || (task.task)().map_err(|reason| EngineError::Task {
                    task: task.name.to_string(),
                    reason,
                });

                let result = if use_pool_guard {
                    match pool_guard.read() {
                        Ok(_guard) => run(),
                        Err(_) => Err(EngineError::LockPoisoned("scheduler.texture_pool_guard")),
                    }
                } else {
                    run()
                };
                record_task_time(&task.name, tier_name, started.elapsed(), true);

                if let Some(progress) = &progress {
                    progress.bump(&task.name);
                }

                result.err().map(|e| {
                    let msg = e.to_string();
                    log_error!("{}", msg);
                    msg
                })
            })
            .collect();
        failures.extend(parallel_failures);

        for task in serial {
            let task_name = task.name.as_ref();
            let started = std::time::Instant::now();
            let result = if use_pool_guard {
                match pool_guard.write() {
                    Ok(_guard) => (task.task)(),
                    Err(_) => Err(EngineError::LockPoisoned("scheduler.texture_pool_guard").to_string()),
                }
            } else {
                (task.task)()
            };
            record_task_time(task_name, tier_name, started.elapsed(), false);

            if let Err(reason) = result {
                let wrapped = EngineError::Task {
                    task: task_name.to_string(),
                    reason,
                }
                .to_string();
                log_error!("{}", wrapped);
                failures.push(wrapped);
            }
            if let Some(progress) = &progress {
                progress.bump(task_name);
            }
        }

        if failures.is_empty() {
            log_info!("tier done [{}]", tier_name);
            return Ok(());
        }

        Err(EngineError::Tier {
            tier: tier_name,
            failures,
        })
    }
}

// ── 逐任务耗时记录（性能画像） ───────────────────────────────
//
// 每次转换把每个任务的耗时段记进全局表，转换结束后由
// `process_zip_timed` 取走并输出 top-N。并行任务的时间包含线程争用，
// 因此它衡量的是"墙钟占用"，不是纯 CPU 时间——日志里会标注。

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskTiming {
    pub task: String,
    pub tier: &'static str,
    pub seconds: f32,
    /// 并行任务的时间含线程争用（墙钟占用），不是纯 CPU 时间。
    pub parallel: bool,
}

lazy_static::lazy_static! {
    static ref TASK_TIMINGS: std::sync::Mutex<Vec<TaskTiming>> =
        std::sync::Mutex::new(Vec::new());
}

fn record_task_time(task: &str, tier: &'static str, elapsed: std::time::Duration, parallel: bool) {
    if let Ok(mut list) = TASK_TIMINGS.lock() {
        list.push(TaskTiming {
            task: task.to_string(),
            tier,
            seconds: elapsed.as_secs_f32(),
            parallel,
        });
    }
}

/// 取走并清空本次转换的任务耗时记录（按耗时降序）。
pub fn take_task_timings() -> Vec<TaskTiming> {
    let mut list = match TASK_TIMINGS.lock() {
        Ok(mut l) => std::mem::take(&mut *l),
        Err(_) => Vec::new(),
    };
    list.sort_by(|a, b| b.seconds.partial_cmp(&a.seconds).unwrap_or(std::cmp::Ordering::Equal));
    list
}

struct ProgressTracker {
    total: usize,
    /// Arc-shared so the live-ticker thread can read it without
    /// taking the tracker's other locks.
    done: std::sync::Arc<AtomicUsize>,
    /// Instant the current task started executing. `None` between
    /// tasks. Used by the live ticker to compute fraction. Arc-
    /// shared for the same reason as `done`.
    current_started: std::sync::Arc<std::sync::Mutex<Option<std::time::Instant>>>,
    /// Live progress reporter: while a task is in flight, a
    /// background thread ticks every `TICK` ms and emits
    /// `Progress: ...` lines with a fractional value (e.g. 3.4/10
    /// for "task 3 is 40% done"). Without this the frontend would
    /// only see a single `Progress:` jump per task boundary, which
    /// makes the bar feel frozen for 30-module packs.
    live_handle: std::sync::Mutex<Option<LiveHandle>>,
    prefix: String,
}

/// 进度 ticker 的停止信号：`(是否已停止, 唤醒条件变量)`。
///
/// 用 `Condvar::wait_timeout` 代替 `sleep`：任务结束时立刻唤醒 ticker 线程，
/// 避免 `join()` 白等一个完整的 TICK（曾导致每个任务最多 200 ms 的隐性开销，
/// 38 个任务累计约 1.4 s——正是"任务耗时之和"与"管线耗时"之间的差额）。
type TickerStop = std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>;

struct LiveHandle {
    stop: TickerStop,
    join: Option<std::thread::JoinHandle<()>>,
}

impl ProgressTracker {
    fn new(total: usize, pack_name: String) -> Self {
        let trimmed = pack_name.trim();
        let prefix = if trimmed.is_empty() {
            String::new()
        } else {
            format!("[{}] ", trimmed)
        };
        Self {
            total: total.max(1),
            done: std::sync::Arc::new(AtomicUsize::new(0)),
            current_started: std::sync::Arc::new(std::sync::Mutex::new(None)),
            live_handle: std::sync::Mutex::new(None),
            prefix,
        }
    }

    /// Mark `task_name` as in-flight and start the live ticker.
    /// Called right *before* executing the task so the frontend
    /// sees smooth motion between bumps.
    fn start_task(&self, task_name: &str) {
        // Stash the new start time.
        if let Ok(mut g) = self.current_started.lock() {
            *g = Some(std::time::Instant::now());
        }
        self.spawn_live_ticker(task_name.to_string());
    }

    fn bump(&self, task_name: &str) {
        // Stop the live ticker for the just-finished task and emit a
        // final integer Progress line.
        self.stop_live_ticker();
        let current = self.done.fetch_add(1, Ordering::SeqCst) + 1;
        // Log every 50th module plus the final one. Repacking alone can
        // emit 1000+ lines in ~1.5s (≈800 logs/s); that flood wastes
        // CPU, bloats the log file and, combined with a parallel batch,
        // starves the WebView2 renderer process of the main window
        // (dead buttons / frozen animations during conversion). The
        // live ticker above still gives smooth per-200ms feedback, so
        // the file stays useful for debugging without the flood.
        if current % 50 == 0 || current == self.total {
            let percent = (current * 100) / self.total;
            log_info!(
                "{}Progress: {}/{} ({}%) - {}",
                self.prefix,
                current,
                self.total,
                percent,
                task_name
            );
        }
    }

    fn stop_live_ticker(&self) {
        let handle = self.live_handle.lock().ok().and_then(|mut g| g.take());
        if let Some(mut h) = handle {
            // 置位 + 唤醒：ticker 若正在 wait_timeout 会立即返回，join 不再等待
            {
                let (lock, cvar) = &*h.stop;
                if let Ok(mut stopped) = lock.lock() {
                    *stopped = true;
                    cvar.notify_all();
                }
            }
            if let Some(j) = h.join.take() {
                let _ = j.join();
            }
        }
    }

    fn spawn_live_ticker(&self, task_name: String) {
        // Stop any in-flight ticker (defensive — start_task should
        // only run between bumps, but be safe).
        self.stop_live_ticker();

        let stop: TickerStop =
            std::sync::Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
        let done = std::sync::Arc::clone(&self.done);
        let started = std::sync::Arc::clone(&self.current_started);
        let total = self.total;
        let prefix = self.prefix.clone();
        let stop_clone = stop.clone();

        // We don't know each task's true duration, so we use a
        // conservative 1500 ms estimate. The fractional component
        // is clamped to [0, 0.95] so we never *reach* the next task
        // boundary — that's the integer bump's job. Wrong estimates
        // only affect the bar's slope, not its final position.
        const TICK_MS: u64 = 200;
        const ESTIMATED_TASK_MS: u128 = 1500;

        let join = std::thread::Builder::new()
            .name("progress-ticker".into())
            .spawn(move || {
                let (lock, cvar) = &*stop_clone;
                loop {
                    // 可中断等待：stop 时 Condvar 会立刻唤醒，不再睡满 TICK。
                    // 先查谓词再等待，避免"通知发生在未等待期间"造成的丢唤醒
                    // （否则又会白等一个完整 TICK）。
                    let guard = match lock.lock() {
                        Ok(g) => g,
                        Err(_) => break,
                    };
                    if *guard {
                        break;
                    }
                    let (guard, _timeout) = match cvar
                        .wait_timeout(guard, std::time::Duration::from_millis(TICK_MS))
                    {
                        Ok(pair) => pair,
                        Err(_) => break,
                    };
                    if *guard {
                        break;
                    }
                    drop(guard);

                    let done_now = done.load(Ordering::Relaxed);
                    let elapsed_ms = started
                        .lock()
                        .ok()
                        .and_then(|g| g.map(|i| i.elapsed().as_millis()))
                        .unwrap_or(0);
                    let within = (elapsed_ms as f64 / ESTIMATED_TASK_MS as f64).min(0.95);
                    let fractional = done_now as f64 + within;
                    let percent = ((fractional / total as f64) * 100.0) as u32;
                    log_info!(
                        "{}Progress: {:.1}/{} ({}%) - {}",
                        prefix,
                        fractional,
                        total,
                        percent,
                        task_name
                    );
                }
            })
            .expect("failed to spawn progress ticker thread");

        if let Ok(mut g) = self.live_handle.lock() {
            *g = Some(LiveHandle {
                stop,
                join: Some(join),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `plan()` 只暴露「选哪些任务」，不得改变选择结果：名字非空、无重复、正反向不同。
    #[test]
    fn plan_exposes_the_same_selection_as_execution() {
        let scheduler = Scheduler::new();

        let forward = scheduler.plan(1, 97).expect("plan 1 -> 97");
        assert!(!forward.is_empty(), "1 → 97 应当有任务");
        let mut deduped = forward.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(deduped.len(), forward.len(), "计划里不应有重复名字：{forward:?}");

        let reverse = scheduler.plan(97, 1).expect("plan 97 -> 1");
        assert!(!reverse.is_empty(), "97 → 1 应当有任务");
        assert_ne!(forward, reverse, "正向与反向的计划不应相同");
    }

    /// **执行引擎的正题**（§9.132）：`execute_version_conversion` 必须按**阶段分桶**执行，
    /// 而不是按计划顺序、也不是按注册顺序。
    ///
    /// 这条断言是 `Scheduler` 仍在生产的**唯一理由**：驱动（`native_run`）自己按计划顺序
    /// 逐任务派发 `Tx`，而 Bedrock 边任务（j2b/b2j）仍走本引擎。因此"分桶语义"必须被钉住——
    /// 它是本引擎与驱动**不同**的地方，也最容易被无声改掉。
    ///
    /// 取 `(1,97)` 计划里阶段不同、且**计划顺序与阶段顺序相反**的三项：
    /// `generate_tipped_arrow_images`（Architect，计划第 2 位）、
    /// `fix_ui_survival`（Surgeon，第 3 位）、`convert_animated_textures`（Eraser，第 11 位）。
    /// 先断言计划顺序确实是 Architect → Surgeon → Eraser，再断言**执行**顺序是
    /// Eraser → Architect → Surgeon —— 两者相反才说明分桶真的发生了。
    ///
    /// 用例里按**计划逆序**注册（Surgeon → Architect → Eraser），于是"注册顺序"这第三种
    /// 可能也被排除。
    #[test]
    fn execution_buckets_by_tier_not_by_plan_or_registration_order() {
        let names = [
            "convert_animated_textures",    // Eraser
            "generate_tipped_arrow_images", // Architect
            "fix_ui_survival",              // Surgeon
        ];

        // ① 先钉住计划顺序：必须是 Architect → Surgeon → Eraser（与阶段顺序相反）
        let probe = Scheduler::new();
        let plan = probe.plan(1, 97).expect("plan 1 -> 97");
        let in_plan: Vec<&str> = plan
            .iter()
            .map(String::as_str)
            .filter(|n| names.contains(n))
            .collect();
        assert_eq!(
            in_plan,
            vec![
                "generate_tipped_arrow_images",
                "fix_ui_survival",
                "convert_animated_textures"
            ],
            "计划顺序变了——本用例的判据依赖它与阶段顺序相反"
        );

        let dir = tempfile::tempdir().expect("tempdir");
        let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));

        let mut scheduler = Scheduler::new();
        // ② 按**计划逆序**注册（Surgeon → Architect → Eraser）
        for (name, tier) in [
            ("fix_ui_survival", Tier::Surgeon),
            ("generate_tipped_arrow_images", Tier::Architect),
            ("convert_animated_textures", Tier::Eraser),
        ] {
            let log = std::sync::Arc::clone(&log);
            let marker = dir.path().join(format!("{name}.done"));
            scheduler.register_task(name, TaskType::Parallel, tier, move || {
                std::fs::write(&marker, b"done").map_err(|e| e.to_string())?;
                log.lock()
                    .map_err(|_| "log poisoned".to_string())?
                    .push(name.to_string());
                Ok(())
            });
        }

        scheduler
            .execute_version_conversion(1, 97, "bucketing-test")
            .expect("execute_version_conversion");

        // ③ 执行顺序必须是阶段分桶，而不是计划顺序或注册顺序
        let executed = log.lock().expect("log").clone();
        assert_eq!(
            executed,
            vec![
                "convert_animated_textures",
                "generate_tipped_arrow_images",
                "fix_ui_survival"
            ],
            "执行顺序必须是阶段分桶（Eraser → Architect → Surgeon）；实际：{executed:?}"
        );
        for name in names {
            assert!(
                dir.path().join(format!("{name}.done")).exists(),
                "`{name}` 在计划里，必须被执行"
            );
        }
    }

    fn has(map: &VersionMap, key: (u32, u32), task: &str) -> bool {
        map.get(&key)
            .map(|v| v.iter().any(|t| t == task))
            .unwrap_or(false)
    }

    /// (6,7) 升版必须同时登记雪球贴图生成与着色器体系适配。
    /// 回归点：`generate_snow_bucket` 曾被后一条 `forward.insert((6,7), ..)`
    /// 静默覆盖，导致 6→7 转换从不生成雪球贴图。
    #[test]
    fn forward_6_to_7_keeps_both_tasks() {
        let maps = ConversionMaps::new();
        assert!(
            has(&maps.forward, (6, 7), "generate_snow_bucket"),
            "(6,7) 丢失 generate_snow_bucket：HashMap::insert 覆盖语义导致任务被吞"
        );
        assert!(
            has(&maps.forward, (6, 7), "adapt_java_shaders"),
            "(6,7) 丢失 adapt_java_shaders"
        );
    }

    /// (7,6) 降版必须保留着色器体系适配（曾被一条空表覆盖成空）。
    #[test]
    fn reverse_7_to_6_keeps_shader_task() {
        let maps = ConversionMaps::new();
        assert!(
            has(&maps.reverse, (7, 6), "adapt_java_shaders"),
            "(7,6) 丢失 adapt_java_shaders：空表 insert 覆盖了有效登记"
        );
    }

    /// (69,64) 逆变换应保留铜材质逆生成（曾被先登记的空表覆盖）。
    #[test]
    fn reverse_69_to_64_keeps_copper_tasks() {
        let maps = ConversionMaps::new();
        for task in [
            "reverse_generate_copper_ingot",
            "reverse_generate_copper_block",
            "reverse_generate_copper_tools",
            "reverse_generate_copper_armor_models",
        ] {
            assert!(
                has(&maps.reverse, (69, 64), task),
                "(69,64) 丢失 {}",
                task
            );
        }
    }


    /// 关键跨版本区间必须存在登记（空表也算登记，用于“只改 pack_format”的直通段）。
    #[test]
    fn core_segments_are_registered() {
        let maps = ConversionMaps::new();
        for key in [(1, 2), (5, 6), (6, 7), (7, 8), (88, 97), (97, 1000)] {
            assert!(maps.forward.contains_key(&key), "forward 缺少区间 {:?}", key);
        }
        for key in [(7, 6), (6, 5), (97, 88), (1000, 97)] {
            assert!(maps.reverse.contains_key(&key), "reverse 缺少区间 {:?}", key);
        }
    }
}
