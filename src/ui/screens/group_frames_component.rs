use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::screens::party_frame_component::{PartyFrameState, party_frame_screen};
use crate::ui::screens::raid_frame_component::{RaidFrameState, raid_frame_screen};
use crate::ui::strata::FrameStrata;

pub const ACTION_GROUP_MENU_TARGET: &str = "group_menu_target";
pub const ACTION_GROUP_MENU_INSPECT: &str = "group_menu_inspect";
pub const ACTION_GROUP_MENU_CLOSE: &str = "group_menu_close";

pub const GROUP_MENU_W: f32 = 140.0;
pub const GROUP_MENU_H: f32 = 96.0;

const GROUP_MENU_BG: &str = "0.03,0.03,0.03,0.96";
const GROUP_MENU_BORDER: &str = "0.68,0.54,0.25,1.0";
const GROUP_MENU_TITLE: &str = "1.0,0.82,0.0,1.0";
const GROUP_MENU_DIVIDER: &str = "0.25,0.22,0.16,1.0";

const GROUP_MENU_BTN_W: f32 = GROUP_MENU_W - 12.0;
const GROUP_MENU_BTN_H: f32 = 22.0;
const GROUP_MENU_BTN_X: f32 = 6.0;
const GROUP_MENU_BTN_START_Y: f32 = -28.0;
const GROUP_MENU_BTN_GAP: f32 = 3.0;

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GroupFramesState {
    pub party: PartyFrameState,
    pub raid: RaidFrameState,
    pub menu: GroupContextMenuState,
}

impl Default for GroupFramesState {
    fn default() -> Self {
        Self {
            party: PartyFrameState::default(),
            raid: RaidFrameState::default(),
            menu: GroupContextMenuState::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct GroupContextMenuState {
    pub visible: bool,
    pub title: String,
    pub x: f32,
    pub y: f32,
}

pub fn group_frames_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<GroupFramesState>()
        .expect("GroupFramesState must be in SharedContext");
    rsx! {
        {party_frame_screen(ctx)}
        {raid_frame_screen(ctx)}
        {group_context_menu(&state.menu)}
    }
}

fn group_context_menu(state: &GroupContextMenuState) -> Element {
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "GroupContextMenu",
            width: {GROUP_MENU_W},
            height: {GROUP_MENU_H},
            hidden: hide,
            strata: FrameStrata::Dialog,
            frame_level: 60.0,
            background_color: GROUP_MENU_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {state.x},
                y: {-state.y},
            }
            r#frame {
                name: "GroupContextMenuBorderTop",
                width: {GROUP_MENU_W},
                height: 1.0,
                background_color: GROUP_MENU_BORDER,
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
            r#frame {
                name: "GroupContextMenuBorderBottom",
                width: {GROUP_MENU_W},
                height: 1.0,
                background_color: GROUP_MENU_BORDER,
                anchor { point: AnchorPoint::BottomLeft, relative_point: AnchorPoint::BottomLeft }
            }
            r#frame {
                name: "GroupContextMenuBorderLeft",
                width: 1.0,
                height: {GROUP_MENU_H},
                background_color: GROUP_MENU_BORDER,
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
            r#frame {
                name: "GroupContextMenuBorderRight",
                width: 1.0,
                height: {GROUP_MENU_H},
                background_color: GROUP_MENU_BORDER,
                anchor { point: AnchorPoint::TopRight, relative_point: AnchorPoint::TopRight }
            }
            fontstring {
                name: "GroupContextMenuTitle",
                width: {GROUP_MENU_W - 12.0},
                height: 14.0,
                text: {state.title.as_str()},
                font_size: 10.0,
                font_color: GROUP_MENU_TITLE,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "6",
                    y: "-6",
                }
            }
            r#frame {
                name: "GroupContextMenuDivider",
                width: {GROUP_MENU_W - 12.0},
                height: 1.0,
                background_color: GROUP_MENU_DIVIDER,
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "6",
                    y: "-22",
                }
            }
            {group_menu_button("GroupContextMenuTarget", "Target", ACTION_GROUP_MENU_TARGET, 0)}
            {group_menu_button("GroupContextMenuInspect", "Inspect", ACTION_GROUP_MENU_INSPECT, 1)}
            {group_menu_button("GroupContextMenuClose", "Close", ACTION_GROUP_MENU_CLOSE, 2)}
        }
    }
}

fn group_menu_button(name: &str, label: &str, action: &str, index: usize) -> Element {
    let y = GROUP_MENU_BTN_START_Y - index as f32 * (GROUP_MENU_BTN_H + GROUP_MENU_BTN_GAP);
    rsx! {
        button {
            name: {DynName(name.to_string())},
            width: {GROUP_MENU_BTN_W},
            height: {GROUP_MENU_BTN_H},
            text: {label},
            font_size: 10.0,
            onclick: {action},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {GROUP_MENU_BTN_X},
                y: {y},
            }
        }
    }
}
