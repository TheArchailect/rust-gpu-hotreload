use crate::spirv_patch::SpirvPatcher;
use crate::DEFAULT_TARGET;
use bevy::prelude::Resource;
use spirv_builder::Capability;
use spirv_builder::ModuleResult;
use spirv_builder::SpirvBuilder;
use spirv_builder::SpirvMetadata;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use vulkano::device::Device;
use vulkano::shader::{ShaderModule, ShaderModuleCreateInfo};

#[derive(Resource, Clone)]
pub struct ShaderOutputDir {
    path: PathBuf,
}

impl ShaderOutputDir {
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

    pub fn shader_path(&self, shader_name: impl AsRef<str>) -> PathBuf {
        self.path.join(shader_name.as_ref())
    }

    pub fn load_shader(
        &self,
        device: Arc<Device>,
        shader_name: impl AsRef<str>,
    ) -> Result<Arc<ShaderModule>, Box<dyn std::error::Error>> {
        let path = self.shader_path(shader_name);
        load_shader_from_file(device, &path)
    }
}

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
    patcher: Option<&SpirvPatcher>,
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

    let result = builder
        .spirv_metadata(SpirvMetadata::NameVariables)
        .build()?;

    if let Some(patcher) = patcher {
        match &result.module {
            ModuleResult::SingleModule(path) => {
                patcher.patch_file(path)?;
            }
            ModuleResult::MultiModule(map) => {
                for path in map.values() {
                    patcher.patch_file(path)?;
                }
            }
        }
    }

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