use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::screens::menu_primitives::{DropdownButton, dropdown_button};
use crate::ui::strata::FrameStrata;

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub const FRAME_W: f32 = 600.0;
pub const FRAME_H: f32 = 480.0;
const HEADER_H: f32 = 28.0;
const ROLE_ROW_Y: f32 = HEADER_H + 8.0;
const ROLE_CHECK_SIZE: f32 = 20.0;
const ROLE_LABEL_W: f32 = 60.0;
const ROLE_GAP: f32 = 16.0;
const ROLE_INSET: f32 = 12.0;
const DROPDOWN_W: f32 = 180.0;
const DROPDOWN_H: f32 = 26.0;
const CONTENT_TOP: f32 = ROLE_ROW_Y + ROLE_CHECK_SIZE + 12.0;
const CONTENT_INSET: f32 = 8.0;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const CHECK_BG: &str = "0.1,0.1,0.1,0.9";
const CHECK_ON: &str = "0.0,1.0,0.0,1.0";
const ROLE_LABEL_COLOR: &str = "1.0,1.0,1.0,1.0";
const DROPDOWN_BG: &str = "0.08,0.07,0.06,0.88";
const DROPDOWN_COLOR: &str = "0.6,0.6,0.6,1.0";
const CONTENT_BG: &str = "0.0,0.0,0.0,0.3";

// Group list layout
const GROUP_HEADER_H: f32 = 20.0;
const GROUP_ROW_H: f32 = 28.0;
const GROUP_ROW_GAP: f32 = 1.0;
const GROUP_INSET: f32 = 4.0;
const APPLY_BTN_W: f32 = 80.0;
const APPLY_BTN_H: f32 = 24.0;
const GROUP_HEADER_BG: &str = "0.12,0.1,0.08,0.9";
const GROUP_HEADER_COLOR: &str = "0.8,0.8,0.8,1.0";
const GROUP_ROW_EVEN: &str = "0.04,0.04,0.04,0.6";
const GROUP_ROW_ODD: &str = "0.06,0.06,0.06,0.6";
const GROUP_TEXT_COLOR: &str = "1.0,1.0,1.0,1.0";
const APPLY_BTN_BG: &str = "0.15,0.12,0.05,0.95";
const APPLY_BTN_TEXT: &str = "1.0,0.82,0.0,1.0";

// Create group form
const FORM_INSET: f32 = 8.0;
const FORM_LABEL_W: f32 = 90.0;
const FORM_INPUT_H: f32 = 24.0;
const FORM_INPUT_W: f32 = 200.0;
const FORM_ROW_GAP: f32 = 6.0;
const FORM_BTN_W: f32 = 100.0;
const FORM_BTN_H: f32 = 26.0;
const FORM_LABEL_COLOR: &str = "0.8,0.8,0.8,1.0";
const FORM_INPUT_BG: &str = "0.1,0.1,0.1,0.9";
const FORM_BTN_BG: &str = "0.15,0.12,0.05,0.95";
const FORM_BTN_TEXT: &str = "1.0,0.82,0.0,1.0";

pub const ROLES: &[&str] = &["Tank", "Healer", "DPS"];
pub const MAX_GROUP_ROWS: usize = 10;
pub const GROUP_COLUMNS: &[(&str, f32)] = &[
    ("Leader", 0.25),
    ("Members", 0.15),
    ("Activity", 0.30),
    ("Note", 0.30),
];

#[derive(Clone, Debug, PartialEq)]
pub struct GroupListEntry {
    pub leader: String,
    pub members: String,
    pub activity: String,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct CreateGroupForm {
    pub title: String,
    pub description: String,
    pub item_level: String,
    pub voice_chat: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LFGListFrameState {
    pub visible: bool,
    pub tank_checked: bool,
    pub healer_checked: bool,
    pub dps_checked: bool,
    pub activity: String,
    pub groups: Vec<GroupListEntry>,
    pub create_form: CreateGroupForm,
}

impl Default for LFGListFrameState {
    fn default() -> Self {
        Self {
            visible: false,
            tank_checked: false,
            healer_checked: false,
            dps_checked: true,
            activity: "Dungeons".into(),
            groups: vec![],
            create_form: CreateGroupForm::default(),
        }
    }
}

pub fn lfg_list_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<LFGListFrameState>()
        .expect("LFGListFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "LFGListFrame",
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
            {role_checkboxes(state)}
            {activity_dropdown(&state.activity)}
            {group_list_panel(&state.groups)}
            {create_group_form(&state.create_form)}
        }
    }
}

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "LFGListFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Group Finder",
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

fn role_checkboxes(state: &LFGListFrameState) -> Element {
    let checks = [
        ("Tank", state.tank_checked),
        ("Healer", state.healer_checked),
        ("DPS", state.dps_checked),
    ];
    checks
        .iter()
        .enumerate()
        .flat_map(|(i, (label, checked))| {
            let x = ROLE_INSET + i as f32 * (ROLE_CHECK_SIZE + ROLE_LABEL_W + ROLE_GAP);
            role_checkbox(i, label, *checked, x)
        })
        .collect()
}

fn role_checkbox(idx: usize, label: &str, checked: bool, x: f32) -> Element {
    let check_text = if checked { "\u{2713}" } else { "" };
    rsx! {
        {role_check_box(DynName(format!("LFGRoleCheck{idx}")), DynName(format!("LFGRoleCheck{idx}Text")), check_text, x)}
        {role_check_label(DynName(format!("LFGRoleLabel{idx}")), label, x + ROLE_CHECK_SIZE + 4.0)}
    }
}

fn role_check_box(id: DynName, text_id: DynName, check: &str, x: f32) -> Element {
    rsx! {
        r#frame {
            name: id,
            width: {ROLE_CHECK_SIZE},
            height: {ROLE_CHECK_SIZE},
            background_color: CHECK_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {-ROLE_ROW_Y},
            }
            fontstring {
                name: text_id,
                width: {ROLE_CHECK_SIZE},
                height: {ROLE_CHECK_SIZE},
                text: check,
                font_size: 14.0,
                font_color: CHECK_ON,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
    }
}

fn role_check_label(id: DynName, text: &str, x: f32) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {ROLE_LABEL_W},
            height: {ROLE_CHECK_SIZE},
            text: text,
            font_size: 10.0,
            font_color: ROLE_LABEL_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {x}, y: {-ROLE_ROW_Y} }
        }
    }
}

fn activity_dropdown(activity: &str) -> Element {
    let x = FRAME_W - DROPDOWN_W - ROLE_INSET;
    dropdown_button(DropdownButton {
        frame_name: "LFGActivityDropdown",
        label_name: "LFGActivityDropdownText",
        arrow_name: "LFGActivityDropdownArrow",
        text: activity,
        width: DROPDOWN_W,
        height: DROPDOWN_H,
        x,
        y: -ROLE_ROW_Y,
        background_color: DROPDOWN_BG,
        text_color: DROPDOWN_COLOR,
        arrow_color: DROPDOWN_COLOR,
        onclick: None,
    })
}

fn group_list_panel(groups: &[GroupListEntry]) -> Element {
    let content_y = -CONTENT_TOP;
    let content_w = FRAME_W - 2.0 * CONTENT_INSET;
    let content_h = FRAME_H - CONTENT_TOP - CONTENT_INSET;
    let list_w = content_w - 2.0 * GROUP_INSET;
    let rows: Element = groups
        .iter()
        .enumerate()
        .take(MAX_GROUP_ROWS)
        .flat_map(|(i, g)| group_row(i, g, list_w))
        .collect();
    let btn_y = -(content_h - APPLY_BTN_H - GROUP_INSET);
    rsx! {
        r#frame {
            name: "LFGContentArea",
            width: {content_w},
            height: {content_h},
            background_color: CONTENT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {CONTENT_INSET},
                y: {content_y},
            }
            {group_header(list_w)}
            {rows}
            {apply_button(btn_y)}
        }
    }
}

fn apply_button(y: f32) -> Element {
    rsx! {
        r#frame {
            name: "LFGApplyButton",
            width: {APPLY_BTN_W},
            height: {APPLY_BTN_H},
            background_color: APPLY_BTN_BG,
            anchor {
                point: AnchorPoint::TopRight,
                relative_point: AnchorPoint::TopRight,
                x: {-GROUP_INSET},
                y: {y},
            }
            fontstring {
                name: "LFGApplyButtonText",
                width: {APPLY_BTN_W},
                height: {APPLY_BTN_H},
                text: "Apply",
                font_size: 10.0,
                font_color: APPLY_BTN_TEXT,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
    }
}

fn group_header_cell(i: usize, name: &str, list_w: f32) -> Element {
    let id = DynName(format!("LFGGroupCol{i}"));
    rsx! {
        fontstring {
            name: id,
            width: {group_col_w(list_w, i)},
            height: {GROUP_HEADER_H},
            text: name,
            font_size: 9.0,
            font_color: GROUP_HEADER_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {group_col_x(list_w, i)}, y: "0" }
        }
    }
}

fn group_header(list_w: f32) -> Element {
    let cols: Element = GROUP_COLUMNS
        .iter()
        .enumerate()
        .flat_map(|(i, (name, _))| group_header_cell(i, name, list_w))
        .collect();
    rsx! {
        r#frame {
            name: "LFGGroupHeader",
            width: {list_w},
            height: {GROUP_HEADER_H},
            background_color: GROUP_HEADER_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {GROUP_INSET},
                y: {-GROUP_INSET},
            }
            {cols}
        }
    }
}

fn group_row_cell(row: usize, col: usize, text: &str, list_w: f32) -> Element {
    let id = DynName(format!("LFGGroup{row}Col{col}"));
    rsx! {
        fontstring {
            name: id,
            width: {group_col_w(list_w, col)},
            height: {GROUP_ROW_H},
            text: text,
            font_size: 9.0,
            font_color: GROUP_TEXT_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {group_col_x(list_w, col)}, y: "0" }
        }
    }
}

fn group_row(idx: usize, group: &GroupListEntry, list_w: f32) -> Element {
    let row_id = DynName(format!("LFGGroup{idx}"));
    let y = -(GROUP_INSET + GROUP_HEADER_H + idx as f32 * (GROUP_ROW_H + GROUP_ROW_GAP));
    let bg = if idx % 2 == 0 {
        GROUP_ROW_EVEN
    } else {
        GROUP_ROW_ODD
    };
    let values = [&group.leader, &group.members, &group.activity, &group.note];
    let cells: Element = values
        .iter()
        .enumerate()
        .flat_map(|(col, text)| group_row_cell(idx, col, text, list_w))
        .collect();
    rsx! {
        r#frame {
            name: row_id,
            width: {list_w},
            height: {GROUP_ROW_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {GROUP_INSET},
                y: {y},
            }
            {cells}
        }
    }
}

fn group_col_x(list_w: f32, col: usize) -> f32 {
    let mut x = 4.0;
    for i in 0..col {
        x += GROUP_COLUMNS[i].1 * list_w;
    }
    x
}

fn group_col_w(list_w: f32, col: usize) -> f32 {
    GROUP_COLUMNS[col].1 * list_w
}

// --- Create Group Form ---

fn create_group_form(form: &CreateGroupForm) -> Element {
    let content_w = FRAME_W - 2.0 * CONTENT_INSET;
    let content_h = FRAME_H - CONTENT_TOP - CONTENT_INSET;
    let voice_text = if form.voice_chat { "\u{2713}" } else { "" };
    rsx! {
        r#frame {
            name: "LFGCreateGroupForm",
            width: {content_w},
            height: {content_h},
            hidden: true,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {CONTENT_INSET},
                y: {-CONTENT_TOP},
            }
            {form_row("LFGFormTitle", "Title:", &form.title, 0)}
            {form_row("LFGFormDesc", "Description:", &form.description, 1)}
            {form_row("LFGFormILevel", "Item Level:", &form.item_level, 2)}
            {form_checkbox_row(voice_text)}
            {form_submit_button()}
        }
    }
}

fn form_row(prefix: &str, label: &str, _value: &str, row: usize) -> Element {
    let label_name = DynName(format!("{prefix}Label"));
    let input_name = DynName(format!("{prefix}Input"));
    let y = -(FORM_INSET + row as f32 * (FORM_INPUT_H + FORM_ROW_GAP));
    rsx! {
        fontstring {
            name: label_name,
            width: {FORM_LABEL_W},
            height: {FORM_INPUT_H},
            text: label,
            font_size: 10.0,
            font_color: FORM_LABEL_COLOR,
            justify_h: "RIGHT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {FORM_INSET},
                y: {y},
            }
        }
        r#frame {
            name: input_name,
            width: {FORM_INPUT_W},
            height: {FORM_INPUT_H},
            background_color: FORM_INPUT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {FORM_INSET + FORM_LABEL_W + 4.0},
                y: {y},
            }
        }
    }
}

fn form_checkbox_row(voice_text: &str) -> Element {
    let y = -(FORM_INSET + 3.0 * (FORM_INPUT_H + FORM_ROW_GAP));
    let check_x = FORM_INSET + FORM_LABEL_W + 4.0;
    rsx! {
        {form_check_box(voice_text, check_x, y)}
        {form_check_label(check_x + ROLE_CHECK_SIZE + 4.0, y)}
    }
}

fn form_check_box(text: &str, x: f32, y: f32) -> Element {
    rsx! {
        r#frame {
            name: "LFGFormVoiceCheck",
            width: {ROLE_CHECK_SIZE},
            height: {ROLE_CHECK_SIZE},
            background_color: CHECK_BG,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {x}, y: {y} }
            fontstring {
                name: "LFGFormVoiceCheckText",
                width: {ROLE_CHECK_SIZE},
                height: {ROLE_CHECK_SIZE},
                text: text,
                font_size: 14.0,
                font_color: CHECK_ON,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
    }
}

fn form_check_label(x: f32, y: f32) -> Element {
    rsx! {
        fontstring {
            name: "LFGFormVoiceLabel",
            width: {FORM_INPUT_W},
            height: {ROLE_CHECK_SIZE},
            text: "Voice Chat",
            font_size: 10.0,
            font_color: FORM_LABEL_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {x}, y: {y} }
        }
    }
}

fn form_submit_button() -> Element {
    let y = -(FORM_INSET + 4.0 * (FORM_INPUT_H + FORM_ROW_GAP) + FORM_ROW_GAP);
    rsx! {
        r#frame {
            name: "LFGFormSubmitButton",
            width: {FORM_BTN_W},
            height: {FORM_BTN_H},
            background_color: FORM_BTN_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {FORM_INSET + FORM_LABEL_W + 4.0},
                y: {y},
            }
            fontstring {
                name: "LFGFormSubmitButtonText",
                width: {FORM_BTN_W},
                height: {FORM_BTN_H},
                text: "Create Group",
                font_size: 10.0,
                font_color: FORM_BTN_TEXT,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "lfg_list_frame_component_tests.rs"]
mod tests;
