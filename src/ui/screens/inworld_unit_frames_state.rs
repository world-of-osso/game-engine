use super::{UNKNOWN_PORTRAIT_TEXTURE_FILE, UnitFrameState};

pub fn default_player_frame_state() -> UnitFrameState {
    default_unit_frame_state("Player")
}

pub fn fallback_target_frame_state() -> UnitFrameState {
    default_unit_frame_state("No Target")
}

pub fn fill_width(max_width: f32, current: Option<f32>, max: Option<f32>) -> f32 {
    let Some(max) = max.filter(|value| *value > 0.0) else {
        return 0.0;
    };
    let pct = current.unwrap_or(0.0).clamp(0.0, max) / max;
    (max_width * pct).clamp(0.0, max_width)
}

pub fn format_value_text(current: Option<f32>, max: Option<f32>) -> String {
    match (current, max) {
        (Some(current), Some(max)) => format!("{:.0} / {:.0}", current, max),
        (Some(current), None) => format!("{current:.0}"),
        _ => String::new(),
    }
}

pub fn missing_target_name() -> &'static str {
    "Target"
}

fn default_unit_frame_state(name: &str) -> UnitFrameState {
    UnitFrameState {
        portrait_texture_file: UNKNOWN_PORTRAIT_TEXTURE_FILE.into(),
        name: name.to_string(),
        level_text: String::new(),
        resting_text: String::new(),
        health_text: String::new(),
        mana_text: String::new(),
        health_fill_width: 0.0,
        mana_fill_width: 0.0,
        secondary_resource: None,
        has_mana: false,
        show_combat_icon: false,
        show_resting_icon: false,
        target_buffs: Vec::new(),
        target_debuffs: Vec::new(),
    }
}
