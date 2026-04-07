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

pub const ROLES: &[&str] = &["Tank", "Healer", "DPS"];

#[derive(Clone, Debug, PartialEq)]
pub struct LFGListFrameState {
    pub visible: bool,
    pub tank_checked: bool,
    pub healer_checked: bool,
    pub dps_checked: bool,
    pub activity: String,
}

impl Default for LFGListFrameState {
    fn default() -> Self {
        Self {
            visible: false,
            tank_checked: false,
            healer_checked: false,
            dps_checked: true,
            activity: "Dungeons".into(),
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
            {content_area()}
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

fn content_area() -> Element {
    let content_y = -CONTENT_TOP;
    let content_w = FRAME_W - 2.0 * CONTENT_INSET;
    let content_h = FRAME_H - CONTENT_TOP - CONTENT_INSET;
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
}
