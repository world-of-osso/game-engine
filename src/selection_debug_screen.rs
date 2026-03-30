use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::selection_debug_component::{
    SelectionDebugAction, SelectionDebugEntry, SelectionDebugState, selection_debug_screen,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::char_select_input::{cursor_pos, find_clicked_action};
use crate::game_state::GameState;

#[derive(Debug, Clone, Resource)]
struct SelectionDebugModel {
    entries: Vec<SelectionDebugEntry>,
    selected_index: usize,
    pinned: bool,
    last_action: String,
}

struct SelectionDebugScreenRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for SelectionDebugScreenRes {}
unsafe impl Sync for SelectionDebugScreenRes {}

#[derive(Resource)]
struct SelectionDebugScreenWrap(SelectionDebugScreenRes);

#[derive(Message)]
struct SelectionDebugClickEvent(String);

pub struct SelectionDebugScreenPlugin;

impl Plugin for SelectionDebugScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SelectionDebugClickEvent>();
        app.add_systems(OnEnter(GameState::SelectionDebug), build_selection_debug_ui);
        app.add_systems(
            OnExit(GameState::SelectionDebug),
            teardown_selection_debug_ui,
        );
        app.add_systems(
            Update,
            (
                selection_debug_mouse_input,
                selection_debug_keyboard_input,
                dispatch_selection_debug_action,
                sync_selection_debug_ui,
            )
                .chain()
                .run_if(in_state(GameState::SelectionDebug)),
        );
    }
}

impl Default for SelectionDebugModel {
    fn default() -> Self {
        Self {
            entries: vec![
                SelectionDebugEntry {
                    label: "Local Player".to_string(),
                    subtitle: "Self-target and keyboard recovery".to_string(),
                    detail: "Use this candidate when debugging self-target, default focus recovery, or cases where there is no hovered remote entity.".to_string(),
                },
                SelectionDebugEntry {
                    label: "Quest NPC".to_string(),
                    subtitle: "Friendly unit with hover and click affordances".to_string(),
                    detail: "This row stands in for a talkable NPC so you can compare hover, click, and retained selection styling without needing a full scene.".to_string(),
                },
                SelectionDebugEntry {
                    label: "Enemy Creature".to_string(),
                    subtitle: "Hostile unit with target-ring expectations".to_string(),
                    detail: "Use this for hostile-target visuals, tab-target ordering, and any selection ring color or visibility regressions.".to_string(),
                },
                SelectionDebugEntry {
                    label: "Corpse / Invalid".to_string(),
                    subtitle: "Non-interactive or stale target".to_string(),
                    detail: "This candidate is useful when a selection exists in data but should not present the same interaction affordances as a live target.".to_string(),
                },
                SelectionDebugEntry {
                    label: "World Object".to_string(),
                    subtitle: "Mailbox, chest, or clickable prop".to_string(),
                    detail: "Use this row to inspect how non-unit interactions surface in the selection UI and whether click routing differs from unit selection.".to_string(),
                },
            ],
            selected_index: 0,
            pinned: false,
            last_action: "Initialized selection debug screen".to_string(),
        }
    }
}

fn build_selection_debug_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);

    let model = SelectionDebugModel::default();
    let mut shared = SharedContext::new();
    shared.insert(selection_debug_state(&model));
    let mut screen = Screen::new(selection_debug_screen);
    screen.sync(&shared, &mut ui.registry);

    commands.insert_resource(SelectionDebugScreenWrap(SelectionDebugScreenRes {
        screen,
        shared,
    }));
    commands.insert_resource(model);
}

fn teardown_selection_debug_ui(
    mut ui: ResMut<UiState>,
    mut screen: Option<ResMut<SelectionDebugScreenWrap>>,
    mut commands: Commands,
) {
    if let Some(res) = screen.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<SelectionDebugScreenWrap>();
    commands.remove_resource::<SelectionDebugModel>();
    ui.focused_frame = None;
}

fn selection_debug_state(model: &SelectionDebugModel) -> SelectionDebugState {
    SelectionDebugState {
        entries: model.entries.clone(),
        selected_index: model.selected_index,
        pinned: model.pinned,
        last_action: model.last_action.clone(),
    }
}

fn selection_debug_mouse_input(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    ui: Res<UiState>,
    mut events: MessageWriter<SelectionDebugClickEvent>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = cursor_pos(&windows) else {
        return;
    };
    if let Some(action) = find_clicked_action(&ui, cursor.x, cursor.y) {
        events.write(SelectionDebugClickEvent(action));
    }
}

fn selection_debug_keyboard_input(
    mut key_events: MessageReader<KeyboardInput>,
    mut events: MessageWriter<SelectionDebugClickEvent>,
) {
    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        let action = match event.key_code {
            KeyCode::ArrowUp | KeyCode::ArrowLeft => Some(SelectionDebugAction::Prev),
            KeyCode::ArrowDown | KeyCode::ArrowRight => Some(SelectionDebugAction::Next),
            KeyCode::Enter | KeyCode::Space => Some(SelectionDebugAction::TogglePinned),
            KeyCode::Escape => Some(SelectionDebugAction::Back),
            _ => None,
        };
        if let Some(action) = action {
            events.write(SelectionDebugClickEvent(action.to_string()));
        }
    }
}

fn dispatch_selection_debug_action(
    mut events: MessageReader<SelectionDebugClickEvent>,
    mut model: ResMut<SelectionDebugModel>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for event in events.read() {
        match SelectionDebugAction::parse(&event.0) {
            Some(SelectionDebugAction::SelectEntry(index)) => select_entry(&mut model, index),
            Some(SelectionDebugAction::Prev) => cycle_selection(&mut model, -1),
            Some(SelectionDebugAction::Next) => cycle_selection(&mut model, 1),
            Some(SelectionDebugAction::TogglePinned) => {
                model.pinned = !model.pinned;
                let label = current_label(&model);
                model.last_action = if model.pinned {
                    format!("Pinned {label}")
                } else {
                    format!("Unpinned {label}")
                };
            }
            Some(SelectionDebugAction::Back) => next_state.set(GameState::Login),
            None => {}
        }
    }
}

fn select_entry(model: &mut SelectionDebugModel, index: usize) {
    if index >= model.entries.len() {
        return;
    }
    model.selected_index = index;
    model.last_action = format!("Selected {}", current_label(model));
}

fn cycle_selection(model: &mut SelectionDebugModel, delta: isize) {
    let count = model.entries.len();
    if count == 0 {
        return;
    }
    let count = count as isize;
    let current = model.selected_index as isize;
    let next = (current + delta).rem_euclid(count) as usize;
    model.selected_index = next;
    model.last_action = format!("Focused {}", current_label(model));
}

fn current_label(model: &SelectionDebugModel) -> &str {
    model
        .entries
        .get(model.selected_index)
        .map(|entry| entry.label.as_str())
        .unwrap_or("Unknown")
}

fn sync_selection_debug_ui(
    model: Res<SelectionDebugModel>,
    mut screen: Option<ResMut<SelectionDebugScreenWrap>>,
    mut ui: ResMut<UiState>,
) {
    if !model.is_changed() {
        return;
    }
    let Some(screen) = screen.as_mut() else {
        return;
    };
    let wrap = &mut screen.0;
    wrap.shared.insert(selection_debug_state(&model));
    let shared = &wrap.shared;
    wrap.screen.sync(shared, &mut ui.registry);
    ui.focused_frame = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_selection_wraps_both_directions() {
        let mut model = SelectionDebugModel::default();
        cycle_selection(&mut model, -1);
        assert_eq!(model.selected_index, model.entries.len() - 1);

        cycle_selection(&mut model, 1);
        assert_eq!(model.selected_index, 0);
    }

    #[test]
    fn select_entry_updates_last_action() {
        let mut model = SelectionDebugModel::default();
        select_entry(&mut model, 2);
        assert_eq!(model.selected_index, 2);
        assert_eq!(model.last_action, "Selected Enemy Creature");
    }
}
