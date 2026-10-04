//! Retail `ContainerFrameTemplate` (AddOns/Blizzard_UIPanels_Game/Mainline/ContainerFrame.xml:218-265):
//! a `PortraitFrameFlatTemplate` window with the bag name as title, a close button and one
//! `ContainerFrameItemButtonTemplate` (:77-159) per slot over its `bags-item-slot64`
//! empty background. Forever's Mainline ContainerFrame.xml uses the same templates; its
//! art comes from the active skin's atlas set.

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::screens::quest_art::{DynName, named_atlas_texture, portrait_flat_chrome};
use crate::ui::strata::FrameStrata;

/// ContainerFrame.lua:9 `CONTAINER_WIDTH`.
const CONTAINER_WIDTH: f32 = 178.0;
/// `ContainerFrameItemButtonTemplate` size (ContainerFrame.xml:78).
const SLOT_SIZE: f32 = 37.0;
/// ContainerFrame.lua:11-12 `ITEM_SPACING_X` / `ITEM_SPACING_Y`.
const SLOT_GAP: f32 = 5.0;
/// ContainerFrame.lua:783-789 `GetColumns` for a single bag.
const GRID_COLS: usize = 4;
/// ContainerFrame.lua:779-781 `GetFirstButtonOffsetY`, and the slot grid's right inset
/// (`GetInitialItemAnchor`, :872-874).
const FIRST_BUTTON_OFFSET_Y: f32 = 9.0;
const GRID_RIGHT: f32 = 7.0;
/// ContainerFrame.lua:853-856 `GetPaddingHeight`: the title bar and attic above the grid.
const ATTIC: f32 = 48.0;
/// ContainerFrame.lua:683 `SetTitleOffsets(35)`.
const TITLE_LEFT: f32 = 35.0;
/// ContainerFrame.xml:80 `emptyBackgroundAtlas`.
const SLOT_BACKGROUND: &str = "bags-item-slot64";
/// The dimmed icon of a slot whose item is on the cursor.
pub const LOCKED_ICON: &str = "0.5,0.5,0.5,1.0";

/// `bag_close:<bag>` on a container's close button (`CloseBag`, ContainerFrame.lua:739-741).
pub const ACTION_BAG_CLOSE_PREFIX: &str = "bag_close:";
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
    /// `ContainerFrameMixin:CalculateWidth`/`CalculateHeight` (ContainerFrame.lua:838-847).
    /// The backpack's search box and money frame rows (:2517-2523) are not drawn, so it
    /// takes the plain bag size.
    pub fn bag_dimensions(slot_count: usize) -> (f32, f32) {
        let rows = slot_count.div_ceil(GRID_COLS);
        let grid = rows as f32 * SLOT_SIZE + rows.saturating_sub(1) as f32 * SLOT_GAP;
        (CONTAINER_WIDTH, grid + FIRST_BUTTON_OFFSET_Y + ATTIC)
    }
}

pub fn bag_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<BagFrameState>()
        .expect("BagFrameState must be in SharedContext");
    state.bags.iter().flat_map(bag_container).collect()
}

pub fn parse_bag_close_action(action: &str) -> Option<usize> {
    action.strip_prefix(ACTION_BAG_CLOSE_PREFIX)?.parse().ok()
}

fn bag_container(bag: &BagContainerState) -> Element {
    let size = BagFrameState::bag_dimensions(bag.slots.len());
    let hide = !bag.visible;
    let prefix = format!("ContainerFrame{}", bag.bag_index);
    let close = format!("{ACTION_BAG_CLOSE_PREFIX}{}", bag.bag_index);
    let mut children = portrait_flat_chrome(&prefix, size, (&bag.title, TITLE_LEFT), &close);
    children.extend(bag_slot_grid(bag.bag_index, &bag.slots, size));
    let x_offset = 300.0 + bag.bag_index as f32 * 20.0;
    rsx! {
        r#frame {
            name: {DynName(prefix)},
            width: {size.0},
            height: {size.1},
            strata: FrameStrata::Dialog,
            hidden: hide,
            pos_type: "absolute",
            left: {x_offset},
            top: 100.0,
            {children}
        }
    }
}

/// `AnchorUtil.GridLayout` `BottomRightToTopLeft` from BOTTOMRIGHT (-7, 9) over the items
/// in reverse slot order (ContainerFrame.lua:868-874, 918-949): the last slot sits
/// bottom-right and slot 1 top-left, so a partial row leaves its gap top-left.
fn slot_position(index: usize, slot_count: usize, (width, height): (f32, f32)) -> (f32, f32) {
    let from_end = slot_count - 1 - index;
    let col = (from_end % GRID_COLS) as f32;
    let row = (from_end / GRID_COLS) as f32;
    let step = SLOT_SIZE + SLOT_GAP;
    (
        width - GRID_RIGHT - SLOT_SIZE - col * step,
        height - FIRST_BUTTON_OFFSET_Y - SLOT_SIZE - row * step,
    )
}

fn bag_slot_grid(bag_index: usize, slots: &[BagSlotState], size: (f32, f32)) -> Element {
    slots
        .iter()
        .enumerate()
        .flat_map(|(i, slot)| {
            bag_slot_frame(bag_index, i, slot, slot_position(i, slots.len(), size))
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
    let mut children = named_atlas_texture(
        format!("{prefix}Background"),
        SLOT_BACKGROUND,
        (0.0, 0.0, SLOT_SIZE, SLOT_SIZE),
    );
    if slot.icon_fdid != 0 {
        children.extend(slot_contents(&prefix, slot));
    }
    rsx! {
        r#frame {
            name: {DynName(prefix)},
            width: {SLOT_SIZE},
            height: {SLOT_SIZE},
            onclick: {action.as_str()},
            mouse_enabled: true,
            pos_type: "absolute",
            left: {x},
            top: {y},
            {children}
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
