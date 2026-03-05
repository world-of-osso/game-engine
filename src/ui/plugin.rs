use bevy::prelude::*;

use crate::ui::event::EventBus;
use crate::ui::registry::FrameRegistry;
use crate::ui::wasm_host::WasmHost;

/// Central UI state, accessible as a Bevy Resource.
#[derive(Resource)]
pub struct UiState {
    pub registry: FrameRegistry,
    pub event_bus: EventBus,
    pub wasm_host: WasmHost,
    /// Currently focused frame (receives keyboard input).
    pub focused_frame: Option<u64>,
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        let state = UiState {
            registry: FrameRegistry::new(1920.0, 1080.0),
            event_bus: EventBus::new(),
            wasm_host: WasmHost::new(),
            focused_frame: None,
        };
        app.insert_resource(state);
        app.add_systems(Startup, crate::ui::render::setup_ui_camera);
        app.add_systems(Update, (
            crate::ui::render::sync_ui_quads,
            crate::ui::render::sync_ui_text,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_adds_ui_state() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(UiPlugin);
        app.update();
        assert!(app.world().get_resource::<UiState>().is_some());
    }
}
