use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc::Receiver;

use bevy::log::{info, warn};
use bevy::prelude::*;
use ui_toolkit::anchor::AnchorPoint;
use ui_toolkit::plugin::UiState;

use crate::ui::addon_watcher::{scan_addon_dir, start_addon_watcher};

mod apply;
mod js;
#[cfg(test)]
mod tests;

const ADDON_DIR: &str = "addons";

#[derive(Debug, Clone, PartialEq)]
enum AddonOperation {
    CreateFrame {
        name: String,
        parent: Option<String>,
    },
    CreateFontString {
        name: String,
        parent: Option<String>,
        text: String,
    },
    SetSize {
        name: String,
        width: f32,
        height: f32,
    },
    SetPoint {
        name: String,
        point: AnchorPoint,
        relative_to: Option<String>,
        relative_point: AnchorPoint,
        x: f32,
        y: f32,
    },
    SetText {
        name: String,
        text: String,
    },
    Show {
        name: String,
    },
    Hide {
        name: String,
    },
    SetAlpha {
        name: String,
        alpha: f32,
    },
    SetBackgroundColor {
        name: String,
        color: [f32; 4],
    },
    SetFontColor {
        name: String,
        color: [f32; 4],
    },
}

#[derive(Debug, Clone)]
struct LoadedAddon {
    name: String,
    operations: Vec<AddonOperation>,
    owned_frames: HashSet<String>,
}

#[derive(Resource, Default)]
struct AddonRuntime {
    addon_dir: PathBuf,
    watcher: Option<Mutex<Receiver<PathBuf>>>,
    addons: HashMap<PathBuf, LoadedAddon>,
}

pub struct AddonRuntimePlugin;

impl Plugin for AddonRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_addon_runtime);
        app.add_systems(Update, (reload_changed_addons, apply_loaded_addons).chain());
    }
}

fn init_addon_runtime(mut commands: Commands, mut ui: ResMut<UiState>) {
    let addon_dir = PathBuf::from(ADDON_DIR);
    if let Err(err) = std::fs::create_dir_all(&addon_dir) {
        warn!(
            "failed to create addon directory {}: {err}",
            addon_dir.display()
        );
    }
    let watcher = start_addon_watcher(&addon_dir)
        .map(Mutex::new)
        .map_err(|err| {
            warn!("addon watcher unavailable: {err}");
            err
        });
    let mut runtime = AddonRuntime {
        addon_dir,
        watcher: watcher.ok(),
        addons: HashMap::new(),
    };
    runtime.refresh_all(&mut ui.registry);
    commands.insert_resource(runtime);
}

fn reload_changed_addons(mut ui: ResMut<UiState>, runtime: Option<ResMut<AddonRuntime>>) {
    let Some(mut runtime) = runtime else { return };
    runtime.reload_pending_changes(&mut ui.registry);
}

fn apply_loaded_addons(mut ui: ResMut<UiState>, runtime: Option<Res<AddonRuntime>>) {
    let Some(runtime) = runtime else { return };
    runtime.apply(&mut ui.registry);
}

impl AddonRuntime {
    fn refresh_all(&mut self, registry: &mut ui_toolkit::registry::FrameRegistry) {
        let Ok(paths) = scan_addon_dir(&self.addon_dir) else {
            return;
        };
        for path in paths {
            self.reload_path(path, registry);
        }
    }

    fn reload_pending_changes(&mut self, registry: &mut ui_toolkit::registry::FrameRegistry) {
        let Some(watcher) = &self.watcher else { return };
        let Ok(receiver) = watcher.lock() else {
            warn!("addon watcher lock poisoned");
            return;
        };
        let mut changed_paths = Vec::new();
        while let Ok(path) = receiver.try_recv() {
            changed_paths.push(path);
        }
        drop(receiver);
        for path in changed_paths {
            self.reload_path(path, registry);
        }
    }

    fn reload_path(&mut self, path: PathBuf, registry: &mut ui_toolkit::registry::FrameRegistry) {
        self.unload_path(&path, registry);
        if !path.exists() {
            return;
        }
        if let Some("js") = path.extension().and_then(|ext| ext.to_str()) {
            match load_js_addon(&path) {
                Ok(addon) => {
                    info!("loaded JS addon: {}", addon.name);
                    self.addons.insert(path, addon);
                }
                Err(err) => warn!("failed to load addon {}: {err}", path.display()),
            }
        }
    }

    fn unload_path(&mut self, path: &Path, registry: &mut ui_toolkit::registry::FrameRegistry) {
        let Some(addon) = self.addons.remove(path) else {
            return;
        };
        apply::remove_owned_frames(registry, &addon.owned_frames);
    }

    fn apply(&self, registry: &mut ui_toolkit::registry::FrameRegistry) {
        let mut addons = self.addons.values().collect::<Vec<_>>();
        addons.sort_by(|left, right| left.name.cmp(&right.name));
        for addon in addons {
            apply::apply_addon(addon, registry);
        }
    }
}

fn load_js_addon(path: &Path) -> Result<LoadedAddon, String> {
    let script = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let operations = js::run_js_addon_to_operations(&script)?;
    let owned_frames = collect_owned_frames(&operations);
    let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("addon")
        .to_string();
    Ok(LoadedAddon {
        name,
        operations,
        owned_frames,
    })
}

fn collect_owned_frames(operations: &[AddonOperation]) -> HashSet<String> {
    let mut owned = HashSet::new();
    for operation in operations {
        match operation {
            AddonOperation::CreateFrame { name, .. }
            | AddonOperation::CreateFontString { name, .. } => {
                owned.insert(name.clone());
            }
            _ => {}
        }
    }
    owned
}
