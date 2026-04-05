use crate::spirv_patch::SpirvPatcher;
use crate::{DEFAULT_DEBOUNCE_MS, DEFAULT_TARGET, ShaderHotReloader};
use spirv_builder::Capability;
use std::path::{Path, PathBuf};

pub struct ShaderHotReloaderBuilder {
    shader_crate_path: PathBuf,
    target: String,
    capabilities: Vec<Capability>,
    extensions: Vec<String>,
    multimodule: bool,
    debounce_ms: u64,
    spirv_patch: Option<SpirvPatcher>,
}

impl ShaderHotReloaderBuilder {
    pub fn new(shader_crate_path: impl AsRef<Path>) -> Self {
        Self {
            shader_crate_path: shader_crate_path.as_ref().to_path_buf(),
            target: DEFAULT_TARGET.to_string(),
            capabilities: Vec::new(),
            extensions: Vec::new(),
            multimodule: false,
            debounce_ms: DEFAULT_DEBOUNCE_MS,
            spirv_patch: None,
        }
    }

    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = target.into();
        self
    }

    pub fn capability(mut self, capability: Capability) -> Self {
        self.capabilities.push(capability);
        self
    }

    pub fn extension(mut self, extension: impl Into<String>) -> Self {
        self.extensions.push(extension.into());
        self
    }

    pub fn multimodule(mut self, enabled: bool) -> Self {
        self.multimodule = enabled;
        self
    }

    pub fn debounce_ms(mut self, ms: u64) -> Self {
        self.debounce_ms = ms;
        self
    }

    /// Strips `VulkanMemoryModel` and forces `GLSL450` memory model.
    /// Required for Vulkan 1.2 targets without the `vulkanMemoryModel` device
    /// feature, including macOS via MoltenVK and all Apple Silicon GPUs.
    pub fn vulkan1_2_compat(mut self) -> Self {
        self.spirv_patch = Some(SpirvPatcher::vulkan1_2_compat());
        self
    }

    /// Applies a custom [`SpirvPatcher`] to all compiled output files.
    pub fn spirv_patch(mut self, patcher: SpirvPatcher) -> Self {
        self.spirv_patch = Some(patcher);
        self
    }

    pub fn build(self) -> Result<ShaderHotReloader, Box<dyn std::error::Error>> {
        ShaderHotReloader::new_with_config(
            &self.shader_crate_path,
            &self.target,
            self.capabilities,
            self.extensions,
            self.multimodule,
            self.debounce_ms,
            self.spirv_patch,
        )
    }
}