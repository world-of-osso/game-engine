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
#[path = "raid_frame_component_tests.rs"]
mod tests;
