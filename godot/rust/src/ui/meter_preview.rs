//! Offline data for capture_ui_screen.gd, never connected to combat state.
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
