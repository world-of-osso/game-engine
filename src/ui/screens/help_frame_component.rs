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

pub const FRAME_W: f32 = 400.0;
pub const FRAME_H: f32 = 460.0;
const HEADER_H: f32 = 28.0;
const BUTTON_W: f32 = 240.0;
const BUTTON_H: f32 = 36.0;
const BUTTON_GAP: f32 = 8.0;
const BUTTON_TOP: f32 = HEADER_H + 20.0;
const CONTENT_INSET: f32 = 8.0;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const BUTTON_BG: &str = "0.15,0.12,0.05,0.95";
const BUTTON_TEXT_COLOR: &str = "1.0,0.82,0.0,1.0";
const CONTENT_BG: &str = "0.0,0.0,0.0,0.3";

pub const CATEGORY_BUTTONS: &[&str] = &["Knowledge Base", "Submit Ticket", "Bug Report"];

#[derive(Clone, Debug, PartialEq)]
pub struct HelpFrameState {
    pub visible: bool,
}

impl Default for HelpFrameState {
    fn default() -> Self {
        Self { visible: false }
    }
}

pub fn help_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<HelpFrameState>()
        .expect("HelpFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "HelpFrame",
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
            {category_buttons()}
            {content_area()}
        }
    }
}

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "HelpFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Help",
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

fn category_buttons() -> Element {
    let x_start = (FRAME_W - BUTTON_W) / 2.0;
    CATEGORY_BUTTONS
        .iter()
        .enumerate()
        .flat_map(|(i, label)| {
            let y = -(BUTTON_TOP + i as f32 * (BUTTON_H + BUTTON_GAP));
            category_button(i, label, x_start, y)
        })
        .collect()
}

fn category_button(idx: usize, label: &str, x: f32, y: f32) -> Element {
    let btn_name = DynName(format!("HelpCategoryBtn{idx}"));
    let txt_name = DynName(format!("HelpCategoryBtn{idx}Text"));
    rsx! {
        r#frame {
            name: btn_name,
            width: {BUTTON_W},
            height: {BUTTON_H},
            background_color: BUTTON_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            fontstring {
                name: txt_name,
                width: {BUTTON_W},
                height: {BUTTON_H},
                text: label,
                font_size: 12.0,
                font_color: BUTTON_TEXT_COLOR,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
        }
    }
}

fn content_area() -> Element {
    let buttons_h = CATEGORY_BUTTONS.len() as f32 * (BUTTON_H + BUTTON_GAP);
    let content_y = -(BUTTON_TOP + buttons_h);
    let content_w = FRAME_W - 2.0 * CONTENT_INSET;
    let content_h = FRAME_H - BUTTON_TOP - buttons_h - CONTENT_INSET;
    rsx! {
        r#frame {
            name: "HelpContentArea",
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

    fn build_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(HelpFrameState { visible: true });
        Screen::new(help_frame_screen).sync(&shared, &mut reg);
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
        assert!(reg.get_by_name("HelpFrame").is_some());
        assert!(reg.get_by_name("HelpFrameTitle").is_some());
    }

    #[test]
    fn builds_category_buttons() {
        let reg = build_registry();
        for i in 0..CATEGORY_BUTTONS.len() {
            assert!(
                reg.get_by_name(&format!("HelpCategoryBtn{i}")).is_some(),
                "HelpCategoryBtn{i} missing"
            );
            assert!(
                reg.get_by_name(&format!("HelpCategoryBtn{i}Text"))
                    .is_some(),
                "HelpCategoryBtn{i}Text missing"
            );
        }
    }

    #[test]
    fn builds_content_area() {
        let reg = build_registry();
        assert!(reg.get_by_name("HelpContentArea").is_some());
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(HelpFrameState::default());
        Screen::new(help_frame_screen).sync(&shared, &mut reg);
        let id = reg.get_by_name("HelpFrame").expect("frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    // --- Coord validation ---

    #[test]
    fn coord_main_frame_centered() {
        let reg = layout_registry();
        let r = rect(&reg, "HelpFrame");
        let expected_x = (1920.0 - FRAME_W) / 2.0;
        let expected_y = (1080.0 - FRAME_H) / 2.0;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.y - expected_y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
    }

    #[test]
    fn coord_first_button() {
        let reg = layout_registry();
        let frame_x = (1920.0 - FRAME_W) / 2.0;
        let frame_y = (1080.0 - FRAME_H) / 2.0;
        let r = rect(&reg, "HelpCategoryBtn0");
        let expected_x = frame_x + (FRAME_W - BUTTON_W) / 2.0;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.y - (frame_y + BUTTON_TOP)).abs() < 1.0);
        assert!((r.width - BUTTON_W).abs() < 1.0);
        assert!((r.height - BUTTON_H).abs() < 1.0);
    }

    #[test]
    fn coord_button_spacing() {
        let reg = layout_registry();
        let b0 = rect(&reg, "HelpCategoryBtn0");
        let b1 = rect(&reg, "HelpCategoryBtn1");
        let spacing = b1.y - b0.y;
        let expected = BUTTON_H + BUTTON_GAP;
        assert!(
            (spacing - expected).abs() < 1.0,
            "spacing: expected {expected}, got {spacing}"
        );
    }
}
