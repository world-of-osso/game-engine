pub(super) use super::navigation_art_common::navigation_layers;
pub use super::navigation_art_common::{part_widths, sync_navigation_art_registry};

#[cfg(all(test, feature = "dev"))]
#[path = "navigation_snap_bevy.rs"]
mod navigation_snap_bevy;

#[cfg(all(test, feature = "dev"))]
pub fn sync_navigation_art(
    mut ui: bevy::prelude::ResMut<crate::ui::plugin::UiState>,
    windows: bevy::prelude::Query<
        &bevy::prelude::Window,
        bevy::prelude::With<bevy::window::PrimaryWindow>,
    >,
) {
    if let Some(window) = windows.iter().next() {
        navigation_snap_bevy::snap_navigation_parts(&mut ui.registry, window.scale_factor());
    }
    sync_navigation_art_registry(&mut ui.registry);
}

#[cfg(all(test, feature = "dev"))]
#[path = "navigation_art_tests.rs"]
mod tests;
