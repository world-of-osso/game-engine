//! Unit-frame aura state changes only with what the frames draw: a ticking countdown
//! must not change the screen state, which would resync the unit frames every frame.

use game_engine_ui_model::aura_display_data::{AuraInstance, DebuffType};
use game_engine_ui_model::inworld_unit_frames_component::{
    TargetAuraView, UnitFrameState, player_aura_icons, set_target_auras,
};

/// A one-hour follower buff (Power Word: Fortitude) with `remaining` seconds left.
fn fortitude(remaining: f32) -> AuraInstance {
    AuraInstance {
        instance_id: 7,
        spell_id: 21562,
        name: "Power Word: Fortitude".into(),
        description: String::new(),
        icon_fdid: 135987,
        source: "Follower2".into(),
        from_local_player: false,
        from_player: false,
        duration: 3600.0,
        remaining,
        stacks: 1,
        is_debuff: false,
        debuff_type: DebuffType::Magic,
    }
}

#[test]
fn player_aura_state_changes_with_its_timer_text_only() {
    // Two frames 20 ms apart both read "60 m".
    let earlier = player_aura_icons(&[fortitude(3598.46)]);
    assert_eq!(earlier, player_aura_icons(&[fortitude(3598.44)]));
    assert_eq!(earlier[0].timer, "60 m");
    let (late, later) = (
        player_aura_icons(&[fortitude(89.4)]),
        player_aura_icons(&[fortitude(88.6)]),
    );
    assert_eq!(
        (late[0].timer.as_str(), later[0].timer.as_str()),
        ("89 s", "88 s")
    );
    assert_ne!(late, later);
}

#[test]
fn target_aura_state_holds_while_its_swipe_advances() {
    let view = TargetAuraView {
        player_is_target: false,
        friendly: true,
        hostile_npc: false,
    };
    let mut before = UnitFrameState::named("Garrick");
    let mut after = before.clone();
    let (before_swipes, _) = set_target_auras(&mut before, &[fortitude(1800.0)], view);
    let (after_swipes, _) = set_target_auras(&mut after, &[fortitude(1799.0)], view);
    assert_eq!(before, after);
    assert!(before.target_buffs[0].timed);
    assert_eq!(before_swipes, vec![Some(0.5)]);
    assert!(after_swipes[0].is_some_and(|elapsed| elapsed > 0.5));
}
