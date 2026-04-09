use super::*;

#[test]
fn health_fraction() {
    assert!((unit(90, 100).health_fraction() - 0.9).abs() < 0.01);
    assert_eq!(unit(0, 0).health_fraction(), 0.0);
    assert!((unit(100, 100).health_fraction() - 1.0).abs() < 0.01);
}

#[test]
fn power_fraction() {
    let u = GroupUnitState {
        power_current: 50,
        power_max: 200,
        ..unit(100, 100)
    };
    assert!((u.power_fraction() - 0.25).abs() < 0.01);
    assert_eq!(unit(100, 100).power_fraction(), 0.0);
}

#[test]
fn incoming_heals_fraction() {
    let u = GroupUnitState {
        health_current: 70,
        health_max: 100,
        incoming_heals: 20,
        ..unit(70, 100)
    };
    assert!((u.incoming_heals_fraction() - 0.2).abs() < 0.01);

    let over = GroupUnitState {
        incoming_heals: 50,
        ..u
    };
    assert!((over.incoming_heals_fraction() - 0.3).abs() < 0.01);

    let full = GroupUnitState {
        health_current: 100,
        incoming_heals: 10,
        ..unit(100, 100)
    };
    assert_eq!(full.incoming_heals_fraction(), 0.0);
}

#[test]
fn health_text_format() {
    assert_eq!(unit(450, 1000).health_text(), "450/1000");
}

#[test]
fn is_dead() {
    assert!(unit(0, 100).is_dead());
    assert!(!unit(1, 100).is_dead());
    assert!(!unit(0, 0).is_dead());
}

#[test]
fn party_member_count() {
    let state = PartyState {
        members: vec![unit(100, 100), unit(50, 100)],
        ..Default::default()
    };
    assert_eq!(state.member_count(), 2);
}

#[test]
fn party_all_ready() {
    let ready = PartyState {
        members: vec![
            GroupUnitState {
                ready_check: ReadyCheck::Accepted,
                ..unit(100, 100)
            },
            GroupUnitState {
                ready_check: ReadyCheck::Accepted,
                ..unit(100, 100)
            },
        ],
        ready_check_active: true,
        ..Default::default()
    };
    assert!(ready.all_ready());

    let not_ready = PartyState {
        members: vec![
            GroupUnitState {
                ready_check: ReadyCheck::Accepted,
                ..unit(100, 100)
            },
            GroupUnitState {
                ready_check: ReadyCheck::Pending,
                ..unit(100, 100)
            },
        ],
        ready_check_active: true,
        ..Default::default()
    };
    assert!(!not_ready.all_ready());
}

#[test]
fn raid_total_members() {
    let state = RaidState {
        groups: vec![
            RaidGroupData {
                members: vec![unit(100, 100), unit(100, 100)],
            },
            RaidGroupData {
                members: vec![unit(100, 100)],
            },
        ],
        ..Default::default()
    };
    assert_eq!(state.total_members(), 3);
}

#[test]
fn raid_alive_count() {
    let state = RaidState {
        groups: vec![RaidGroupData {
            members: vec![unit(100, 100), unit(0, 100), unit(50, 100)],
        }],
        ..Default::default()
    };
    assert_eq!(state.alive_count(), 2);
}

#[test]
fn raid_all_ready() {
    let state = RaidState {
        groups: vec![RaidGroupData {
            members: vec![GroupUnitState {
                ready_check: ReadyCheck::Accepted,
                ..unit(100, 100)
            }],
        }],
        ready_check_active: true,
    };
    assert!(state.all_ready());
}

#[test]
fn party_assign_role() {
    let mut state = PartyState {
        members: vec![named_unit("Alice"), named_unit("Bob")],
        ..Default::default()
    };
    assert!(state.assign_role("Alice", GroupRole::Tank));
    assert_eq!(state.members[0].role, GroupRole::Tank);
    assert_eq!(state.members[1].role, GroupRole::Dps);
}

#[test]
fn party_assign_role_not_found() {
    let mut state = PartyState {
        members: vec![named_unit("Alice")],
        ..Default::default()
    };
    assert!(!state.assign_role("Unknown", GroupRole::Healer));
}

#[test]
fn party_role_count() {
    let mut state = PartyState {
        members: vec![named_unit("A"), named_unit("B"), named_unit("C")],
        ..Default::default()
    };
    state.assign_role("A", GroupRole::Tank);
    state.assign_role("B", GroupRole::Healer);
    assert_eq!(state.role_count(GroupRole::Tank), 1);
    assert_eq!(state.role_count(GroupRole::Healer), 1);
    assert_eq!(state.role_count(GroupRole::Dps), 1);
}
