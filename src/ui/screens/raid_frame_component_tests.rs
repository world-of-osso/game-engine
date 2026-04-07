use super::*;
use ui_toolkit::layout::{LayoutRect, recompute_layouts};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn rm(name: &str, hp: u32, max: u32, in_range: bool, rc: RaidReadyCheck, heals: f32) -> RaidMember {
    RaidMember {
        name: name.into(),
        health_current: hp,
        health_max: max,
        alive: hp > 0,
        in_range,
        ready_check: rc,
        incoming_heals: heals,
    }
}

fn sample_groups() -> Vec<RaidGroup> {
    vec![
        RaidGroup {
            members: vec![
                rm("Tank1", 50000, 50000, true, RaidReadyCheck::Accepted, 0.0),
                rm("Healer1", 30000, 35000, false, RaidReadyCheck::None, 0.15),
            ],
        },
        RaidGroup {
            members: vec![rm("Dps1", 5000, 40000, true, RaidReadyCheck::Pending, 0.0)],
        },
        RaidGroup::default(),
    ]
}

fn build_registry() -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(RaidFrameState {
        visible: true,
        groups: sample_groups(),
    });
    Screen::new(raid_frame_screen).sync(&shared, &mut reg);
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
fn builds_raid_frame() {
    let reg = build_registry();
    assert!(reg.get_by_name("RaidFrame").is_some());
}

#[test]
fn builds_group_labels() {
    let reg = build_registry();
    for gi in 0..NUM_GROUPS {
        assert!(
            reg.get_by_name(&format!("RaidGroup{gi}Label")).is_some(),
            "RaidGroup{gi}Label missing"
        );
    }
}

#[test]
fn builds_all_cells() {
    let reg = build_registry();
    for gi in 0..NUM_GROUPS {
        for mi in 0..MEMBERS_PER_GROUP {
            assert!(
                reg.get_by_name(&format!("RaidCell{gi}_{mi}")).is_some(),
                "RaidCell{gi}_{mi} missing"
            );
        }
    }
}

#[test]
fn filled_cells_have_fill_and_name() {
    let reg = build_registry();
    assert!(reg.get_by_name("RaidCell0_0Fill").is_some());
    assert!(reg.get_by_name("RaidCell0_0Name").is_some());
    assert!(reg.get_by_name("RaidCell0_1Fill").is_some());
    assert!(reg.get_by_name("RaidCell0_1Name").is_some());
    assert!(reg.get_by_name("RaidCell0_2Fill").is_none());
    assert!(reg.get_by_name("RaidCell0_2Name").is_none());
}

#[test]
fn empty_group_has_only_empty_cells() {
    let reg = build_registry();
    assert!(reg.get_by_name("RaidCell2_0").is_some());
    assert!(reg.get_by_name("RaidCell2_0Fill").is_none());
}

#[test]
fn hidden_when_not_visible() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(RaidFrameState::default());
    Screen::new(raid_frame_screen).sync(&shared, &mut reg);
    let id = reg.get_by_name("RaidFrame").expect("frame");
    assert!(reg.get(id).expect("data").hidden);
}

// --- Data model tests ---

fn member(hp: u32, max: u32) -> RaidMember {
    RaidMember {
        name: "A".into(),
        health_current: hp,
        health_max: max,
        alive: true,
        in_range: true,
        ready_check: RaidReadyCheck::None,
        incoming_heals: 0.0,
    }
}

#[test]
fn health_fraction() {
    assert!((member(100, 100).health_fraction() - 1.0).abs() < 0.01);
    assert!((member(50, 100).health_fraction() - 0.5).abs() < 0.01);
    assert_eq!(member(0, 0).health_fraction(), 0.0);
}

// --- Coord validation ---

#[test]
fn coord_raid_frame_position() {
    let reg = layout_registry();
    let r = rect(&reg, "RaidFrame");
    assert!((r.x - ANCHOR_X).abs() < 1.0);
    assert!((r.y - ANCHOR_Y).abs() < 1.0);
    assert!((r.width - GRID_W).abs() < 1.0);
    assert!((r.height - GRID_H).abs() < 1.0);
}

#[test]
fn coord_group_labels_spaced() {
    let reg = layout_registry();
    let frame_r = rect(&reg, "RaidFrame");
    for gi in 0..NUM_GROUPS {
        let r = rect(&reg, &format!("RaidGroup{gi}Label"));
        let expected_x = frame_r.x + gi as f32 * (CELL_W + GROUP_GAP);
        assert!(
            (r.x - expected_x).abs() < 1.0,
            "RaidGroup{gi}Label x: expected {expected_x}, got {}",
            r.x
        );
        assert!((r.y - frame_r.y).abs() < 1.0);
        assert!((r.width - CELL_W).abs() < 1.0);
    }
}

#[test]
fn coord_cells_in_grid() {
    let reg = layout_registry();
    let frame_r = rect(&reg, "RaidFrame");
    let r00 = rect(&reg, "RaidCell0_0");
    assert!((r00.x - frame_r.x).abs() < 1.0);
    assert!((r00.y - (frame_r.y + GROUP_LABEL_H)).abs() < 1.0);
    assert!((r00.width - CELL_W).abs() < 1.0);
    assert!((r00.height - CELL_H).abs() < 1.0);
    let r40 = rect(&reg, "RaidCell4_0");
    let expected_x4 = frame_r.x + 4.0 * (CELL_W + GROUP_GAP);
    assert!((r40.x - expected_x4).abs() < 1.0);
    let r07 = rect(&reg, "RaidCell0_7");
    let expected_y7 = frame_r.y + GROUP_LABEL_H + 7.0 * (CELL_H + CELL_GAP);
    assert!((r07.y - expected_y7).abs() < 1.0);
}

#[test]
fn coord_fill_inside_cell() {
    let reg = layout_registry();
    let cell_r = rect(&reg, "RaidCell0_0");
    let fill_r = rect(&reg, "RaidCell0_0Fill");
    assert!((fill_r.x - (cell_r.x + FILL_INSET)).abs() < 1.0);
    assert!((fill_r.y - (cell_r.y + FILL_INSET)).abs() < 1.0);
    assert!((fill_r.height - FILL_H).abs() < 1.0);
    let expected_fill_w = CELL_W - 2.0 * FILL_INSET;
    assert!((fill_r.width - expected_fill_w).abs() < 1.0);
}

#[test]
fn coord_low_health_fill_width() {
    let reg = layout_registry();
    let fill_r = rect(&reg, "RaidCell1_0Fill");
    let expected_w = 0.125 * (CELL_W - 2.0 * FILL_INSET);
    assert!((fill_r.width - expected_w).abs() < 1.0);
}

// --- Overlay tests ---

#[test]
fn builds_overlay_elements_for_filled_cells() {
    let reg = build_registry();
    assert!(reg.get_by_name("RaidCell0_0Heal").is_some());
    assert!(reg.get_by_name("RaidCell0_0Ready").is_some());
    assert!(reg.get_by_name("RaidCell0_0Fade").is_some());
    assert!(reg.get_by_name("RaidCell2_0Heal").is_none());
}

#[test]
fn ready_check_hidden_when_none() {
    let reg = build_registry();
    let id = reg.get_by_name("RaidCell0_1Ready").expect("rc");
    assert!(reg.get(id).expect("data").hidden);
}

#[test]
fn ready_check_visible_when_active() {
    let reg = build_registry();
    let id = reg.get_by_name("RaidCell0_0Ready").expect("rc");
    assert!(!reg.get(id).expect("data").hidden);
}

#[test]
fn range_fade_hidden_when_in_range() {
    let reg = build_registry();
    let id = reg.get_by_name("RaidCell0_0Fade").expect("fade");
    assert!(reg.get(id).expect("data").hidden);
}

#[test]
fn range_fade_visible_when_out_of_range() {
    let reg = build_registry();
    let id = reg.get_by_name("RaidCell0_1Fade").expect("fade");
    assert!(!reg.get(id).expect("data").hidden);
}

#[test]
fn incoming_heals_hidden_when_zero() {
    let reg = build_registry();
    let id = reg.get_by_name("RaidCell0_0Heal").expect("heal");
    assert!(reg.get(id).expect("data").hidden);
}

#[test]
fn incoming_heals_visible_when_nonzero() {
    let reg = build_registry();
    let id = reg.get_by_name("RaidCell0_1Heal").expect("heal");
    assert!(!reg.get(id).expect("data").hidden);
}

#[test]
fn ready_check_labels() {
    assert_eq!(RaidReadyCheck::Accepted.label(), "✓");
    assert_eq!(RaidReadyCheck::Pending.label(), "?");
    assert_eq!(RaidReadyCheck::Declined.label(), "✗");
    assert_eq!(RaidReadyCheck::None.label(), "");
}

// --- Overlay coord tests ---

#[test]
fn coord_range_fade_covers_cell() {
    let reg = layout_registry();
    let cell_r = rect(&reg, "RaidCell0_1");
    let fade_r = rect(&reg, "RaidCell0_1Fade");
    assert!((fade_r.x - cell_r.x).abs() < 1.0);
    assert!((fade_r.y - cell_r.y).abs() < 1.0);
    assert!((fade_r.width - CELL_W).abs() < 1.0);
    assert!((fade_r.height - CELL_H).abs() < 1.0);
}

#[test]
fn coord_incoming_heals_position() {
    let reg = layout_registry();
    let cell_r = rect(&reg, "RaidCell0_1");
    let heal_r = rect(&reg, "RaidCell0_1Heal");
    let frac = 30000.0 / 35000.0;
    let bar_inner = CELL_W - 2.0 * FILL_INSET;
    let expected_x = cell_r.x + FILL_INSET + frac * bar_inner;
    assert!((heal_r.x - expected_x).abs() < 1.0);
    let heal_frac = 0.15_f32.min(1.0 - frac);
    let expected_w = heal_frac * bar_inner;
    assert!((heal_r.width - expected_w).abs() < 1.0);
}

#[test]
fn coord_grid_total_dimensions() {
    let reg = layout_registry();
    let r = rect(&reg, "RaidFrame");
    assert!((r.width - GRID_W).abs() < 1.0);
    assert!((r.height - GRID_H).abs() < 1.0);
}

#[test]
fn coord_second_group_offset() {
    let reg = layout_registry();
    let c00 = rect(&reg, "RaidCell0_0");
    let c10 = rect(&reg, "RaidCell1_0");
    let expected_gap = CELL_W + GROUP_GAP;
    assert!((c10.x - c00.x - expected_gap).abs() < 1.0);
    assert!((c10.y - c00.y).abs() < 1.0);
}

#[test]
fn coord_row_spacing() {
    let reg = layout_registry();
    let c00 = rect(&reg, "RaidCell0_0");
    let c01 = rect(&reg, "RaidCell0_1");
    let expected_gap = CELL_H + CELL_GAP;
    assert!((c01.y - c00.y - expected_gap).abs() < 1.0);
    assert!((c01.x - c00.x).abs() < 1.0);
}

// --- Text content tests ---

fn fontstring_text(reg: &FrameRegistry, name: &str) -> String {
    use ui_toolkit::frame::WidgetData;
    let id = reg.get_by_name(name).expect(name);
    let frame = reg.get(id).expect("frame data");
    match frame.widget_data.as_ref() {
        Some(WidgetData::FontString(fs)) => fs.text.clone(),
        _ => panic!("{name} is not a FontString"),
    }
}

#[test]
fn group_label_text() {
    let reg = build_registry();
    for gi in 0..NUM_GROUPS {
        let expected = format!("Group {}", gi + 1);
        assert_eq!(
            fontstring_text(&reg, &format!("RaidGroup{gi}Label")),
            expected
        );
    }
}

#[test]
fn member_name_text() {
    let reg = build_registry();
    assert_eq!(fontstring_text(&reg, "RaidCell0_0Name"), "Tank1");
    assert_eq!(fontstring_text(&reg, "RaidCell0_1Name"), "Healer1");
    assert_eq!(fontstring_text(&reg, "RaidCell1_0Name"), "Dps1");
}

#[test]
fn ready_check_icon_text() {
    let reg = build_registry();
    assert_eq!(fontstring_text(&reg, "RaidCell0_0Ready"), "✓");
    assert_eq!(fontstring_text(&reg, "RaidCell1_0Ready"), "?");
    assert_eq!(fontstring_text(&reg, "RaidCell0_1Ready"), "");
}

#[test]
fn health_fraction_overcapped() {
    assert_eq!(member(150, 100).health_fraction(), 1.0);
}

#[test]
fn fill_width_dead_member() {
    use ui_toolkit::frame::Dimension;
    let dead = rm("Dead", 0, 50000, true, RaidReadyCheck::None, 0.0);
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(RaidFrameState {
        visible: true,
        groups: vec![RaidGroup {
            members: vec![dead],
        }],
    });
    Screen::new(raid_frame_screen).sync(&shared, &mut reg);
    let id = reg.get_by_name("RaidCell0_0Fill").expect("fill");
    let frame = reg.get(id).expect("data");
    assert_eq!(frame.width, Dimension::Fixed(0.0));
}
