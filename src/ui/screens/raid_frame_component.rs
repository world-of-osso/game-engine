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

/// Health fraction below which the bar turns red.
const LOW_HEALTH_THRESHOLD: f32 = 0.3;

// --- Data types ---

#[derive(Clone, Debug, PartialEq)]
pub struct RaidMember {
    pub name: String,
    pub health_current: u32,
    pub health_max: u32,
    pub alive: bool,
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

fn raid_cell(group_idx: usize, member_idx: usize, member: Option<&RaidMember>, y: f32) -> Element {
    let col_x = group_idx as f32 * (CELL_W + GROUP_GAP);
    let cell_id = DynName(format!("RaidCell{group_idx}_{member_idx}"));
    let fill_id = DynName(format!("RaidCell{group_idx}_{member_idx}Fill"));
    let name_id = DynName(format!("RaidCell{group_idx}_{member_idx}Name"));
    match member {
        Some(m) => filled_cell(cell_id, fill_id, name_id, m, col_x, y),
        None => empty_cell(cell_id, col_x, y),
    }
}

fn filled_cell(
    cell_id: DynName,
    fill_id: DynName,
    name_id: DynName,
    member: &RaidMember,
    x: f32,
    y: f32,
) -> Element {
    let frac = member.health_fraction();
    let fill_w = frac * (CELL_W - 2.0 * FILL_INSET);
    let is_low = frac < LOW_HEALTH_THRESHOLD && frac > 0.0;
    let fill_color = if is_low { HEALTH_LOW_FILL } else { HEALTH_FILL };
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
            r#frame {
                name: fill_id,
                width: {fill_w},
                height: {FILL_H},
                background_color: fill_color,
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {FILL_INSET},
                    y: {-FILL_INSET},
                }
            }
            fontstring {
                name: name_id,
                width: {CELL_W - 4.0},
                height: {NAME_H},
                text: {member.name.as_str()},
                font_size: 8.0,
                font_color: NAME_COLOR,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "2",
                    y: "-2",
                }
            }
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
                    },
                    RaidMember {
                        name: "Healer1".into(),
                        health_current: 30000,
                        health_max: 35000,
                        alive: true,
                    },
                ],
            },
            RaidGroup {
                members: vec![RaidMember {
                    name: "Dps1".into(),
                    health_current: 5000,
                    health_max: 40000,
                    alive: true,
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

    #[test]
    fn health_fraction() {
        let full = RaidMember {
            name: "A".into(),
            health_current: 100,
            health_max: 100,
            alive: true,
        };
        assert!((full.health_fraction() - 1.0).abs() < 0.01);

        let half = RaidMember {
            health_current: 50,
            ..full.clone()
        };
        assert!((half.health_fraction() - 0.5).abs() < 0.01);

        let zero_max = RaidMember {
            health_max: 0,
            health_current: 0,
            ..full
        };
        assert_eq!(zero_max.health_fraction(), 0.0);
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
}
