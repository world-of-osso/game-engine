//! Deterministic snapshots of the production cast reducer; never opens a session/socket.
use godot::prelude::*;
use shared::casting::CastState;
use shared::spell_data::CastFailReason;
use ui_toolkit::atlas::ActiveSkin;

use super::{RegistryUi, party_preview};
use crate::nameplate_casts::{PlateCasts, player_casting_bar_state};
use game_engine_ui_model::casting_bar_frame_component::CastingBarState;

const PLAYER: u64 = 42;
const SPELL: u32 = 133;
const EVENT_TIME: f32 = 100.0;

pub(super) fn sample_state(phase: &str, timestamp: f32) -> Result<CastingBarState, String> {
    if !timestamp.is_finite() || timestamp < EVENT_TIME {
        return Err("cast preview timestamp must be finite and >= 100.0".into());
    }
    let mut casts = PlateCasts::default();
    let mut cast = if phase == "channel" {
        CastState::channel(SPELL, 0, 2.0, 1.0, true)
    } else {
        CastState::normal(SPELL, 0, 2.0, true)
    };
    cast.spell_name = if phase == "channel" {
        "Arcane Missiles"
    } else {
        "Fireball"
    }
    .into();
    cast.elapsed = if phase == "channel" { 2.0 } else { 1.0 };
    casts.observe(PLAYER, Some(&cast));
    match phase {
        "midcast" => {}
        "finish" => casts.spell_go(PLAYER, SPELL),
        "interrupted" => casts.spell_failure(PLAYER, SPELL, CastFailReason::Interrupted, None),
        "failed" => casts.spell_failure(PLAYER, SPELL, CastFailReason::OutOfRange, None),
        "channel" => casts.observe(PLAYER, None),
        _ => return Err(format!("unknown cast preview phase {phase}")),
    }
    casts.advance(timestamp - EVENT_TIME, |_| true);
    Ok(casts
        .get(PLAYER)
        .map(|bar| player_casting_bar_state(bar, Some(135932)))
        .unwrap_or_default())
}

#[godot_api(secondary)]
impl RegistryUi {
    /// Existing capture_ui_screen.gd calls this with explicit skin, phase and timestamp.
    #[func]
    pub fn show_castbaranim_preview(&mut self) -> GString {
        GString::from(
            self.show_castbar_snapshot()
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }

    fn show_castbar_snapshot(&mut self) -> Result<(), String> {
        let skin = match std::env::var("GODOT_CASTBAR_SKIN").as_deref() {
            Ok("modern") => ActiveSkin::Modern,
            Ok("forever") => ActiveSkin::Forever,
            _ => return Err("GODOT_CASTBAR_SKIN must be modern or forever".into()),
        };
        let phase = std::env::var("GODOT_CASTBAR_PHASE")
            .map_err(|error| format!("GODOT_CASTBAR_PHASE: {error}"))?;
        let time = std::env::var("GODOT_CASTBAR_TIME")
            .map_err(|error| format!("GODOT_CASTBAR_TIME: {error}"))?;
        let timestamp = time
            .parse::<f32>()
            .map_err(|error| format!("cast timestamp {time}: {error}"))?;
        party_preview::load_data_root()?;
        ui_toolkit::atlas::set_thread_skin(skin);
        self.set_ui_scale(1.0)?;
        self.show_casting_bar(sample_state(&phase, timestamp)?)
    }
}
