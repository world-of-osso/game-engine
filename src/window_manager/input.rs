//! Pointer interaction with placed windows: click-to-raise.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::ui::input::ui_cursor_position;
use game_engine::ui::plugin::UiState;

use super::placement::window_size;
use super::{WindowId, WindowManager, WindowPlacements};
use crate::ui_input_mode::UiInputMode;

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
