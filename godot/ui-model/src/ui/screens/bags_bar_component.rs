//! Original authored standalone bag HUD, independent of action bars and micro controls.
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::hud_layout::{HudLayout, hud_layout};
use crate::ui::screens::bag_frame_component::bag_toggle_action;

struct DynName(String);

/// Retail bag bar art (UiTextureAtlas 2098, FDID 4691255; no Forever set-1 members).
/// `bag-main` (member 16752): the backpack button.
const BACKPACK: &str = "bag-main";
/// `bag-border-empty` (16751): an empty bag slot.
const BAG_SLOT_EMPTY: &str = "bag-border-empty";
/// `bag-border` (16750): a bag slot holding a bag.
const BAG_SLOT: &str = "bag-border";
/// `bag-reagent-border` (16753): the reagent bag slot holding a bag.
const REAGENT_SLOT: &str = "bag-reagent-border";
/// `bag-reagent-border-empty` (16754): the empty reagent bag slot.
const REAGENT_SLOT_EMPTY: &str = "bag-reagent-border-empty";
/// `bag-arrow` (16749): `BagBarExpandToggle`, pointing left unrotated.
const BAG_ARROW: &str = "bag-arrow";

/// Retail BagsBar: 47 high, backpack 48x48, bag slots 30x30 chained with no padding
/// (Blizzard_MainMenuBarBagButtons/Mainline/MainMenuBarBagButtons.xml).
pub const BAGS_BAR_H: f32 = 47.0;
pub(super) const BAG_SLOT_SIZE: f32 = 30.0;
pub(super) const BACKPACK_SIZE: f32 = 48.0;
pub(super) const BAG_SLOT_GAP: f32 = 0.0;
pub(super) const BAG_COUNT: usize = 4;
/// `BagBarExpandToggle`: 10x16 between the backpack and `CharacterBag0Slot`.
const EXPAND_TOGGLE_W: f32 = 10.0;
const EXPAND_TOGGLE_H: f32 = 16.0;
/// Reagent bag container (`Enum.BagIndex.ReagentBag`).
const REAGENT_BAG: usize = 5;
/// `CircularItemButtonTemplate` `CircleMask`: TOPLEFT 2,-2 and BOTTOMRIGHT -4,4 of the
/// 30x30 slot clip its full-size `icon` (ItemButtonTemplate.xml:5-21).
const ICON_INSET: f32 = 2.0;
const ICON_SIZE: f32 = BAG_SLOT_SIZE - 6.0;
pub const ACTION_BAG_BAR_EXPAND_TOGGLE: &str = "bag_bar_expand_toggle";
const MONEY_DISPLAY_W: f32 = 160.0;
const MONEY_DISPLAY_H: f32 = 14.0;
const MONEY_TEXT_COLOR: &str = "1.0,0.82,0.0,1.0";
/// Backpack `Count`: `NumberFontNormal` (ARIALN 14 outline, white), CENTER 0,-10
/// (ItemButtonTemplate.xml:59, MainMenuBarBagButtons.lua:239-240).
const COUNT_FONT_SIZE: f32 = 14.0;
const COUNT_H: f32 = 14.0;
const COUNT_BELOW_CENTRE: f32 = 10.0;
const WHITE: &str = "1.0,1.0,1.0,1.0";

/// Player money in total copper, matching the original money updater, the free slots
/// over every bag (`C_Container.CalculateTotalNumberOfFreeBagSlots`), the icon of the bag
/// equipped in each bag slot (bags 1-4, then the reagent bag) and whether
/// `BagBarExpandToggle` collapsed the four bag slots.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BagBarState {
    pub money: u64,
    pub free_slots: usize,
    pub bag_icons: [Option<u32>; 5],
    pub collapsed: bool,
}

/// Bag button atlases of Camelot's own 45×45 square bag bar, which only Forever's set 1
/// has: `BaseBagSlotButtonMixin:GetSlotAtlases`, the keyring's slot art
/// (`KeyRingMixin:OnBagUpdate`) and `KeyRingMixin:GetSlotAtlases`
/// (Camelot/MainMenuBarBagButtons.lua:2, 143-145, 211). This bar keeps Retail's round bag
/// buttons, so it draws none of them.
pub const FOREVER_ONLY_BAG_ATLASES: [&str; 4] = [
    "ui-hud-actionbar-iconframe-bags",
    "UI-HUD-ActionBar-IconFrame-Slot-Small",
    "UI-HUD-ActionBar-IconFrame-Small",
    "UI-HUD-ActionBar-Keyring-Small",
];

/// The bar's width: money, the five bag slots, the toggle and the backpack. Like
/// `GetBagBarLength`, it counts hidden bag buttons too: the bar keeps its length.
pub fn bags_bar_width() -> f32 {
    let bags_w = (BAG_COUNT + 1) as f32 * (BAG_SLOT_SIZE + BAG_SLOT_GAP);
    MONEY_DISPLAY_W + bags_w + EXPAND_TOGGLE_W + BACKPACK_SIZE
}

pub fn bags_bar_screen(ctx: &SharedContext) -> Element {
    bag_bar(ctx.get::<BagBarState>().copied(), hud_layout(ctx))
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

/// `BagsBarMixin:Layout` with `Enum.BagsDirection.Left`: the backpack at the right, then
/// `BagBarExpandToggle`, then every shown bag button chained leftward, vertically centred
/// on the backpack. Collapsed, the four bag slots hide and the reagent slot, which stays
/// shown, chains on from the toggle (BagsBar.lua:62-107, MainMenuBarBagButtons.lua:259,
/// 391). Money (not part of the Retail bar) sits left of the bags.
fn bag_bar(synced: Option<BagBarState>, layout: &HudLayout) -> Element {
    let state = synced.unwrap_or_default();
    let total_w = bags_bar_width();
    let backpack_x = total_w - BACKPACK_SIZE;
    let toggle_x = backpack_x - EXPAND_TOGGLE_W;
    let centre_y = |size: f32| (BAGS_BAR_H - size) / 2.0;
    let shown_bags = if state.collapsed { 0 } else { BAG_COUNT };
    let slot_x = |i: usize| toggle_x - (i + 1) as f32 * (BAG_SLOT_SIZE + BAG_SLOT_GAP);
    let bags: Element = (0..shown_bags)
        .flat_map(|i| {
            // CharacterBag0Slot (bag 1) sits next to the toggle.
            let icon = state.bag_icons[i];
            let slot = BagSlot {
                name: format!("CharacterBag{i}Slot"),
                index: i + 1,
                size: BAG_SLOT_SIZE,
                art: if icon.is_some() {
                    BAG_SLOT
                } else {
                    BAG_SLOT_EMPTY
                },
                icon,
            };
            bag_slot(slot, slot_x(i), centre_y(BAG_SLOT_SIZE), Vec::new())
        })
        .collect();
    let reagent_icon = state.bag_icons[REAGENT_BAG - 1];
    let reagent = BagSlot {
        name: "CharacterReagentBag0Slot".to_string(),
        index: REAGENT_BAG,
        size: BAG_SLOT_SIZE,
        art: if reagent_icon.is_some() {
            REAGENT_SLOT
        } else {
            REAGENT_SLOT_EMPTY
        },
        icon: reagent_icon,
    };
    let backpack = BagSlot {
        name: "MainMenuBarBackpackButton".to_string(),
        index: 0,
        size: BACKPACK_SIZE,
        art: BACKPACK,
        icon: None,
    };
    let count = synced.map_or_else(Vec::new, |s| backpack_count(s.free_slots));
    let at = layout.bags_bar.place((total_w, BAGS_BAR_H));
    rsx! {
        r#frame {
            name: "BagsBar",
            width: {total_w},
            height: {BAGS_BAR_H},
            hidden: {layout.hide_utility_bars},
            pos_type: "absolute",
            left: {at.left.as_str()},
            right: {at.right.as_str()},
            top: {at.top.as_str()},
            bottom: {at.bottom.as_str()},
            margin_left: {at.margin_left},
            margin_top: {at.margin_top},
            {bag_slot(backpack, backpack_x, centre_y(BACKPACK_SIZE), count)}
            {expand_toggle(toggle_x, centre_y(EXPAND_TOGGLE_H), state.collapsed)}
            {bags}
            {bag_slot(reagent, slot_x(shown_bags), centre_y(BAG_SLOT_SIZE), Vec::new())}
            {money_display(centre_y(MONEY_DISPLAY_H), synced.map(|s| s.money))}
        }
    }
}

/// `BagBarExpandToggleMixin:GetRotation`: the left-pointing arrow turns by pi while the
/// bar is expanded (MainMenuBarBagButtons.lua:404-425).
fn expand_toggle(x: f32, y: f32, collapsed: bool) -> Element {
    let rotation = if collapsed { 0.0 } else { std::f32::consts::PI };
    rsx! {
        button {
            name: "BagBarExpandToggle",
            width: {EXPAND_TOGGLE_W},
            height: {EXPAND_TOGGLE_H},
            text: "",
            font_size: 8.0,
            onclick: ACTION_BAG_BAR_EXPAND_TOGGLE,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
            texture {
                name: "BagBarExpandToggleNormalTexture",
                width: {EXPAND_TOGGLE_W},
                height: {EXPAND_TOGGLE_H},
                texture_atlas: BAG_ARROW,
                rotation: {rotation},
                pos_type: "absolute",
                left: 0.0,
                top: 0.0,
            }
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

/// `MainMenuBarBackpackMixin:UpdateFreeSlots`: `(%s)` of the free slots, shown from
/// PLAYER_ENTERING_WORLD on (MainMenuBarBagButtons.lua:278-304).
fn backpack_count(free_slots: usize) -> Element {
    let text = format!("({free_slots})");
    rsx! {
        fontstring {
            name: "MainMenuBarBackpackButtonCount",
            width: {BACKPACK_SIZE},
            height: {COUNT_H},
            text: text.as_str(),
            font: "ArialNarrow",
            font_size: COUNT_FONT_SIZE,
            font_color: WHITE,
            outline: "OUTLINE",
            justify_h: "CENTER",
            pos_type: "absolute",
            left: 0.0,
            top: {(BACKPACK_SIZE - COUNT_H) / 2.0 + COUNT_BELOW_CENTRE},
        }
    }
}

struct BagSlot {
    name: String,
    index: usize,
    size: f32,
    art: &'static str,
    /// The equipped bag's icon, under the slot art (`BaseBagSlotButtonMixin:UpdateTextures`).
    icon: Option<u32>,
}

fn bag_slot(slot: BagSlot, x: f32, y: f32, overlay: Element) -> Element {
    let action = bag_toggle_action(slot.index);
    let art_name = DynName(format!("{}Art", slot.name));
    let icon = slot
        .icon
        .map(|fdid| bag_icon(format!("{}IconTexture", slot.name), fdid))
        .unwrap_or_default();
    let art: Element = rsx! {
        texture {
            name: art_name,
            width: {slot.size},
            height: {slot.size},
            texture_atlas: slot.art,
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
        }
    };
    let children: Element = icon.into_iter().chain(art).chain(overlay).collect();
    if slot.index == 0 {
        return rsx! {
            button {
                name: DynName(slot.name),
                width: {slot.size},
                height: {slot.size},
                text: "",
                font_size: 8.0,
                button_default_skin: false,
                onclick: {action.as_str()},
                pos_type: "absolute",
                pos_x: x,
                pos_y: y,
                {children}
            }
        };
    }
    // BaseBagSlotButtonMixin:OnLoadInternal `RegisterForClicks("AnyUp")`: a bag slot takes
    // right clicks too; the backpack overrides it and keeps left clicks.
    rsx! {
        r#frame {
            name: DynName(slot.name),
            width: {slot.size},
            height: {slot.size},
            onclick: {action.as_str()},
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
            {children}
        }
    }
}

/// The bag's `icon`, cropped to the `CircleMask` rect; the round mask itself is applied
/// by the host to every `*SlotIconTexture`.
fn bag_icon(name: String, fdid: u32) -> Element {
    let low = ICON_INSET / BAG_SLOT_SIZE;
    let high = (ICON_INSET + ICON_SIZE) / BAG_SLOT_SIZE;
    let coords = format!("{low},{high},{low},{high}");
    rsx! {
        texture {
            name: DynName(name),
            width: {ICON_SIZE},
            height: {ICON_SIZE},
            texture_fdid: fdid,
            tex_coords: {coords.as_str()},
            pos_type: "absolute",
            left: {ICON_INSET},
            top: {ICON_INSET},
        }
    }
}
