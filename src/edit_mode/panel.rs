//! Edit-mode screens (selection boxes + manager panel) and panel actions.

use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::ui::frame::WidgetData;
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::edit_mode_component::{
    ACTION_EDIT_MODE_DELETE, ACTION_EDIT_MODE_EXIT, ACTION_EDIT_MODE_NEW,
    ACTION_EDIT_MODE_NEXT_LAYOUT, ACTION_EDIT_MODE_PREV_LAYOUT, ACTION_EDIT_MODE_RENAME,
    ACTION_EDIT_MODE_REVERT, ACTION_EDIT_MODE_SAVE, EDIT_MODE_NAME_INPUT, EditModeOverlayState,
    EditModePanelState, EditModeSelectionBox, PANEL_H, PANEL_TOP, PANEL_W,
    edit_mode_overlay_screen, edit_mode_panel_screen,
};
use ui_toolkit::screen::{Screen, SharedContext};

use super::elements::EDIT_MODE_ELEMENTS;
use super::layouts::PRESET_LAYOUT;
use super::{EditMode, element_rect};
use crate::networking::SelectedCharacterId;
use crate::ui_input::{mutate_editbox_from_key, walk_up_for_onclick};
use crate::ui_layout_store::{UiLayoutStore, character_key};

struct EditModeScreens {
    overlay: Screen,
    panel: Screen,
    shared: SharedContext,
}

unsafe impl Send for EditModeScreens {}
unsafe impl Sync for EditModeScreens {}

#[derive(Resource)]
pub(super) struct EditModeScreensRes(EditModeScreens);

/// Panel rect in UI units; it is centered horizontally at `PANEL_TOP`.
pub(super) fn panel_contains(registry: &FrameRegistry, cursor: Vec2) -> bool {
    let min = Vec2::new((registry.screen_width - PANEL_W) * 0.5, PANEL_TOP);
    Rect::from_corners(min, min + Vec2::new(PANEL_W, PANEL_H)).contains(cursor)
}

fn overlay_state(edit: &EditMode, registry: &FrameRegistry) -> EditModeOverlayState {
    EditModeOverlayState {
        boxes: EDIT_MODE_ELEMENTS
            .iter()
            .filter_map(|element| {
                let rect = element_rect(edit, registry, element)?;
                Some(EditModeSelectionBox {
                    key: element.key.to_string(),
                    label: element.label.to_string(),
                    rect: [rect.min.x, rect.min.y, rect.width(), rect.height()],
                    selected: edit.selected == Some(element.key),
                })
            })
            .collect(),
    }
}

fn panel_state(edit: &EditMode) -> EditModePanelState {
    EditModePanelState {
        layout_name: edit.layout_name.clone(),
        name_draft: edit.name_draft.clone(),
        preset: edit.layout_name == PRESET_LAYOUT,
        dirty: edit.is_dirty(),
        status: edit.status.clone(),
    }
}

pub(super) fn sync_edit_mode_screens(
    mut commands: Commands,
    mut ui: ResMut<UiState>,
    edit: Res<EditMode>,
    screens: Option<ResMut<EditModeScreensRes>>,
) {
    if !edit.active {
        if let Some(mut screens) = screens {
            screens.0.overlay.teardown(&mut ui.registry);
            screens.0.panel.teardown(&mut ui.registry);
            commands.remove_resource::<EditModeScreensRes>();
        }
        return;
    }
    let overlay = overlay_state(&edit, &ui.registry);
    let panel = panel_state(&edit);
    let Some(mut screens) = screens else {
        let mut shared = SharedContext::new();
        shared.insert(overlay);
        shared.insert(panel);
        let mut overlay_screen = Screen::new(edit_mode_overlay_screen);
        let mut panel_screen = Screen::new(edit_mode_panel_screen);
        overlay_screen.sync(&shared, &mut ui.registry);
        panel_screen.sync(&shared, &mut ui.registry);
        commands.insert_resource(EditModeScreensRes(EditModeScreens {
            overlay: overlay_screen,
            panel: panel_screen,
            shared,
        }));
        return;
    };
    let screens = &mut screens.0;
    if screens.shared.get::<EditModeOverlayState>() != Some(&overlay) {
        screens.shared.insert(overlay);
    }
    if screens.shared.get::<EditModePanelState>() != Some(&panel) {
        screens.shared.insert(panel);
    }
    screens.overlay.sync(&screens.shared, &mut ui.registry);
    screens.panel.sync(&screens.shared, &mut ui.registry);
}

pub(super) fn handle_edit_mode_panel_click(
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut ui: ResMut<UiState>,
    mut edit: ResMut<EditMode>,
    mut store: ResMut<UiLayoutStore>,
    selected: Option<Res<SelectedCharacterId>>,
) {
    let pressed = mouse.is_some_and(|mouse| mouse.just_pressed(MouseButton::Left));
    if !edit.active || !pressed {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = ui_cursor_position(&ui.registry, window) else {
        return;
    };
    if !panel_contains(&ui.registry, cursor) {
        return;
    }
    let Some(frame_id) = find_frame_at(&ui.registry, cursor.x, cursor.y) else {
        return;
    };
    if ui.registry.get_by_name(EDIT_MODE_NAME_INPUT.0) == Some(frame_id) {
        ui.registry.click_frame(frame_id);
        ui.focused_frame = ui.registry.focused_frame;
        return;
    }
    let Some(action) = walk_up_for_onclick(&ui.registry, frame_id) else {
        return;
    };
    let character = character_key(selected.as_deref());
    dispatch_edit_mode_action(&action, &mut edit, &mut store, character.as_deref());
}

pub(super) fn handle_edit_mode_name_input(
    key_events: Option<MessageReader<KeyboardInput>>,
    mut ui: ResMut<UiState>,
    mut edit: ResMut<EditMode>,
) {
    let Some(mut key_events) = key_events else {
        return;
    };
    let input = ui.registry.get_by_name(EDIT_MODE_NAME_INPUT.0);
    let focused = ui.focused_frame.or(ui.registry.focused_frame);
    if !edit.active || input.is_none() || focused != input {
        key_events.clear();
        return;
    }
    let Some(input) = input else { return };
    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        if event.key_code == KeyCode::Enter {
            ui.focused_frame = None;
            ui.registry.focused_frame = None;
            continue;
        }
        if mutate_editbox_from_key(&mut ui.registry, input, event) {
            edit.name_draft = editbox_text(&ui.registry, input);
        }
    }
}

fn editbox_text(registry: &FrameRegistry, id: u64) -> String {
    match registry
        .get(id)
        .and_then(|frame| frame.widget_data.as_ref())
    {
        Some(WidgetData::EditBox(editbox)) => editbox.text.clone(),
        _ => String::new(),
    }
}

/// Applies one manager-panel action and persists layout changes.
pub fn dispatch_edit_mode_action(
    action: &str,
    edit: &mut EditMode,
    store: &mut UiLayoutStore,
    character: Option<&str>,
) {
    let result = match action {
        ACTION_EDIT_MODE_PREV_LAYOUT => switch_layout(edit, store, character, -1),
        ACTION_EDIT_MODE_NEXT_LAYOUT => switch_layout(edit, store, character, 1),
        ACTION_EDIT_MODE_NEW => new_layout(edit, store, character),
        ACTION_EDIT_MODE_RENAME => rename_layout(edit, store),
        ACTION_EDIT_MODE_DELETE => delete_layout(edit, store, character),
        ACTION_EDIT_MODE_REVERT => {
            edit.working = edit.saved.clone();
            Ok(format!("Reverted {}", edit.layout_name))
        }
        ACTION_EDIT_MODE_SAVE => save_layout(edit, store, character),
        ACTION_EDIT_MODE_EXIT => {
            edit.exit();
            return;
        }
        _ => return,
    };
    edit.status = result.unwrap_or_else(|err| err);
}

fn activate(edit: &mut EditMode, store: &mut UiLayoutStore, character: Option<&str>, name: String) {
    store.file.edit_mode.set_active(character, &name);
    let layout = store.file.edit_mode.layout(&name);
    edit.load_layout(name, layout);
}

fn switch_layout(
    edit: &mut EditMode,
    store: &mut UiLayoutStore,
    character: Option<&str>,
    step: isize,
) -> Result<String, String> {
    let names = store.file.edit_mode.layout_names();
    let current = names
        .iter()
        .position(|name| *name == edit.layout_name)
        .unwrap_or(0);
    let next = (current as isize + step).rem_euclid(names.len() as isize) as usize;
    activate(edit, store, character, names[next].clone());
    store.save();
    Ok(format!("Active layout: {}", edit.layout_name))
}

/// New layout from the current placements, named by the name box when unused.
fn new_layout(
    edit: &mut EditMode,
    store: &mut UiLayoutStore,
    character: Option<&str>,
) -> Result<String, String> {
    let draft = edit.name_draft.trim().to_string();
    let layouts = &store.file.edit_mode;
    let name = if draft.is_empty() || draft == PRESET_LAYOUT || layouts.layouts.contains_key(&draft)
    {
        layouts.new_layout_name()
    } else {
        draft
    };
    store
        .file
        .edit_mode
        .save_layout(&name, edit.working.clone())?;
    activate(edit, store, character, name);
    store.save();
    Ok(format!("Created {}", edit.layout_name))
}

fn rename_layout(edit: &mut EditMode, store: &mut UiLayoutStore) -> Result<String, String> {
    let to = edit.name_draft.trim().to_string();
    store.file.edit_mode.rename_layout(&edit.layout_name, &to)?;
    edit.layout_name = to;
    store.save();
    Ok(format!("Renamed to {}", edit.layout_name))
}

fn delete_layout(
    edit: &mut EditMode,
    store: &mut UiLayoutStore,
    character: Option<&str>,
) -> Result<String, String> {
    let deleted = edit.layout_name.clone();
    store.file.edit_mode.delete_layout(&deleted)?;
    activate(edit, store, character, PRESET_LAYOUT.to_string());
    store.save();
    Ok(format!("Deleted {deleted}"))
}

/// Saves into the active layout; saving the preset creates a new layout.
fn save_layout(
    edit: &mut EditMode,
    store: &mut UiLayoutStore,
    character: Option<&str>,
) -> Result<String, String> {
    if edit.layout_name == PRESET_LAYOUT {
        let name = store.file.edit_mode.new_layout_name();
        store
            .file
            .edit_mode
            .save_layout(&name, edit.working.clone())?;
        activate(edit, store, character, name);
    } else {
        store
            .file
            .edit_mode
            .save_layout(&edit.layout_name, edit.working.clone())?;
        edit.saved = edit.working.clone();
    }
    store.save();
    Ok(format!("Saved {}", edit.layout_name))
}
