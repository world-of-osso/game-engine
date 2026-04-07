use super::*;
use ui_toolkit::layout::{LayoutRect, recompute_layouts};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn member(
    name: &str,
    hp: u32,
    max: u32,
    role: PartyRole,
    debuffs: Vec<PartyDebuff>,
    in_range: bool,
    rc: ReadyCheckState,
    heals: f32,
) -> PartyMemberState {
    PartyMemberState {
        name: name.into(),
        health_current: hp,
        health_max: max,
        role,
        debuffs,
        online: true,
        in_range,
        ready_check: rc,
        incoming_heals: heals,
    }
}

fn debuff(name: &str, fdid: u32) -> PartyDebuff {
    PartyDebuff {
        name: name.into(),
        icon_fdid: fdid,
    }
}

fn sample_members() -> Vec<PartyMemberState> {
    vec![
        member(
            "Tankadin",
            45000,
            50000,
            PartyRole::Tank,
            vec![debuff("Bleed", 1)],
            true,
            ReadyCheckState::Accepted,
            0.1,
        ),
        member(
            "Healbot",
            30000,
            35000,
            PartyRole::Healer,
            vec![],
            true,
            ReadyCheckState::None,
            0.0,
        ),
        member(
            "Stabsworth",
            28000,
            32000,
            PartyRole::Dps,
            vec![debuff("Poison", 2), debuff("Curse", 3)],
            false,
            ReadyCheckState::Pending,
            0.0,
        ),
        member(
            "Pewpew",
            0,
            30000,
            PartyRole::Dps,
            vec![],
            true,
            ReadyCheckState::Declined,
            0.0,
        ),
    ]
}

fn build_registry() -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(PartyFrameState {
        visible: true,
        members: sample_members(),
    });
    Screen::new(party_frame_screen).sync(&shared, &mut reg);
    reg
}

fn layout_registry() -> FrameRegistry {
    let mut reg = build_registry();
    recompute_layouts(&mut reg);
    reg
}

fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
    reg.get(reg.get_by_name(name).expect(name))
        .and_then(|f| f.layout_rect.clone())
        .unwrap_or_else(|| panic!("{name} has no layout_rect"))
}

// --- Structure tests ---

#[test]
fn builds_party_frame() {
    let reg = build_registry();
    assert!(reg.get_by_name("PartyFrame").is_some());
}

#[test]
fn builds_four_member_frames() {
    let reg = build_registry();
    for i in 0..4 {
        assert!(
            reg.get_by_name(&format!("PartyMember{i}")).is_some(),
            "PartyMember{i} missing"
        );
    }
}

#[test]
fn builds_member_sub_elements() {
    let reg = build_registry();
    for i in 0..4 {
        assert!(reg.get_by_name(&format!("PartyMember{i}Name")).is_some());
        assert!(reg.get_by_name(&format!("PartyMember{i}Role")).is_some());
        assert!(
            reg.get_by_name(&format!("PartyMember{i}RoleLabel"))
                .is_some()
        );
        assert!(
            reg.get_by_name(&format!("PartyMember{i}HealthBg"))
                .is_some()
        );
        assert!(
            reg.get_by_name(&format!("PartyMember{i}HealthFill"))
                .is_some()
        );
        assert!(
            reg.get_by_name(&format!("PartyMember{i}HealthText"))
                .is_some()
        );
    }
}

#[test]
fn builds_debuff_icons() {
    let reg = build_registry();
    // Member 0 has 1 debuff, member 2 has 2
    assert!(reg.get_by_name("PartyMember0Debuff0").is_some());
    assert!(reg.get_by_name("PartyMember0Debuff1").is_none());
    assert!(reg.get_by_name("PartyMember2Debuff0").is_some());
    assert!(reg.get_by_name("PartyMember2Debuff1").is_some());
    assert!(reg.get_by_name("PartyMember2Debuff2").is_none());
}

#[test]
fn hidden_when_not_visible() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(PartyFrameState::default());
    Screen::new(party_frame_screen).sync(&shared, &mut reg);
    let id = reg.get_by_name("PartyFrame").expect("frame");
    assert!(reg.get(id).expect("data").hidden);
}

// --- Data model tests ---

#[test]
fn health_fraction() {
    let member = &sample_members()[0];
    assert!((member.health_fraction() - 0.9).abs() < 0.01);

    let dead = &sample_members()[3];
    assert_eq!(dead.health_fraction(), 0.0);
}

#[test]
fn health_fraction_zero_max() {
    let member = PartyMemberState {
        health_max: 0,
        health_current: 0,
        ..sample_members()[0].clone()
    };
    assert_eq!(member.health_fraction(), 0.0);
}

#[test]
fn health_text_format() {
    let member = &sample_members()[0];
    assert_eq!(member.health_text(), "45000/50000");
}

#[test]
fn role_labels_and_colors() {
    assert_eq!(PartyRole::Tank.label(), "T");
    assert_eq!(PartyRole::Healer.label(), "H");
    assert_eq!(PartyRole::Dps.label(), "D");
    // Colors are non-empty strings
    assert!(!PartyRole::Tank.color().is_empty());
    assert!(!PartyRole::Healer.color().is_empty());
    assert!(!PartyRole::Dps.color().is_empty());
}

// --- Coord validation ---

#[test]
fn coord_party_frame_top_left() {
    let reg = layout_registry();
    let r = rect(&reg, "PartyFrame");
    assert!((r.x - ANCHOR_X).abs() < 1.0);
    assert!((r.y - ANCHOR_Y).abs() < 1.0);
    assert!((r.width - UNIT_W).abs() < 1.0);
}

#[test]
fn coord_members_stacked_vertically() {
    let reg = layout_registry();
    let frame_r = rect(&reg, "PartyFrame");
    for i in 0..4 {
        let r = rect(&reg, &format!("PartyMember{i}"));
        let expected_y = frame_r.y + i as f32 * (UNIT_H + UNIT_GAP);
        assert!(
            (r.y - expected_y).abs() < 1.0,
            "PartyMember{i} y: expected {expected_y}, got {}",
            r.y
        );
        assert!((r.width - UNIT_W).abs() < 1.0);
        assert!((r.height - UNIT_H).abs() < 1.0);
    }
}

#[test]
fn coord_health_bar_inside_unit() {
    let reg = layout_registry();
    let unit_r = rect(&reg, "PartyMember0");
    let bar_r = rect(&reg, "PartyMember0HealthBg");
    assert!((bar_r.x - (unit_r.x + BAR_INSET)).abs() < 1.0);
    assert!((bar_r.y - (unit_r.y + BAR_Y)).abs() < 1.0);
    assert!((bar_r.width - BAR_W).abs() < 1.0);
    assert!((bar_r.height - BAR_H).abs() < 1.0);
}

#[test]
fn coord_role_icon_top_left_of_unit() {
    let reg = layout_registry();
    let unit_r = rect(&reg, "PartyMember0");
    let role_r = rect(&reg, "PartyMember0Role");
    assert!((role_r.x - (unit_r.x + ROLE_ICON_INSET)).abs() < 1.0);
    assert!((role_r.y - unit_r.y).abs() < 1.0);
    assert!((role_r.width - ROLE_ICON_SIZE).abs() < 1.0);
}

#[test]
fn coord_name_right_of_role() {
    let reg = layout_registry();
    let unit_r = rect(&reg, "PartyMember0");
    let name_r = rect(&reg, "PartyMember0Name");
    let expected_x = unit_r.x + NAME_INSET_X;
    assert!((name_r.x - expected_x).abs() < 1.0);
    assert!((name_r.y - unit_r.y).abs() < 1.0);
}

#[test]
fn coord_debuff_below_health_bar() {
    let reg = layout_registry();
    let unit_r = rect(&reg, "PartyMember0");
    let debuff_r = rect(&reg, "PartyMember0Debuff0");
    let expected_y = unit_r.y + DEBUFF_Y;
    assert!((debuff_r.y - expected_y).abs() < 1.0);
    assert!((debuff_r.width - DEBUFF_ICON_SIZE).abs() < 1.0);
}

#[test]
fn coord_health_fill_proportional() {
    let reg = layout_registry();
    let fill_r = rect(&reg, "PartyMember0HealthFill");
    // Member 0: 45000/50000 = 0.9
    let expected_w = 0.9 * BAR_W;
    assert!((fill_r.width - expected_w).abs() < 1.0);
}

// --- Ready check tests ---

#[test]
fn builds_ready_check_icons() {
    let reg = build_registry();
    for i in 0..4 {
        assert!(
            reg.get_by_name(&format!("PartyMember{i}ReadyCheck"))
                .is_some()
        );
    }
}

#[test]
fn ready_check_hidden_when_none() {
    let reg = build_registry();
    // Member 1 has ReadyCheckState::None
    let id = reg.get_by_name("PartyMember1ReadyCheck").expect("rc");
    assert!(reg.get(id).expect("data").hidden);
}

#[test]
fn ready_check_visible_when_active() {
    let reg = build_registry();
    // Member 0 has ReadyCheckState::Accepted
    let id = reg.get_by_name("PartyMember0ReadyCheck").expect("rc");
    assert!(!reg.get(id).expect("data").hidden);
}

#[test]
fn ready_check_state_labels() {
    assert_eq!(ReadyCheckState::Accepted.label(), "✓");
    assert_eq!(ReadyCheckState::Pending.label(), "?");
    assert_eq!(ReadyCheckState::Declined.label(), "✗");
    assert_eq!(ReadyCheckState::None.label(), "");
}

// --- Range fade tests ---

#[test]
fn builds_range_fade_overlays() {
    let reg = build_registry();
    for i in 0..4 {
        assert!(
            reg.get_by_name(&format!("PartyMember{i}RangeFade"))
                .is_some()
        );
    }
}

#[test]
fn range_fade_hidden_when_in_range() {
    let reg = build_registry();
    // Member 0 is in_range=true → fade hidden
    let id = reg.get_by_name("PartyMember0RangeFade").expect("fade");
    assert!(reg.get(id).expect("data").hidden);
}

#[test]
fn range_fade_visible_when_out_of_range() {
    let reg = build_registry();
    // Member 2 is in_range=false → fade visible
    let id = reg.get_by_name("PartyMember2RangeFade").expect("fade");
    assert!(!reg.get(id).expect("data").hidden);
}

// --- Incoming heals tests ---

#[test]
fn builds_incoming_heals_overlay() {
    let reg = build_registry();
    for i in 0..4 {
        assert!(
            reg.get_by_name(&format!("PartyMember{i}IncomingHeal"))
                .is_some()
        );
    }
}

#[test]
fn incoming_heals_hidden_when_zero() {
    let reg = build_registry();
    // Member 1 has incoming_heals=0.0
    let id = reg.get_by_name("PartyMember1IncomingHeal").expect("heal");
    assert!(reg.get(id).expect("data").hidden);
}

#[test]
fn incoming_heals_visible_when_nonzero() {
    let reg = build_registry();
    // Member 0 has incoming_heals=0.1
    let id = reg.get_by_name("PartyMember0IncomingHeal").expect("heal");
    assert!(!reg.get(id).expect("data").hidden);
}

#[test]
fn coord_incoming_heals_after_health() {
    let reg = layout_registry();
    let bar_r = rect(&reg, "PartyMember0HealthBg");
    let heal_r = rect(&reg, "PartyMember0IncomingHeal");
    // Member 0: health=0.9, heals=0.1 → heal starts at 0.9*BAR_W
    let expected_x = bar_r.x + 0.9 * BAR_W;
    assert!((heal_r.x - expected_x).abs() < 1.0);
    let expected_w = 0.1 * BAR_W;
    assert!((heal_r.width - expected_w).abs() < 1.0);
}

#[test]
fn coord_range_fade_covers_unit() {
    let reg = layout_registry();
    // Member 2 is out of range — fade covers the entire unit frame
    let unit_r = rect(&reg, "PartyMember2");
    let fade_r = rect(&reg, "PartyMember2RangeFade");
    assert!((fade_r.x - unit_r.x).abs() < 1.0);
    assert!((fade_r.y - unit_r.y).abs() < 1.0);
    assert!((fade_r.width - UNIT_W).abs() < 1.0);
    assert!((fade_r.height - UNIT_H).abs() < 1.0);
}

#[test]
fn coord_ready_check_top_right() {
    let reg = layout_registry();
    // Member 0 has Accepted ready check
    let unit_r = rect(&reg, "PartyMember0");
    let rc_r = rect(&reg, "PartyMember0ReadyCheck");
    // Anchored top-right of unit, offset -2px inward
    let expected_right = unit_r.x + unit_r.width;
    assert!((rc_r.x + rc_r.width - expected_right).abs() < 3.0);
    assert!((rc_r.y - unit_r.y).abs() < 1.0);
    assert!((rc_r.width - READY_CHECK_SIZE).abs() < 1.0);
}
