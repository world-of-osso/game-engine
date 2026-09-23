//! Single owner of which input consumer receives keyboard input this frame.

use std::sync::LazyLock;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use game_engine::input_bindings::{InputAction, InputBindings};
use game_engine::ui::plugin::UiState;
use game_engine::ui::spellbook_runtime::SpellbookUiRuntime;

use crate::scenes::game_menu::UiModalOpen;

/// Exactly one mode is active per frame. Gameplay keybinds and panel toggles run
/// only in [`UiInputMode::World`].
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum UiInputMode {
    #[default]
    World,
    /// A UI editbox (or the spellbook search) owns keyboard focus.
    Text,
    /// The game menu / options overlay owns input.
    Modal,
    /// Something is on the cursor. Reserved: no producer exists yet.
    Drag,
}

impl UiInputMode {
    pub fn is_world(self) -> bool {
        self == Self::World
    }
}

pub struct UiInputModePlugin;

impl Plugin for UiInputModePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiInputMode>();
        app.add_systems(
            PreUpdate,
            derive_ui_input_mode.after(bevy::input::InputSystems),
        );
    }
}

pub fn derive_ui_input_mode(
    modal: Option<Res<UiModalOpen>>,
    ui: Res<UiState>,
    spellbook: Option<NonSend<SpellbookUiRuntime>>,
    mut mode: ResMut<UiInputMode>,
) {
    let next = if modal.is_some() {
        UiInputMode::Modal
    } else if editbox_has_focus(&ui) || spellbook.is_some_and(|runtime| runtime.has_focus()) {
        UiInputMode::Text
    } else {
        UiInputMode::World
    };
    mode.set_if_neq(next);
}

pub fn focused_editbox(ui: &UiState) -> Option<u64> {
    ui.focused_frame
        .or(ui.registry.focused_frame)
        .filter(|id| ui.registry.get(*id).is_some_and(|frame| frame.is_editbox()))
}

fn editbox_has_focus(ui: &UiState) -> bool {
    focused_editbox(ui).is_some()
}

static NO_KEYS: LazyLock<ButtonInput<KeyCode>> = LazyLock::new(ButtonInput::default);

/// Keyboard state that gameplay bindings may read: the real keys in World mode,
/// no keys otherwise. Mouse input stays live so mouse-look works while typing.
pub fn gameplay_keys(mode: UiInputMode, keys: &ButtonInput<KeyCode>) -> &ButtonInput<KeyCode> {
    if mode.is_world() { keys } else { &NO_KEYS }
}

/// Discrete in-world keybinds (panel toggles, targeting, action slots, mute).
#[derive(SystemParam)]
pub struct WorldKeybinds<'w> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    mouse_buttons: Res<'w, ButtonInput<MouseButton>>,
    bindings: Res<'w, InputBindings>,
    mode: Res<'w, UiInputMode>,
    reconnect: Option<Res<'w, crate::networking::ReconnectState>>,
}

impl WorldKeybinds<'_> {
    pub fn active(&self) -> bool {
        self.mode.is_world() && !self.reconnect.as_ref().is_some_and(|r| r.is_active())
    }

    pub fn just_pressed(&self, action: InputAction) -> bool {
        self.active()
            && self
                .bindings
                .is_just_pressed(action, &self.keys, &self.mouse_buttons)
    }

    /// Fixed-by-design keys (e.g. edit-mode F10) still obey the input mode.
    pub fn fixed_key_just_pressed(&self, key: KeyCode) -> bool {
        self.active() && self.keys.just_pressed(key)
    }
}

#[cfg(test)]
#[path = "../tests/unit/ui_input_mode_tests.rs"]
pub(crate) mod tests;
