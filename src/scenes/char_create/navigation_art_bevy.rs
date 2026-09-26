use bevy::prelude::*;
use game_engine::ui::plugin::UiState;
use game_engine::ui::screens::char_create_component::navigation_art::sync_navigation_art_registry;

/// Host-owned window scale and UI resource adapter for authored navigation art.
pub fn sync_navigation_art(
    mut ui: ResMut<UiState>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    sync_navigation_art_registry(
        &mut ui.registry,
        windows.iter().next().map(Window::scale_factor),
    );
}
