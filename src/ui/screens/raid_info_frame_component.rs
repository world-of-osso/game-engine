//! Retail `RaidInfoFrame` (Blizzard_RaidFrame/Mainline/RaidFrame.xml/.lua) beside the
//! FriendsFrame Raid tab: "Raid Information", "Your saved raid instance status.", one
//! `RaidInfoInstanceTemplate` row per saved instance (name, difficulty, time to reset or
//! "Expired" in grey, extended mark), the Extend Raid Lock button and Close. The Raid
//! tab's "Raid Info" button (`RaidFrameRaidInfoButton`) opens it while there are saved
//! instances.

use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::strata::FrameStrata;

pub const ACTION_RAID_INFO_TOGGLE: &str = "raid_info_toggle";
pub const ACTION_RAID_INFO_CLOSE: &str = "raid_info_close";
pub const ACTION_RAID_INFO_EXTEND: &str = "raid_info_extend";
pub const ACTION_RAID_INFO_SELECT_PREFIX: &str = "raid_info_select:";

pub const RAID_INFO_W: f32 = 300.0;
pub const RAID_INFO_H: f32 = 300.0;
const ROW_H: f32 = 32.0;
const ROWS_TOP: f32 = 52.0;
const INSET: f32 = 10.0;
const BUTTON_H: f32 = 22.0;

const FRAME_BG: &str = "0.06,0.05,0.04,0.95";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const TEXT_COLOR: &str = "1.0,1.0,1.0,1.0";
const EXPIRED_COLOR: &str = "0.5,0.5,0.5,1.0";
const DESC_COLOR: &str = "0.8,0.8,0.8,1.0";
const ROW_SELECTED_BG: &str = "0.25,0.2,0.05,0.9";
const ROW_BG: &str = "0.0,0.0,0.0,0.25";
const BUTTON_BG: &str = "0.15,0.12,0.05,0.95";
const BUTTON_DISABLED_BG: &str = "0.08,0.08,0.08,0.95";

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One `RaidInfoInstanceTemplate` row (`RaidInfoFrame_InitButton`).
#[derive(Clone, Debug, PartialEq)]
pub struct RaidInfoRow {
    pub name: String,
    /// `difficultyName`.
    pub difficulty: String,
    /// `SecondsToTime(reset, true, nil, 3)`, or `RAID_INSTANCE_EXPIRES_EXPIRED`.
    pub reset: String,
    /// Neither locked nor extended: name and reset grey.
    pub expired: bool,
    pub extended: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct RaidInfoState {
    /// `RaidInfoFrame:IsShown`.
    pub visible: bool,
    /// `RaidFrameRaidInfoButton:SetEnabled(GetNumSavedInstances() > 0)`.
    pub button_enabled: bool,
    pub rows: Vec<RaidInfoRow>,
    pub selected: Option<usize>,
    /// `EXTEND_RAID_LOCK`, `UNEXTEND_RAID_LOCK` or `REACTIVATE_RAID_LOCK`.
    pub extend_text: String,
    pub extend_enabled: bool,
}

/// The Raid tab body: the "Raid Info" button at its top right.
pub fn raid_tab_body(state: &RaidInfoState, content_w: f32) -> Element {
    let bg = if state.button_enabled {
        BUTTON_BG
    } else {
        BUTTON_DISABLED_BG
    };
    let color = if state.button_enabled {
        TITLE_COLOR
    } else {
        EXPIRED_COLOR
    };
    let label = rsx! {
        fontstring {
            name: "RaidFrameRaidInfoButtonText",
            width: 90.0,
            height: {BUTTON_H},
            text: "Raid Info",
            font_size: 10.0,
            font_color: color,
            justify_h: "CENTER",
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
        }
    };
    if state.button_enabled {
        rsx! {
            r#frame {
                name: "RaidFrameRaidInfoButton",
                width: 90.0,
                height: {BUTTON_H},
                background_color: bg,
                onclick: ACTION_RAID_INFO_TOGGLE,
                pos_type: "absolute",
                left: {content_w - 90.0 - 4.0},
                top: 4.0,
                {label}
            }
        }
    } else {
        rsx! {
            r#frame {
                name: "RaidFrameRaidInfoButton",
                width: 90.0,
                height: {BUTTON_H},
                background_color: bg,
                pos_type: "absolute",
                left: {content_w - 90.0 - 4.0},
                top: 4.0,
                {label}
            }
        }
    }
}

/// `RaidInfoFrame`, anchored beside the FriendsFrame at `left`.
pub fn raid_info_frame(state: &RaidInfoState, left: f32) -> Element {
    let hidden = !state.visible;
    let rows: Element = state
        .rows
        .iter()
        .enumerate()
        .flat_map(|(index, row)| raid_info_row(index, row, state.selected == Some(index)))
        .collect();
    rsx! {
        r#frame {
            name: "RaidInfoFrame",
            width: {RAID_INFO_W},
            height: {RAID_INFO_H},
            hidden: hidden,
            strata: FrameStrata::Dialog,
            background_color: FRAME_BG,
            pos_type: "absolute",
            left: {left},
            top: 0.0,
            fontstring {
                name: "RaidInfoFrameTitle",
                width: {RAID_INFO_W},
                height: 20.0,
                text: "Raid Information",
                font_size: 14.0,
                font_color: TITLE_COLOR,
                justify_h: "CENTER",
                pos_type: "absolute",
                left: 0.0,
                top: 6.0,
            }
            fontstring {
                name: "RaidInfoFrameDesc",
                width: {RAID_INFO_W - 2.0 * INSET},
                height: 16.0,
                text: "Your saved raid instance status.",
                font_size: 10.0,
                font_color: DESC_COLOR,
                justify_h: "LEFT",
                pos_type: "absolute",
                left: {INSET},
                top: 30.0,
            }
            {rows}
            {raid_info_button("RaidInfoExtendButton", &state.extend_text, state.extend_enabled, ACTION_RAID_INFO_EXTEND, INSET)}
            {raid_info_button("RaidInfoCloseButton", "Close", true, ACTION_RAID_INFO_CLOSE, RAID_INFO_W - INSET - 80.0)}
        }
    }
}

/// `RaidInfoInstanceTemplate`: gold name (`GameFontNormal`), white difficulty and reset
/// (`GameFontHighlightSmall`), both grey once expired; "Extended" under the reset.
fn raid_info_row(index: usize, row: &RaidInfoRow, selected: bool) -> Element {
    let name_color = if row.expired {
        EXPIRED_COLOR
    } else {
        TITLE_COLOR
    };
    let reset_color = if row.expired {
        EXPIRED_COLOR
    } else {
        TEXT_COLOR
    };
    let action = format!("{ACTION_RAID_INFO_SELECT_PREFIX}{index}");
    let extended_hidden = !row.extended;
    let width = RAID_INFO_W - 2.0 * INSET;
    let bg = if selected { ROW_SELECTED_BG } else { ROW_BG };
    rsx! {
        r#frame {
            name: DynName(format!("RaidInfoInstance{index}")),
            width: {width},
            height: {ROW_H},
            background_color: bg,
            onclick: action.as_str(),
            pos_type: "absolute",
            left: {INSET},
            top: {ROWS_TOP + index as f32 * (ROW_H + 2.0)},
            fontstring {
                name: DynName(format!("RaidInfoInstance{index}Name")),
                width: {width * 0.6},
                height: 14.0,
                text: row.name.as_str(),
                font_size: 11.0,
                font_color: name_color,
                justify_h: "LEFT",
                pos_type: "absolute",
                left: 5.0,
                top: 2.0,
            }
            fontstring {
                name: DynName(format!("RaidInfoInstance{index}Reset")),
                width: {width * 0.4 - 5.0},
                height: 14.0,
                text: row.reset.as_str(),
                font_size: 10.0,
                font_color: reset_color,
                justify_h: "RIGHT",
                pos_type: "absolute",
                right: 5.0,
                top: 2.0,
            }
            fontstring {
                name: DynName(format!("RaidInfoInstance{index}Difficulty")),
                width: {width - 10.0},
                height: 12.0,
                text: row.difficulty.as_str(),
                font_size: 9.0,
                font_color: TEXT_COLOR,
                justify_h: "LEFT",
                pos_type: "absolute",
                left: 5.0,
                top: 17.0,
            }
            fontstring {
                name: DynName(format!("RaidInfoInstance{index}Extended")),
                width: {width * 0.4 - 5.0},
                height: 12.0,
                text: "Extended",
                hidden: extended_hidden,
                font_size: 9.0,
                font_color: TEXT_COLOR,
                justify_h: "RIGHT",
                pos_type: "absolute",
                right: 5.0,
                top: 17.0,
            }
        }
    }
}

fn raid_info_button(name: &str, text: &str, enabled: bool, action: &str, left: f32) -> Element {
    let width = if name == "RaidInfoCloseButton" {
        80.0
    } else {
        170.0
    };
    let bg = if enabled {
        BUTTON_BG
    } else {
        BUTTON_DISABLED_BG
    };
    let color = if enabled { TITLE_COLOR } else { EXPIRED_COLOR };
    let label = rsx! {
        fontstring {
            name: DynName(format!("{name}Text")),
            width: {width},
            height: {BUTTON_H},
            text: text,
            font_size: 10.0,
            font_color: color,
            justify_h: "CENTER",
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
        }
    };
    let top = RAID_INFO_H - BUTTON_H - INSET;
    if enabled {
        rsx! {
            r#frame {
                name: DynName(name.into()),
                width: {width},
                height: {BUTTON_H},
                background_color: bg,
                onclick: action,
                pos_type: "absolute",
                left: {left},
                top: {top},
                {label}
            }
        }
    } else {
        rsx! {
            r#frame {
                name: DynName(name.into()),
                width: {width},
                height: {BUTTON_H},
                background_color: bg,
                pos_type: "absolute",
                left: {left},
                top: {top},
                {label}
            }
        }
    }
}
