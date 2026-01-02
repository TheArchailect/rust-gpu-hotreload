use crate::DEFAULT_TARGET;
use bevy::prelude::Resource;
use spirv_builder::Capability;
use spirv_builder::SpirvBuilder;
use spirv_builder::{MetadataPrintout, SpirvMetadata};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use vulkano::device::Device;
use vulkano::shader::{ShaderModule, ShaderModuleCreateInfo};

/// Resource for locating and loading compiled SPIR-V shaders.
///
/// Manages the output directory where spirv-builder places compiled shaders
/// and provides utilities for loading them into Vulkan shader modules.
///
/// # Example
///
/// ```rust,no_run
/// use rust_gpu_hotreload::ShaderOutputDir;
/// use std::path::PathBuf;
/// use std::sync::Arc;
/// use vulkano::device::Device;
///
/// let shader_crate_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
///     .join("..")
///     .join("shader-source"); // your shader crate for example
///
/// let shader_output_dir = ShaderOutputDir::from_crate_path(
///     &shader_crate_path,
///     None,
///     None
/// );
///
/// // Load a shader module
/// # fn get_device() -> Arc<Device> { unimplemented!() }
/// let device = get_device();
/// let vertex_shader = shader_output_dir
///     .load_shader(device.clone(), "my-vertex.spv")
///     .expect("Failed to load vertex shader");
/// ```
#[derive(Resource, Clone)]
pub struct ShaderOutputDir {
    path: PathBuf,
}

impl ShaderOutputDir {
    /// Creates a new ShaderOutputDir from shader crate name.
    ///
    /// # Arguments
    ///
    /// * `shader_crate_name` - Name of the shader crate
    /// * `target` - Optional SPIR-V target (defaults to spirv-unknown-vulkan1.3)
    /// * `profile` - Optional build profile (defaults to release)
    pub fn new(
        shader_crate_name: impl AsRef<str>,
        target: Option<&str>,
        profile: Option<&str>,
    ) -> Self {
        let path = calculate_shader_output_dir(
            shader_crate_name.as_ref(),
            target.unwrap_or(DEFAULT_TARGET),
            profile.unwrap_or("release"),
        );
        Self { path }
    }

    /// Creates a new ShaderOutputDir from shader crate path.
    ///
    /// Extracts the crate name from the path's final component.
    ///
    /// # Arguments
    ///
    /// * `shader_crate_path` - Path to the shader crate directory
    /// * `target` - Optional SPIR-V target (defaults to spirv-unknown-vulkan1.3)
    /// * `profile` - Optional build profile (defaults to release)
    ///
    /// # Panics
    ///
    /// Panics if the path does not have a valid final component.
    pub fn from_crate_path(
        shader_crate_path: impl AsRef<Path>,
        target: Option<&str>,
        profile: Option<&str>,
    ) -> Self {
        let crate_name = shader_crate_path
            .as_ref()
            .file_name()
            .and_then(|s| s.to_str())
            .expect("Invalid shader crate path");
        Self::new(crate_name, target, profile)
    }

    // /// Returns the base output directory for compiled shaders.
    // pub fn shader_out_dir(&self) -> &Path {
    //     &self.path
    // }

    /// Constructs the full path to a specific shader file.
    ///
    /// # Arguments
    ///
    /// * `shader_name` - Name of the shader file (e.g., "main.spv")
    pub fn shader_path(&self, shader_name: impl AsRef<str>) -> PathBuf {
        self.path.join(shader_name.as_ref())
    }

    /// Loads a compiled SPIR-V shader into a Vulkan shader module.
    ///
    /// # Arguments
    ///
    /// * `device` - Vulkan device to create the shader module on
    /// * `shader_name` - Name of the shader file to load
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The shader file cannot be read
    /// - The SPIR-V binary is invalid
    /// - Vulkan shader module creation fails
    pub fn load_shader(
        &self,
        device: Arc<Device>,
        shader_name: impl AsRef<str>,
    ) -> Result<Arc<ShaderModule>, Box<dyn std::error::Error>> {
        let shader_path = self.shader_path(shader_name);
        load_shader_from_file(device, &shader_path)
    }
}

/// Calculates the output directory path for compiled SPIR-V shaders.
///
/// The path follows the structure created by spirv-builder:
/// `{workspace}/target/spirv-builder/{target}/{profile}/deps/{crate_name}.spvs`
///
/// # Arguments
///
/// * `shader_crate_name` - Name of the shader crate
/// * `target` - SPIR-V target architecture
/// * `profile` - Build profile (release or debug)
///
/// # Panics
///
/// Panics if workspace root cannot be determined from environment variables.
pub fn calculate_shader_output_dir(
    shader_crate_name: &str,
    target: &str,
    profile: &str,
) -> PathBuf {
    let workspace_root = std::env::var("CARGO_WORKSPACE_DIR")
        .ok()
        .or_else(|| {
            std::env::var("CARGO_MANIFEST_DIR").ok().and_then(|p| {
                PathBuf::from(&p)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
            })
        })
        .expect("Could not determine workspace root");

    let shader_crate_name_normalized = shader_crate_name.replace('-', "_");

    PathBuf::from(&workspace_root)
        .join("target")
        .join("spirv-builder")
        .join(target)
        .join(profile)
        .join("deps")
        .join(format!("{}.spvs", shader_crate_name_normalized))
}

pub(crate) fn compile_shaders(
    shader_crate_path: &Path,
    target: &str,
    capabilities: &[Capability],
    extensions: &[String],
    multimodule: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = SpirvBuilder::new(shader_crate_path, target);

    for capability in capabilities {
        builder = builder.capability(*capability);
    }

    for extension in extensions {
        builder = builder.extension(extension.as_str());
    }

    if multimodule {
        builder = builder.multimodule(true);
    }

    builder
        .spirv_metadata(SpirvMetadata::NameVariables)
        .print_metadata(MetadataPrintout::DependencyOnly)
        .build()?;

    Ok(())
}

fn load_shader_from_file(
    device: Arc<Device>,
    path: &Path,
) -> Result<Arc<ShaderModule>, Box<dyn std::error::Error>> {
    let shader_bytes = std::fs::read(path)?;
    let shader_words: Vec<u32> = shader_bytes
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();

    unsafe {
        Ok(ShaderModule::new(
            device,
            ShaderModuleCreateInfo::new(&shader_words),
        )?)
    }
}
