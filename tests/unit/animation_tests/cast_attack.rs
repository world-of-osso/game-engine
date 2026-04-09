use super::*;

#[test]
fn cast_anim_directed_id() {
    assert_eq!(CastAnimKind::Directed.cast_anim_id(), 51);
}

#[test]
fn cast_anim_omni_id() {
    assert_eq!(CastAnimKind::Omni.cast_anim_id(), 52);
}

#[test]
fn cast_anim_channel_id() {
    assert_eq!(CastAnimKind::Channel.cast_anim_id(), 76);
}

#[test]
fn ready_anim_directed_id() {
    assert_eq!(CastAnimKind::Directed.ready_anim_id(), 55);
}

#[test]
fn ready_anim_omni_id() {
    assert_eq!(CastAnimKind::Omni.ready_anim_id(), 56);
}

#[test]
fn cast_anim_state_lifecycle() {
    let mut state = CastAnimState::new(CastAnimKind::Directed, 2.5);
    assert!(!state.is_finished());
    assert_eq!(state.kind, CastAnimKind::Directed);
    state.tick(1.0);
    assert!(!state.is_finished());
    state.tick(2.0);
    assert!(state.is_finished());
}

#[test]
fn cast_anim_state_tick_clamps() {
    let mut state = CastAnimState::new(CastAnimKind::Omni, 0.5);
    state.tick(999.0);
    assert_eq!(state.remaining, 0.0);
    assert!(state.is_finished());
}

#[test]
fn cast_anim_default_is_directed() {
    assert_eq!(CastAnimKind::default(), CastAnimKind::Directed);
}

#[test]
fn channel_is_looping() {
    assert!(CastAnimKind::Channel.is_looping());
    assert!(!CastAnimKind::Directed.is_looping());
    assert!(!CastAnimKind::Omni.is_looping());
}

#[test]
fn channel_state_should_loop_while_active() {
    let state = CastAnimState::channel(5.0);
    assert!(state.should_loop());
    assert_eq!(state.kind, CastAnimKind::Channel);
}

#[test]
fn channel_state_stops_looping_when_finished() {
    let mut state = CastAnimState::channel(0.5);
    state.tick(1.0);
    assert!(state.is_finished());
    assert!(!state.should_loop());
}

#[test]
fn directed_cast_does_not_loop() {
    let state = CastAnimState::new(CastAnimKind::Directed, 2.5);
    assert!(!state.should_loop());
}

#[test]
fn current_anim_id_cast_vs_hold() {
    let mut state = CastAnimState::new(CastAnimKind::Directed, 2.5);
    assert_eq!(state.current_anim_id(), ANIM_SPELL_CAST_DIRECTED);
    state.holding = true;
    assert_eq!(state.current_anim_id(), ANIM_READY_SPELL_DIRECTED);
}

#[test]
fn channel_current_anim_id_always_channel() {
    let state = CastAnimState::channel(5.0);
    assert_eq!(state.current_anim_id(), ANIM_CHANNEL);
    let mut held = CastAnimState::channel(5.0);
    held.holding = true;
    assert_eq!(held.current_anim_id(), ANIM_CHANNEL);
}

#[test]
fn attack_1h_anim_id() {
    assert_eq!(MeleeWeaponKind::OneHand.attack_anim_id(), ANIM_ATTACK_1H);
}

#[test]
fn attack_2h_anim_id() {
    assert_eq!(MeleeWeaponKind::TwoHand.attack_anim_id(), ANIM_ATTACK_2H);
}

#[test]
fn attack_off_anim_id() {
    assert_eq!(MeleeWeaponKind::OffHand.attack_anim_id(), ANIM_ATTACK_OFF);
}

#[test]
fn parry_1h_uses_1h_anim() {
    assert_eq!(MeleeWeaponKind::OneHand.parry_anim_id(), ANIM_PARRY_1H);
    assert_eq!(MeleeWeaponKind::OffHand.parry_anim_id(), ANIM_PARRY_1H);
}

#[test]
fn parry_2h_uses_2h_anim() {
    assert_eq!(MeleeWeaponKind::TwoHand.parry_anim_id(), ANIM_PARRY_2H);
}

#[test]
fn ready_stance_by_weapon() {
    assert_eq!(MeleeWeaponKind::OneHand.ready_anim_id(), ANIM_READY_1H);
    assert_eq!(MeleeWeaponKind::TwoHand.ready_anim_id(), ANIM_READY_2H);
    assert_eq!(MeleeWeaponKind::OffHand.ready_anim_id(), ANIM_READY_1H);
}

#[test]
fn attack_anim_state_lifecycle() {
    let mut state = AttackAnimState::new(MeleeWeaponKind::TwoHand, 2.0);
    assert!(!state.is_finished());
    assert_eq!(state.anim_id(), ANIM_ATTACK_2H);
    state.tick(1.0);
    assert!(!state.is_finished());
    state.tick(1.5);
    assert!(state.is_finished());
}

#[test]
fn attack_anim_ids_all_distinct() {
    let ids = [
        MeleeWeaponKind::OneHand.attack_anim_id(),
        MeleeWeaponKind::TwoHand.attack_anim_id(),
        MeleeWeaponKind::OffHand.attack_anim_id(),
    ];
    assert_ne!(ids[0], ids[1]);
    assert_ne!(ids[0], ids[2]);
    assert_ne!(ids[1], ids[2]);
}
