use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

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
/// The dimmed icon of a slot whose item is on the cursor.
pub const LOCKED_ICON: &str = "0.5,0.5,0.5,1.0";

pub const ACTION_BAG_TOGGLE_PREFIX: &str = "bag_toggle:";
/// `bag_slot:<bag>:<slot>` on every bag slot.
pub const ACTION_BAG_SLOT_PREFIX: &str = "bag_slot:";

pub fn parse_bag_slot_action(action: &str) -> Option<(usize, usize)> {
    let (bag, slot) = action
        .strip_prefix(ACTION_BAG_SLOT_PREFIX)?
        .split_once(':')?;
    Some((bag.parse().ok()?, slot.parse().ok()?))
}

pub fn bag_toggle_action(index: usize) -> String {
    format!("{ACTION_BAG_TOGGLE_PREFIX}{index}")
}

#[derive(Clone, Debug, PartialEq)]
pub struct BagSlotState {
    pub icon_fdid: u32,
    pub count: u32,
    /// RGBA color string for quality border (empty string = no border).
    pub quality_border: String,
    /// On the cursor: `SetItemButtonDesaturated(locked)` greys the icon.
    pub locked: bool,
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
        let rows = slot_count.div_ceil(GRID_COLS);
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
    rsx! {
        r#frame {
            name: frame_name,
            width: {frame_w},
            height: {frame_h},
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: FRAME_BG,
            pos_type: "absolute",
            left: {x_offset},
            top: 100.0,
            {bag_title(title_name, &bag.title, frame_w)}
            {bag_slot_grid(bag.bag_index, &bag.slots)}
        }
    }
}

fn bag_title(id: DynName, text: &str, w: f32) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {w},
            height: {TITLE_H},
            text: text,
            font_size: 13.0,
            font_color: TITLE_COLOR,
            justify_h: "CENTER",
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            top: -0.0,
        }
    }
}

fn bag_slot_grid(bag_index: usize, slots: &[BagSlotState]) -> Element {
    slots
        .iter()
        .enumerate()
        .flat_map(|(i, slot)| {
            let col = i % GRID_COLS;
            let row = i / GRID_COLS;
            let x = INSET + col as f32 * (SLOT_SIZE + SLOT_GAP);
            let y = -(TITLE_H + INSET + row as f32 * (SLOT_SIZE + SLOT_GAP));
            bag_slot_frame(bag_index, i, slot, (x, y))
        })
        .collect()
}

fn bag_slot_frame(
    bag_index: usize,
    slot_index: usize,
    slot: &BagSlotState,
    (x, y): (f32, f32),
) -> Element {
    let prefix = format!("ContainerFrame{bag_index}Slot{slot_index}");
    let action = format!("{ACTION_BAG_SLOT_PREFIX}{bag_index}:{slot_index}");
    let contents = if slot.icon_fdid == 0 {
        Element::default()
    } else {
        slot_contents(&prefix, slot)
    };
    rsx! {
        r#frame {
            name: {DynName(prefix)},
            width: {SLOT_SIZE},
            height: {SLOT_SIZE},
            background_color: SLOT_BG,
            onclick: {action.as_str()},
            mouse_enabled: true,
            pos_type: "absolute",
            left: {x},
            top: {-(y)},
            {contents}
        }
    }
}

/// Item icon and, above 1, the stack count at the bottom right.
fn slot_contents(prefix: &str, slot: &BagSlotState) -> Element {
    let mut children = rsx! {
        texture {
            name: {DynName(format!("{prefix}Icon"))},
            width: {SLOT_SIZE},
            height: {SLOT_SIZE},
            texture_fdid: {slot.icon_fdid},
            vertex_color: {if slot.locked { LOCKED_ICON } else { "1.0,1.0,1.0,1.0" }},
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
        }
    };
    if slot.count > 1 {
        let count = slot.count.to_string();
        children.extend(rsx! {
            fontstring {
                name: {DynName(format!("{prefix}Count"))},
                width: {SLOT_SIZE - 3.0},
                height: 14.0,
                text: {count.as_str()},
                font_size: 12.0,
                font_color: "1.0,1.0,1.0,1.0",
                justify_h: "RIGHT",
                pos_type: "absolute",
                left: 0.0,
                top: {SLOT_SIZE - 15.0},
            }
        });
    }
    children
}
