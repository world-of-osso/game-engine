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

pub const FRAME_W: f32 = 450.0;
pub const FRAME_H: f32 = 500.0;
const HEADER_H: f32 = 28.0;
const PREVIEW_H: f32 = 220.0;
const PREVIEW_INSET: f32 = 12.0;
const OPTION_ROW_H: f32 = 28.0;
const OPTION_ROW_GAP: f32 = 4.0;
const OPTION_INSET: f32 = 12.0;
const OPTION_LABEL_W: f32 = 120.0;
const OPTION_VALUE_W: f32 = 160.0;
const ARROW_BTN_SIZE: f32 = 24.0;
const ARROW_GAP: f32 = 4.0;
const BUTTON_W: f32 = 100.0;
const BUTTON_H: f32 = 28.0;
const BUTTON_GAP: f32 = 12.0;
const COST_H: f32 = 18.0;
const OPTIONS_TOP: f32 = HEADER_H + PREVIEW_H + PREVIEW_INSET;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const PREVIEW_BG: &str = "0.02,0.02,0.02,0.95";
const LABEL_COLOR: &str = "0.8,0.8,0.8,1.0";
const VALUE_BG: &str = "0.1,0.1,0.1,0.9";
const VALUE_COLOR: &str = "1.0,1.0,1.0,1.0";
const ARROW_BG: &str = "0.15,0.12,0.05,0.95";
const ARROW_COLOR: &str = "1.0,0.82,0.0,1.0";
const ACCEPT_BG: &str = "0.15,0.25,0.1,0.95";
const ACCEPT_COLOR: &str = "0.2,1.0,0.2,1.0";
const CANCEL_BG: &str = "0.25,0.1,0.1,0.95";
const CANCEL_COLOR: &str = "1.0,0.4,0.4,1.0";
const COST_COLOR: &str = "1.0,0.82,0.0,1.0";

pub const MAX_OPTION_ROWS: usize = 8;

#[derive(Clone, Debug, PartialEq)]
pub struct CustomizationOption {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BarberShopFrameState {
    pub visible: bool,
    pub options: Vec<CustomizationOption>,
    pub cost: String,
}

impl Default for BarberShopFrameState {
    fn default() -> Self {
        Self {
            visible: false,
            options: vec![],
            cost: "Free".into(),
        }
    }
}

pub fn barber_shop_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<BarberShopFrameState>()
        .expect("BarberShopFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "BarberShopFrame",
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
            {model_preview()}
            {option_rows(&state.options)}
            {cost_display(&state.cost)}
            {action_buttons()}
        }
    }
}

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "BarberShopFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Barber Shop",
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

fn model_preview() -> Element {
    let preview_w = FRAME_W - 2.0 * PREVIEW_INSET;
    rsx! {
        r#frame {
            name: "BarberShopModelPreview",
            width: {preview_w},
            height: {PREVIEW_H},
            background_color: PREVIEW_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {PREVIEW_INSET},
                y: {-HEADER_H},
            }
        }
    }
}

fn option_rows(options: &[CustomizationOption]) -> Element {
    options
        .iter()
        .enumerate()
        .take(MAX_OPTION_ROWS)
        .flat_map(|(i, opt)| option_row(i, opt))
        .collect()
}

fn option_row(idx: usize, opt: &CustomizationOption) -> Element {
    let row_id = DynName(format!("BarberShopOption{idx}"));
    let label_id = DynName(format!("BarberShopOption{idx}Label"));
    let value_id = DynName(format!("BarberShopOption{idx}Value"));
    let left_id = DynName(format!("BarberShopOption{idx}Left"));
    let right_id = DynName(format!("BarberShopOption{idx}Right"));
    let y = -(OPTIONS_TOP + idx as f32 * (OPTION_ROW_H + OPTION_ROW_GAP));
    let arrow_x = OPTION_INSET + OPTION_LABEL_W;
    let value_x = arrow_x + ARROW_BTN_SIZE + ARROW_GAP;
    let right_x = value_x + OPTION_VALUE_W + ARROW_GAP;
    rsx! {
        r#frame {
            name: row_id,
            width: {FRAME_W - 2.0 * OPTION_INSET},
            height: {OPTION_ROW_H},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {OPTION_INSET},
                y: {y},
            }
            fontstring {
                name: label_id,
                width: {OPTION_LABEL_W},
                height: {OPTION_ROW_H},
                text: {opt.label.as_str()},
                font_size: 10.0,
                font_color: LABEL_COLOR,
                justify_h: "RIGHT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
            {arrow_button(left_id, "<", arrow_x - OPTION_INSET)}
            r#frame {
                name: value_id,
                width: {OPTION_VALUE_W},
                height: {OPTION_ROW_H},
                background_color: VALUE_BG,
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {value_x - OPTION_INSET},
                    y: "0",
                }
                fontstring {
                    name: DynName(format!("BarberShopOption{idx}ValueText")),
                    width: {OPTION_VALUE_W},
                    height: {OPTION_ROW_H},
                    text: {opt.value.as_str()},
                    font_size: 10.0,
                    font_color: VALUE_COLOR,
                    justify_h: "CENTER",
                    anchor {
                        point: AnchorPoint::TopLeft,
                        relative_point: AnchorPoint::TopLeft,
                    }
                }
            }
            {arrow_button(right_id, ">", right_x - OPTION_INSET)}
        }
    }
}

fn arrow_button(name: DynName, text: &str, x: f32) -> Element {
    let text_name = DynName(format!("{}Text", name.0));
    rsx! {
        r#frame {
            name,
            width: {ARROW_BTN_SIZE},
            height: {ARROW_BTN_SIZE},
            background_color: ARROW_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: "0",
            }
            fontstring {
                name: text_name,
                width: {ARROW_BTN_SIZE},
                height: {ARROW_BTN_SIZE},
                text,
                font_size: 14.0,
                font_color: ARROW_COLOR,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
        }
    }
}

fn cost_display(cost: &str) -> Element {
    let y = -(FRAME_H - BUTTON_H - COST_H - 16.0);
    rsx! {
        fontstring {
            name: "BarberShopCost",
            width: {FRAME_W},
            height: {COST_H},
            text: cost,
            font_size: 11.0,
            font_color: COST_COLOR,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: {y},
            }
        }
    }
}

fn action_buttons() -> Element {
    let total_w = 2.0 * BUTTON_W + BUTTON_GAP;
    let x_start = (FRAME_W - total_w) / 2.0;
    let y = -(FRAME_H - BUTTON_H - 8.0);
    rsx! {
        r#frame {
            name: "BarberShopAcceptButton",
            width: {BUTTON_W},
            height: {BUTTON_H},
            background_color: ACCEPT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x_start},
                y: {y},
            }
            fontstring {
                name: "BarberShopAcceptButtonText",
                width: {BUTTON_W},
                height: {BUTTON_H},
                text: "Accept",
                font_size: 11.0,
                font_color: ACCEPT_COLOR,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
        }
        r#frame {
            name: "BarberShopCancelButton",
            width: {BUTTON_W},
            height: {BUTTON_H},
            background_color: CANCEL_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x_start + BUTTON_W + BUTTON_GAP},
                y: {y},
            }
            fontstring {
                name: "BarberShopCancelButtonText",
                width: {BUTTON_W},
                height: {BUTTON_H},
                text: "Cancel",
                font_size: 11.0,
                font_color: CANCEL_COLOR,
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

    fn sample_options() -> Vec<CustomizationOption> {
        vec![
            CustomizationOption {
                label: "Hair Style".into(),
                value: "Style 3".into(),
            },
            CustomizationOption {
                label: "Hair Color".into(),
                value: "Brown".into(),
            },
            CustomizationOption {
                label: "Facial Hair".into(),
                value: "Goatee".into(),
            },
        ]
    }

    fn make_test_state() -> BarberShopFrameState {
        BarberShopFrameState {
            visible: true,
            options: sample_options(),
            cost: "1g 50s".into(),
        }
    }

    fn build_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(barber_shop_frame_screen).sync(&shared, &mut reg);
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
        assert!(reg.get_by_name("BarberShopFrame").is_some());
        assert!(reg.get_by_name("BarberShopFrameTitle").is_some());
    }

    #[test]
    fn builds_model_preview() {
        let reg = build_registry();
        assert!(reg.get_by_name("BarberShopModelPreview").is_some());
    }

    #[test]
    fn builds_option_rows_with_arrows() {
        let reg = build_registry();
        for i in 0..3 {
            assert!(
                reg.get_by_name(&format!("BarberShopOption{i}")).is_some(),
                "BarberShopOption{i} missing"
            );
            assert!(
                reg.get_by_name(&format!("BarberShopOption{i}Left"))
                    .is_some(),
                "BarberShopOption{i}Left missing"
            );
            assert!(
                reg.get_by_name(&format!("BarberShopOption{i}Right"))
                    .is_some(),
                "BarberShopOption{i}Right missing"
            );
            assert!(
                reg.get_by_name(&format!("BarberShopOption{i}Value"))
                    .is_some(),
                "BarberShopOption{i}Value missing"
            );
        }
    }

    #[test]
    fn builds_accept_cancel_buttons() {
        let reg = build_registry();
        assert!(reg.get_by_name("BarberShopAcceptButton").is_some());
        assert!(reg.get_by_name("BarberShopCancelButton").is_some());
        assert!(reg.get_by_name("BarberShopCost").is_some());
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        let mut state = make_test_state();
        state.visible = false;
        shared.insert(state);
        Screen::new(barber_shop_frame_screen).sync(&shared, &mut reg);

        let id = reg.get_by_name("BarberShopFrame").expect("frame");
        let frame = reg.get(id).expect("data");
        assert!(frame.hidden);
    }

    // --- Coord validation ---

    #[test]
    fn coord_main_frame_centered() {
        let reg = layout_registry();
        let r = rect(&reg, "BarberShopFrame");
        let expected_x = (1920.0 - FRAME_W) / 2.0;
        let expected_y = (1080.0 - FRAME_H) / 2.0;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.y - expected_y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
        assert!((r.height - FRAME_H).abs() < 1.0);
    }

    #[test]
    fn coord_model_preview() {
        let reg = layout_registry();
        let frame_x = (1920.0 - FRAME_W) / 2.0;
        let frame_y = (1080.0 - FRAME_H) / 2.0;
        let r = rect(&reg, "BarberShopModelPreview");
        assert!((r.x - (frame_x + PREVIEW_INSET)).abs() < 1.0);
        assert!((r.y - (frame_y + HEADER_H)).abs() < 1.0);
        assert!((r.width - (FRAME_W - 2.0 * PREVIEW_INSET)).abs() < 1.0);
        assert!((r.height - PREVIEW_H).abs() < 1.0);
    }

    #[test]
    fn coord_first_option_row() {
        let reg = layout_registry();
        let frame_x = (1920.0 - FRAME_W) / 2.0;
        let frame_y = (1080.0 - FRAME_H) / 2.0;
        let r = rect(&reg, "BarberShopOption0");
        assert!((r.x - (frame_x + OPTION_INSET)).abs() < 1.0);
        let expected_y = frame_y + OPTIONS_TOP;
        assert!(
            (r.y - expected_y).abs() < 1.0,
            "y: expected {expected_y}, got {}",
            r.y
        );
    }

    #[test]
    fn coord_second_option_row_offset() {
        let reg = layout_registry();
        let r0 = rect(&reg, "BarberShopOption0");
        let r1 = rect(&reg, "BarberShopOption1");
        let expected_gap = OPTION_ROW_H + OPTION_ROW_GAP;
        let actual_gap = r1.y - r0.y;
        assert!(
            (actual_gap - expected_gap).abs() < 1.0,
            "row gap: expected {expected_gap}, got {actual_gap}"
        );
    }

    #[test]
    fn coord_accept_cancel_buttons() {
        let reg = layout_registry();
        let accept = rect(&reg, "BarberShopAcceptButton");
        let cancel = rect(&reg, "BarberShopCancelButton");
        assert!((accept.width - BUTTON_W).abs() < 1.0);
        assert!((cancel.width - BUTTON_W).abs() < 1.0);
        let spacing = cancel.x - accept.x;
        let expected = BUTTON_W + BUTTON_GAP;
        assert!(
            (spacing - expected).abs() < 1.0,
            "button spacing: expected {expected}, got {spacing}"
        );
    }
}
