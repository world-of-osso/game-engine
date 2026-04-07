use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::strata::FrameStrata;

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// --- Layout constants ---

/// Anchor offset from top-left of screen.
pub const ANCHOR_X: f32 = 10.0;
pub const ANCHOR_Y: f32 = 300.0;

pub const NUM_GROUPS: usize = 5;
pub const MEMBERS_PER_GROUP: usize = 8;

const CELL_W: f32 = 72.0;
const CELL_H: f32 = 28.0;
const CELL_GAP: f32 = 2.0;
const GROUP_GAP: f32 = 4.0;
const GROUP_LABEL_H: f32 = 14.0;

const FILL_INSET: f32 = 1.0;
const FILL_H: f32 = CELL_H - 2.0 * FILL_INSET;
const NAME_H: f32 = 10.0;

/// Total width: 5 columns + 4 gaps between groups.
pub const GRID_W: f32 = NUM_GROUPS as f32 * CELL_W + (NUM_GROUPS as f32 - 1.0) * GROUP_GAP;
/// Total height: label + 8 rows + 7 gaps.
pub const GRID_H: f32 =
    GROUP_LABEL_H + MEMBERS_PER_GROUP as f32 * CELL_H + (MEMBERS_PER_GROUP as f32 - 1.0) * CELL_GAP;

// --- Colors ---

const FRAME_BG: &str = "0.0,0.0,0.0,0.0";
const CELL_BG: &str = "0.04,0.04,0.04,0.85";
const HEALTH_FILL: &str = "0.1,0.65,0.1,0.95";
const HEALTH_LOW_FILL: &str = "0.7,0.1,0.1,0.95";
const NAME_COLOR: &str = "1.0,1.0,1.0,1.0";
const GROUP_LABEL_COLOR: &str = "1.0,0.82,0.0,1.0";
const EMPTY_CELL_BG: &str = "0.03,0.03,0.03,0.5";
const RANGE_FADE_BG: &str = "0.0,0.0,0.0,0.55";
const INCOMING_HEAL_COLOR: &str = "0.3,0.8,0.3,0.45";
const READY_ICON_SIZE: f32 = 10.0;
const READY_ACCEPTED_COLOR: &str = "0.0,1.0,0.0,1.0";
const READY_PENDING_COLOR: &str = "1.0,0.82,0.0,1.0";
const READY_DECLINED_COLOR: &str = "1.0,0.0,0.0,1.0";

/// Health fraction below which the bar turns red.
const LOW_HEALTH_THRESHOLD: f32 = 0.3;

// --- Data types ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RaidReadyCheck {
    #[default]
    None,
    Pending,
    Accepted,
    Declined,
}

impl RaidReadyCheck {
    fn color(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Pending => READY_PENDING_COLOR,
            Self::Accepted => READY_ACCEPTED_COLOR,
            Self::Declined => READY_DECLINED_COLOR,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Pending => "?",
            Self::Accepted => "✓",
            Self::Declined => "✗",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RaidMember {
    pub name: String,
    pub health_current: u32,
    pub health_max: u32,
    pub alive: bool,
    pub in_range: bool,
    pub ready_check: RaidReadyCheck,
    /// Incoming heals as fraction of max health (0.0–1.0).
    pub incoming_heals: f32,
}

impl RaidMember {
    pub fn health_fraction(&self) -> f32 {
        if self.health_max == 0 {
            return 0.0;
        }
        (self.health_current as f32 / self.health_max as f32).min(1.0)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct RaidGroup {
    pub members: Vec<RaidMember>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct RaidFrameState {
    pub visible: bool,
    pub groups: Vec<RaidGroup>,
}

// --- Screen entry ---

pub fn raid_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<RaidFrameState>()
        .expect("RaidFrameState must be in SharedContext");
    let hide = !state.visible;
    let columns: Element = (0..NUM_GROUPS)
        .flat_map(|gi| {
            let group = state.groups.get(gi);
            group_column(gi, group)
        })
        .collect();
    rsx! {
        r#frame {
            name: "RaidFrame",
            width: {GRID_W},
            height: {GRID_H},
            strata: FrameStrata::Medium,
            hidden: hide,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {ANCHOR_X},
                y: {-ANCHOR_Y},
            }
            {columns}
        }
    }
}

// --- Group column ---

fn group_column(group_idx: usize, group: Option<&RaidGroup>) -> Element {
    let col_x = group_idx as f32 * (CELL_W + GROUP_GAP);
    let label_id = DynName(format!("RaidGroup{group_idx}Label"));
    let label_text = format!("Group {}", group_idx + 1);
    let cells: Element = (0..MEMBERS_PER_GROUP)
        .flat_map(|mi| {
            let member = group.and_then(|g| g.members.get(mi));
            let cell_y = GROUP_LABEL_H + mi as f32 * (CELL_H + CELL_GAP);
            raid_cell(group_idx, mi, member, cell_y)
        })
        .collect();
    rsx! {
        fontstring {
            name: label_id,
            width: {CELL_W},
            height: {GROUP_LABEL_H},
            text: {label_text.as_str()},
            font_size: 9.0,
            font_color: GROUP_LABEL_COLOR,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {col_x},
                y: "0",
            }
        }
        {cells}
    }
}

// --- Single raid cell ---

fn raid_cell(gi: usize, mi: usize, member: Option<&RaidMember>, y: f32) -> Element {
    let col_x = gi as f32 * (CELL_W + GROUP_GAP);
    let cell_id = DynName(format!("RaidCell{gi}_{mi}"));
    match member {
        Some(m) => filled_cell(gi, mi, cell_id, m, col_x, y),
        None => empty_cell(cell_id, col_x, y),
    }
}

fn raid_cell_fill(id: DynName, w: f32, color: &str) -> Element {
    rsx! {
        r#frame {
            name: id,
            width: {w},
            height: {FILL_H},
            background_color: color,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {FILL_INSET}, y: {-FILL_INSET} }
        }
    }
}

fn raid_cell_name(id: DynName, text: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {CELL_W - 4.0},
            height: {NAME_H},
            text: text,
            font_size: 8.0,
            font_color: NAME_COLOR,
            justify_h: "CENTER",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "2", y: "-2" }
        }
    }
}

fn filled_cell(
    gi: usize,
    mi: usize,
    cell_id: DynName,
    member: &RaidMember,
    x: f32,
    y: f32,
) -> Element {
    let frac = member.health_fraction();
    let bar_inner_w = CELL_W - 2.0 * FILL_INSET;
    let fill_w = frac * bar_inner_w;
    let fill_color = if frac < LOW_HEALTH_THRESHOLD && frac > 0.0 {
        HEALTH_LOW_FILL
    } else {
        HEALTH_FILL
    };
    rsx! {
        r#frame {
            name: cell_id,
            width: {CELL_W},
            height: {CELL_H},
            background_color: CELL_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {-y},
            }
            {raid_cell_fill(DynName(format!("RaidCell{gi}_{mi}Fill")), fill_w, fill_color)}
            {raid_cell_name(DynName(format!("RaidCell{gi}_{mi}Name")), &member.name)}
            {raid_incoming_heal(gi, mi, member, bar_inner_w)}
            {raid_ready_check(gi, mi, member.ready_check)}
            {raid_range_fade(gi, mi, member.in_range)}
        }
    }
}

fn empty_cell(cell_id: DynName, x: f32, y: f32) -> Element {
    rsx! {
        r#frame {
            name: cell_id,
            width: {CELL_W},
            height: {CELL_H},
            background_color: EMPTY_CELL_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {-y},
            }
        }
    }
}

fn raid_incoming_heal(gi: usize, mi: usize, member: &RaidMember, bar_w: f32) -> Element {
    let id = DynName(format!("RaidCell{gi}_{mi}Heal"));
    let has_heals = member.incoming_heals > 0.0;
    let hide = !has_heals;
    let frac = member.health_fraction();
    let heal_frac = member.incoming_heals.min(1.0 - frac);
    let heal_w = heal_frac * bar_w;
    let heal_x = FILL_INSET + frac * bar_w;
    rsx! {
        r#frame {
            name: id,
            width: {heal_w},
            height: {FILL_H},
            hidden: hide,
            background_color: INCOMING_HEAL_COLOR,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {heal_x},
                y: {-FILL_INSET},
            }
        }
    }
}

fn raid_ready_check(gi: usize, mi: usize, state: RaidReadyCheck) -> Element {
    let id = DynName(format!("RaidCell{gi}_{mi}Ready"));
    let is_none = state == RaidReadyCheck::None;
    rsx! {
        fontstring {
            name: id,
            width: {READY_ICON_SIZE},
            height: {READY_ICON_SIZE},
            hidden: is_none,
            text: {state.label()},
            font_size: 8.0,
            font_color: {state.color()},
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::TopRight,
                relative_point: AnchorPoint::TopRight,
                x: "-1",
                y: "0",
            }
        }
    }
}

fn raid_range_fade(gi: usize, mi: usize, in_range: bool) -> Element {
    let id = DynName(format!("RaidCell{gi}_{mi}Fade"));
    rsx! {
        r#frame {
            name: id,
            width: {CELL_W},
            height: {CELL_H},
            hidden: in_range,
            background_color: RANGE_FADE_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: "0",
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_toolkit::layout::{LayoutRect, recompute_layouts};
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn sample_groups() -> Vec<RaidGroup> {
        vec![
            RaidGroup {
                members: vec![
                    RaidMember {
                        name: "Tank1".into(),
                        health_current: 50000,
                        health_max: 50000,
                        alive: true,
                        in_range: true,
                        ready_check: RaidReadyCheck::Accepted,
                        incoming_heals: 0.0,
                    },
                    RaidMember {
                        name: "Healer1".into(),
                        health_current: 30000,
                        health_max: 35000,
                        alive: true,
                        in_range: false,
                        ready_check: RaidReadyCheck::None,
                        incoming_heals: 0.15,
                    },
                ],
            },
            RaidGroup {
                members: vec![RaidMember {
                    name: "Dps1".into(),
                    health_current: 5000,
                    health_max: 40000,
                    alive: true,
                    in_range: true,
                    ready_check: RaidReadyCheck::Pending,
                    incoming_heals: 0.0,
                }],
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
        // All 5×8 = 40 cells should exist (filled or empty)
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
        // Group 0 has 2 members
        assert!(reg.get_by_name("RaidCell0_0Fill").is_some());
        assert!(reg.get_by_name("RaidCell0_0Name").is_some());
        assert!(reg.get_by_name("RaidCell0_1Fill").is_some());
        assert!(reg.get_by_name("RaidCell0_1Name").is_some());
        // Group 0 member 2 is empty — no fill/name
        assert!(reg.get_by_name("RaidCell0_2Fill").is_none());
        assert!(reg.get_by_name("RaidCell0_2Name").is_none());
    }

    #[test]
    fn empty_group_has_only_empty_cells() {
        let reg = build_registry();
        // Group 2 is empty
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
        // Check corners of the grid: first and last cell of first and last group
        let r00 = rect(&reg, "RaidCell0_0");
        let expected_x0 = frame_r.x;
        let expected_y0 = frame_r.y + GROUP_LABEL_H;
        assert!((r00.x - expected_x0).abs() < 1.0);
        assert!((r00.y - expected_y0).abs() < 1.0);
        assert!((r00.width - CELL_W).abs() < 1.0);
        assert!((r00.height - CELL_H).abs() < 1.0);

        // Last group, first member
        let r40 = rect(&reg, "RaidCell4_0");
        let expected_x4 = frame_r.x + 4.0 * (CELL_W + GROUP_GAP);
        assert!((r40.x - expected_x4).abs() < 1.0);

        // First group, last member
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
        // Member 0 is full health: fill width should be CELL_W - 2*FILL_INSET
        let expected_fill_w = CELL_W - 2.0 * FILL_INSET;
        assert!((fill_r.width - expected_fill_w).abs() < 1.0);
    }

    #[test]
    fn coord_low_health_fill_width() {
        let reg = layout_registry();
        // Group 1, member 0: 5000/40000 = 0.125
        let fill_r = rect(&reg, "RaidCell1_0Fill");
        let expected_w = 0.125 * (CELL_W - 2.0 * FILL_INSET);
        assert!((fill_r.width - expected_w).abs() < 1.0);
    }

    // --- Overlay tests ---

    #[test]
    fn builds_overlay_elements_for_filled_cells() {
        let reg = build_registry();
        // Cell 0_0 is filled → has overlays
        assert!(reg.get_by_name("RaidCell0_0Heal").is_some());
        assert!(reg.get_by_name("RaidCell0_0Ready").is_some());
        assert!(reg.get_by_name("RaidCell0_0Fade").is_some());
        // Cell 2_0 is empty → no overlays
        assert!(reg.get_by_name("RaidCell2_0Heal").is_none());
    }

    #[test]
    fn ready_check_hidden_when_none() {
        let reg = build_registry();
        // Cell 0_1 (Healer1) has RaidReadyCheck::None
        let id = reg.get_by_name("RaidCell0_1Ready").expect("rc");
        assert!(reg.get(id).expect("data").hidden);
    }

    #[test]
    fn ready_check_visible_when_active() {
        let reg = build_registry();
        // Cell 0_0 (Tank1) has Accepted
        let id = reg.get_by_name("RaidCell0_0Ready").expect("rc");
        assert!(!reg.get(id).expect("data").hidden);
    }

    #[test]
    fn range_fade_hidden_when_in_range() {
        let reg = build_registry();
        // Cell 0_0 (Tank1) is in_range=true
        let id = reg.get_by_name("RaidCell0_0Fade").expect("fade");
        assert!(reg.get(id).expect("data").hidden);
    }

    #[test]
    fn range_fade_visible_when_out_of_range() {
        let reg = build_registry();
        // Cell 0_1 (Healer1) is in_range=false
        let id = reg.get_by_name("RaidCell0_1Fade").expect("fade");
        assert!(!reg.get(id).expect("data").hidden);
    }

    #[test]
    fn incoming_heals_hidden_when_zero() {
        let reg = build_registry();
        // Cell 0_0 (Tank1) has incoming_heals=0.0
        let id = reg.get_by_name("RaidCell0_0Heal").expect("heal");
        assert!(reg.get(id).expect("data").hidden);
    }

    #[test]
    fn incoming_heals_visible_when_nonzero() {
        let reg = build_registry();
        // Cell 0_1 (Healer1) has incoming_heals=0.15
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
        // Cell 0_1 (Healer1) is out of range
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
        // Cell 0_1 (Healer1): health 30000/35000 ≈ 0.857, incoming 0.15
        let cell_r = rect(&reg, "RaidCell0_1");
        let heal_r = rect(&reg, "RaidCell0_1Heal");
        let frac = 30000.0 / 35000.0;
        let bar_inner = CELL_W - 2.0 * FILL_INSET;
        let expected_x = cell_r.x + FILL_INSET + frac * bar_inner;
        assert!((heal_r.x - expected_x).abs() < 1.0);
        // Heal fraction capped at remaining: min(0.15, 1.0-0.857) ≈ 0.143
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
        // Same Y for first row of each group
        assert!((c10.y - c00.y).abs() < 1.0);
    }

    #[test]
    fn coord_row_spacing() {
        let reg = layout_registry();
        let c00 = rect(&reg, "RaidCell0_0");
        let c01 = rect(&reg, "RaidCell0_1");
        let expected_gap = CELL_H + CELL_GAP;
        assert!((c01.y - c00.y - expected_gap).abs() < 1.0);
        // Same X within a group
        assert!((c01.x - c00.x).abs() < 1.0);
    }
}
