//! Players' stand state (`PlayerStandState`, TrinityCore `UnitData::StandState`): what the
//! client asks the server for, and the pose a still player's model holds.

use shared::{
    components::{StandState, UnitPose},
    protocol::EmoteKind,
};

use crate::npc_gear_data::NpcGearData;

/// Retail `SITORSTAND` (blizzard-ui Bindings_Standard.xml:60, `SitStandOrDescendStart`):
/// a standing player sits down, any other stand state stands up.
pub(crate) fn sit_or_stand(current: StandState) -> StandState {
    if current == StandState::Stand {
        StandState::Sit
    } else {
        StandState::Stand
    }
}

/// The stand state a text emote asks for with its `CMSG_STAND_STATE_CHANGE`: TrinityCore
/// `HandleTextEmoteOpcode` (ChatHandler.cpp:703-761) leaves `EMOTE_STATE_SIT`, `_SLEEP`
/// and `_KNEEL` to the client.
pub(crate) fn emote_stand_state(emote: EmoteKind) -> Option<StandState> {
    match emote {
        EmoteKind::Sit => Some(StandState::Sit),
        EmoteKind::Sleep => Some(StandState::Sleep),
        EmoteKind::Kneel => Some(StandState::Kneel),
        EmoteKind::Dance | EmoteKind::Wave => None,
    }
}

/// The looping clip a still player in `state` holds (`unit_pose_anim_id`); none standing.
pub(crate) fn stand_state_anim(
    state: StandState,
    gear: &NpcGearData,
) -> Result<Option<u16>, String> {
    if state.is_stand() {
        return Ok(None);
    }
    gear.pose_anim_id(&UnitPose {
        stand_state: state,
        ..UnitPose::default()
    })
}

/// The pose a player with stand pose `held` shows: the pose while it stands still, else
/// none, and moving ends the pose (`*held` is cleared) until the server's next stand
/// state, as the server stands a moving player up.
pub(crate) fn held_stand_pose(held: &mut Option<u16>, standing_still: bool) -> Option<u16> {
    if !standing_still {
        *held = None;
    }
    *held
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gear() -> NpcGearData {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/db2/12.1.0.69933");
        NpcGearData::load(&dir).unwrap()
    }

    #[test]
    fn x_sits_a_standing_player_and_stands_every_other_state() {
        assert_eq!(sit_or_stand(StandState::Stand), StandState::Sit);
        assert_eq!(sit_or_stand(StandState::Sit), StandState::Stand);
        assert_eq!(sit_or_stand(StandState::Sleep), StandState::Stand);
        assert_eq!(sit_or_stand(StandState::Kneel), StandState::Stand);
        assert_eq!(sit_or_stand(StandState::SitLowChair), StandState::Stand);
    }

    #[test]
    fn a_stand_pose_holds_until_the_player_moves() {
        let mut held = Some(97);
        assert_eq!(held_stand_pose(&mut held, true), Some(97));
        assert_eq!(held, Some(97));
        assert_eq!(held_stand_pose(&mut held, false), None);
        assert_eq!(held, None);
        assert_eq!(held_stand_pose(&mut held, true), None);
    }

    #[test]
    fn stand_state_emotes_ask_for_their_stand_state() {
        assert_eq!(emote_stand_state(EmoteKind::Sit), Some(StandState::Sit));
        assert_eq!(emote_stand_state(EmoteKind::Sleep), Some(StandState::Sleep));
        assert_eq!(emote_stand_state(EmoteKind::Kneel), Some(StandState::Kneel));
        assert_eq!(emote_stand_state(EmoteKind::Dance), None);
        assert_eq!(emote_stand_state(EmoteKind::Wave), None);
    }

    /// wowdev AnimationList: SitGround 97, Sleep 100, SitChairLow/Med/High 102-104,
    /// KneelLoop 115; standing holds nothing.
    #[test]
    fn stand_states_hold_their_retail_pose_clips() {
        let gear = gear();
        let anim = |state| stand_state_anim(state, &gear).unwrap();
        assert_eq!(anim(StandState::Stand), None);
        assert_eq!(anim(StandState::Sit), Some(97));
        assert_eq!(anim(StandState::Sleep), Some(100));
        assert_eq!(anim(StandState::SitLowChair), Some(102));
        assert_eq!(anim(StandState::SitMediumChair), Some(103));
        assert_eq!(anim(StandState::SitHighChair), Some(104));
        assert_eq!(anim(StandState::Kneel), Some(115));
    }
}
