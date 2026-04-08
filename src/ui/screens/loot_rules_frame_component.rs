use std::fmt;

use game_engine::raid_party_data::{LootMethod, LootThreshold};
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::strata::FrameStrata;

const FRAME_W: f32 = 420.0;
const FRAME_H: f32 = 264.0;
const INSET: f32 = 12.0;
const HEADER_H: f32 = 28.0;
const BUTTON_W: f32 = 188.0;
const BUTTON_H: f32 = 24.0;
const BUTTON_GAP: f32 = 8.0;
const BUTTONS_PER_ROW: usize = 2;

const FRAME_BG: &str = "0.06,0.05,0.04,0.94";
const PANEL_BG: &str = "0.02,0.02,0.02,0.48";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const LABEL_COLOR: &str = "0.88,0.82,0.70,1.0";
const BODY_COLOR: &str = "0.82,0.82,0.82,1.0";
const ACTIVE_BG: &str = "0.30,0.20,0.07,0.98";
const INACTIVE_BG: &str = "0.10,0.09,0.08,0.92";
const ACTIVE_TEXT: &str = "1.0,0.86,0.34,1.0";
const INACTIVE_TEXT: &str = "0.72,0.72,0.72,1.0";

pub const ACTION_CLOSE: &str = "loot_rules_close";
pub const ACTION_METHOD_PREFIX: &str = "loot_rules_method:";
pub const ACTION_THRESHOLD_PREFIX: &str = "loot_rules_threshold:";

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LootRulesFrameState {
    pub visible: bool,
    pub group_summary: String,
    pub current_method: LootMethod,
    pub current_threshold: LootThreshold,
}

impl Default for LootRulesFrameState {
    fn default() -> Self {
        Self {
            visible: false,
            group_summary: "Not in a group".into(),
            current_method: LootMethod::default(),
            current_threshold: LootThreshold::default(),
        }
    }
}

pub fn loot_rules_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<LootRulesFrameState>()
        .expect("LootRulesFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "LootRulesFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "780",
                y: "-120",
            }
            {header()}
            {summary_text(&state.group_summary)}
            {section_label("LootRulesMethodLabel", "Distribution Method", INSET, -(HEADER_H + 42.0))}
            {method_buttons(state)}
            {section_label("LootRulesThresholdLabel", "Loot Threshold", INSET, -(HEADER_H + 144.0))}
            {threshold_buttons(state)}
        }
    }
}

fn header() -> Element {
    rsx! {
        fontstring {
            name: "LootRulesTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Loot Rules",
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
        button {
            name: "LootRulesClose",
            width: 22.0,
            height: 22.0,
            text: "X",
            font_size: 11.0,
            onclick: ACTION_CLOSE,
            anchor {
                point: AnchorPoint::TopRight,
                relative_point: AnchorPoint::TopRight,
                x: "-8",
                y: "-6",
            }
        }
    }
}

fn summary_text(text: &str) -> Element {
    rsx! {
        fontstring {
            name: "LootRulesSummary",
            width: {FRAME_W - 2.0 * INSET},
            height: 18.0,
            text: text,
            font_size: 11.0,
            font_color: BODY_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INSET},
                y: {-(HEADER_H + 16.0)},
            }
        }
    }
}

fn section_label(name: &str, text: &str, x: f32, y: f32) -> Element {
    rsx! {
        fontstring {
            name: {DynName(name.to_string())},
            width: {FRAME_W - 2.0 * INSET},
            height: 14.0,
            text: text,
            font_size: 11.0,
            font_color: LABEL_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
        }
    }
}

fn method_buttons(state: &LootRulesFrameState) -> Element {
    loot_rule_buttons(
        "LootMethod",
        &[
            LootMethod::GroupLoot,
            LootMethod::NeedBeforeGreed,
            LootMethod::MasterLooter,
            LootMethod::PersonalLoot,
        ],
        |method| state.current_method == *method,
        |method| format!("{ACTION_METHOD_PREFIX}{}", loot_method_token(*method)),
        -(HEADER_H + 64.0),
    )
}

fn threshold_buttons(state: &LootRulesFrameState) -> Element {
    loot_rule_buttons(
        "LootThreshold",
        &[
            LootThreshold::Common,
            LootThreshold::Uncommon,
            LootThreshold::Rare,
            LootThreshold::Epic,
        ],
        |threshold| state.current_threshold == *threshold,
        |threshold| {
            format!(
                "{ACTION_THRESHOLD_PREFIX}{}",
                loot_threshold_token(*threshold)
            )
        },
        -(HEADER_H + 166.0),
    )
}

fn loot_rule_buttons<T, FActive, FAction>(
    prefix: &str,
    items: &[T],
    is_active: FActive,
    action: FAction,
    start_y: f32,
) -> Element
where
    T: Copy + LootRuleLabel,
    FActive: Fn(&T) -> bool,
    FAction: Fn(&T) -> String,
{
    let rows = items.len().div_ceil(BUTTONS_PER_ROW);
    let panel_h = rows as f32 * BUTTON_H + (rows.saturating_sub(1)) as f32 * BUTTON_GAP + 12.0;
    let buttons: Element = items
        .iter()
        .enumerate()
        .flat_map(|(index, item)| {
            let row = index / BUTTONS_PER_ROW;
            let col = index % BUTTONS_PER_ROW;
            let x = 6.0 + col as f32 * (BUTTON_W + BUTTON_GAP);
            let y = -(6.0 + row as f32 * (BUTTON_H + BUTTON_GAP));
            let button_name = DynName(format!("{prefix}Button{index}"));
            let label_name = DynName(format!("{prefix}Button{index}Label"));
            let active = is_active(item);
            let bg = if active { ACTIVE_BG } else { INACTIVE_BG };
            let text_color = if active { ACTIVE_TEXT } else { INACTIVE_TEXT };
            let action = action(item);
            rsx! {
                button {
                    name: button_name,
                    width: {BUTTON_W},
                    height: {BUTTON_H},
                    background_color: bg,
                    onclick: {action.as_str()},
                    anchor {
                        point: AnchorPoint::TopLeft,
                        relative_point: AnchorPoint::TopLeft,
                        x: {x},
                        y: {y},
                    }
                    fontstring {
                        name: label_name,
                        width: {BUTTON_W},
                        height: {BUTTON_H},
                        text: {item.label()},
                        font_size: 10.0,
                        font_color: text_color,
                        justify_h: "CENTER",
                        anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
                    }
                }
            }
        })
        .collect();
    rsx! {
        r#frame {
            name: {DynName(format!("{prefix}Panel"))},
            width: {FRAME_W - 2.0 * INSET},
            height: {panel_h},
            background_color: PANEL_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INSET},
                y: {start_y},
            }
            {buttons}
        }
    }
}

trait LootRuleLabel {
    fn label(&self) -> &'static str;
}

impl LootRuleLabel for LootMethod {
    fn label(&self) -> &'static str {
        LootMethod::label(*self)
    }
}

impl LootRuleLabel for LootThreshold {
    fn label(&self) -> &'static str {
        LootThreshold::label(*self)
    }
}

pub fn loot_method_token(method: LootMethod) -> &'static str {
    match method {
        LootMethod::FreeForAll => "free_for_all",
        LootMethod::RoundRobin => "round_robin",
        LootMethod::MasterLooter => "master_looter",
        LootMethod::GroupLoot => "group_loot",
        LootMethod::NeedBeforeGreed => "need_before_greed",
        LootMethod::PersonalLoot => "personal_loot",
    }
}

pub fn loot_threshold_token(threshold: LootThreshold) -> &'static str {
    match threshold {
        LootThreshold::Poor => "poor",
        LootThreshold::Common => "common",
        LootThreshold::Uncommon => "uncommon",
        LootThreshold::Rare => "rare",
        LootThreshold::Epic => "epic",
        LootThreshold::Legendary => "legendary",
    }
}

#[cfg(test)]
#[path = "loot_rules_frame_component_tests.rs"]
mod tests;
