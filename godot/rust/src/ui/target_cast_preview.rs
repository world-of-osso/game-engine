//! Offline target cast presentation samples; no session, clock or server mutations.
use game_engine_ui_model::casting_bar_frame_component::CastingBarState;
use game_engine_ui_model::inworld_unit_frames_component::{InWorldUnitFramesState, UnitFrameState};
use godot::prelude::*;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

use super::{RegistryUi, party_preview};

fn cast_sample(phase: &str) -> Result<CastingBarState, String> {
    let mut cast = CastingBarState {
        visible: true,
        spell_name: "Frostbolt".into(),
        icon_fdid: Some(135_846),
        timer_text: "1.5".into(),
        progress: 0.5,
        ..Default::default()
    };
    match phase {
        "cast" => {}
        "channel" => cast.is_channel = true,
        "locked" => cast.is_interruptible = false,
        "interrupted" => {
            cast.is_interrupted = true;
            cast.spell_name = "Interrupted".into();
            cast.timer_text.clear();
        }
        _ => return Err(format!("Unknown target cast preview phase {phase}")),
    }
    Ok(cast)
}

fn sample_units(cast: CastingBarState) -> InWorldUnitFramesState {
    InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
        player: UnitFrameState::named("Player"),
        target: Some(UnitFrameState::named("Enemy")),
        target_cast: Some(cast),
        target_of_target: None,
        focus: None,
        pet: None,
        bosses: vec![],
        menu: Default::default(),
        personal_resource: None,
    }
}

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_target_cast_preview(&mut self, forever: bool, phase: GString) -> GString {
        let result = (|| {
            let cast = cast_sample(&phase.to_string())?;
            party_preview::load_data_root()?;
            set_thread_skin(if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            });
            self.set_ui_scale(1.0)?;
            self.show_unit_frames(sample_units(cast))
        })();
        result.err().unwrap_or_default().as_str().into()
    }
}
