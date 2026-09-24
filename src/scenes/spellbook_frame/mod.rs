use bevy::prelude::*;
use game_engine::input_bindings::InputAction;
use game_engine::ui::game_plugin::{
    SpellbookUiSystems, register_spellbook_frame_systems, set_spellbook_open, teardown_spellbook_ui,
};
use game_engine::ui::plugin::UiState;
use game_engine::ui::spellbook_runtime::SpellbookUiRuntime;

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::ui_input_mode::WorldKeybinds;
use crate::window_manager::{WindowId, WindowManager};

pub struct SpellbookFramePlugin;

impl Plugin for SpellbookFramePlugin {
    fn build(&self, app: &mut App) {
        register_spellbook_frame_systems(app);
        app.configure_sets(
            Update,
            SpellbookUiSystems::Sync
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
        app.configure_sets(
            Update,
            SpellbookUiSystems::Input
                .run_if(
                    in_state(GameState::InWorld)
                        .and_then(crate::networking::gameplay_input_allowed),
                )
                .run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(
            Update,
            (toggle_spellbook_frame, sync_spellbook_window)
                .chain()
                .before(SpellbookUiSystems::Sync)
                .run_if(in_state(GameState::InWorld)),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_spellbook_ui);
    }
}

fn toggle_spellbook_frame(keybinds: WorldKeybinds, mut window_manager: ResMut<WindowManager>) {
    if keybinds.just_pressed(InputAction::ToggleSpellbook) {
        window_manager.toggle(WindowId::Spellbook);
    }
}

/// The spellbook runtime shows exactly what the window manager has open.
fn sync_spellbook_window(
    window_manager: Res<WindowManager>,
    mut ui: ResMut<UiState>,
    runtime: Option<NonSendMut<SpellbookUiRuntime>>,
) {
    let Some(mut runtime) = runtime else { return };
    let open = window_manager.is_open(WindowId::Spellbook);
    if runtime.is_open() != open {
        set_spellbook_open(&mut ui, &mut runtime, open);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spellbook_open_state_follows_the_window_manager() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.insert_state(GameState::InWorld);
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.init_resource::<game_engine::input_bindings::InputBindings>();
        app.init_resource::<crate::ui_input_mode::UiInputMode>();
        app.insert_resource(game_engine::ui::plugin::UiState {
            registry: game_engine::ui::registry::FrameRegistry::new(1920.0, 1080.0),
            event_bus: game_engine::ui::event::EventBus::new(),
            focused_frame: None,
        });
        app.insert_non_send(SpellbookUiRuntime::new());
        app.add_plugins(crate::window_manager::WindowManagerPlugin);
        app.add_systems(
            Update,
            (toggle_spellbook_frame, sync_spellbook_window).chain(),
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyP);
        app.update();
        assert!(
            app.world()
                .resource::<WindowManager>()
                .is_open(WindowId::Spellbook)
        );
        assert!(app.world().non_send::<SpellbookUiRuntime>().is_open());

        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release_all();
            keys.clear();
        }
        let mut windows = app.world_mut().resource_mut::<WindowManager>();
        windows.open(WindowId::Character);
        windows.open(WindowId::Friends);
        app.update();
        assert!(
            !app.world().non_send::<SpellbookUiRuntime>().is_open(),
            "the oldest of three panels (spellbook) closes"
        );
    }
}
