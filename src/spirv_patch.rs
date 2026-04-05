use std::collections::HashSet;
use std::path::Path;

const SPIRV_MAGIC: u32 = 0x07230203;
const OP_CAPABILITY: u32 = 17;
const OP_EXTENSION: u32 = 10;
const OP_MEMORY_MODEL: u32 = 14;

pub const MEMORY_MODEL_GLSL450: u32 = 1;
pub const CAP_VULKAN_MEMORY_MODEL: u32 = 5345;
pub const CAP_VULKAN_MEMORY_MODEL_DEVICE_SCOPE: u32 = 5346;

/// Post-compilation SPIR-V binary patcher.
///
/// Strips declared capabilities/extensions and rewrites the memory model.
/// Use `vulkan1_2_compat()` to fix the `VulkanMemoryModel` capability that
/// newer rust-gpu emits by default, which Vulkan 1.2 without the
/// `vulkanMemoryModel` device feature will reject.
#[derive(Debug, Clone, Default)]
pub struct SpirvPatcher {
    denied_capabilities: HashSet<u32>,
    denied_extensions: HashSet<String>,
    force_memory_model: Option<u32>,
}

impl SpirvPatcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Strips `VulkanMemoryModel` and `VulkanMemoryModelDeviceScope` and forces
    /// the memory model to `GLSL450`. Safe for all shaders that do not use
    /// explicit Vulkan memory ordering semantics.
    pub fn vulkan1_2_compat() -> Self {
        Self::new()
            .deny_capability(CAP_VULKAN_MEMORY_MODEL)
            .deny_capability(CAP_VULKAN_MEMORY_MODEL_DEVICE_SCOPE)
            .force_memory_model(MEMORY_MODEL_GLSL450)
    }

    pub fn deny_capability(mut self, cap: u32) -> Self {
        self.denied_capabilities.insert(cap);
        self
    }

    pub fn deny_extension(mut self, ext: impl Into<String>) -> Self {
        self.denied_extensions.insert(ext.into());
        self
    }

    pub fn force_memory_model(mut self, model: u32) -> Self {
        self.force_memory_model = Some(model);
        self
    }

    pub(crate) fn patch_file(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let bytes = std::fs::read(path)?;
        let patched = self.patch(&bytes)?;
        std::fs::write(path, patched)?;
        Ok(())
    }

    fn patch(&self, bytes: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if bytes.len() % 4 != 0 {
            return Err("SPIR-V byte length is not a multiple of 4".into());
        }

        let words: Vec<u32> = bytes
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        if words.len() < 5 || words[0] != SPIRV_MAGIC {
            return Err("Not a valid SPIR-V binary".into());
        }

        let mut out = Vec::with_capacity(words.len());
        out.extend_from_slice(&words[..5]);

        let mut i = 5;
        while i < words.len() {
            let word_count = (words[i] >> 16) as usize;
            let opcode = words[i] & 0xFFFF;

            if word_count == 0 || i + word_count > words.len() {
                return Err(format!("Malformed SPIR-V instruction at word {i}").into());
            }

            let instr = &words[i..i + word_count];

            match opcode {
                OP_CAPABILITY if word_count >= 2 => {
                    if !self.denied_capabilities.contains(&instr[1]) {
                        out.extend_from_slice(instr);
                    }
                }
                OP_EXTENSION if word_count >= 2 => {
                    let name = packed_string(&instr[1..]);
                    if !self.denied_extensions.contains(&name) {
                        out.extend_from_slice(instr);
                    }
                }
                OP_MEMORY_MODEL => {
                    if let Some(model) = self.force_memory_model {
                        if word_count >= 3 {
                            out.push(instr[0]);
                            out.push(instr[1]);
                            out.push(model);
                            out.extend_from_slice(&instr[3..]);
                        } else {
                            out.extend_from_slice(instr);
                        }
                    } else {
                        out.extend_from_slice(instr);
                    }
                }
                _ => out.extend_from_slice(instr),
            }

            i += word_count;
        }

        Ok(out.iter().flat_map(|w| w.to_le_bytes()).collect())
    }
}

fn packed_string(words: &[u32]) -> String {
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}