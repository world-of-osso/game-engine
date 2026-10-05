//! Retail `ContainerFrameTemplate` (AddOns/Blizzard_UIPanels_Game/Mainline/ContainerFrame.xml:218-265):
//! a `PortraitFrameFlatTemplate` window with the bag name as title, a close button and one
//! `ContainerFrameItemButtonTemplate` (:77-159) per slot over its `bags-item-slot64`
//! empty background. Forever's Mainline ContainerFrame.xml uses the same templates; its
//! art comes from the active skin's atlas set.

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::screens::merchant_frame_component::{MoneyAlign, money};
use crate::ui::screens::quest_art::{
    DynName, HIGHLIGHT_FONT_COLOR, named_atlas_texture, portrait_flat_chrome,
};
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
/// `Interface/Icons/Inv_misc_bag_08` (ContainerFrame.lua:825).
pub const BACKPACK_PORTRAIT: u32 = 133_633;
/// `SetPortraitTextureSizeAndOffset(36, -4, 1)` (ContainerFrame.lua:682): 36×36 at
/// TOPLEFT (-4, 1); the host rounds it with `TempPortraitAlphaMask`
/// (SharedUIPanelTemplates.xml:564-572 `CircleMask`).
const PORTRAIT: (f32, f32, f32) = (-4.0, -1.0, 36.0);
/// The dimmed icon of a slot whose item is on the cursor.
pub const LOCKED_ICON: &str = "0.5,0.5,0.5,1.0";
/// `ContainerFrameBackpackMixin:GetPaddingHeight` adds 30 for the search box
/// (ContainerFrame.lua:2517-2519).
const SEARCH_ATTIC: f32 = 30.0;
/// `BagItemSearchBox` 96×18 (ContainerFrame.xml:308-310) at TOPLEFT (42, -37)
/// (`SetSearchBoxPoint`, ContainerFrame.lua:964-967; Forever's ContainerFrame.lua:1109-1111).
pub const SEARCH_BOX: &str = "BagItemSearchBox";
pub const ACTION_SEARCH_CLEAR: &str = "bag_search_clear";
const SEARCH_RECT: (f32, f32, f32, f32) = (42.0, 37.0, 96.0, 18.0);
/// `GameFontDisable`, the `SearchBoxTemplate` instructions (InputBoxTemplates.xml:206-245).
const DISABLED_FONT_COLOR: &str = "0.5,0.5,0.5,1.0";
/// `ItemContextOverlay:SetColorTexture(0, 0, 0, 0.8)` on a slot that fails the search
/// (`GetItemContextOverlayMode` Standard, ItemButtonTemplate.lua:73-79, 101-104).
const SEARCH_OVERLAY: &str = "0.0,0.0,0.0,0.8";
/// `SmallMoneyFrameTemplate` height (MoneyFrame.xml:82; `UpdateMoneyFrame`,
/// ContainerFrame.lua:743-751), added below the grid by `CalculateExtraHeight` (:2521-2523).
const MONEY_H: f32 = 13.0;
/// `MoneyFrame` BOTTOMLEFT (8, 8) / BOTTOMRIGHT (-8, 8) without a token tracker
/// (ContainerFrame.lua:2494-2497).
const MONEY_INSET: f32 = 8.0;
/// `CopperButton` RIGHT (-13, 0) inside the money frame (MoneyFrame.xml:105-109).
const MONEY_RIGHT: f32 = 13.0;
/// `ContainerFrameCurrencyBorderTemplate` 17 high with 8 wide caps (ContainerFrame.xml:161-188).
const COINBOX_H: f32 = 17.0;
const COINBOX_CAP: f32 = 8.0;
/// The backpack's grid hangs from the money frame's TOPRIGHT (0, 4)
/// (`ContainerFrameBackpackMixin:GetInitialItemAnchor`, ContainerFrame.lua:2513-2515).
const BACKPACK_GRID_GAP: f32 = 4.0;

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
    /// The item's name, matched by the bag search.
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BagContainerState {
    pub bag_index: usize,
    pub title: String,
    /// `UpdateMiscellaneousFrames` (ContainerFrame.lua:823-829): the backpack's
    /// [`BACKPACK_PORTRAIT`], another bag's item icon (`SetPortraitToBag`).
    pub portrait_fdid: u32,
    pub slots: Vec<BagSlotState>,
    pub visible: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct BagFrameState {
    pub bags: Vec<BagContainerState>,
    /// `C_Container.SetItemSearch` text, shared by every `ITEM_SEARCHBAR_LIST` box
    /// (UIPanelTemplatesShared.lua:1-31).
    pub search: String,
    /// The player's copper, shown by the backpack's `MoneyFrame` (`MoneyFrame_SetType(self,
    /// "PLAYER")`, ContainerFrame.xml:190-213).
    pub money: u64,
}

impl BagFrameState {
    /// `ContainerFrameMixin:CalculateWidth`/`CalculateHeight` (ContainerFrame.lua:838-847);
    /// the backpack adds its search box and money rows (:2517-2523).
    pub fn bag_dimensions(bag_index: usize, slot_count: usize) -> (f32, f32) {
        let rows = slot_count.div_ceil(GRID_COLS);
        let grid = rows as f32 * SLOT_SIZE + rows.saturating_sub(1) as f32 * SLOT_GAP;
        let backpack = if bag_index == 0 {
            SEARCH_ATTIC + MONEY_H
        } else {
            0.0
        };
        (
            CONTAINER_WIDTH,
            grid + FIRST_BUTTON_OFFSET_Y + ATTIC + backpack,
        )
    }
}

/// Whether an item survives the bag search: an empty search keeps everything, otherwise
/// the item's name must contain it, ignoring case (`isFiltered`, ContainerFrame.lua:1073).
pub fn matches_search(name: &str, search: &str) -> bool {
    let search = search.trim();
    search.is_empty() || name.to_lowercase().contains(&search.to_lowercase())
}

pub fn bag_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<BagFrameState>()
        .expect("BagFrameState must be in SharedContext");
    state
        .bags
        .iter()
        .flat_map(|bag| bag_container(bag, state))
        .collect()
}

pub fn parse_bag_close_action(action: &str) -> Option<usize> {
    action.strip_prefix(ACTION_BAG_CLOSE_PREFIX)?.parse().ok()
}

fn bag_container(bag: &BagContainerState, state: &BagFrameState) -> Element {
    let size = BagFrameState::bag_dimensions(bag.bag_index, bag.slots.len());
    let hide = !bag.visible;
    let prefix = format!("ContainerFrame{}", bag.bag_index);
    let close = format!("{ACTION_BAG_CLOSE_PREFIX}{}", bag.bag_index);
    let mut children = portrait_flat_chrome(&prefix, size, (&bag.title, TITLE_LEFT), &close);
    children.extend(portrait(&prefix, bag.portrait_fdid));
    let backpack = bag.bag_index == 0;
    if backpack {
        children.extend(search_box(&state.search));
        children.extend(money_frame(&prefix, state.money, size));
    }
    // `UpdateSearchResults` / `SetMatchesSearch(not isFiltered)` (ContainerFrame.lua:527-533).
    let filtered: Vec<bool> = bag
        .slots
        .iter()
        .map(|slot| slot.icon_fdid != 0 && !matches_search(&slot.name, &state.search))
        .collect();
    let grid_bottom = if backpack {
        MONEY_INSET + MONEY_H + BACKPACK_GRID_GAP
    } else {
        FIRST_BUTTON_OFFSET_Y
    };
    let grid_right = if backpack { MONEY_INSET } else { GRID_RIGHT };
    children.extend(bag_slot_grid(
        bag.bag_index,
        &bag.slots,
        &filtered,
        (size, grid_right, grid_bottom),
    ));
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

/// `<prefix>Portrait`: the bag's round portrait over the metal ring.
fn portrait(prefix: &str, fdid: u32) -> Element {
    let (x, y, size) = PORTRAIT;
    rsx! {
        texture {
            name: {DynName(format!("{prefix}Portrait"))},
            width: size,
            height: size,
            texture_fdid: fdid,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

/// `BagItemSearchBox` (`BagSearchBoxTemplate` < `SearchBoxTemplate`,
/// UIPanelTemplates.xml:4-10, InputBoxTemplates.xml:206-245): the `InputBoxVisualTemplate`
/// border (left cap 8×20 at LEFT -5, right cap 8×20 at RIGHT, InputBoxTemplates.xml:43-66),
/// the 10×10 magnifying glass at LEFT (1, -1), text insets 16/20 and the "Search"
/// instructions while empty. The registry keeps what the player types; the driver feeds it
/// back as [`BagFrameState::search`].
fn search_box(search: &str) -> Element {
    let (x, y, w, h) = SEARCH_RECT;
    let border_y = y + (h - 20.0) / 2.0;
    let mut children = named_atlas_texture(
        format!("{SEARCH_BOX}Left"),
        "common-search-border-left",
        (x - 5.0, border_y, 8.0, 20.0),
    );
    children.extend(named_atlas_texture(
        format!("{SEARCH_BOX}Middle"),
        "common-search-border-middle",
        (x + 3.0, border_y, w - 11.0, 20.0),
    ));
    children.extend(named_atlas_texture(
        format!("{SEARCH_BOX}Right"),
        "common-search-border-right",
        (x + w - 8.0, border_y, 8.0, 20.0),
    ));
    children.extend(named_atlas_texture(
        format!("{SEARCH_BOX}SearchIcon"),
        "common-search-magnifyingglass",
        (x + 1.0, y + (h - 10.0) / 2.0 + 1.0, 10.0, 10.0),
    ));
    children.extend(rsx! {
        editbox {
            name: {DynName(SEARCH_BOX.to_string())},
            width: w,
            height: h,
            text: search,
            font: ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
            font_size: 12.0,
            font_color: HIGHLIGHT_FONT_COLOR,
            text_insets: "16,20,0,0",
            pos_type: "absolute",
            left: x,
            top: y,
        }
    });
    // SearchBoxTemplate XML:221-244 and Lua:192-210: 17x17 clear hit target,
    // 10x10 normal art at half alpha. This client's contract is text-only visibility.
    if !search.is_empty() {
        children.extend(rsx! {
            button {
                name: DynName(format!("{SEARCH_BOX}ClearButton")),
                width: 17,
                height: 17,
                pos_type: "absolute",
                left: {x + w - 20.0},
                top: {y + (h - 17.0) / 2.0},
                button_default_skin: false,
                onclick: ACTION_SEARCH_CLEAR,
                texture {
                    width: 10,
                    height: 10,
                    texture_atlas: "common-search-clearbutton",
                    alpha: {0.5},
                    pos_type: "absolute",
                    left: 3,
                    top: 3,
                }
            }
        });
    }
    if search.is_empty() {
        children.extend(rsx! {
            fontstring {
                name: {DynName(format!("{SEARCH_BOX}Instructions"))},
                width: {w - 36.0},
                height: h,
                text: "Search",
                font: ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
                font_size: 12.0,
                font_color: DISABLED_FONT_COLOR,
                justify_h: "LEFT",
                pos_type: "absolute",
                left: {x + 16.0},
                top: y,
            }
        });
    }
    children
}

/// `<prefix>MoneyFrame` (`ContainerMoneyFrameTemplate`, ContainerFrame.xml:190-213, 275):
/// the coin box border across the frame and the player's gold, silver and copper
/// right-aligned 13 in from its right edge.
fn money_frame(prefix: &str, copper: u64, (width, height): (f32, f32)) -> Element {
    let name = format!("{prefix}MoneyFrame");
    let (left, right) = (MONEY_INSET, width - MONEY_INSET);
    let top = height - MONEY_INSET - MONEY_H;
    let border_y = top + (MONEY_H - COINBOX_H) / 2.0;
    let mut children = named_atlas_texture(
        format!("{name}BorderLeft"),
        "common-coinbox-left",
        (left, border_y, COINBOX_CAP, COINBOX_H),
    );
    children.extend(named_atlas_texture(
        format!("{name}BorderMiddle"),
        "_common-coinbox-center",
        (
            left + COINBOX_CAP,
            border_y,
            right - left - 2.0 * COINBOX_CAP,
            COINBOX_H,
        ),
    ));
    children.extend(named_atlas_texture(
        format!("{name}BorderRight"),
        "common-coinbox-right",
        (right - COINBOX_CAP, border_y, COINBOX_CAP, COINBOX_H),
    ));
    // `money` draws a 14 high row from its bottom edge; centre it on the 13 high frame.
    children.extend(money(
        &name,
        copper,
        (right - MONEY_RIGHT, top + MONEY_H + 0.5),
        MoneyAlign::Right,
        false,
    ));
    children
}

/// `AnchorUtil.GridLayout` `BottomRightToTopLeft` over the items in reverse slot order
/// (ContainerFrame.lua:868-874, 918-949) from the initial anchor `grid_right`/`grid_bottom`
/// in from the window's BOTTOMRIGHT: (-7, 9) for a bag, the money frame's TOPRIGHT for the
/// backpack. The last slot sits bottom-right and slot 1 top-left, so a partial row leaves
/// its gap top-left.
fn slot_position(
    index: usize,
    slot_count: usize,
    ((width, height), grid_right, grid_bottom): ((f32, f32), f32, f32),
) -> (f32, f32) {
    let from_end = slot_count - 1 - index;
    let col = (from_end % GRID_COLS) as f32;
    let row = (from_end / GRID_COLS) as f32;
    let step = SLOT_SIZE + SLOT_GAP;
    (
        width - grid_right - SLOT_SIZE - col * step,
        height - grid_bottom - SLOT_SIZE - row * step,
    )
}

fn bag_slot_grid(
    bag_index: usize,
    slots: &[BagSlotState],
    filtered: &[bool],
    anchor: ((f32, f32), f32, f32),
) -> Element {
    slots
        .iter()
        .zip(filtered)
        .enumerate()
        .flat_map(|(i, (slot, filtered))| {
            let position = slot_position(i, slots.len(), anchor);
            bag_slot_frame((bag_index, i), slot, *filtered, position)
        })
        .collect()
}

fn bag_slot_frame(
    (bag_index, slot_index): (usize, usize),
    slot: &BagSlotState,
    filtered: bool,
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
    if filtered {
        children.extend(rsx! {
            r#frame {
                name: {DynName(format!("{prefix}SearchOverlay"))},
                width: {SLOT_SIZE},
                height: {SLOT_SIZE},
                background_color: SEARCH_OVERLAY,
                pos_type: "absolute",
                left: 0.0,
                top: 0.0,
            }
        });
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
