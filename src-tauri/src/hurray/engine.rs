use std::path::Path;
use std::sync::Arc;

use crate::hurray::context::HurrayContext;
use crate::hurray::error::EngineResult;
use crate::hurray::resolution::ResolutionTransducer;
use crate::hurray::scheduler::{Scheduler, TaskTier, TaskType};
use crate::hurray::texture::TexturePool;

/// 2-Pyramid engine core.
pub struct HurrayEngine {
    context: Arc<HurrayContext>,
    scheduler: Scheduler,
    texture_pool: TexturePool,
    resolution_transducer: ResolutionTransducer,
}

impl HurrayEngine {
    pub fn new(temp_dir: &str) -> Self {
        let context = Arc::new(HurrayContext::new(temp_dir));

        Self {
            context,
            scheduler: Scheduler::new(),
            texture_pool: TexturePool::new(),
            resolution_transducer: ResolutionTransducer::new(),
        }
    }

    pub fn initialize(&mut self, resource_pack_path: &Path) -> EngineResult<()> {
        self.resolution_transducer.detect_resolution(resource_pack_path)?;
        self.texture_pool.initialize(self.context.clone());
        Ok(())
    }

    pub fn register_task<F>(&mut self, name: &str, task_type: TaskType, tier: TaskTier, task: F)
    where
        F: Fn(&HurrayContext) -> Result<(), String> + Send + Sync + 'static,
    {
        self.scheduler.register_task(name, task_type, tier, task);
    }

    pub fn execute(&mut self) -> EngineResult<()> {
        // 首先执行scheduler中的任务
        self.scheduler
            .execute(&self.context, &mut self.texture_pool, &self.resolution_transducer)?;
        
        // 直接执行GuiSurgeon的转换，因为它需要texture_pool和resolution参数
        crate::converters::gui_surgeon::GuiSurgeon::execute_transformation(
            &self.context,
            &mut self.texture_pool,
            &self.resolution_transducer
        ).map_err(|e| crate::hurray::error::EngineError::Task { task: "gui_surgeon".to_string(), reason: e })?;
        
        Ok(())
    }

    pub fn execute_version_conversion(
        &mut self,
        source_version: u32,
        target_version: u32,
    ) -> EngineResult<()> {
        // 执行版本转换任务
        self.scheduler.execute_version_conversion(
            &self.context,
            &mut self.texture_pool,
            source_version,
            target_version,
        )?;
        
        // 如果目标版本需要GUI裁剪，执行GuiSurgeon的转换
        // 这里简单处理，只要版本转换完成就执行GUI裁剪
        crate::converters::gui_surgeon::GuiSurgeon::execute_transformation(
            &self.context,
            &mut self.texture_pool,
            &self.resolution_transducer
        ).map_err(|e| crate::hurray::error::EngineError::Task { task: "gui_surgeon".to_string(), reason: e })?;
        
        Ok(())
    }

    pub fn get_scale_factor(&self) -> f32 {
        self.resolution_transducer.get_scale_factor()
    }

    pub fn commit(&mut self) -> EngineResult<()> {
        self.texture_pool.commit_all()
    }
}