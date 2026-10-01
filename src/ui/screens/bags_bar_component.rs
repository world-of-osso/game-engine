//! Original authored standalone bag HUD, independent of action bars and micro controls.
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::screens::bag_frame_component::bag_toggle_action;
use crate::ui::screens::bags_bar_art::{BACKPACK, BAG_SLOT_EMPTY, SheetCrop};

struct DynName(String);

// Retail MicroButtonAndBagsBar placement, retained for the standalone strip.
const MICRO_BAGS_INSET: f32 = 6.0;
const MICRO_BAGS_BAR_H: f32 = 80.0;
const BAGS_ABOVE_BAR: f32 = 10.0;
/// Retail BagsBar: 47 high, backpack 48x48, bag slots 30x30 chained with no padding
/// (Blizzard_MainMenuBarBagButtons/Mainline/MainMenuBarBagButtons.xml).
const BAGS_BAR_H: f32 = 47.0;
const BAGS_BAR_RIGHT: f32 = MICRO_BAGS_INSET;
const BAGS_BAR_BOTTOM: f32 = MICRO_BAGS_INSET + MICRO_BAGS_BAR_H + BAGS_ABOVE_BAR - BAGS_BAR_H;
pub(super) const BAG_SLOT_SIZE: f32 = 30.0;
pub(super) const BACKPACK_SIZE: f32 = 48.0;
pub(super) const BAG_SLOT_GAP: f32 = 0.0;
pub(super) const BAG_COUNT: usize = 4;
const MONEY_DISPLAY_W: f32 = 160.0;
const MONEY_DISPLAY_H: f32 = 14.0;
const MONEY_TEXT_COLOR: &str = "1.0,0.82,0.0,1.0";

/// Player money in total copper, matching the original money updater.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BagBarState {
    pub money: u64,
}

pub fn bags_bar_screen(ctx: &SharedContext) -> Element {
    bag_bar(ctx.get::<BagBarState>().map(|state| state.money))
}

// Same leading-denomination omission as auction_house_data::Money::display.
fn format_money(money: u64) -> String {
    let gold = money / 10_000;
    let silver = (money % 10_000) / 100;
    let copper = money % 100;
    if gold > 0 {
        format!("{gold}g {silver}s {copper}c")
    } else if silver > 0 {
        format!("{silver}s {copper}c")
    } else {
        format!("{copper}c")
    }
}

/// Bags chained leftward from the backpack, vertically centred on it, as in Retail. Money
/// (not part of the Retail bar) sits left of the bags.
fn bag_bar(money: Option<u64>) -> Element {
    let bags_w = BAG_COUNT as f32 * (BAG_SLOT_SIZE + BAG_SLOT_GAP);
    let total_w = MONEY_DISPLAY_W + bags_w + BACKPACK_SIZE;
    let backpack_x = total_w - BACKPACK_SIZE;
    let centre_y = |size: f32| (BAGS_BAR_H - size) / 2.0;
    let bags: Element = (0..BAG_COUNT)
        .flat_map(|i| {
            // CharacterBag0Slot sits next to the backpack.
            let x = backpack_x - (i + 1) as f32 * (BAG_SLOT_SIZE + BAG_SLOT_GAP);
            let slot = BagSlot {
                name: format!("CharacterBag{i}Slot"),
                index: i + 1,
                size: BAG_SLOT_SIZE,
                art: BAG_SLOT_EMPTY,
            };
            bag_slot(slot, x, centre_y(BAG_SLOT_SIZE))
        })
        .collect();
    let backpack = BagSlot {
        name: "MainMenuBarBackpackButton".to_string(),
        index: 0,
        size: BACKPACK_SIZE,
        art: BACKPACK,
    };
    rsx! {
        r#frame {
            name: "BagsBar",
            width: {total_w},
            height: {BAGS_BAR_H},
            pos_type: "absolute",
            right: {BAGS_BAR_RIGHT},
            bottom: {BAGS_BAR_BOTTOM},
            {bag_slot(backpack, backpack_x, centre_y(BACKPACK_SIZE))}
            {bags}
            {money_display(centre_y(MONEY_DISPLAY_H), money)}
        }
    }
}

fn money_display(y: f32, money: Option<u64>) -> Element {
    let text = money.map(format_money).unwrap_or_else(|| "0g 0s 0c".into());
    rsx! {
        fontstring {
            name: "BagsBarMoneyDisplay",
            width: {MONEY_DISPLAY_W - 6.0},
            height: {MONEY_DISPLAY_H},
            text: text.as_str(),
            font: "ArialNarrow",
            font_size: 11.0,
            font_color: MONEY_TEXT_COLOR,
            justify_h: "RIGHT",
            pos_type: "absolute",
            left: 0.0,
            pos_y: y,
        }
    }
}

struct BagSlot {
    name: String,
    index: usize,
    size: f32,
    art: SheetCrop,
}

fn bag_slot(slot: BagSlot, x: f32, y: f32) -> Element {
    let action = bag_toggle_action(slot.index);
    let art_name = DynName(format!("{}Art", slot.name));
    let coords = slot.art.tex_coords();
    rsx! {
        button {
            name: DynName(slot.name),
            width: {slot.size},
            height: {slot.size},
            text: "",
            font_size: 8.0,
            onclick: {action.as_str()},
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
            texture {
                name: art_name,
                width: {slot.size},
                height: {slot.size},
                texture_fdid: {slot.art.fdid},
                tex_coords: {coords.as_str()},
                pos_type: "absolute",
                left: 0.0,
                top: 0.0,
            }
        }
    }
}
