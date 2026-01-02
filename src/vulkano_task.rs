use crate::ShaderOutputDir;
use bevy::prelude::*;
use parking_lot::Mutex;
use std::sync::Arc;
use vulkano::device::Device;

/// Trait for render tasks that support shader hot reloading.
///
/// Implementing this trait allows a task to be automatically rebuilt
/// when its shaders are recompiled.
///
/// # Example
///
/// ```rust,no_run
/// use rust_gpu_hotreload::{HotReloadable, ShaderOutputDir};
/// use std::sync::Arc;
/// use vulkano::device::Device;
///
/// struct MyRenderTask {
///     pipeline: Arc<GraphicsPipeline>,
///     render_pass: Arc<RenderPass>,
/// }
///
/// impl MyRenderTask {
///     const VERTEX_SHADER: &'static str = "my-vertex.spv";
///     const FRAGMENT_SHADER: &'static str = "my-fragment.spv";
///
///     fn create_pipeline(
///         device: Arc<Device>,
///         render_pass: Arc<RenderPass>,
///         shader_paths: &ShaderOutputDir,
///     ) -> Arc<GraphicsPipeline> {
///         let vs = shader_paths.load_shader(device.clone(), Self::VERTEX_SHADER).unwrap();
///         let fs = shader_paths.load_shader(device.clone(), Self::FRAGMENT_SHADER).unwrap();
///
///         // Build pipeline...
///         # unimplemented!()
///     }
/// }
///
/// impl HotReloadable for MyRenderTask {
///     fn recreate_pipeline(
///         &mut self,
///         device: Arc<Device>,
///         shader_paths: &ShaderOutputDir,
///     ) -> Result<(), Box<dyn std::error::Error>> {
///         println!("Recreating pipeline");
///         self.pipeline = Self::create_pipeline(
///             device,
///             self.render_pass.clone(),
///             shader_paths
///         );
///         Ok(())
///     }
/// }
/// ```
pub trait HotReloadable {
    /// Recreates the pipeline with newly compiled shaders.
    ///
    /// This method should reload shader modules and rebuild the pipeline
    /// without waiting for device idle or destroying old resources.
    fn recreate_pipeline(
        &mut self,
        device: Arc<Device>,
        shader_paths: &ShaderOutputDir,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

/// Resource wrapper for a hot-reloadable task.
///
/// Stores the task in a way that allows the hot reload system
/// to access and rebuild it independently from the task graph execution.
#[derive(Resource)]
pub struct HotReloadableTask<T> {
    pub task: Arc<Mutex<T>>,
}

impl<T> HotReloadableTask<T> {
    pub fn new(task: T) -> Self {
        Self {
            task: Arc::new(Mutex::new(task)),
        }
    }
}

impl<T> Clone for HotReloadableTask<T> {
    fn clone(&self) -> Self {
        Self {
            task: self.task.clone(),
        }
    }
}
