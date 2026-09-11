use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, Weak};

use bevy::log::{info, warn};
use bevy::prelude::*;
use ui_toolkit::anchor::AnchorTarget;
use ui_toolkit::plugin::{UiProcessingEnabled, UiState};

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
    SetPos {
        name: String,
        x: f32,
        y: f32,
    },
    SetPosType {
        name: String,
        position_type: PositionType,
    },
    SetAnchor {
        name: String,
        target: AnchorTarget,
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
    addons: HashMap<PathBuf, LoadedAddon>,
}

#[derive(Default)]
struct PendingAddonChanges {
    paths: Mutex<Vec<PathBuf>>,
    dirty: AtomicBool,
}

#[derive(Resource, Clone, Default)]
struct AddonReloadSignal(Arc<PendingAddonChanges>);

impl AddonReloadSignal {
    fn from_receiver(receiver: Receiver<PathBuf>) -> Result<Self, String> {
        let signal = Self::default();
        let pending = Arc::downgrade(&signal.0);
        std::thread::Builder::new()
            .name("addon-reload-bridge".into())
            .spawn(move || forward_addon_changes(receiver, pending))
            .map_err(|error| format!("failed to spawn addon reload bridge: {error}"))?;
        Ok(signal)
    }

    fn is_dirty(&self) -> bool {
        self.0.dirty.load(Ordering::Acquire)
    }

    fn take_paths(&self) -> Vec<PathBuf> {
        let mut paths = self
            .0
            .paths
            .lock()
            .expect("addon change queue lock poisoned");
        let changed = std::mem::take(&mut *paths);
        // Publish/clear under the same lock so concurrent notifications cannot be lost.
        self.0.dirty.store(false, Ordering::Release);
        changed
    }
}

fn forward_addon_changes(receiver: Receiver<PathBuf>, pending: Weak<PendingAddonChanges>) {
    while let Ok(path) = receiver.recv() {
        let Some(pending) = pending.upgrade() else {
            return;
        };
        let mut paths = pending
            .paths
            .lock()
            .expect("addon change queue lock poisoned");
        paths.push(path);
        pending.dirty.store(true, Ordering::Release);
    }
    if pending.strong_count() > 0 {
        warn!("addon watcher notification channel closed");
    }
}

pub struct AddonRuntimePlugin;

impl Plugin for AddonRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_addon_runtime.run_if(ui_updates_enabled));
        app.add_systems(
            Update,
            (
                reload_changed_addons.run_if(addon_reload_pending),
                apply_loaded_addons.run_if(addons_changed),
            )
                .chain()
                .run_if(ui_updates_enabled),
        );
    }
}

fn ui_updates_enabled(enabled: Res<UiProcessingEnabled>) -> bool {
    enabled.0
}

fn init_addon_runtime(mut commands: Commands, mut ui: ResMut<UiState>) {
    let addon_dir = PathBuf::from(ADDON_DIR);
    if let Err(err) = std::fs::create_dir_all(&addon_dir) {
        warn!(
            "failed to create addon directory {}: {err}",
            addon_dir.display()
        );
    }
    match start_addon_watcher(&addon_dir).and_then(AddonReloadSignal::from_receiver) {
        Ok(signal) => {
            commands.insert_resource(signal);
        }
        Err(err) => warn!("addon watcher unavailable: {err}"),
    }
    let mut runtime = AddonRuntime {
        addon_dir,
        addons: HashMap::new(),
    };
    runtime.refresh_all(&mut ui.registry);
    commands.insert_resource(runtime);
}

fn addon_reload_pending(signal: Option<Res<AddonReloadSignal>>) -> bool {
    signal.is_some_and(|signal| signal.is_dirty())
}

fn reload_changed_addons(
    signal: Res<AddonReloadSignal>,
    mut ui: ResMut<UiState>,
    mut runtime: ResMut<AddonRuntime>,
) {
    for path in signal.take_paths() {
        runtime.reload_path(path, &mut ui.registry);
    }
}

fn addons_changed(runtime: Option<Res<AddonRuntime>>) -> bool {
    runtime.is_some_and(|runtime| runtime.is_changed())
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
