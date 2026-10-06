//! Offline data for capture_ui_screen.gd, never connected to combat state.
use godot::prelude::*;

use super::RegistryUi;
use game_engine_ui_model::damage_meter_data::{DamageMeterRow, DamageMeterView, class_color};

pub(super) fn view() -> DamageMeterView {
    let rows = [
        ("Jaina", 8),
        ("Guldan", 9),
        ("Uther", 2),
        ("Garrosh", 1),
        ("Thrall", 7),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (name, class_id))| DamageMeterRow {
        name_text: format!("{}. {name}", index + 1),
        value_text: format!("{} (120)", 2400 - index * 300),
        fraction: 1.0 - index as f32 * 0.125,
        color: class_color(class_id),
        class_id,
        is_local_player: index == 0,
    })
    .collect();
    DamageMeterView {
        rows,
        ..Default::default()
    }
}

#[godot_api(secondary)]
impl RegistryUi {
    /// Offline meter rows for the existing capture_ui_screen.gd fixture; no network state.
    #[func]
    pub fn show_forever_damage_meter_preview(&mut self) -> GString {
        if let Err(error) = super::party_preview::load_data_root() {
            return GString::from(error.as_str());
        }
        ui_toolkit::atlas::set_thread_skin(ui_toolkit::atlas::ActiveSkin::Forever);
        GString::from(
            self.set_ui_scale(1.0)
                .and_then(|()| self.show_damage_meter(view()))
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }
}
