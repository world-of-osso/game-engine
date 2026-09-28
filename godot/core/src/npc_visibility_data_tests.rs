use crate::npc_visibility_data::{
    NpcSchedule, NpcVisibilityPolicy, npc_should_be_visible, npc_visibility_day_phase,
    npc_visibility_policy, schedule_is_active,
};

#[test]
fn authored_npc_template_policies_and_default() {
    assert_eq!(npc_visibility_policy(6491), NpcVisibilityPolicy::DeadOnly);
    assert_eq!(
        npc_visibility_policy(918),
        NpcVisibilityPolicy::Scheduled(NpcSchedule::NightOnly)
    );
    assert_eq!(
        npc_visibility_policy(12783),
        NpcVisibilityPolicy::Scheduled(NpcSchedule::DayOnly)
    );
    for id in [
        32820, 26724, 26738, 26739, 26740, 26741, 26742, 26743, 26744, 26745, 26747, 26748, 26749,
        26750, 26751, 26752, 26753, 26754, 26755, 26756, 26757, 26758, 26759, 26765, 33252,
    ] {
        assert_eq!(
            npc_visibility_policy(id),
            NpcVisibilityPolicy::Hidden,
            "{id}"
        );
    }
    for id in [0, 26737, 26746, 26760, 33253] {
        assert_eq!(
            npc_visibility_policy(id),
            NpcVisibilityPolicy::Always,
            "{id}"
        );
    }
}

#[test]
fn day_night_boundary_is_half_open_and_night_is_complement() {
    for (minutes, day) in [
        (0.0, false),
        (719.9, false),
        (720.0, true),
        (1440.0, true),
        (2159.9, true),
        (2160.0, false),
        (2879.9, false),
    ] {
        assert_eq!(schedule_is_active(NpcSchedule::DayOnly, minutes), day);
        assert_eq!(schedule_is_active(NpcSchedule::NightOnly, minutes), !day);
        assert_eq!(
            npc_visibility_day_phase(minutes),
            if day {
                crate::npc_visibility_data::NpcVisibilityDayPhase::Day
            } else {
                crate::npc_visibility_data::NpcVisibilityDayPhase::Night
            }
        );
        assert_eq!(
            npc_should_be_visible(npc_visibility_policy(12783), true, minutes),
            day
        );
        assert_eq!(
            npc_should_be_visible(npc_visibility_policy(918), true, minutes),
            !day
        );
    }
}

#[test]
fn dead_only_depends_on_local_alive_not_clock() {
    for minutes in [0.0, 720.0, 2160.0] {
        assert!(!npc_should_be_visible(
            npc_visibility_policy(6491),
            true,
            minutes
        ));
        assert!(npc_should_be_visible(
            npc_visibility_policy(6491),
            false,
            minutes
        ));
        assert!(!npc_should_be_visible(
            NpcVisibilityPolicy::Hidden,
            false,
            minutes
        ));
        assert!(npc_should_be_visible(
            NpcVisibilityPolicy::Always,
            true,
            minutes
        ));
    }
}
