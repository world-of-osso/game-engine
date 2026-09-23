//! Pointer interaction with placed windows: click-to-raise and title-bar drag.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::UiState;

use super::placement::{clamp_to_screen, window_size};
use super::{WindowId, WindowManager, WindowPlacements};
use crate::networking::SelectedCharacterId;
use crate::ui_input::walk_up_for_onclick;
use crate::ui_input_mode::UiInputMode;
use crate::ui_layout_store::{UiLayoutStore, character_key};

/// Height of the draggable title region at the top of every window.
pub const TITLE_REGION_H: f32 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct ActiveWindowDrag {
    id: WindowId,
    /// Cursor minus window top-left at grab time.
    grab: Vec2,
}

#[derive(Resource, Default, Debug)]
pub struct WindowDrag(Option<ActiveWindowDrag>);

/// Topmost open window containing `cursor` (UI units, top-left origin).
pub fn window_at(
    manager: &WindowManager,
    placements: &WindowPlacements,
    ui: &UiState,
    cursor: Vec2,
) -> Option<WindowId> {
    manager
        .open_windows()
        .iter()
        .copied()
        .filter(|id| {
            let (Some(pos), Some(size)) =
                (placements.position(*id), window_size(&ui.registry, *id))
            else {
                return false;
            };
            Rect::from_corners(pos, pos + size).contains(cursor)
        })
        .max_by_key(|id| manager.raise_rank(*id).unwrap_or(0))
}

pub fn raise_window_on_click(
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mode: Option<Res<UiInputMode>>,
    ui: Res<UiState>,
    placements: Res<WindowPlacements>,
    mut manager: ResMut<WindowManager>,
) {
    let pressed = mouse.is_some_and(|mouse| mouse.just_pressed(MouseButton::Left));
    if !pressed || mode.is_some_and(|mode| *mode == UiInputMode::Modal) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = ui_cursor_position(&ui.registry, window) else {
        return;
    };
    let Some(id) = window_at(&manager, &placements, &ui, cursor) else {
        return;
    };
    let topmost = manager
        .open_windows()
        .iter()
        .copied()
        .max_by_key(|other| manager.raise_rank(*other).unwrap_or(0));
    if topmost != Some(id) {
        manager.raise(id);
    }
}

#[derive(SystemParam)]
pub struct WindowPointer<'w, 's> {
    mouse: Option<Res<'w, ButtonInput<MouseButton>>>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    mode: Option<Res<'w, UiInputMode>>,
}

/// Drags a window by its title region; the position is saved for the current
/// character when the button is released.
pub fn drag_window_by_title(
    pointer: WindowPointer,
    ui: Res<UiState>,
    manager: Res<WindowManager>,
    placements: Res<WindowPlacements>,
    selected: Option<Res<SelectedCharacterId>>,
    store: Option<ResMut<UiLayoutStore>>,
    mut drag: ResMut<WindowDrag>,
) {
    let WindowPointer {
        mouse,
        windows,
        mode,
    } = pointer;
    let (Some(mouse), Some(mut store)) = (mouse, store) else {
        return;
    };
    let Some(character) = character_key(selected.as_deref()) else {
        return;
    };
    if mouse.just_released(MouseButton::Left) && drag.0.take().is_some() {
        store.save();
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = ui_cursor_position(&ui.registry, window) else {
        return;
    };
    if mouse.just_pressed(MouseButton::Left) {
        if mode.is_some_and(|mode| *mode == UiInputMode::Modal) {
            return;
        }
        drag.0 = title_grab(&manager, &placements, &ui, cursor);
        return;
    }
    let Some(active) = drag.0.filter(|_| mouse.pressed(MouseButton::Left)) else {
        return;
    };
    if !manager.is_open(active.id) {
        drag.0 = None;
        return;
    }
    let size = window_size(&ui.registry, active.id).unwrap_or(Vec2::ZERO);
    let screen = Vec2::new(ui.registry.screen_width, ui.registry.screen_height);
    let pos = clamp_to_screen(cursor - active.grab, size, screen);
    let key = active.id.root_frame_name();
    if store.window_position(&character, &key) != Some(pos) {
        store.set_window_position(&character, &key, pos);
    }
}

/// A press on a window's title region, away from its buttons, grabs it.
fn title_grab(
    manager: &WindowManager,
    placements: &WindowPlacements,
    ui: &UiState,
    cursor: Vec2,
) -> Option<ActiveWindowDrag> {
    let id = window_at(manager, placements, ui, cursor)?;
    let pos = placements.position(id)?;
    if cursor.y - pos.y > TITLE_REGION_H {
        return None;
    }
    let on_button = find_frame_at(&ui.registry, cursor.x, cursor.y)
        .is_some_and(|frame| walk_up_for_onclick(&ui.registry, frame).is_some());
    if on_button {
        return None;
    }
    Some(ActiveWindowDrag {
        id,
        grab: cursor - pos,
    })
}
