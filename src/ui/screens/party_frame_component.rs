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
pub const ANCHOR_Y: f32 = 220.0;

pub const UNIT_W: f32 = 160.0;
pub const UNIT_H: f32 = 46.0;
const UNIT_GAP: f32 = 4.0;

const ROLE_ICON_SIZE: f32 = 14.0;
const ROLE_ICON_INSET: f32 = 4.0;

const NAME_H: f32 = 14.0;
const NAME_INSET_X: f32 = ROLE_ICON_INSET + ROLE_ICON_SIZE + 4.0;
const NAME_W: f32 = UNIT_W - NAME_INSET_X - 4.0;

const BAR_H: f32 = 12.0;
const BAR_INSET: f32 = 4.0;
const BAR_W: f32 = UNIT_W - 2.0 * BAR_INSET;
const BAR_Y: f32 = NAME_H + 2.0;

const DEBUFF_ICON_SIZE: f32 = 14.0;
const DEBUFF_GAP: f32 = 2.0;
const DEBUFF_Y: f32 = BAR_Y + BAR_H + 2.0;
const MAX_DEBUFFS: usize = 4;

const READY_CHECK_SIZE: f32 = 14.0;
const INCOMING_HEAL_COLOR: &str = "0.3,0.8,0.3,0.45";

// --- Colors ---

const FRAME_BG: &str = "0.04,0.04,0.04,0.85";
const NAME_COLOR: &str = "1.0,1.0,1.0,1.0";
const HEALTH_BG: &str = "0.15,0.15,0.15,0.9";
const HEALTH_FILL: &str = "0.1,0.7,0.1,0.95";
const HEALTH_TEXT_COLOR: &str = "1.0,1.0,1.0,1.0";
const ROLE_BG: &str = "0.1,0.1,0.1,0.8";
const DEBUFF_BG: &str = "0.2,0.0,0.0,0.8";
const RANGE_FADE_BG: &str = "0.0,0.0,0.0,0.55";
const READY_ACCEPTED_COLOR: &str = "0.0,1.0,0.0,1.0";
const READY_PENDING_COLOR: &str = "1.0,0.82,0.0,1.0";
const READY_DECLINED_COLOR: &str = "1.0,0.0,0.0,1.0";

// --- Data types ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PartyRole {
    #[default]
    Dps,
    Tank,
    Healer,
}

impl PartyRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::Dps => "D",
            Self::Tank => "T",
            Self::Healer => "H",
        }
    }

    pub fn color(self) -> &'static str {
        match self {
            Self::Dps => "1.0,0.3,0.3,1.0",
            Self::Tank => "0.5,0.7,1.0,1.0",
            Self::Healer => "0.3,1.0,0.3,1.0",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ReadyCheckState {
    #[default]
    None,
    Pending,
    Accepted,
    Declined,
}

impl ReadyCheckState {
    pub fn color(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Pending => READY_PENDING_COLOR,
            Self::Accepted => READY_ACCEPTED_COLOR,
            Self::Declined => READY_DECLINED_COLOR,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Pending => "?",
            Self::Accepted => "✓",
            Self::Declined => "✗",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PartyDebuff {
    pub name: String,
    pub icon_fdid: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PartyMemberState {
    pub name: String,
    pub health_current: u32,
    pub health_max: u32,
    pub role: PartyRole,
    pub debuffs: Vec<PartyDebuff>,
    pub online: bool,
    pub in_range: bool,
    pub ready_check: ReadyCheckState,
    /// Incoming heals as fraction of max health (0.0–1.0).
    pub incoming_heals: f32,
}

impl PartyMemberState {
    pub fn health_fraction(&self) -> f32 {
        if self.health_max == 0 {
            return 0.0;
        }
        (self.health_current as f32 / self.health_max as f32).min(1.0)
    }

    pub fn health_text(&self) -> String {
        format!("{}/{}", self.health_current, self.health_max)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct PartyFrameState {
    pub visible: bool,
    pub members: Vec<PartyMemberState>,
}

// --- Screen entry ---

pub fn party_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<PartyFrameState>()
        .expect("PartyFrameState must be in SharedContext");
    let hide = !state.visible;
    let member_frames: Element = state
        .members
        .iter()
        .enumerate()
        .take(4)
        .flat_map(|(i, member)| {
            let y = -(i as f32 * (UNIT_H + UNIT_GAP));
            party_unit_frame(i, member, y)
        })
        .collect();
    rsx! {
        r#frame {
            name: "PartyFrame",
            width: {UNIT_W},
            height: {4.0 * UNIT_H + 3.0 * UNIT_GAP},
            strata: FrameStrata::Medium,
            hidden: hide,
            background_color: "0.0,0.0,0.0,0.0",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {ANCHOR_X},
                y: {-ANCHOR_Y},
            }
            {member_frames}
        }
    }
}

// --- Single party unit frame ---

fn party_unit_frame(idx: usize, member: &PartyMemberState, y: f32) -> Element {
    let frame_id = DynName(format!("PartyMember{idx}"));
    let health_fill_w = member.health_fraction() * BAR_W;
    rsx! {
        r#frame {
            name: frame_id,
            width: {UNIT_W},
            height: {UNIT_H},
            mouse_enabled: true,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: {y},
            }
            {role_icon(idx, &member.role)}
            {member_name(idx, &member.name)}
            {health_bar(idx, health_fill_w, member)}
            {incoming_heals_overlay(idx, member)}
            {debuff_row(idx, &member.debuffs)}
            {ready_check_icon(idx, member.ready_check)}
            {range_fade_overlay(idx, member.in_range)}
        }
    }
}

fn role_icon(idx: usize, role: &PartyRole) -> Element {
    let icon_id = DynName(format!("PartyMember{idx}Role"));
    let label_id = DynName(format!("PartyMember{idx}RoleLabel"));
    rsx! {
        r#frame {
            name: icon_id,
            width: {ROLE_ICON_SIZE},
            height: {ROLE_ICON_SIZE},
            background_color: ROLE_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {ROLE_ICON_INSET},
                y: "0",
            }
            fontstring {
                name: label_id,
                width: {ROLE_ICON_SIZE},
                height: {ROLE_ICON_SIZE},
                text: {role.label()},
                font_size: 9.0,
                font_color: {role.color()},
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
    }
}

fn member_name(idx: usize, name: &str) -> Element {
    let name_id = DynName(format!("PartyMember{idx}Name"));
    rsx! {
        fontstring {
            name: name_id,
            width: {NAME_W},
            height: {NAME_H},
            text: name,
            font_size: 10.0,
            font_color: NAME_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {NAME_INSET_X},
                y: "0",
            }
        }
    }
}

fn health_fill(id: DynName, w: f32) -> Element {
    rsx! {
        r#frame {
            name: id,
            width: {w},
            height: {BAR_H},
            background_color: HEALTH_FILL,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "0", y: "0" }
        }
    }
}

fn health_text_overlay(id: DynName, text: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {BAR_W},
            height: {BAR_H},
            text: text,
            font_size: 8.0,
            font_color: HEALTH_TEXT_COLOR,
            justify_h: "CENTER",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "0", y: "0" }
        }
    }
}

fn health_bar(idx: usize, fill_w: f32, member: &PartyMemberState) -> Element {
    let bar_id = DynName(format!("PartyMember{idx}HealthBg"));
    let health_text = member.health_text();
    rsx! {
        r#frame {
            name: bar_id,
            width: {BAR_W},
            height: {BAR_H},
            background_color: HEALTH_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {BAR_INSET},
                y: {-BAR_Y},
            }
            {health_fill(DynName(format!("PartyMember{idx}HealthFill")), fill_w)}
            {health_text_overlay(DynName(format!("PartyMember{idx}HealthText")), &health_text)}
        }
    }
}

fn debuff_row(idx: usize, debuffs: &[PartyDebuff]) -> Element {
    debuffs
        .iter()
        .enumerate()
        .take(MAX_DEBUFFS)
        .flat_map(|(di, _debuff)| {
            let debuff_id = DynName(format!("PartyMember{idx}Debuff{di}"));
            let x = BAR_INSET + di as f32 * (DEBUFF_ICON_SIZE + DEBUFF_GAP);
            rsx! {
                r#frame {
                    name: debuff_id,
                    width: {DEBUFF_ICON_SIZE},
                    height: {DEBUFF_ICON_SIZE},
                    background_color: DEBUFF_BG,
                    anchor {
                        point: AnchorPoint::TopLeft,
                        relative_point: AnchorPoint::TopLeft,
                        x: {x},
                        y: {-DEBUFF_Y},
                    }
                }
            }
        })
        .collect()
}

// --- Incoming heals overlay (bar segment after health fill) ---

fn incoming_heals_overlay(idx: usize, member: &PartyMemberState) -> Element {
    let id = DynName(format!("PartyMember{idx}IncomingHeal"));
    let has_heals = member.incoming_heals > 0.0;
    let hide = !has_heals;
    let health_frac = member.health_fraction();
    let heal_frac = member.incoming_heals.min(1.0 - health_frac);
    let heal_w = heal_frac * BAR_W;
    let heal_x = BAR_INSET + health_frac * BAR_W;
    rsx! {
        r#frame {
            name: id,
            width: {heal_w},
            height: {BAR_H},
            hidden: hide,
            background_color: INCOMING_HEAL_COLOR,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {heal_x},
                y: {-BAR_Y},
            }
        }
    }
}

// --- Ready check icon (top-right corner of unit frame) ---

fn ready_check_icon(idx: usize, state: ReadyCheckState) -> Element {
    let id = DynName(format!("PartyMember{idx}ReadyCheck"));
    let label_id = DynName(format!("PartyMember{idx}ReadyCheckLabel"));
    let is_none = state == ReadyCheckState::None;
    rsx! {
        r#frame {
            name: id,
            width: {READY_CHECK_SIZE},
            height: {READY_CHECK_SIZE},
            hidden: is_none,
            background_color: ROLE_BG,
            anchor {
                point: AnchorPoint::TopRight,
                relative_point: AnchorPoint::TopRight,
                x: "-2",
                y: "0",
            }
            fontstring {
                name: label_id,
                width: {READY_CHECK_SIZE},
                height: {READY_CHECK_SIZE},
                text: {state.label()},
                font_size: 10.0,
                font_color: {state.color()},
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
    }
}

// --- Range fade overlay (semi-transparent cover when out of range) ---

fn range_fade_overlay(idx: usize, in_range: bool) -> Element {
    let id = DynName(format!("PartyMember{idx}RangeFade"));
    rsx! {
        r#frame {
            name: id,
            width: {UNIT_W},
            height: {UNIT_H},
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
#[cfg(test)]
#[path = "party_frame_component_tests.rs"]
mod tests;
