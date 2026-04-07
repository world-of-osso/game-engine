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

pub const FRAME_W: f32 = 400.0;
pub const FRAME_H: f32 = 360.0;
const HEADER_H: f32 = 28.0;
const INSET: f32 = 8.0;
const CONTENT_TOP: f32 = HEADER_H + 4.0;

const PANEL_GAP: f32 = 8.0;
const PANEL_W: f32 = (FRAME_W - 2.0 * INSET - PANEL_GAP) / 2.0;
const PANEL_LABEL_H: f32 = 18.0;

const SLOT_SIZE: f32 = 32.0;
const SLOT_GAP: f32 = 4.0;
const SLOT_COUNT: usize = 7;

const MONEY_ROW_H: f32 = 20.0;
const MONEY_LABEL_W: f32 = 50.0;
const MONEY_INPUT_W: f32 = PANEL_W - MONEY_LABEL_W - 8.0;

const BTN_W: f32 = 90.0;
const BTN_H: f32 = 26.0;
const BTN_GAP: f32 = 12.0;

// --- Colors ---

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const PANEL_BG: &str = "0.0,0.0,0.0,0.3";
const PANEL_LABEL_COLOR: &str = "1.0,0.82,0.0,1.0";
const SLOT_BG: &str = "0.08,0.08,0.08,0.8";
const MONEY_LABEL_COLOR: &str = "0.8,0.8,0.8,1.0";
const MONEY_INPUT_BG: &str = "0.1,0.1,0.1,0.9";
const MONEY_INPUT_COLOR: &str = "1.0,1.0,1.0,1.0";
const ACCEPT_BG: &str = "0.15,0.25,0.1,0.95";
const ACCEPT_TEXT: &str = "0.2,1.0,0.2,1.0";
const ACCEPT_HIGHLIGHT_BG: &str = "0.2,0.35,0.15,0.95";
const CANCEL_BG: &str = "0.2,0.08,0.08,0.95";
const CANCEL_TEXT: &str = "1.0,0.3,0.3,1.0";

// --- Data types ---

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TradeSlot {
    pub name: String,
    pub icon_fdid: u32,
    pub quantity: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TradeAcceptState {
    #[default]
    Pending,
    Accepted,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TradePlayerPanel {
    pub name: String,
    pub slots: Vec<TradeSlot>,
    pub money: u32,
    pub accept_state: TradeAcceptState,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TradeFrameState {
    pub visible: bool,
    pub player: TradePlayerPanel,
    pub other: TradePlayerPanel,
}

// --- Screen entry ---

pub fn trade_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<TradeFrameState>()
        .expect("TradeFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "TradeFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
                x: "0",
                y: "0",
            }
            {title_bar()}
            {trade_panel("TradePlayer", &state.player, INSET, true)}
            {trade_panel("TradeOther", &state.other, INSET + PANEL_W + PANEL_GAP, false)}
            {action_buttons(&state.player.accept_state, &state.other.accept_state)}
        }
    }
}

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "TradeFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Trade",
            font_size: 16.0,
            font_color: TITLE_COLOR,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                x: "0",
                y: "0",
            }
        }
    }
}

// --- Trade panel (one per player) ---

fn trade_panel(prefix: &str, panel: &TradePlayerPanel, x: f32, show_input: bool) -> Element {
    let panel_id = DynName(format!("{prefix}Panel"));
    let label_id = DynName(format!("{prefix}Label"));
    let panel_h = FRAME_H - CONTENT_TOP - INSET - BTN_H - 8.0;
    let slots: Element = (0..SLOT_COUNT)
        .flat_map(|i| {
            let slot = panel.slots.get(i);
            let slot_y = PANEL_LABEL_H + 4.0 + i as f32 * (SLOT_SIZE + SLOT_GAP);
            trade_slot(prefix, i, slot, slot_y)
        })
        .collect();
    let money_y = PANEL_LABEL_H + 4.0 + SLOT_COUNT as f32 * (SLOT_SIZE + SLOT_GAP) + 4.0;
    rsx! {
        r#frame {
            name: panel_id,
            width: {PANEL_W},
            height: {panel_h},
            background_color: PANEL_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {-CONTENT_TOP},
            }
            fontstring {
                name: label_id,
                width: {PANEL_W},
                height: {PANEL_LABEL_H},
                text: {panel.name.as_str()},
                font_size: 11.0,
                font_color: PANEL_LABEL_COLOR,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "0",
                    y: "0",
                }
            }
            {slots}
            {money_row(prefix, panel.money, money_y, show_input)}
        }
    }
}

fn trade_slot(prefix: &str, idx: usize, slot: Option<&TradeSlot>, y: f32) -> Element {
    let slot_id = DynName(format!("{prefix}Slot{idx}"));
    let has_item = slot.is_some_and(|s| !s.name.is_empty());
    let bg = if has_item { SLOT_BG } else { SLOT_BG };
    rsx! {
        r#frame {
            name: slot_id,
            width: {SLOT_SIZE},
            height: {SLOT_SIZE},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "4",
                y: {-y},
            }
        }
    }
}

fn money_row(prefix: &str, money: u32, y: f32, show_input: bool) -> Element {
    let label_id = DynName(format!("{prefix}MoneyLabel"));
    let value_id = DynName(format!("{prefix}MoneyValue"));
    let money_text = format_money(money);
    let value_bg = if show_input {
        MONEY_INPUT_BG
    } else {
        "0.0,0.0,0.0,0.0"
    };
    rsx! {
        fontstring {
            name: label_id,
            width: {MONEY_LABEL_W},
            height: {MONEY_ROW_H},
            text: "Gold:",
            font_size: 10.0,
            font_color: MONEY_LABEL_COLOR,
            justify_h: "RIGHT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "4",
                y: {-y},
            }
        }
        r#frame {
            name: value_id,
            width: {MONEY_INPUT_W},
            height: {MONEY_ROW_H},
            background_color: value_bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {MONEY_LABEL_W + 8.0},
                y: {-y},
            }
            fontstring {
                name: DynName(format!("{prefix}MoneyText")),
                width: {MONEY_INPUT_W},
                height: {MONEY_ROW_H},
                text: {money_text.as_str()},
                font_size: 10.0,
                font_color: MONEY_INPUT_COLOR,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "4",
                    y: "0",
                }
            }
        }
    }
}

fn format_money(copper: u32) -> String {
    let gold = copper / 10000;
    let silver = (copper % 10000) / 100;
    let copper_rem = copper % 100;
    if gold > 0 {
        format!("{gold}g {silver}s {copper_rem}c")
    } else if silver > 0 {
        format!("{silver}s {copper_rem}c")
    } else {
        format!("{copper_rem}c")
    }
}

// --- Action buttons ---

fn action_buttons(player_state: &TradeAcceptState, other_state: &TradeAcceptState) -> Element {
    let y = -(FRAME_H - BTN_H - 8.0);
    let center = FRAME_W / 2.0;
    let accept_x = center - BTN_W - BTN_GAP / 2.0;
    let cancel_x = center + BTN_GAP / 2.0;
    let both_accepted =
        *player_state == TradeAcceptState::Accepted && *other_state == TradeAcceptState::Accepted;
    let accept_bg = if both_accepted {
        ACCEPT_HIGHLIGHT_BG
    } else {
        ACCEPT_BG
    };
    rsx! {
        r#frame {
            name: "TradeAcceptBtn",
            width: {BTN_W},
            height: {BTN_H},
            background_color: accept_bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {accept_x},
                y: {y},
            }
            fontstring {
                name: "TradeAcceptBtnText",
                width: {BTN_W},
                height: {BTN_H},
                text: "Accept",
                font_size: 11.0,
                font_color: ACCEPT_TEXT,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
        r#frame {
            name: "TradeCancelBtn",
            width: {BTN_W},
            height: {BTN_H},
            background_color: CANCEL_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {cancel_x},
                y: {y},
            }
            fontstring {
                name: "TradeCancelBtnText",
                width: {BTN_W},
                height: {BTN_H},
                text: "Cancel",
                font_size: 11.0,
                font_color: CANCEL_TEXT,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
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

    fn sample_state() -> TradeFrameState {
        TradeFrameState {
            visible: true,
            player: TradePlayerPanel {
                name: "Tankadin".into(),
                slots: vec![
                    TradeSlot {
                        name: "Iron Ore".into(),
                        icon_fdid: 1,
                        quantity: 20,
                    },
                    TradeSlot {
                        name: "Copper Bar".into(),
                        icon_fdid: 2,
                        quantity: 5,
                    },
                ],
                money: 150000,
                accept_state: TradeAcceptState::Pending,
            },
            other: TradePlayerPanel {
                name: "Healbot".into(),
                slots: vec![TradeSlot {
                    name: "Healing Potion".into(),
                    icon_fdid: 3,
                    quantity: 10,
                }],
                money: 0,
                accept_state: TradeAcceptState::Pending,
            },
        }
    }

    fn build_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(sample_state());
        Screen::new(trade_frame_screen).sync(&shared, &mut reg);
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
        assert!(reg.get_by_name("TradeFrame").is_some());
        assert!(reg.get_by_name("TradeFrameTitle").is_some());
    }

    #[test]
    fn builds_both_panels() {
        let reg = build_registry();
        assert!(reg.get_by_name("TradePlayerPanel").is_some());
        assert!(reg.get_by_name("TradePlayerLabel").is_some());
        assert!(reg.get_by_name("TradeOtherPanel").is_some());
        assert!(reg.get_by_name("TradeOtherLabel").is_some());
    }

    #[test]
    fn builds_seven_slots_per_panel() {
        let reg = build_registry();
        for i in 0..7 {
            assert!(
                reg.get_by_name(&format!("TradePlayerSlot{i}")).is_some(),
                "TradePlayerSlot{i} missing"
            );
            assert!(
                reg.get_by_name(&format!("TradeOtherSlot{i}")).is_some(),
                "TradeOtherSlot{i} missing"
            );
        }
    }

    #[test]
    fn builds_money_rows() {
        let reg = build_registry();
        assert!(reg.get_by_name("TradePlayerMoneyLabel").is_some());
        assert!(reg.get_by_name("TradePlayerMoneyValue").is_some());
        assert!(reg.get_by_name("TradePlayerMoneyText").is_some());
        assert!(reg.get_by_name("TradeOtherMoneyLabel").is_some());
        assert!(reg.get_by_name("TradeOtherMoneyValue").is_some());
    }

    #[test]
    fn builds_action_buttons() {
        let reg = build_registry();
        assert!(reg.get_by_name("TradeAcceptBtn").is_some());
        assert!(reg.get_by_name("TradeAcceptBtnText").is_some());
        assert!(reg.get_by_name("TradeCancelBtn").is_some());
        assert!(reg.get_by_name("TradeCancelBtnText").is_some());
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(TradeFrameState::default());
        Screen::new(trade_frame_screen).sync(&shared, &mut reg);
        let id = reg.get_by_name("TradeFrame").expect("frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    // --- Data model tests ---

    #[test]
    fn format_money_gold() {
        assert_eq!(format_money(150000), "15g 0s 0c");
    }

    #[test]
    fn format_money_silver() {
        assert_eq!(format_money(350), "3s 50c");
    }

    #[test]
    fn format_money_copper_only() {
        assert_eq!(format_money(42), "42c");
    }

    // --- Coord validation ---

    #[test]
    fn coord_main_frame_centered() {
        let reg = layout_registry();
        let r = rect(&reg, "TradeFrame");
        let expected_x = (1920.0 - FRAME_W) / 2.0;
        let expected_y = (1080.0 - FRAME_H) / 2.0;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.y - expected_y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
        assert!((r.height - FRAME_H).abs() < 1.0);
    }

    #[test]
    fn coord_panels_side_by_side() {
        let reg = layout_registry();
        let frame_r = rect(&reg, "TradeFrame");
        let player_r = rect(&reg, "TradePlayerPanel");
        let other_r = rect(&reg, "TradeOtherPanel");
        // Player panel on the left
        assert!((player_r.x - (frame_r.x + INSET)).abs() < 1.0);
        // Other panel on the right
        let expected_other_x = frame_r.x + INSET + PANEL_W + PANEL_GAP;
        assert!((other_r.x - expected_other_x).abs() < 1.0);
        // Same width
        assert!((player_r.width - PANEL_W).abs() < 1.0);
        assert!((other_r.width - PANEL_W).abs() < 1.0);
        // Same Y
        assert!((player_r.y - other_r.y).abs() < 1.0);
    }

    #[test]
    fn coord_slots_stacked_vertically() {
        let reg = layout_registry();
        let panel_r = rect(&reg, "TradePlayerPanel");
        let slot0 = rect(&reg, "TradePlayerSlot0");
        let slot1 = rect(&reg, "TradePlayerSlot1");
        // First slot offset from panel top
        let expected_y0 = panel_r.y + PANEL_LABEL_H + 4.0;
        assert!((slot0.y - expected_y0).abs() < 1.0);
        assert!((slot0.width - SLOT_SIZE).abs() < 1.0);
        // Second slot below first
        let expected_gap = SLOT_SIZE + SLOT_GAP;
        assert!((slot1.y - slot0.y - expected_gap).abs() < 1.0);
    }

    #[test]
    fn coord_buttons_centered_at_bottom() {
        let reg = layout_registry();
        let frame_r = rect(&reg, "TradeFrame");
        let accept_r = rect(&reg, "TradeAcceptBtn");
        let cancel_r = rect(&reg, "TradeCancelBtn");
        let expected_y = frame_r.y + FRAME_H - BTN_H - 8.0;
        assert!((accept_r.y - expected_y).abs() < 1.0);
        assert!((cancel_r.y - expected_y).abs() < 1.0);
        assert!(cancel_r.x > accept_r.x);
        assert!((accept_r.width - BTN_W).abs() < 1.0);
    }
}
