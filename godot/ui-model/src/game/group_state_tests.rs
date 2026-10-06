use shared::components::{Position, PowerEntry, PowerType};
use shared::death::DeathState;
use shared::loot::LootMode;
use shared::protocol::ReadyCheckMemberSnapshot;

use super::*;

fn member(name: &str, leader: bool, online: bool) -> GroupMemberSnapshot {
    GroupMemberSnapshot {
        name: name.into(),
        role: GroupRoleSnapshot::None,
        is_leader: leader,
        online,
        subgroup: 1,
        class: 2,
        level: 12,
        entity: online.then_some(7),
        portrait: Default::default(),
    }
}

fn roster(is_raid: bool, members: Vec<GroupMemberSnapshot>) -> GroupRosterSnapshot {
    GroupRosterSnapshot {
        is_raid,
        ready_count: 0,
        total_count: 0,
        members,
        loot_method: LootMode::PersonalLoot,
    }
}

fn live(name: &str, health: u32) -> GroupMemberState {
    GroupMemberState {
        name: name.into(),
        health,
        max_health: 1000,
        power: Some(PowerEntry {
            power: PowerType::Mana,
            current: 50,
            max: 100,
            partial: 0,
            regen_per_sec: 0.0,
        }),
        death: DeathState::Alive,
        position: Position {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        debuffs: Vec::new(),
    }
}

fn party_of(names: &[(&str, bool)]) -> GroupState {
    let mut state = GroupState::default();
    state.apply_roster(roster(
        false,
        names
            .iter()
            .map(|(name, leader)| member(name, *leader, true))
            .collect(),
    ));
    state
}

fn ready_update(answers: &[(&str, ReadyCheckAnswer)], finished: bool) -> ReadyCheckUpdate {
    ReadyCheckUpdate {
        initiator_name: "Ann".into(),
        time_remaining_secs: 20.0,
        members: answers
            .iter()
            .map(|(name, answer)| ReadyCheckMemberSnapshot {
                name: (*name).into(),
                answer: *answer,
            })
            .collect(),
        finished,
    }
}

#[test]
fn member_states_are_kept_for_roster_members_and_dropped_when_they_go_offline() {
    let mut state = party_of(&[("Ann", true), ("Bob", false)]);
    state.apply_member_states(vec![live("Ann", 900), live("Bob", 412)]);
    assert_eq!(state.live.len(), 2);
    assert_eq!(state.live["Bob"].health, 412);

    state.apply_roster(roster(
        false,
        vec![member("Ann", true, true), member("Bob", false, false)],
    ));

    assert!(state.live.contains_key("Ann"));
    assert!(!state.live.contains_key("Bob"));
}

/// Live: Zed accepts; the server's tick queues Zed's state ahead of the roster that adds
/// Zed and sends it only once. The state waits for the roster instead of being lost; a
/// name the next roster does not list is dropped.
#[test]
fn a_state_that_arrives_before_its_roster_is_kept_until_the_roster_decides() {
    let mut state = party_of(&[("Ann", true)]);
    state.apply_member_states(vec![live("Zed", 412), live("Yan", 7)]);
    state.apply_roster(roster(
        false,
        vec![member("Ann", true, true), member("Zed", false, true)],
    ));
    assert_eq!(state.live["Zed"].health, 412);
    assert!(!state.live.contains_key("Yan"));
}

#[test]
fn leaving_the_group_clears_members_live_state_and_ready_check() {
    let mut state = party_of(&[("Ann", true), ("Bob", false)]);
    state.apply_member_states(vec![live("Ann", 900)]);
    state.apply_ready_check(ready_update(&[("Ann", ReadyCheckAnswer::Ready)], false));

    state.apply_roster(roster(false, Vec::new()));

    assert!(!state.in_group());
    assert!(state.live.is_empty());
    assert_eq!(state.ready_check, None);
}

#[test]
fn ready_marks_follow_answers_and_unanswered_turn_not_ready_when_finished() {
    let mut state = party_of(&[("Ann", true), ("Bob", false), ("Cid", false)]);
    let answers = [
        ("Ann", ReadyCheckAnswer::Ready),
        ("Bob", ReadyCheckAnswer::Pending),
        ("Cid", ReadyCheckAnswer::NotReady),
    ];
    state.apply_ready_check(ready_update(&answers, false));
    assert_eq!(state.ready_mark("Ann"), Some(ReadyMark::Ready));
    assert_eq!(state.ready_mark("Bob"), Some(ReadyMark::Waiting));
    assert_eq!(state.ready_mark("Cid"), Some(ReadyMark::NotReady));
    assert!(state.awaits_ready_answer("Bob"));
    assert!(!state.awaits_ready_answer("Ann"));

    state.apply_ready_check(ready_update(&answers, true));

    assert_eq!(state.ready_mark("Bob"), Some(ReadyMark::NotReady));
    assert!(!state.awaits_ready_answer("Bob"));
}

#[test]
fn finished_ready_check_marks_decay_after_eleven_seconds() {
    let mut state = party_of(&[("Ann", true)]);
    state.apply_ready_check(ready_update(&[("Ann", ReadyCheckAnswer::Pending)], false));
    state.tick_ready_check(40.0);
    assert!(
        state.ready_check.is_some(),
        "a running check does not decay"
    );

    state.apply_ready_check(ready_update(&[("Ann", ReadyCheckAnswer::Ready)], true));
    state.tick_ready_check(10.9);
    assert_eq!(state.ready_mark("Ann"), Some(ReadyMark::Ready));
    state.tick_ready_check(0.2);

    assert_eq!(state.ready_mark("Ann"), None);
}

#[test]
fn ungrouped_player_can_invite_others_and_has_no_self_entries() {
    let state = GroupState::default();

    assert_eq!(
        group_menu_entries(&state, "Ann", "Bob"),
        [GroupMenuEntry::Invite]
    );
    assert!(group_menu_entries(&state, "Ann", "Ann").is_empty());
}

#[test]
fn party_leader_menus_offer_member_management_and_convert_to_raid() {
    let state = party_of(&[("Ann", true), ("Bob", false)]);

    let for_bob = group_menu_entries(&state, "Ann", "bob");
    assert_eq!(
        for_bob[..2],
        [GroupMenuEntry::Promote, GroupMenuEntry::Uninvite]
    );
    assert!(for_bob.contains(&GroupMenuEntry::SetRole(GroupRoleSnapshot::Healer)));
    let for_self = group_menu_entries(&state, "Ann", "Ann");
    assert!(for_self.contains(&GroupMenuEntry::ConvertToRaid));
    assert!(for_self.contains(&GroupMenuEntry::ReadyCheck));
    assert_eq!(for_self.last(), Some(&GroupMenuEntry::Leave));
    assert_eq!(
        group_menu_entries(&state, "Ann", "Cid"),
        [GroupMenuEntry::Invite]
    );
}

#[test]
fn party_member_menus_only_offer_own_role_and_leave() {
    let state = party_of(&[("Ann", true), ("Bob", false)]);

    assert!(group_menu_entries(&state, "Bob", "Ann").is_empty());
    assert!(group_menu_entries(&state, "Bob", "Cid").is_empty());
    let for_self = group_menu_entries(&state, "Bob", "Bob");
    assert!(!for_self.contains(&GroupMenuEntry::ConvertToRaid));
    assert!(!for_self.contains(&GroupMenuEntry::ReadyCheck));
    assert_eq!(
        for_self,
        [
            GroupMenuEntry::SetRole(GroupRoleSnapshot::Tank),
            GroupMenuEntry::SetRole(GroupRoleSnapshot::Healer),
            GroupMenuEntry::SetRole(GroupRoleSnapshot::Damage),
            GroupMenuEntry::SetRole(GroupRoleSnapshot::None),
            GroupMenuEntry::Leave,
        ]
    );
}

#[test]
fn small_raid_leader_can_convert_back_but_not_a_six_member_raid() {
    let mut state = GroupState::default();
    state.apply_roster(roster(
        true,
        vec![member("Ann", true, true), member("Bob", false, true)],
    ));
    assert!(group_menu_entries(&state, "Ann", "Ann").contains(&GroupMenuEntry::ConvertToParty));

    let six = ["Ann", "B", "C", "D", "E", "F"]
        .iter()
        .enumerate()
        .map(|(i, name)| member(name, i == 0, true))
        .collect();
    state.apply_roster(roster(true, six));
    let entries = group_menu_entries(&state, "Ann", "Ann");
    assert!(!entries.contains(&GroupMenuEntry::ConvertToParty));
    assert!(!entries.contains(&GroupMenuEntry::ConvertToRaid));
}

#[test]
fn menu_actions_round_trip_to_commands_for_the_menu_unit() {
    for entry in ALL_ENTRIES {
        assert_eq!(GroupMenuEntry::from_action(entry.action()), Some(entry));
    }
    assert_eq!(GroupMenuEntry::from_action("unit_menu_close"), None);
    assert_eq!(
        GroupMenuEntry::Promote.command("Bob"),
        GroupCommand::Promote("Bob".into())
    );
    assert_eq!(
        GroupMenuEntry::SetRole(GroupRoleSnapshot::Tank).command("Ann"),
        GroupCommand::SetRole {
            name: "Ann".into(),
            role: GroupRoleSnapshot::Tank,
        }
    );
    assert_eq!(GroupMenuEntry::Leave.command("Ann"), GroupCommand::Leave);
}
