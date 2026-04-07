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

fn health_bar(idx: usize, fill_w: f32, member: &PartyMemberState) -> Element {
    let bar_id = DynName(format!("PartyMember{idx}HealthBg"));
    let fill_id = DynName(format!("PartyMember{idx}HealthFill"));
    let text_id = DynName(format!("PartyMember{idx}HealthText"));
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
            r#frame {
                name: fill_id,
                width: {fill_w},
                height: {BAR_H},
                background_color: HEALTH_FILL,
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "0",
                    y: "0",
                }
            }
            fontstring {
                name: text_id,
                width: {BAR_W},
                height: {BAR_H},
                text: {health_text.as_str()},
                font_size: 8.0,
                font_color: HEALTH_TEXT_COLOR,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "0",
                    y: "0",
                }
            }
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
mod tests {
    use super::*;
    use ui_toolkit::layout::{LayoutRect, recompute_layouts};
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn sample_members() -> Vec<PartyMemberState> {
        vec![
            PartyMemberState {
                name: "Tankadin".into(),
                health_current: 45000,
                health_max: 50000,
                role: PartyRole::Tank,
                debuffs: vec![PartyDebuff {
                    name: "Bleed".into(),
                    icon_fdid: 1,
                }],
                online: true,
                in_range: true,
                ready_check: ReadyCheckState::Accepted,
                incoming_heals: 0.1,
            },
            PartyMemberState {
                name: "Healbot".into(),
                health_current: 30000,
                health_max: 35000,
                role: PartyRole::Healer,
                debuffs: vec![],
                online: true,
                in_range: true,
                ready_check: ReadyCheckState::None,
                incoming_heals: 0.0,
            },
            PartyMemberState {
                name: "Stabsworth".into(),
                health_current: 28000,
                health_max: 32000,
                role: PartyRole::Dps,
                debuffs: vec![
                    PartyDebuff {
                        name: "Poison".into(),
                        icon_fdid: 2,
                    },
                    PartyDebuff {
                        name: "Curse".into(),
                        icon_fdid: 3,
                    },
                ],
                online: true,
                in_range: false,
                ready_check: ReadyCheckState::Pending,
                incoming_heals: 0.0,
            },
            PartyMemberState {
                name: "Pewpew".into(),
                health_current: 0,
                health_max: 30000,
                role: PartyRole::Dps,
                debuffs: vec![],
                online: true,
                in_range: true,
                ready_check: ReadyCheckState::Declined,
                incoming_heals: 0.0,
            },
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
}
