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

const TITLE_H: f32 = 24.0;
const SLOT_SIZE: f32 = 36.0;
const SLOT_GAP: f32 = 4.0;
const GRID_COLS: usize = 4;
const INSET: f32 = 8.0;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const SLOT_BG: &str = "0.08,0.07,0.06,0.88";

#[derive(Clone, Debug, PartialEq)]
pub struct BagSlotState {
    pub icon_fdid: u32,
    pub count: u32,
    /// RGBA color string for quality border (empty string = no border).
    pub quality_border: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BagContainerState {
    pub bag_index: usize,
    pub title: String,
    pub slots: Vec<BagSlotState>,
    pub visible: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BagFrameState {
    pub bags: Vec<BagContainerState>,
}

impl BagFrameState {
    /// Compute frame dimensions for a bag based on slot count.
    pub fn bag_dimensions(slot_count: usize) -> (f32, f32) {
        let rows = (slot_count + GRID_COLS - 1) / GRID_COLS;
        let w = 2.0 * INSET + GRID_COLS as f32 * SLOT_SIZE + (GRID_COLS - 1) as f32 * SLOT_GAP;
        let h = TITLE_H
            + INSET
            + rows as f32 * SLOT_SIZE
            + (rows.saturating_sub(1)) as f32 * SLOT_GAP
            + INSET;
        (w, h)
    }
}

pub fn bag_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<BagFrameState>()
        .expect("BagFrameState must be in SharedContext");
    state.bags.iter().flat_map(bag_container).collect()
}

fn bag_container(bag: &BagContainerState) -> Element {
    let slot_count = bag.slots.len();
    let (frame_w, frame_h) = BagFrameState::bag_dimensions(slot_count);
    let hide = !bag.visible;
    let frame_name = DynName(format!("ContainerFrame{}", bag.bag_index));
    let title_name = DynName(format!("ContainerFrame{}Title", bag.bag_index));
    let x_offset = 300.0 + bag.bag_index as f32 * 20.0;
    let slots = bag_slot_grid(bag.bag_index, &bag.slots);
    rsx! {
        r#frame {
            name: frame_name,
            width: {frame_w},
            height: {frame_h},
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x_offset},
                y: "-100",
            }
            fontstring {
                name: title_name,
                width: {frame_w},
                height: {TITLE_H},
                text: {bag.title.as_str()},
                font_size: 13.0,
                font_color: TITLE_COLOR,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::Top,
                    relative_point: AnchorPoint::Top,
                    x: "0",
                    y: "0",
                }
            }
            {slots}
        }
    }
}

fn bag_slot_grid(bag_index: usize, slots: &[BagSlotState]) -> Element {
    slots
        .iter()
        .enumerate()
        .flat_map(|(i, _slot)| {
            let col = i % GRID_COLS;
            let row = i / GRID_COLS;
            let x = INSET + col as f32 * (SLOT_SIZE + SLOT_GAP);
            let y = -(TITLE_H + INSET + row as f32 * (SLOT_SIZE + SLOT_GAP));
            bag_slot_frame(bag_index, i, x, y)
        })
        .collect()
}

fn bag_slot_frame(bag_index: usize, slot_index: usize, x: f32, y: f32) -> Element {
    let slot_name = DynName(format!("ContainerFrame{}Slot{slot_index}", bag_index));
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_toolkit::layout::{LayoutRect, recompute_layouts};
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn make_bag(index: usize, slot_count: usize) -> BagContainerState {
        BagContainerState {
            bag_index: index,
            title: format!("Bag {index}"),
            slots: (0..slot_count)
                .map(|_| BagSlotState {
                    icon_fdid: 0,
                    count: 0,
                    quality_border: String::new(),
                })
                .collect(),
            visible: true,
        }
    }

    fn build_registry(bags: Vec<BagContainerState>) -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(BagFrameState { bags });
        Screen::new(bag_frame_screen).sync(&shared, &mut reg);
        reg
    }

    fn layout_registry(bags: Vec<BagContainerState>) -> FrameRegistry {
        let mut reg = build_registry(bags);
        recompute_layouts(&mut reg);
        reg
    }

    fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
        reg.get(reg.get_by_name(name).expect(name))
            .and_then(|f| f.layout_rect.clone())
            .unwrap_or_else(|| panic!("{name} has no layout_rect"))
    }

    #[test]
    fn builds_container_frame_and_title() {
        let reg = build_registry(vec![make_bag(0, 16)]);
        assert!(reg.get_by_name("ContainerFrame0").is_some());
        assert!(reg.get_by_name("ContainerFrame0Title").is_some());
    }

    #[test]
    fn builds_correct_number_of_slots() {
        let reg = build_registry(vec![make_bag(0, 16)]);
        for i in 0..16 {
            assert!(
                reg.get_by_name(&format!("ContainerFrame0Slot{i}"))
                    .is_some(),
                "ContainerFrame0Slot{i} missing"
            );
        }
        assert!(reg.get_by_name("ContainerFrame0Slot16").is_none());
    }

    #[test]
    fn builds_multiple_bags() {
        let reg = build_registry(vec![make_bag(0, 8), make_bag(1, 12)]);
        assert!(reg.get_by_name("ContainerFrame0").is_some());
        assert!(reg.get_by_name("ContainerFrame1").is_some());
        assert!(reg.get_by_name("ContainerFrame0Slot7").is_some());
        assert!(reg.get_by_name("ContainerFrame1Slot11").is_some());
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut bag = make_bag(0, 4);
        bag.visible = false;
        let reg = build_registry(vec![bag]);
        let id = reg.get_by_name("ContainerFrame0").expect("frame");
        let frame = reg.get(id).expect("data");
        assert!(frame.hidden);
    }

    #[test]
    fn variable_bag_sizes_produce_different_heights() {
        let (_, h8) = BagFrameState::bag_dimensions(8);
        let (_, h16) = BagFrameState::bag_dimensions(16);
        assert!(h16 > h8, "16-slot bag should be taller than 8-slot");
    }

    #[test]
    fn dimensions_match_grid_layout() {
        let (w, h) = BagFrameState::bag_dimensions(16);
        let expected_w = 2.0 * INSET + 4.0 * SLOT_SIZE + 3.0 * SLOT_GAP;
        let expected_h = TITLE_H + INSET + 4.0 * SLOT_SIZE + 3.0 * SLOT_GAP + INSET;
        assert!((w - expected_w).abs() < 0.1);
        assert!((h - expected_h).abs() < 0.1);
    }

    // --- Coord validation ---

    #[test]
    fn coord_container_frame() {
        let reg = layout_registry(vec![make_bag(0, 16)]);
        let (w, h) = BagFrameState::bag_dimensions(16);
        let r = rect(&reg, "ContainerFrame0");
        assert!((r.x - 300.0).abs() < 1.0);
        assert!((r.y - 100.0).abs() < 1.0);
        assert!((r.width - w).abs() < 1.0);
        assert!((r.height - h).abs() < 1.0);
    }

    #[test]
    fn coord_first_slot() {
        let reg = layout_registry(vec![make_bag(0, 16)]);
        let r = rect(&reg, "ContainerFrame0Slot0");
        let frame_x = 300.0;
        let frame_y = 100.0;
        assert!((r.x - (frame_x + INSET)).abs() < 1.0);
        assert!((r.y - (frame_y + TITLE_H + INSET)).abs() < 1.0);
        assert!((r.width - SLOT_SIZE).abs() < 1.0);
        assert!((r.height - SLOT_SIZE).abs() < 1.0);
    }

    #[test]
    fn coord_second_row_slot() {
        let reg = layout_registry(vec![make_bag(0, 16)]);
        let r = rect(&reg, "ContainerFrame0Slot4");
        let frame_x = 300.0;
        let frame_y = 100.0;
        let expected_y = frame_y + TITLE_H + INSET + SLOT_SIZE + SLOT_GAP;
        assert!((r.x - (frame_x + INSET)).abs() < 1.0);
        assert!((r.y - expected_y).abs() < 1.0);
    }

    #[test]
    fn coord_last_column_slot() {
        let reg = layout_registry(vec![make_bag(0, 16)]);
        let r = rect(&reg, "ContainerFrame0Slot3");
        let frame_x = 300.0;
        let frame_y = 100.0;
        let expected_x = frame_x + INSET + 3.0 * (SLOT_SIZE + SLOT_GAP);
        let expected_y = frame_y + TITLE_H + INSET;
        assert!(
            (r.x - expected_x).abs() < 1.0,
            "x: expected {expected_x}, got {}",
            r.x
        );
        assert!((r.y - expected_y).abs() < 1.0);
    }

    #[test]
    fn coord_second_bag_offset() {
        let reg = layout_registry(vec![make_bag(0, 8), make_bag(1, 12)]);
        let r0 = rect(&reg, "ContainerFrame0");
        let r1 = rect(&reg, "ContainerFrame1");
        // Bag 1 is offset 20px further right than bag 0
        assert!((r1.x - r0.x - 20.0).abs() < 1.0);
        // Both at same y
        assert!((r0.y - r1.y).abs() < 1.0);
    }

    #[test]
    fn coord_variable_bag_height() {
        let reg = layout_registry(vec![make_bag(0, 8), make_bag(1, 16)]);
        let r8 = rect(&reg, "ContainerFrame0");
        let r16 = rect(&reg, "ContainerFrame1");
        let (_, h8) = BagFrameState::bag_dimensions(8);
        let (_, h16) = BagFrameState::bag_dimensions(16);
        assert!((r8.height - h8).abs() < 1.0);
        assert!((r16.height - h16).abs() < 1.0);
    }
}
