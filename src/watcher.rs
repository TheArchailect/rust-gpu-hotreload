use crate::ShaderHotReloaderBuilder;
use crate::compile::compile_shaders;
use crate::spirv_patch::SpirvPatcher;
use bevy::prelude::Resource;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use spirv_builder::Capability;
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Resource)]
pub struct ShaderHotReloader {
    _watcher: RecommendedWatcher,
    reload_receiver: Arc<Mutex<Receiver<()>>>,
}

impl ShaderHotReloader {
    pub fn builder(shader_crate_path: impl AsRef<Path>) -> ShaderHotReloaderBuilder {
        ShaderHotReloaderBuilder::new(shader_crate_path)
    }

    pub(crate) fn new_with_config(
        shader_crate_path: &Path,
        target: &str,
        capabilities: Vec<Capability>,
        extensions: Vec<String>,
        multimodule: bool,
        debounce_ms: u64,
        spirv_patch: Option<SpirvPatcher>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (reload_tx, reload_rx): (Sender<()>, Receiver<()>) = channel();
        let reload_tx = Arc::new(Mutex::new(reload_tx));
        let last_compile_time = Arc::new(Mutex::new(None::<Instant>));
        let last_compile_time_clone = last_compile_time.clone();

        let shader_crate_path_buf = shader_crate_path.to_path_buf();

        println!("Performing initial shader compilation...");
        compile_shaders(
            &shader_crate_path_buf,
            target,
            &capabilities,
            &extensions,
            multimodule,
            spirv_patch.as_ref(),
        )?;
        println!("Initial shader compilation complete");

        let shader_crate_path_for_closure = shader_crate_path_buf.clone();
        let target_owned = target.to_string();
        let capabilities_owned = capabilities.clone();
        let extensions_owned = extensions.clone();
        let spirv_patch_owned = spirv_patch;

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    if matches!(
                        event.kind,
                        notify::EventKind::Modify(_) | notify::EventKind::Create(_)
                    ) {
                        let is_rust_file = event
                            .paths
                            .iter()
                            .any(|p| p.extension().and_then(|e| e.to_str()) == Some("rs"));

                        if is_rust_file {
                            if let Ok(mut last_time) = last_compile_time_clone.lock() {
                                if let Some(last) = *last_time {
                                    if last.elapsed() < Duration::from_millis(debounce_ms) {
                                        return;
                                    }
                                }
                                *last_time = Some(Instant::now());
                            }

                            println!("Shader source changed, recompiling...");

                            if let Err(e) = compile_shaders(
                                &shader_crate_path_for_closure,
                                &target_owned,
                                &capabilities_owned,
                                &extensions_owned,
                                multimodule,
                                spirv_patch_owned.as_ref(),
                            ) {
                                eprintln!("Shader compilation failed: {}", e);
                            } else {
                                println!("Shaders recompiled successfully");
                                if let Ok(tx) = reload_tx.lock() {
                                    let _ = tx.send(());
                                }
                            }
                        }
                    }
                }
            },
            Config::default().with_poll_interval(Duration::from_millis(debounce_ms)),
        )?;

        watcher.watch(&shader_crate_path_buf, RecursiveMode::Recursive)?;
        println!("Shader hot reloading enabled");
        println!("Watching: {}", shader_crate_path_buf.display());

        Ok(Self {
            _watcher: watcher,
            reload_receiver: Arc::new(Mutex::new(reload_rx)),
        })
    }

    #[inline]
    pub fn check_for_reload(&self) -> bool {
        if let Ok(receiver) = self.reload_receiver.lock() {
            let mut has_reload = false;
            while receiver.try_recv().is_ok() {
                has_reload = true;
            }
            has_reload
        } else {
            false
        }
    }
}