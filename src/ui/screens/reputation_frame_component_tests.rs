use super::*;
use ui_toolkit::layout::{LayoutRect, recompute_layouts};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn fac(
    name: &str,
    standing: Standing,
    current: u32,
    max: u32,
    paragon: Option<ParagonProgress>,
) -> FactionEntry {
    FactionEntry {
        name: name.into(),
        standing,
        current,
        max,
        paragon,
    }
}

fn sample_categories() -> Vec<FactionCategory> {
    vec![
        FactionCategory {
            name: "Alliance".into(),
            collapsed: false,
            factions: vec![
                fac("Stormwind", Standing::Honored, 8000, 12000, None),
                fac("Ironforge", Standing::Friendly, 3000, 6000, None),
            ],
        },
        FactionCategory {
            name: "Horde".into(),
            collapsed: true,
            factions: vec![fac("Orgrimmar", Standing::Hated, 0, 36000, None)],
        },
        FactionCategory {
            name: "Neutral".into(),
            collapsed: false,
            factions: vec![fac(
                "Cenarion Circle",
                Standing::Exalted,
                0,
                0,
                Some(ParagonProgress {
                    current: 5000,
                    max: 10000,
                    reward_pending: true,
                }),
            )],
        },
    ]
}

fn build_registry() -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(ReputationFrameState {
        visible: true,
        categories: sample_categories(),
        hovered_faction: None,
    });
    Screen::new(reputation_frame_screen).sync(&shared, &mut reg);
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
fn builds_frame_and_title() {
    let reg = build_registry();
    assert!(reg.get_by_name("ReputationFrame").is_some());
    assert!(reg.get_by_name("ReputationFrameTitle").is_some());
    assert!(reg.get_by_name("ReputationList").is_some());
}

#[test]
fn builds_category_headers() {
    let reg = build_registry();
    for i in 0..3 {
        assert!(reg.get_by_name(&format!("RepCat{i}")).is_some());
        assert!(reg.get_by_name(&format!("RepCat{i}Label")).is_some());
        assert!(reg.get_by_name(&format!("RepCat{i}Icon")).is_some());
    }
}

#[test]
fn builds_faction_rows_for_expanded() {
    let reg = build_registry();
    // Category 0 (Alliance) expanded: 2 factions
    assert!(reg.get_by_name("RepFaction0_0").is_some());
    assert!(reg.get_by_name("RepFaction0_0Name").is_some());
    assert!(reg.get_by_name("RepFaction0_1").is_some());
    // Category 2 (Neutral) expanded: 1 faction
    assert!(reg.get_by_name("RepFaction2_0").is_some());
}

#[test]
fn collapsed_category_hides_factions() {
    let reg = build_registry();
    // Category 1 (Horde) collapsed — no faction rows
    assert!(reg.get_by_name("RepFaction1_0").is_none());
}

#[test]
fn builds_reputation_bars() {
    let reg = build_registry();
    assert!(reg.get_by_name("RepBar0_0").is_some());
    assert!(reg.get_by_name("RepBar0_0Fill").is_some());
    assert!(reg.get_by_name("RepBar0_0Text").is_some());
    assert!(reg.get_by_name("RepBar0_0Standing").is_some());
}

#[test]
fn hidden_when_not_visible() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(ReputationFrameState::default());
    Screen::new(reputation_frame_screen).sync(&shared, &mut reg);
    let id = reg.get_by_name("ReputationFrame").expect("frame");
    assert!(reg.get(id).expect("data").hidden);
}

// --- Data model tests ---

#[test]
fn standing_labels() {
    assert_eq!(Standing::Hated.label(), "Hated");
    assert_eq!(Standing::Neutral.label(), "Neutral");
    assert_eq!(Standing::Exalted.label(), "Exalted");
}

#[test]
fn standing_bar_colors_non_empty() {
    for standing in [
        Standing::Hated,
        Standing::Hostile,
        Standing::Unfriendly,
        Standing::Neutral,
        Standing::Friendly,
        Standing::Honored,
        Standing::Revered,
        Standing::Exalted,
    ] {
        assert!(!standing.bar_color().is_empty());
    }
}

#[test]
fn faction_progress_fraction() {
    let f = FactionEntry {
        name: "X".into(),
        standing: Standing::Friendly,
        current: 3000,
        max: 6000,
        paragon: None,
    };
    assert!((f.progress_fraction() - 0.5).abs() < 0.01);
}

#[test]
fn exalted_progress_is_full() {
    let f = FactionEntry {
        name: "X".into(),
        standing: Standing::Exalted,
        current: 0,
        max: 0,
        paragon: None,
    };
    assert_eq!(f.progress_fraction(), 1.0);
}

#[test]
fn faction_progress_text() {
    let f = FactionEntry {
        name: "X".into(),
        standing: Standing::Neutral,
        current: 1500,
        max: 3000,
        paragon: None,
    };
    assert_eq!(f.progress_text(), "1500/3000");
}

// --- Coord validation ---

#[test]
fn coord_main_frame_centered() {
    let reg = layout_registry();
    let r = rect(&reg, "ReputationFrame");
    let expected_x = (1920.0 - FRAME_W) / 2.0;
    let expected_y = (1080.0 - FRAME_H) / 2.0;
    assert!((r.x - expected_x).abs() < 1.0);
    assert!((r.y - expected_y).abs() < 1.0);
    assert!((r.width - FRAME_W).abs() < 1.0);
    assert!((r.height - FRAME_H).abs() < 1.0);
}

#[test]
fn coord_list_panel() {
    let reg = layout_registry();
    let frame_r = rect(&reg, "ReputationFrame");
    let list_r = rect(&reg, "ReputationList");
    assert!((list_r.x - (frame_r.x + INSET)).abs() < 1.0);
    assert!((list_r.y - (frame_r.y + CONTENT_TOP)).abs() < 1.0);
    assert!((list_r.width - LIST_W).abs() < 1.0);
}

#[test]
fn coord_first_category_at_top() {
    let reg = layout_registry();
    let list_r = rect(&reg, "ReputationList");
    let cat_r = rect(&reg, "RepCat0");
    assert!((cat_r.y - list_r.y).abs() < 1.0);
    assert!((cat_r.height - CAT_HEADER_H).abs() < 1.0);
}

#[test]
fn coord_faction_row_below_header() {
    let reg = layout_registry();
    let cat_r = rect(&reg, "RepCat0");
    let row_r = rect(&reg, "RepFaction0_0");
    let expected_y = cat_r.y + CAT_HEADER_H + ROW_GAP;
    assert!((row_r.y - expected_y).abs() < 1.0);
    assert!((row_r.height - FACTION_ROW_H).abs() < 1.0);
}

#[test]
fn coord_reputation_bar_position() {
    let reg = layout_registry();
    let row_r = rect(&reg, "RepFaction0_0");
    let bar_r = rect(&reg, "RepBar0_0");
    assert!((bar_r.x - (row_r.x + BAR_X)).abs() < 1.0);
    assert!((bar_r.width - BAR_W).abs() < 1.0);
    assert!((bar_r.height - BAR_H).abs() < 1.0);
}

#[test]
fn coord_bar_fill_proportional() {
    let reg = layout_registry();
    let fill_r = rect(&reg, "RepBar0_0Fill");
    // Stormwind: 8000/12000 ≈ 0.667
    let expected_w = (8000.0 / 12000.0) * BAR_W;
    assert!((fill_r.width - expected_w).abs() < 1.0);
}

#[test]
fn coord_collapsed_skips_faction_space() {
    let reg = layout_registry();
    // Cat 1 (Horde) is collapsed, cat 2 (Neutral) header should follow immediately
    let cat1_r = rect(&reg, "RepCat1");
    let cat2_r = rect(&reg, "RepCat2");
    let expected_y = cat1_r.y + CAT_HEADER_H + ROW_GAP;
    assert!((cat2_r.y - expected_y).abs() < 1.0);
}

// --- Paragon tests ---

#[test]
fn paragon_indicator_visible_when_present() {
    let reg = build_registry();
    // Cenarion Circle (cat 2, fac 0) has paragon
    let id = reg.get_by_name("RepParagon2_0").expect("paragon");
    assert!(!reg.get(id).expect("data").hidden);
}

#[test]
fn paragon_indicator_hidden_when_absent() {
    let reg = build_registry();
    // Stormwind (cat 0, fac 0) has no paragon
    let id = reg.get_by_name("RepParagon0_0").expect("paragon");
    assert!(reg.get(id).expect("data").hidden);
}

#[test]
fn paragon_progress_fraction() {
    let p = ParagonProgress {
        current: 5000,
        max: 10000,
        reward_pending: false,
    };
    assert!((p.fraction() - 0.5).abs() < 0.01);
}

#[test]
fn paragon_progress_text() {
    let p = ParagonProgress {
        current: 5000,
        max: 10000,
        reward_pending: false,
    };
    assert_eq!(p.progress_text(), "5000/10000");
}

// --- Tooltip tests ---

#[test]
fn tooltip_hidden_when_no_hover() {
    let reg = build_registry();
    let id = reg.get_by_name("RepTooltip").expect("tooltip");
    assert!(reg.get(id).expect("data").hidden);
}

#[test]
fn tooltip_visible_when_hovered() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(ReputationFrameState {
        visible: true,
        categories: sample_categories(),
        hovered_faction: Some((0, 0)),
    });
    Screen::new(reputation_frame_screen).sync(&shared, &mut reg);
    let id = reg.get_by_name("RepTooltip").expect("tooltip");
    assert!(!reg.get(id).expect("data").hidden);
    assert!(reg.get_by_name("RepTooltipTitle").is_some());
    assert!(reg.get_by_name("RepTooltipStanding").is_some());
    assert!(reg.get_by_name("RepTooltipProgress").is_some());
}

#[test]
fn tooltip_shows_paragon_line_for_paragon_faction() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(ReputationFrameState {
        visible: true,
        categories: sample_categories(),
        hovered_faction: Some((2, 0)), // Cenarion Circle with paragon
    });
    Screen::new(reputation_frame_screen).sync(&shared, &mut reg);
    let id = reg.get_by_name("RepTooltipParagon").expect("paragon line");
    assert!(!reg.get(id).expect("data").hidden);
}

#[test]
fn tooltip_hides_paragon_line_for_normal_faction() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(ReputationFrameState {
        visible: true,
        categories: sample_categories(),
        hovered_faction: Some((0, 0)), // Stormwind, no paragon
    });
    Screen::new(reputation_frame_screen).sync(&shared, &mut reg);
    let id = reg.get_by_name("RepTooltipParagon").expect("paragon line");
    assert!(reg.get(id).expect("data").hidden);
}

// --- Additional coord validation ---

#[test]
fn coord_paragon_indicator_in_row() {
    let reg = layout_registry();
    // Cenarion Circle (cat 2, fac 0) has paragon indicator
    let row_r = rect(&reg, "RepFaction2_0");
    let paragon_r = rect(&reg, "RepParagon2_0");
    // Anchored at left of row, vertically centered
    let expected_x = row_r.x + PARAGON_ICON_X;
    let expected_y = row_r.y + (FACTION_ROW_H - PARAGON_ICON_SIZE) / 2.0;
    assert!((paragon_r.x - expected_x).abs() < 1.0);
    assert!((paragon_r.y - expected_y).abs() < 1.0);
    assert!((paragon_r.width - PARAGON_ICON_SIZE).abs() < 1.0);
    assert!((paragon_r.height - PARAGON_ICON_SIZE).abs() < 1.0);
}

#[test]
fn coord_second_faction_row_spacing() {
    let reg = layout_registry();
    let row0 = rect(&reg, "RepFaction0_0");
    let row1 = rect(&reg, "RepFaction0_1");
    let expected_gap = FACTION_ROW_H + ROW_GAP;
    assert!((row1.y - row0.y - expected_gap).abs() < 1.0);
}

#[test]
fn coord_tooltip_dimensions() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(ReputationFrameState {
        visible: true,
        categories: sample_categories(),
        hovered_faction: Some((0, 0)),
    });
    Screen::new(reputation_frame_screen).sync(&shared, &mut reg);
    recompute_layouts(&mut reg);
    let r = rect(&reg, "RepTooltip");
    assert!((r.width - TOOLTIP_W).abs() < 1.0);
    // No paragon → 2 lines + header + insets
    let expected_h = TOOLTIP_INSET * 2.0 + TOOLTIP_HEADER_H + 2.0 * TOOLTIP_LINE_H;
    assert!((r.height - expected_h).abs() < 1.0);
}

#[test]
fn coord_bar_fill_inside_bar() {
    let reg = layout_registry();
    let bar_r = rect(&reg, "RepBar0_0");
    let fill_r = rect(&reg, "RepBar0_0Fill");
    // Fill starts at left edge of bar
    assert!((fill_r.x - bar_r.x).abs() < 1.0);
    assert!((fill_r.y - bar_r.y).abs() < 1.0);
    assert!((fill_r.height - BAR_H).abs() < 1.0);
}

#[test]
fn coord_category_header_width() {
    let reg = layout_registry();
    let cat_r = rect(&reg, "RepCat0");
    assert!((cat_r.width - (LIST_W - 4.0)).abs() < 1.0);
}
