//! Players' social emotes (`EmoteEvent`): what the emoting player's model plays. A /wave
//! plays once over its stance; /dance is held while it stands still, as the Bevy client's
//! `EmoteAnimState`, and ends when it moves or fights. /sit, /sleep and /kneel play
//! nothing here: TrinityCore `HandleTextEmoteOpcode` (ChatHandler.cpp:703-761) leaves
//! `EMOTE_STATE_SIT`, `_SLEEP` and `_KNEEL` to the client's `CMSG_STAND_STATE_CHANGE`,
//! and the model follows the replicated `PlayerStandState` (`world_stand.rs`).

use shared::protocol::EmoteKind;

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

/// The clip `emote` plays (its `Emotes.AnimID`); `None` for the stand state emotes.
pub(crate) fn emote_clip(
    emote: EmoteKind,
    gear: &NpcGearData,
) -> Result<Option<EmoteClip>, String> {
    let emote_anim = |id| {
        gear.emote_anim_id(id)
            .ok_or_else(|| format!("Emotes.db2 has no animation for emote {id}"))
    };
    Ok(Some(match emote {
        EmoteKind::Wave => EmoteClip::Once(emote_anim(EMOTE_ONESHOT_WAVE)?),
        EmoteKind::Dance => EmoteClip::Held(emote_anim(EMOTE_STATE_DANCE)?),
        EmoteKind::Sit | EmoteKind::Sleep | EmoteKind::Kneel => return Ok(None),
    }))
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

    /// Emotes.db2 build 12.1.0.69933: emote 3 → EmoteWave 67, 10 → EmoteDance 69; the
    /// stand state emotes hold nothing (the replicated stand state poses the model).
    #[test]
    fn emotes_play_their_retail_clips() {
        let gear = gear();
        let clip = |emote| emote_clip(emote, &gear).unwrap();
        assert_eq!(clip(EmoteKind::Wave), Some(EmoteClip::Once(67)));
        assert_eq!(clip(EmoteKind::Dance), Some(EmoteClip::Held(69)));
        assert_eq!(clip(EmoteKind::Sit), None);
        assert_eq!(clip(EmoteKind::Sleep), None);
        assert_eq!(clip(EmoteKind::Kneel), None);
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
