//! `--screen selectiondebug`: the original selection debug screen
//! (`src/scenes/selection_debug/mod.rs`) — five selection candidates, the selected
//! one's detail, pinned mode and the last action. Needs no server.
//!
//! Controls: click a row selects it; Up/Left and Down/Right cycle; Enter or Space
//! toggles pinned mode; Escape (or Back) returns to Login.

use game_engine_ui_model::selection_debug_component::{
    SelectionDebugAction, SelectionDebugEntry, SelectionDebugState, selection_debug_screen,
};
use godot::{
    classes::{INode, InputEvent, InputEventKey},
    global::Key,
    prelude::*,
};

use crate::{GameClient, ui::RegistryUi};

/// The original screen's candidates: label, subtitle, detail.
const ENTRIES: [(&str, &str, &str); 5] = [
    (
        "Local Player",
        "Self-target and keyboard recovery",
        "Use this candidate when debugging self-target, default focus recovery, or cases where there is no hovered remote entity.",
    ),
    (
        "Quest NPC",
        "Friendly unit with hover and click affordances",
        "This row stands in for a talkable NPC so you can compare hover, click, and retained selection styling without needing a full scene.",
    ),
    (
        "Enemy Creature",
        "Hostile unit with target-ring expectations",
        "Use this for hostile-target visuals, tab-target ordering, and any selection ring color or visibility regressions.",
    ),
    (
        "Corpse / Invalid",
        "Non-interactive or stale target",
        "This candidate is useful when a selection exists in data but should not present the same interaction affordances as a live target.",
    ),
    (
        "World Object",
        "Mailbox, chest, or clickable prop",
        "Use this row to inspect how non-unit interactions surface in the selection UI and whether click routing differs from unit selection.",
    ),
];

/// The original `SelectionDebugModel` and its action handling.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SelectionModel {
    pub state: SelectionDebugState,
}

impl Default for SelectionModel {
    fn default() -> Self {
        let entries = ENTRIES
            .iter()
            .map(|(label, subtitle, detail)| SelectionDebugEntry {
                label: (*label).into(),
                subtitle: (*subtitle).into(),
                detail: (*detail).into(),
            })
            .collect();
        Self {
            state: SelectionDebugState {
                entries,
                selected_index: 0,
                pinned: false,
                last_action: "Initialized selection debug screen".into(),
            },
        }
    }
}

impl SelectionModel {
    /// Applies `action`; `false` for Back, which leaves the screen.
    pub fn apply(&mut self, action: &SelectionDebugAction) -> bool {
        match action {
            SelectionDebugAction::SelectEntry(index) => self.select(*index),
            SelectionDebugAction::Prev => self.cycle(-1),
            SelectionDebugAction::Next => self.cycle(1),
            SelectionDebugAction::TogglePinned => self.toggle_pinned(),
            SelectionDebugAction::Back => return false,
        }
        true
    }

    fn select(&mut self, index: usize) {
        if index >= self.state.entries.len() {
            return;
        }
        self.state.selected_index = index;
        self.state.last_action = format!("Selected {}", self.current_label());
    }

    fn cycle(&mut self, delta: isize) {
        let count = self.state.entries.len() as isize;
        if count == 0 {
            return;
        }
        let next = (self.state.selected_index as isize + delta).rem_euclid(count);
        self.state.selected_index = next as usize;
        self.state.last_action = format!("Focused {}", self.current_label());
    }

    fn toggle_pinned(&mut self) {
        self.state.pinned = !self.state.pinned;
        let verb = if self.state.pinned { "Pinned" } else { "Unpinned" };
        self.state.last_action = format!("{verb} {}", self.current_label());
    }

    fn current_label(&self) -> String {
        self.state
            .entries
            .get(self.state.selected_index)
            .map_or_else(|| "Unknown".into(), |entry| entry.label.clone())
    }
}

/// The original keyboard bindings.
pub(crate) fn key_action(key: Key) -> Option<SelectionDebugAction> {
    match key {
        Key::UP | Key::LEFT => Some(SelectionDebugAction::Prev),
        Key::DOWN | Key::RIGHT => Some(SelectionDebugAction::Next),
        Key::ENTER | Key::SPACE => Some(SelectionDebugAction::TogglePinned),
        Key::ESCAPE => Some(SelectionDebugAction::Back),
        _ => None,
    }
}

/// The selection debug screen: its registry UI and candidate model.
#[derive(GodotClass)]
#[class(base = Node, no_init)]
pub struct WowSelectionDebug {
    base: Base<Node>,
    ui: Option<Gd<RegistryUi>>,
    model: SelectionModel,
}

#[godot_api]
impl INode for WowSelectionDebug {
    fn process(&mut self, _delta: f64) {
        let Some(mut ui) = self.ui.clone() else {
            return;
        };
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                return;
            }
            let Some(action) = SelectionDebugAction::parse(&action) else {
                godot_error!("Selection debug: unknown action {action}");
                continue;
            };
            if !self.dispatch(&action) {
                return;
            }
        }
    }

    fn unhandled_input(&mut self, event: Gd<InputEvent>) {
        let Ok(key) = event.try_cast::<InputEventKey>() else {
            return;
        };
        if !key.is_pressed() {
            return;
        }
        let Some(action) = key_action(key.get_keycode()) else {
            return;
        };
        if let Some(mut viewport) = self.base().get_viewport() {
            viewport.set_input_as_handled();
        }
        self.dispatch(&action);
    }
}

#[godot_api]
impl WowSelectionDebug {
    /// The candidate model, for fixtures.
    #[func]
    fn debug_state(&self) -> VarDictionary {
        let state = &self.model.state;
        let mut out = VarDictionary::new();
        out.set("selected_index", state.selected_index as i64);
        out.set("pinned", state.pinned);
        out.set("last_action", state.last_action.as_str());
        out
    }
}

impl WowSelectionDebug {
    /// Applies `action` to the model and the UI; `false` once the screen left.
    fn dispatch(&mut self, action: &SelectionDebugAction) -> bool {
        if !self.model.apply(action) {
            self.leave();
            return false;
        }
        let state = self.model.state.clone();
        if let Some(ui) = self.ui.as_mut()
            && let Err(error) = ui.bind_mut().set_state(state)
        {
            godot_error!("Selection debug: {error}");
        }
        true
    }

    /// The original Back: return to the Login screen.
    fn leave(&mut self) {
        let parent = self.base().get_parent();
        if let Some(mut login) = parent
            .and_then(|parent| parent.get_node_or_null("LoginUI"))
            .and_then(|login| login.try_cast::<RegistryUi>().ok())
        {
            login.set_visible(true);
        }
        self.ui = None;
        self.base_mut().queue_free();
    }
}

impl GameClient {
    /// Shows the selection debug screen in place of the login screen.
    pub(super) fn open_selection_debug(&mut self) -> Result<(), String> {
        if let Some(login) = self.login_ui.as_mut() {
            login.set_visible(false);
        }
        let model = SelectionModel::default();
        let mut scene = Gd::from_init_fn(|base| WowSelectionDebug {
            base,
            ui: None,
            model: model.clone(),
        });
        scene.set_name("SelectionDebug");
        self.base_mut().add_child(&scene);
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("SelectionDebugUI");
        scene.add_child(&ui);
        let shown = ui
            .bind_mut()
            .show_standalone_screen(model.state, selection_debug_screen);
        if let Err(error) = shown {
            scene.free();
            return Err(format!("Selection debug: {error}"));
        }
        scene.bind_mut().ui = Some(ui);
        Ok(())
    }
}

#[cfg(test)]
#[path = "selection_debug_tests.rs"]
mod tests;
