use bevy::prelude::*;
use game_engine::ui::plugin::UiState;
use game_engine::ui::screens::char_create_component::navigation_art::{
    part_widths, sync_navigation_art_registry,
};

#[path = "../../ui/screens/char_create_component/navigation_snap_bevy.rs"]
mod navigation_snap_bevy;

/// Host-owned window scale and UI resource adapter for authored navigation art.
pub fn sync_navigation_art(
    mut ui: ResMut<UiState>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    if let Some(window) = windows.iter().next() {
        navigation_snap_bevy::snap_navigation_parts(&mut ui.registry, window.scale_factor());
    }
    sync_navigation_art_registry(&mut ui.registry);
}
