use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;

use crate::hurray::context::HurrayContext;
use crate::hurray::error::{EngineError, EngineResult};
use crate::hurray::resolution::ResolutionTransducer;
use crate::hurray::texture::TexturePool;
use crate::{log_error, log_info};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskType {
    Parallel,
    Exclusive,
    Hybrid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum TaskTier {
    Eraser = 10,
    Architect = 20,
    Surgeon = 30,
    Closure = 40,
}

type TaskFn = Arc<dyn Fn(&HurrayContext) -> Result<(), String> + Send + Sync>;

#[derive(Clone)]
struct Task {
    name: String,
    task_type: TaskType,
    tier: TaskTier,
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

        forward.insert((1, 2), vec!["delete_blockstates_models".to_string(), "generate_tipped_arrow_images".to_string(), "fix_ui_survival".to_string(), "fix_ui_creative".to_string(), "fix_ui_sub_hand".to_string(), "generate_boat".to_string(), "generate_potion_lingering".to_string(), "generate_shulker_box_ui".to_string(), "fix_brewing_stand_ui".to_string(), "fix_clock_compass".to_string(), "overlay_icons".to_string()]);
        forward.insert((2, 3), vec!["generate_shulker_box_ui".to_string(), "delete_horse_folder".to_string(), "fix_horse_ui".to_string()]);
        forward.insert((3, 4), vec!["rename_blocks_items".to_string(), "fix_sign".to_string(), "fix_sign_entities".to_string(), "generate_furnace".to_string(), "fix_machinery_ui".to_string(), "fix_particles".to_string(), "generate_fish_bucket".to_string(), "generate_crossbow".to_string()]);
        forward.insert((4, 5), vec!["process_chest_folder".to_string(), "generate_netherite_block".to_string(), "generate_netherite_ingot".to_string(), "delete_enchanted_item_glint".to_string(), "generate_netherite_tools".to_string(), "generate_netherite_armor_models".to_string(), "generate_smithing_ui".to_string()]);
        forward.insert((5, 6), vec!["delete_font_folder".to_string()]);
        forward.insert((6, 7), vec!["generate_snow_bucket".to_string()]);
        forward.insert((7, 8), vec!["rename_mcpatcher_to_optifine".to_string()]);
        forward.insert((8, 9), vec![]);
        forward.insert((9, 12), vec!["fix_tabs".to_string(), "generate_redwood_cherry_bamboo_planks".to_string()]);
        forward.insert((12, 13), vec!["fix_smithing2_villager2_ui".to_string(), "fix_slider".to_string()]);
        forward.insert((13, 15), vec![]);
        forward.insert((15, 18), vec!["cut_gui".to_string()]);
        forward.insert((18, 22), vec![]);
        forward.insert((22, 32), vec![]);
        forward.insert((32, 34), vec!["delete_shaders_folder".to_string()]);
        forward.insert((34, 42), vec!["delete_shaders_folder".to_string()]);
        forward.insert((42, 46), vec!["fix2_horse_ui".to_string(), "fix_armor_models".to_string(), "generate_pale_planks".to_string()]);
        forward.insert((46, 55), vec![]);
        forward.insert((55, 63), vec![]);
        forward.insert((63, 64), vec![]);
        forward.insert((64, 69), vec!["generate_copper_ingot".to_string(), "generate_copper_block".to_string(), "generate_copper_tools".to_string(), "generate_copper_armor_models".to_string()]);
        forward.insert((69, 75), vec![]);
        forward.insert((75, 84), vec![]);
        forward.insert((84, 88), vec![]);
        forward.insert((84, 1000), vec![]);

        reverse.insert((1000, 84), vec![]);
        reverse.insert((88, 84), vec![]);
        reverse.insert((84, 75), vec![]);
        reverse.insert((75, 69), vec![]);
        reverse.insert((69, 64), vec![]);
        reverse.insert((64, 63), vec![]);
        reverse.insert((63, 55), vec![]);
        reverse.insert((55, 46), vec![]);
        reverse.insert((46, 42), vec!["reverse_fix_armor_models".to_string(), "reverse_fix2_horse_ui".to_string(), "reverse_generate_pale_planks".to_string()]);
        reverse.insert((42, 34), vec!["reverse_fix2_horse_ui".to_string()]);
        reverse.insert((34, 32), vec![]);
        reverse.insert((32, 22), vec![]);
        reverse.insert((22, 18), vec![]);
        reverse.insert((18, 15), vec!["reverse_cut_gui".to_string()]);
        reverse.insert((15, 13), vec![]);
        reverse.insert((13, 12), vec!["reverse_fix_smithing2_villager2_ui".to_string(), "reverse_fix_slider".to_string()]);
        reverse.insert((12, 9), vec!["reverse_generate_redwood_cherry_bamboo_planks".to_string()]);
        reverse.insert((9, 8), vec![]);
        reverse.insert((8, 7), vec!["reverse_rename_mcpatcher_to_optifine".to_string()]);
        reverse.insert((7, 6), vec![]);
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
    tasks: Vec<Task>,
    conversion_maps: ConversionMaps,
    task_registry: HashMap<String, Task>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            conversion_maps: ConversionMaps::new(),
            task_registry: HashMap::new(),
        }
    }

    pub fn register_task<F>(&mut self, name: &str, task_type: TaskType, tier: TaskTier, task: F)
    where
        F: Fn(&HurrayContext) -> Result<(), String> + Send + Sync + 'static,
    {
        let task = Task {
            name: name.to_string(),
            task_type,
            tier,
            task: Arc::new(task),
        };

        self.tasks.push(task.clone());
        self.task_registry.insert(name.to_string(), task);
    }

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
    pub fn execute(
        &mut self,
        context: &HurrayContext,
        texture_pool: &mut TexturePool,
        _resolution: &ResolutionTransducer,
    ) -> EngineResult<()> {
        self.execute_tasks(&self.tasks.clone(), context, texture_pool, None)
    }

    pub fn execute_version_conversion(
        &mut self,
        context: &HurrayContext,
        texture_pool: &mut TexturePool,
        source_version: u32,
        target_version: u32,
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
        let pack_name = context.get_data("pack_name").unwrap_or_default();
        let progress = Arc::new(ProgressTracker::new(total_tasks, pack_name));
        self.execute_tasks(&filtered_tasks, context, texture_pool, Some(progress))?;
        texture_pool.clear_unused();

        Ok(())
    }

    pub fn clear(&mut self) {
        self.tasks.clear();
        self.task_registry.clear();
    }

    fn execute_tasks(
        &self,
        tasks: &[Task],
        context: &HurrayContext,
        texture_pool: &mut TexturePool,
        progress: Option<Arc<ProgressTracker>>,
    ) -> EngineResult<()> {
        let mut eraser = Vec::new();
        let mut architect = Vec::new();
        let mut surgeon = Vec::new();
        let mut closure = Vec::new();

        for task in tasks {
            match task.tier {
                TaskTier::Eraser => eraser.push(task.clone()),
                TaskTier::Architect => architect.push(task.clone()),
                TaskTier::Surgeon => surgeon.push(task.clone()),
                TaskTier::Closure => closure.push(task.clone()),
            }
        }

        self.execute_serial_tier("Eraser", &eraser, context, progress.clone())?;
        self.execute_parallel_capable_tier("Architect", &architect, context, false, progress.clone())?;
        self.execute_parallel_capable_tier("Surgeon", &surgeon, context, true, progress.clone())?;
        self.execute_serial_tier("Closure", &closure, context, progress.clone())?;

        texture_pool.commit_all()?;
        Ok(())
    }

    fn execute_serial_tier(
        &self,
        tier_name: &'static str,
        tasks: &[Task],
        context: &HurrayContext,
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
            if let Err(reason) = (task.task)(context) {
                let wrapped = EngineError::Task {
                    task: task.name.clone(),
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
        tasks: &[Task],
        context: &HurrayContext,
        use_pool_guard: bool,
        progress: Option<Arc<ProgressTracker>>,
    ) -> EngineResult<()> {
        if tasks.is_empty() {
            return Ok(());
        }

        log_info!("tier start [{}], tasks={}", tier_name, tasks.len());

        let (parallel, serial): (Vec<Task>, Vec<Task>) = tasks
            .iter()
            .cloned()
            .partition(|task| matches!(task.task_type, TaskType::Parallel));

        let mut failures = Vec::new();

        let pool_guard = Arc::new(RwLock::new(()));

        let parallel_failures: Vec<String> = parallel
            .par_iter()
            .filter_map(|task| {
                let run = || (task.task)(context).map_err(|reason| EngineError::Task {
                    task: task.name.clone(),
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
            let task_name = task.name.clone();
            let result = if use_pool_guard {
                match pool_guard.write() {
                    Ok(_guard) => (task.task)(context),
                    Err(_) => Err(EngineError::LockPoisoned("scheduler.texture_pool_guard").to_string()),
                }
            } else {
                (task.task)(context)
            };

            if let Err(reason) = result {
                let wrapped = EngineError::Task {
                    task: task_name.clone(),
                    reason,
                }
                .to_string();
                log_error!("{}", wrapped);
                failures.push(wrapped);
            }
            if let Some(progress) = &progress {
                progress.bump(&task_name);
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

struct LiveHandle {
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
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

    fn stop_live_ticker(&self) {
        let handle = self.live_handle.lock().ok().and_then(|mut g| g.take());
        if let Some(mut h) = handle {
            h.stop.store(true, Ordering::SeqCst);
            if let Some(j) = h.join.take() {
                let _ = j.join();
            }
        }
    }

    fn spawn_live_ticker(&self, task_name: String) {
        // Stop any in-flight ticker (defensive — start_task should
        // only run between bumps, but be safe).
        self.stop_live_ticker();

        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
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
                while !stop_clone.load(Ordering::Relaxed) {
                    std::thread::sleep(std::time::Duration::from_millis(TICK_MS));
                    if stop_clone.load(Ordering::Relaxed) {
                        break;
                    }
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
