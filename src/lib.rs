//! Hot-reloading for rust-gpu SPIR-V shaders in Vulkan applications.
//!
//! This crate provides automatic recompilation and reloading of rust-gpu shaders
//! during development. When you modify shader source files, they're automatically
//! recompiled and pipelines are rebuilt without restarting your application.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use rust_gpu_hotreload::{ShaderHotReloader, ShaderOutputDir, HotReloadable};
//! use bevy::prelude::*;
//! use std::path::PathBuf;
//! use std::sync::Arc;
//! use vulkano::device::Device;
//!
//! fn setup_shader_hotreload(mut commands: Commands) {
//!     let manifest_dir = env!("CARGO_MANIFEST_DIR");
//!     let shader_crate_path = PathBuf::from(manifest_dir)
//!         .join("..")
//!         .join("shader-source");
//!
//!     match ShaderHotReloader::builder(&shader_crate_path)
//!         .target("spirv-unknown-vulkan1.3")
//!         .debounce_ms(500)
//!         .build()
//!     {
//!         Ok(reloader) => commands.insert_resource(reloader),
//!         Err(e) => {
//!             eprintln!("Failed to initialise shader hot reloader: {}", e);
//!             std::process::exit(1);
//!         }
//!     }
//!
//!     let shader_output_dir = ShaderOutputDir::from_crate_path(
//!         &shader_crate_path,
//!         None,
//!         None
//!     );
//!     commands.insert_resource(shader_output_dir);
//! }
//!
//! # struct MyRenderTask;
//! impl HotReloadable for MyRenderTask {
//!     fn recreate_pipeline(
//!         &mut self,
//!         device: Arc<Device>,
//!         shader_paths: &ShaderOutputDir,
//!     ) -> Result<(), Box<dyn std::error::Error>> {
//!         // Rebuild pipeline with new shaders
//!         Ok(())
//!     }
//!
//!     fn shader_dependencies(&self) -> Vec<&'static str> {
//!         vec!["my-vertex.spv", "my-fragment.spv"]
//!     }
//! }
//! ```

pub mod builder;
pub mod compile;
pub mod vulkano_task;
pub mod watcher;

pub use builder::ShaderHotReloaderBuilder;
pub use compile::ShaderOutputDir;
pub use vulkano_task::{HotReloadable, HotReloadableTask};
pub use watcher::ShaderHotReloader;

const DEFAULT_TARGET: &str = "spirv-unknown-vulkan1.3";
const DEFAULT_DEBOUNCE_MS: u64 = 500;
