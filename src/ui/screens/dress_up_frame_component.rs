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

pub const FRAME_W: f32 = 334.0;
pub const FRAME_H: f32 = 423.0;
const HEADER_H: f32 = 28.0;
const PREVIEW_INSET: f32 = 8.0;
const PREVIEW_H: f32 = 280.0;
const SLOT_SIZE: f32 = 32.0;
const SLOT_GAP: f32 = 4.0;
const SLOT_INSET: f32 = 8.0;
const SLOT_ROW_TOP: f32 = HEADER_H + PREVIEW_H + PREVIEW_INSET;
const BUTTON_W: f32 = 80.0;
const BUTTON_H: f32 = 24.0;
const BUTTON_GAP: f32 = 8.0;
const BUTTON_ROW_BOTTOM: f32 = 8.0;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const PREVIEW_BG: &str = "0.02,0.02,0.02,0.95";
const SLOT_BG: &str = "0.08,0.07,0.06,0.88";
const SLOT_LABEL_COLOR: &str = "0.7,0.7,0.7,1.0";
const BUTTON_BG: &str = "0.15,0.12,0.05,0.95";
const BUTTON_TEXT_COLOR: &str = "1.0,0.82,0.0,1.0";

pub const ITEM_SLOTS: &[&str] = &[
    "Head",
    "Shoulder",
    "Chest",
    "Waist",
    "Legs",
    "Feet",
    "Wrist",
    "Hands",
    "Back",
    "Main Hand",
    "Off Hand",
    "Ranged",
];

#[derive(Clone, Debug, PartialEq)]
pub struct DressUpSlot {
    pub label: String,
    pub icon_fdid: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DressUpFrameState {
    pub visible: bool,
    pub slots: Vec<DressUpSlot>,
}

impl Default for DressUpFrameState {
    fn default() -> Self {
        Self {
            visible: false,
            slots: ITEM_SLOTS
                .iter()
                .map(|name| DressUpSlot {
                    label: name.to_string(),
                    icon_fdid: 0,
                })
                .collect(),
        }
    }
}

pub fn dress_up_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<DressUpFrameState>()
        .expect("DressUpFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "DressUpFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "450",
                y: "-80",
            }
            {title_bar()}
            {model_preview()}
            {item_slot_row(&state.slots)}
            {action_buttons()}
        }
    }
}

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "DressUpFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Dressing Room",
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
            name: "DressUpModelPreview",
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

fn item_slot_row(slots: &[DressUpSlot]) -> Element {
    slots
        .iter()
        .enumerate()
        .flat_map(|(i, slot)| {
            let col = i % 6;
            let row = i / 6;
            let x = SLOT_INSET + col as f32 * (SLOT_SIZE + SLOT_GAP);
            let y = -(SLOT_ROW_TOP + row as f32 * (SLOT_SIZE + SLOT_GAP));
            item_slot(i, slot, x, y)
        })
        .collect()
}

fn item_slot(idx: usize, slot: &DressUpSlot, x: f32, y: f32) -> Element {
    let slot_name = DynName(format!("DressUpSlot{idx}"));
    let label_name = DynName(format!("DressUpSlot{idx}Label"));
    rsx! {
        r#frame {
            name: slot_name,
            width: {SLOT_SIZE},
            height: {SLOT_SIZE},
            background_color: SLOT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            fontstring {
                name: label_name,
                width: {SLOT_SIZE + SLOT_GAP},
                height: 10.0,
                text: {slot.label.as_str()},
                font_size: 6.0,
                font_color: SLOT_LABEL_COLOR,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::Bottom,
                    relative_point: AnchorPoint::Bottom,
                    x: "0",
                    y: "10",
                }
            }
        }
    }
}

fn action_buttons() -> Element {
    let total_w = 3.0 * BUTTON_W + 2.0 * BUTTON_GAP;
    let x_start = (FRAME_W - total_w) / 2.0;
    let y = -(FRAME_H - BUTTON_H - BUTTON_ROW_BOTTOM);
    [("Reset", 0), ("Link", 1), ("Close", 2)]
        .iter()
        .flat_map(|(label, i)| {
            let btn_name = DynName(format!("DressUpButton{label}"));
            let txt_name = DynName(format!("DressUpButton{label}Text"));
            let bx = x_start + *i as f32 * (BUTTON_W + BUTTON_GAP);
            rsx! {
                r#frame {
                    name: btn_name,
                    width: {BUTTON_W},
                    height: {BUTTON_H},
                    background_color: BUTTON_BG,
                    anchor {
                        point: AnchorPoint::TopLeft,
                        relative_point: AnchorPoint::TopLeft,
                        x: {bx},
                        y: {y},
                    }
                    fontstring {
                        name: txt_name,
                        width: {BUTTON_W},
                        height: {BUTTON_H},
                        text: label,
                        font_size: 10.0,
                        font_color: BUTTON_TEXT_COLOR,
                        justify_h: "CENTER",
                        anchor {
                            point: AnchorPoint::TopLeft,
                            relative_point: AnchorPoint::TopLeft,
                        }
                    }
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_toolkit::layout::{LayoutRect, recompute_layouts};
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn make_test_state() -> DressUpFrameState {
        DressUpFrameState {
            visible: true,
            ..Default::default()
        }
    }

    fn build_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(dress_up_frame_screen).sync(&shared, &mut reg);
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
        assert!(reg.get_by_name("DressUpFrame").is_some());
        assert!(reg.get_by_name("DressUpFrameTitle").is_some());
    }

    #[test]
    fn builds_model_preview() {
        let reg = build_registry();
        assert!(reg.get_by_name("DressUpModelPreview").is_some());
    }

    #[test]
    fn builds_item_slots() {
        let reg = build_registry();
        for i in 0..ITEM_SLOTS.len() {
            assert!(
                reg.get_by_name(&format!("DressUpSlot{i}")).is_some(),
                "DressUpSlot{i} missing"
            );
            assert!(
                reg.get_by_name(&format!("DressUpSlot{i}Label")).is_some(),
                "DressUpSlot{i}Label missing"
            );
        }
    }

    #[test]
    fn builds_action_buttons() {
        let reg = build_registry();
        for label in ["Reset", "Link", "Close"] {
            assert!(
                reg.get_by_name(&format!("DressUpButton{label}")).is_some(),
                "DressUpButton{label} missing"
            );
        }
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(DressUpFrameState::default());
        Screen::new(dress_up_frame_screen).sync(&shared, &mut reg);
        let id = reg.get_by_name("DressUpFrame").expect("frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    // --- Coord validation ---

    const FRAME_X: f32 = 450.0;
    const FRAME_Y: f32 = 80.0;

    #[test]
    fn coord_main_frame() {
        let reg = layout_registry();
        let r = rect(&reg, "DressUpFrame");
        assert!((r.x - FRAME_X).abs() < 1.0);
        assert!((r.y - FRAME_Y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
        assert!((r.height - FRAME_H).abs() < 1.0);
    }

    #[test]
    fn coord_model_preview() {
        let reg = layout_registry();
        let r = rect(&reg, "DressUpModelPreview");
        assert!((r.x - (FRAME_X + PREVIEW_INSET)).abs() < 1.0);
        assert!((r.y - (FRAME_Y + HEADER_H)).abs() < 1.0);
        assert!((r.height - PREVIEW_H).abs() < 1.0);
    }

    #[test]
    fn coord_first_slot() {
        let reg = layout_registry();
        let r = rect(&reg, "DressUpSlot0");
        assert!((r.x - (FRAME_X + SLOT_INSET)).abs() < 1.0);
        assert!((r.y - (FRAME_Y + SLOT_ROW_TOP)).abs() < 1.0);
        assert!((r.width - SLOT_SIZE).abs() < 1.0);
    }
}
