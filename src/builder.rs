use crate::{DEFAULT_DEBOUNCE_MS, DEFAULT_TARGET, ShaderHotReloader};
use spirv_builder::Capability;
use std::path::{Path, PathBuf};

/// Builder for configuring a ShaderHotReloader instance.
///
/// Provides an interface for customising shader compilation settings
/// including target architecture, SPIR-V capabilities, extensions, and file watching behaviour.
///
/// # Example
///
/// ```rust,no_run
/// use rust_gpu_hotreload::ShaderHotReloader;
/// use spirv_builder::Capability;
/// use std::path::PathBuf;
///
/// let shader_crate_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
///     .join("..")
///     .join("shader-source");
///
/// let reloader = ShaderHotReloader::builder(&shader_crate_path)
///     .target("spirv-unknown-vulkan1.3")
///     .capability(Capability::RayTracingKHR)
///     .extension("SPV_KHR_ray_tracing")
///     .multimodule(true)
///     .debounce_ms(500)
///     .build()
///     .expect("Failed to initialise shader hot reloader");
/// ```
pub struct ShaderHotReloaderBuilder {
    shader_crate_path: PathBuf,
    target: String,
    capabilities: Vec<Capability>,
    extensions: Vec<String>,
    multimodule: bool,
    debounce_ms: u64,
}

impl ShaderHotReloaderBuilder {
    /// Creates a new builder with default settings.
    ///
    /// # Arguments
    ///
    /// * `shader_crate_path` - Path to the shader crate directory
    pub fn new(shader_crate_path: impl AsRef<Path>) -> Self {
        Self {
            shader_crate_path: shader_crate_path.as_ref().to_path_buf(),
            target: DEFAULT_TARGET.to_string(),
            capabilities: Vec::new(),
            extensions: Vec::new(),
            multimodule: false,
            debounce_ms: DEFAULT_DEBOUNCE_MS,
        }
    }

    /// Sets the target SPIR-V architecture.
    ///
    /// # Arguments
    ///
    /// * `target` - Target string (e.g., "spirv-unknown-vulkan1.3")
    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = target.into();
        self
    }

    /// Adds a SPIR-V capability requirement.
    ///
    /// # Arguments
    ///
    /// * `capability` - SPIR-V capability to enable
    pub fn capability(mut self, capability: Capability) -> Self {
        self.capabilities.push(capability);
        self
    }

    /// Adds a SPIR-V extension requirement.
    ///
    /// # Arguments
    ///
    /// * `extension` - Extension name (e.g., "SPV_KHR_ray_tracing")
    pub fn extension(mut self, extension: impl Into<String>) -> Self {
        self.extensions.push(extension.into());
        self
    }

    /// Enables or disables multimodule compilation.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable multimodule output
    pub fn multimodule(mut self, enabled: bool) -> Self {
        self.multimodule = enabled;
        self
    }

    /// Sets the file change debounce interval in milliseconds.
    ///
    /// This prevents rapid recompilation when multiple files change simultaneously.
    ///
    /// # Arguments
    ///
    /// * `ms` - Debounce interval in milliseconds
    pub fn debounce_ms(mut self, ms: u64) -> Self {
        self.debounce_ms = ms;
        self
    }

    /// Builds the ShaderHotReloader with the configured settings.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The shader crate path is invalid
    /// - Initial compilation fails
    /// - File watcher cannot be initialized
    pub fn build(self) -> Result<ShaderHotReloader, Box<dyn std::error::Error>> {
        ShaderHotReloader::new_with_config(
            &self.shader_crate_path,
            &self.target,
            self.capabilities,
            self.extensions,
            self.multimodule,
            self.debounce_ms,
        )
    }
}
