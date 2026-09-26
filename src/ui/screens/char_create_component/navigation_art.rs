pub(super) use super::navigation_art_common::navigation_layers;
pub use super::navigation_art_common::sync_navigation_art_registry;

#[cfg(all(test, feature = "dev"))]
use super::navigation_art_common::part_widths;

#[cfg(all(test, feature = "dev"))]
pub fn sync_navigation_art(
    mut ui: bevy::prelude::ResMut<crate::ui::plugin::UiState>,
    windows: bevy::prelude::Query<
        &bevy::prelude::Window,
        bevy::prelude::With<bevy::window::PrimaryWindow>,
    >,
) {
    sync_navigation_art_registry(
        &mut ui.registry,
        windows
            .iter()
            .next()
            .map(bevy::prelude::Window::scale_factor),
    );
}

#[cfg(all(test, feature = "dev"))]
#[path = "navigation_art_tests.rs"]
mod tests;
