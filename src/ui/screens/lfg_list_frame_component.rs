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
const FORM_INPUT_COLOR: &str = "1.0,1.0,1.0,1.0";
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
    let cb_id = DynName(format!("LFGRoleCheck{idx}"));
    let label_id = DynName(format!("LFGRoleLabel{idx}"));
    let check_text = if checked { "\u{2713}" } else { "" };
    rsx! {
        r#frame {
            name: cb_id,
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
                name: DynName(format!("LFGRoleCheck{idx}Text")),
                width: {ROLE_CHECK_SIZE},
                height: {ROLE_CHECK_SIZE},
                text: check_text,
                font_size: 14.0,
                font_color: CHECK_ON,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
        }
        fontstring {
            name: label_id,
            width: {ROLE_LABEL_W},
            height: {ROLE_CHECK_SIZE},
            text: label,
            font_size: 10.0,
            font_color: ROLE_LABEL_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x + ROLE_CHECK_SIZE + 4.0},
                y: {-ROLE_ROW_Y},
            }
        }
    }
}

fn activity_dropdown(activity: &str) -> Element {
    let x = FRAME_W - DROPDOWN_W - ROLE_INSET;
    rsx! {
        r#frame {
            name: "LFGActivityDropdown",
            width: {DROPDOWN_W},
            height: {DROPDOWN_H},
            background_color: DROPDOWN_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {-ROLE_ROW_Y},
            }
            fontstring {
                name: "LFGActivityDropdownText",
                width: {DROPDOWN_W - 8.0},
                height: {DROPDOWN_H},
                text: activity,
                font_size: 10.0,
                font_color: DROPDOWN_COLOR,
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

fn group_list_panel(groups: &[GroupListEntry]) -> Element {
    let content_y = -CONTENT_TOP;
    let content_w = FRAME_W - 2.0 * CONTENT_INSET;
    let content_h = FRAME_H - CONTENT_TOP - CONTENT_INSET;
    let list_w = content_w - 2.0 * GROUP_INSET;
    let header = group_header(list_w);
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
            {header}
            {rows}
            r#frame {
                name: "LFGApplyButton",
                width: {APPLY_BTN_W},
                height: {APPLY_BTN_H},
                background_color: APPLY_BTN_BG,
                anchor {
                    point: AnchorPoint::TopRight,
                    relative_point: AnchorPoint::TopRight,
                    x: {-GROUP_INSET},
                    y: {btn_y},
                }
                fontstring {
                    name: "LFGApplyButtonText",
                    width: {APPLY_BTN_W},
                    height: {APPLY_BTN_H},
                    text: "Apply",
                    font_size: 10.0,
                    font_color: APPLY_BTN_TEXT,
                    justify_h: "CENTER",
                    anchor {
                        point: AnchorPoint::TopLeft,
                        relative_point: AnchorPoint::TopLeft,
                    }
                }
            }
        }
    }
}

fn group_header(list_w: f32) -> Element {
    let cols: Element = GROUP_COLUMNS
        .iter()
        .enumerate()
        .flat_map(|(i, (name, _))| {
            let x = group_col_x(list_w, i);
            let w = group_col_w(list_w, i);
            let cell_id = DynName(format!("LFGGroupCol{i}"));
            rsx! {
                fontstring {
                    name: cell_id,
                    width: {w},
                    height: {GROUP_HEADER_H},
                    text: name,
                    font_size: 9.0,
                    font_color: GROUP_HEADER_COLOR,
                    justify_h: "LEFT",
                    anchor {
                        point: AnchorPoint::TopLeft,
                        relative_point: AnchorPoint::TopLeft,
                        x: {x},
                        y: "0",
                    }
                }
            }
        })
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
        .flat_map(|(col, text)| {
            let cell_id = DynName(format!("LFGGroup{idx}Col{col}"));
            let x = group_col_x(list_w, col);
            let w = group_col_w(list_w, col);
            rsx! {
                fontstring {
                    name: cell_id,
                    width: {w},
                    height: {GROUP_ROW_H},
                    text: {text.as_str()},
                    font_size: 9.0,
                    font_color: GROUP_TEXT_COLOR,
                    justify_h: "LEFT",
                    anchor {
                        point: AnchorPoint::TopLeft,
                        relative_point: AnchorPoint::TopLeft,
                        x: {x},
                        y: "0",
                    }
                }
            }
        })
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
    let form_w = content_w - 2.0 * FORM_INSET;
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

fn form_row(prefix: &str, label: &str, value: &str, row: usize) -> Element {
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
    rsx! {
        r#frame {
            name: "LFGFormVoiceCheck",
            width: {ROLE_CHECK_SIZE},
            height: {ROLE_CHECK_SIZE},
            background_color: CHECK_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {FORM_INSET + FORM_LABEL_W + 4.0},
                y: {y},
            }
            fontstring {
                name: "LFGFormVoiceCheckText",
                width: {ROLE_CHECK_SIZE},
                height: {ROLE_CHECK_SIZE},
                text: voice_text,
                font_size: 14.0,
                font_color: CHECK_ON,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
        }
        fontstring {
            name: "LFGFormVoiceLabel",
            width: {FORM_INPUT_W},
            height: {ROLE_CHECK_SIZE},
            text: "Voice Chat",
            font_size: 10.0,
            font_color: FORM_LABEL_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {FORM_INSET + FORM_LABEL_W + 4.0 + ROLE_CHECK_SIZE + 4.0},
                y: {y},
            }
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
mod tests {
    use super::*;
    use ui_toolkit::layout::{LayoutRect, recompute_layouts};
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn make_test_state() -> LFGListFrameState {
        LFGListFrameState {
            visible: true,
            ..Default::default()
        }
    }

    fn build_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(lfg_list_frame_screen).sync(&shared, &mut reg);
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

    #[test]
    fn builds_frame_and_title() {
        let reg = build_registry();
        assert!(reg.get_by_name("LFGListFrame").is_some());
        assert!(reg.get_by_name("LFGListFrameTitle").is_some());
    }

    #[test]
    fn builds_role_checkboxes() {
        let reg = build_registry();
        for i in 0..3 {
            assert!(
                reg.get_by_name(&format!("LFGRoleCheck{i}")).is_some(),
                "LFGRoleCheck{i} missing"
            );
            assert!(
                reg.get_by_name(&format!("LFGRoleLabel{i}")).is_some(),
                "LFGRoleLabel{i} missing"
            );
        }
    }

    #[test]
    fn builds_activity_dropdown() {
        let reg = build_registry();
        assert!(reg.get_by_name("LFGActivityDropdown").is_some());
        assert!(reg.get_by_name("LFGActivityDropdownText").is_some());
    }

    #[test]
    fn builds_content_area() {
        let reg = build_registry();
        assert!(reg.get_by_name("LFGContentArea").is_some());
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(LFGListFrameState::default());
        Screen::new(lfg_list_frame_screen).sync(&shared, &mut reg);
        let id = reg.get_by_name("LFGListFrame").expect("frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    // --- Coord validation ---

    #[test]
    fn coord_main_frame_centered() {
        let reg = layout_registry();
        let r = rect(&reg, "LFGListFrame");
        let expected_x = (1920.0 - FRAME_W) / 2.0;
        let expected_y = (1080.0 - FRAME_H) / 2.0;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.y - expected_y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
    }

    #[test]
    fn coord_first_role_checkbox() {
        let reg = layout_registry();
        let frame_x = (1920.0 - FRAME_W) / 2.0;
        let frame_y = (1080.0 - FRAME_H) / 2.0;
        let r = rect(&reg, "LFGRoleCheck0");
        assert!((r.x - (frame_x + ROLE_INSET)).abs() < 1.0);
        assert!((r.y - (frame_y + ROLE_ROW_Y)).abs() < 1.0);
        assert!((r.width - ROLE_CHECK_SIZE).abs() < 1.0);
    }

    #[test]
    fn coord_activity_dropdown() {
        let reg = layout_registry();
        let r = rect(&reg, "LFGActivityDropdown");
        assert!((r.width - DROPDOWN_W).abs() < 1.0);
        assert!((r.height - DROPDOWN_H).abs() < 1.0);
    }

    // --- Group list tests ---

    fn make_group_state() -> LFGListFrameState {
        let mut state = make_test_state();
        state.groups = vec![
            GroupListEntry {
                leader: "Arthas".into(),
                members: "3/5".into(),
                activity: "Deadmines".into(),
                note: "Need healer".into(),
            },
            GroupListEntry {
                leader: "Thrall".into(),
                members: "4/5".into(),
                activity: "SFK".into(),
                note: String::new(),
            },
        ];
        state
    }

    fn group_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_group_state());
        Screen::new(lfg_list_frame_screen).sync(&shared, &mut reg);
        reg
    }

    #[test]
    fn group_list_builds_header_and_rows() {
        let reg = group_registry();
        assert!(reg.get_by_name("LFGGroupHeader").is_some());
        for i in 0..GROUP_COLUMNS.len() {
            assert!(
                reg.get_by_name(&format!("LFGGroupCol{i}")).is_some(),
                "LFGGroupCol{i} missing"
            );
        }
        for i in 0..2 {
            assert!(
                reg.get_by_name(&format!("LFGGroup{i}")).is_some(),
                "LFGGroup{i} missing"
            );
            for col in 0..GROUP_COLUMNS.len() {
                assert!(
                    reg.get_by_name(&format!("LFGGroup{i}Col{col}")).is_some(),
                    "LFGGroup{i}Col{col} missing"
                );
            }
        }
    }

    #[test]
    fn group_list_has_apply_button() {
        let reg = group_registry();
        assert!(reg.get_by_name("LFGApplyButton").is_some());
        assert!(reg.get_by_name("LFGApplyButtonText").is_some());
    }

    // --- Create group form tests ---

    #[test]
    fn create_form_builds_inputs() {
        let reg = build_registry();
        assert!(reg.get_by_name("LFGCreateGroupForm").is_some());
        assert!(reg.get_by_name("LFGFormTitleInput").is_some());
        assert!(reg.get_by_name("LFGFormDescInput").is_some());
        assert!(reg.get_by_name("LFGFormILevelInput").is_some());
    }

    #[test]
    fn create_form_builds_voice_and_submit() {
        let reg = build_registry();
        assert!(reg.get_by_name("LFGFormVoiceCheck").is_some());
        assert!(reg.get_by_name("LFGFormVoiceLabel").is_some());
        assert!(reg.get_by_name("LFGFormSubmitButton").is_some());
    }

    #[test]
    fn create_form_hidden_by_default() {
        let reg = build_registry();
        let id = reg.get_by_name("LFGCreateGroupForm").expect("form");
        assert!(reg.get(id).expect("data").hidden);
    }

    // --- Additional coord validation ---

    #[test]
    fn coord_group_header() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_group_state());
        Screen::new(lfg_list_frame_screen).sync(&shared, &mut reg);
        recompute_layouts(&mut reg);

        let content = rect(&reg, "LFGContentArea");
        let header = rect(&reg, "LFGGroupHeader");
        assert!((header.x - (content.x + GROUP_INSET)).abs() < 1.0);
        assert!((header.y - (content.y + GROUP_INSET)).abs() < 1.0);
        assert!((header.height - GROUP_HEADER_H).abs() < 1.0);
    }

    #[test]
    fn coord_apply_button() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_group_state());
        Screen::new(lfg_list_frame_screen).sync(&shared, &mut reg);
        recompute_layouts(&mut reg);

        let r = rect(&reg, "LFGApplyButton");
        assert!((r.width - APPLY_BTN_W).abs() < 1.0);
        assert!((r.height - APPLY_BTN_H).abs() < 1.0);
    }
}
