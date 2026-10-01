//! Players' social emotes (`EmoteEvent`): what the emoting player's model plays. A /wave
//! plays once over its stance; /dance, /sit, /sleep and /kneel are held while it stands
//! still, as the Bevy client's `EmoteAnimState`, and end when it moves or fights.

use shared::{
    components::{StandState, UnitPose},
    protocol::EmoteKind,
};

use crate::npc_gear_data::NpcGearData;

/// TrinityCore `Emote` rows a text emote plays (`EmotesText`): /dance sets
/// `EMOTE_STATE_DANCE`, /wave plays `EMOTE_ONESHOT_WAVE`.
const EMOTE_STATE_DANCE: u32 = 10;
const EMOTE_ONESHOT_WAVE: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EmoteClip {
    /// Held while the player stands still.
    Held(u16),
    /// Played once over its stance.
    Once(u16),
}

/// The clip `emote` plays: its `Emotes.AnimID`, or its stand state's pose.
pub(crate) fn emote_clip(emote: EmoteKind, gear: &NpcGearData) -> Result<EmoteClip, String> {
    let emote_anim = |id| {
        gear.emote_anim_id(id)
            .ok_or_else(|| format!("Emotes.db2 has no animation for emote {id}"))
    };
    let stand_anim = |stand_state| {
        gear.pose_anim_id(&UnitPose {
            stand_state,
            ..UnitPose::default()
        })?
        .ok_or_else(|| format!("stand state {stand_state:?} holds no animation"))
    };
    Ok(match emote {
        EmoteKind::Wave => EmoteClip::Once(emote_anim(EMOTE_ONESHOT_WAVE)?),
        EmoteKind::Dance => EmoteClip::Held(emote_anim(EMOTE_STATE_DANCE)?),
        EmoteKind::Sit => EmoteClip::Held(stand_anim(StandState::Sit)?),
        EmoteKind::Sleep => EmoteClip::Held(stand_anim(StandState::Sleep)?),
        EmoteKind::Kneel => EmoteClip::Held(stand_anim(StandState::Kneel)?),
    })
}

/// The movement clip a player with held emote `held` plays for locomotion `movement`:
/// the emote while it stands still out of combat, else the movement, which ends the
/// emote (`*held` is cleared).
pub(crate) fn held_emote_movement(
    held: &mut Option<u16>,
    movement: u16,
    standing_still: bool,
    in_combat: bool,
) -> u16 {
    match *held {
        Some(clip) if standing_still && !in_combat => clip,
        _ => {
            *held = None;
            movement
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gear() -> NpcGearData {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/db2/12.1.0.69933");
        NpcGearData::load(&dir).unwrap()
    }

    /// Emotes.db2 build 12.1.0.69933: emote 3 → EmoteWave 67, 10 → EmoteDance 69; stand
    /// states Sit/Sleep/Kneel → SitGround 97, Sleep 100, KneelLoop 115.
    #[test]
    fn emotes_play_their_retail_clips() {
        let gear = gear();
        let clip = |emote| emote_clip(emote, &gear).unwrap();
        assert_eq!(clip(EmoteKind::Wave), EmoteClip::Once(67));
        assert_eq!(clip(EmoteKind::Dance), EmoteClip::Held(69));
        assert_eq!(clip(EmoteKind::Sit), EmoteClip::Held(97));
        assert_eq!(clip(EmoteKind::Sleep), EmoteClip::Held(100));
        assert_eq!(clip(EmoteKind::Kneel), EmoteClip::Held(115));
    }

    #[test]
    fn a_held_emote_lasts_until_the_player_moves_or_fights() {
        let mut held = Some(97);
        assert_eq!(held_emote_movement(&mut held, 0, true, false), 97);
        assert_eq!(held, Some(97));
        assert_eq!(held_emote_movement(&mut held, 5, false, false), 5);
        assert_eq!(held, None);
        assert_eq!(held_emote_movement(&mut held, 0, true, false), 0);
        let mut held = Some(69);
        assert_eq!(held_emote_movement(&mut held, 26, true, true), 26);
        assert_eq!(held, None);
    }
}
