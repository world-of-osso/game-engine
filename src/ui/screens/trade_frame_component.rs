//! Retail `TradeFrame` (Blizzard_UIPanels_Game/Mainline/TradeFrame.xml / .lua, cited
//! as TF.xml / TF.lua): the 344×446 `ButtonFrameTemplate` window with the player's
//! seven item slots on the left and the trade partner's on the right (the seventh
//! "Will not be traded"), the player's money entry and the partner's money, the
//! accept highlights, and the Trade / Cancel buttons. Positions are top-left
//! offsets converted from the anchors.

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::screens::auction_house_frame_component::inset_border;
use crate::ui::screens::bank_art::{
    HIGHLIGHT_FONT_COLOR, ITEM_BUTTON, MoneyBoxNames, SlotItem, WHITE, cropped, item_slot, label,
    money_display, money_input_compact, texture,
};
use crate::ui::screens::quest_art::{DynName, NORMAL_FONT_COLOR, panel_button, window_chrome};
use crate::ui::strata::FrameStrata;

pub const FRAME_NAME: &str = "TradeFrame";
/// TF.xml:179 `<Size x="344" y="446"/>`.
pub const FRAME_W: f32 = 344.0;
pub const FRAME_H: f32 = 446.0;
/// `MAX_TRADE_ITEMS` (TF.lua:1).
pub const TRADE_SLOTS: usize = 7;

pub const ACTION_CLOSE: &str = "trade_close";
/// `TradeFrameTradeButton` (`AcceptTrade`).
pub const ACTION_TRADE: &str = "trade_accept";
/// `TradeFrameCancelButton_OnClick`.
pub const ACTION_CANCEL: &str = "trade_cancel";
/// `trade_player_slot:<0-based slot>`: clicking an offered item takes it back.
pub const ACTION_PLAYER_SLOT_PREFIX: &str = "trade_player_slot:";

/// `TradePlayerInputMoneyFrame` edit boxes.
pub const MONEY_BOXES: MoneyBoxNames = MoneyBoxNames {
    gold: "TradePlayerInputMoneyFrameGold",
    silver: "TradePlayerInputMoneyFrameSilver",
    copper: "TradePlayerInputMoneyFrameCopper",
};

/// `Interface\TradeFrame\UI-TradeFrame-Highlight` (TF.xml:6-20).
const HIGHLIGHT: u32 = 137_073;
/// `Interface\Buttons\UI-EmptySlot` (TF.xml:35).
const EMPTY_SLOT: u32 = 130_766;
/// `Interface\QuestFrame\UI-QuestItemNameFrame` (TF.xml:41).
const NAME_FRAME: u32 = 136_796;
/// `Interface\TradeFrame\UI-TradeFrame-EnchantIcon` (TF.xml:330, 387).
const ENCHANT_ICON: u32 = 137_072;

/// Player items at TOPLEFT 14,-89, recipient items at 182,-89, each 7 below the
/// last; the seventh 28 below the sixth (TF.xml:288-381).
pub fn slot_position(recipient: bool, slot: usize) -> (f32, f32) {
    let x = if recipient { 182.0 } else { 14.0 };
    let step = ITEM_BUTTON + 7.0;
    let y = 89.0 + slot.min(5) as f32 * step;
    let y = if slot == 6 { y + ITEM_BUTTON + 28.0 } else { y };
    (x, y)
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TradeItemView {
    pub item: SlotItem,
    pub name: String,
    /// `ITEM_QUALITY_COLORS` of the item.
    pub name_color: String,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TradeFrameState {
    pub visible: bool,
    pub player_name: String,
    pub recipient_name: String,
    /// `TRADE_SLOTS` entries each.
    pub player_items: Vec<Option<TradeItemView>>,
    pub recipient_items: Vec<Option<TradeItemView>>,
    pub recipient_money: u64,
    pub player_accepted: bool,
    pub recipient_accepted: bool,
}

pub fn trade_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<TradeFrameState>()
        .expect("TradeFrameState must be in SharedContext");
    let hide = !state.visible;
    let mut children = window_chrome(FRAME_NAME, (FRAME_W, FRAME_H), "", ACTION_CLOSE);
    // `TradeRecipientBG` white at .15 from TOPRIGHT -172,-20 to BOTTOMRIGHT (TF.xml:212).
    children.extend(rsx! {
        r#frame {
            name: {DynName(format!("{FRAME_NAME}RecipientBG"))},
            width: 172.0,
            height: {FRAME_H - 20.0},
            background_color: "1.0,1.0,1.0,0.15",
            pos_type: "absolute",
            left: {FRAME_W - 172.0},
            top: 20.0,
        }
    });
    children.extend(names(state));
    children.extend(insets());
    children.extend(highlights(state));
    children.extend(side(state, false));
    children.extend(side(state, true));
    children.extend(money_input_compact(MONEY_BOXES, (11.0, 61.0)));
    // `TradeRecipientMoneyFrame` TOPRIGHT -5,-64 (TF.xml:477).
    children.extend(money_display(
        "TradeRecipientMoneyFrame",
        state.recipient_money,
        (FRAME_W - 5.0, 64.0 + 14.0),
        false,
    ));
    children.extend(buttons(state));
    rsx! {
        r#frame {
            name: {DynName(FRAME_NAME.into())},
            width: FRAME_W,
            height: FRAME_H,
            strata: FrameStrata::Dialog,
            hidden: hide,
            mouse_enabled: true,
            pos_type: "absolute",
            left: 16.0,
            top: 104.0,
            {children}
        }
    }
}

/// `TradeFramePlayerNameText` at 65,-5 and `TradeFrameRecipientNameText` at 230,-5.
fn names(state: &TradeFrameState) -> Element {
    let mut out = label(
        "TradeFramePlayerNameText".into(),
        &state.player_name,
        (65.0, 5.0, 100.0, 12.0),
        (12.0, NORMAL_FONT_COLOR, "CENTER"),
    );
    out.extend(label(
        "TradeFrameRecipientNameText".into(),
        &state.recipient_name,
        (230.0, 5.0, 80.0, 12.0),
        (12.0, NORMAL_FONT_COLOR, "CENTER"),
    ));
    out
}

/// `InsetFrameTemplate` borders under both item columns, the "Will not be traded"
/// slots and both money rows (TF.xml:326-470).
fn insets() -> Element {
    [
        ("TradePlayerItemsInset", (4.0, 83.0, 162.0, 269.0)),
        ("TradeRecipientItemsInset", (175.0, 83.0, 163.0, 269.0)),
        ("TradePlayerEnchantInset", (4.0, 354.0, 162.0, 64.0)),
        ("TradeRecipientEnchantInset", (175.0, 354.0, 163.0, 64.0)),
        ("TradePlayerInputMoneyInset", (4.0, 58.0, 162.0, 24.0)),
        ("TradeRecipientMoneyInset", (175.0, 58.0, 163.0, 23.0)),
    ]
    .into_iter()
    .flat_map(|(name, rect)| inset_border(name, rect))
    .collect()
}

/// `TradeHighlightTemplate`: 161 wide top / middle / bottom strips, `h` high.
fn highlight(name: &str, (x, y, h): (f32, f32, f32)) -> Element {
    let mut out = cropped(
        format!("{name}Top"),
        HIGHLIGHT,
        "0.0,0.62890625,0.0,0.0625",
        (x, y, 161.0, 16.0),
    );
    out.extend(cropped(
        format!("{name}Middle"),
        HIGHLIGHT,
        "0.0,0.62890625,0.0625,0.9375",
        (x, y + 16.0, 161.0, h - 32.0),
    ));
    out.extend(cropped(
        format!("{name}Bottom"),
        HIGHLIGHT,
        "0.0,0.62890625,0.9375,1.0",
        (x, y + h - 16.0, 161.0, 16.0),
    ));
    out
}

/// `TRADE_ACCEPT_UPDATE` shows each side's highlight while it has accepted
/// (TF.lua:206-224): 150×266 at 6,-85 / 176,-85, the enchant ones 61 high 4 below.
fn highlights(state: &TradeFrameState) -> Element {
    let mut out = Element::default();
    for (accepted, name, x) in [
        (state.player_accepted, "TradeHighlightPlayer", 6.0),
        (state.recipient_accepted, "TradeHighlightRecipient", 176.0),
    ] {
        if accepted {
            out.extend(highlight(name, (x, 85.0, 266.0)));
            out.extend(highlight(
                &format!("{name}Enchant"),
                (x, 85.0 + 266.0 + 4.0, 61.0),
            ));
        }
    }
    out
}

fn side(state: &TradeFrameState, recipient: bool) -> Element {
    let (prefix, items) = if recipient {
        ("TradeRecipientItem", &state.recipient_items)
    } else {
        ("TradePlayerItem", &state.player_items)
    };
    let mut out: Element = (0..TRADE_SLOTS)
        .flat_map(|slot| {
            let item = items.get(slot).and_then(Option::as_ref);
            trade_item(prefix, slot, recipient, item)
        })
        .collect();
    // `TradeFramePlayerEnchantText` at 15,-360 and the recipient's 166 right.
    let x = if recipient { 181.0 } else { 15.0 };
    out.extend(label(
        format!("{prefix}EnchantText"),
        "Will not be traded",
        (x, 360.0, 150.0, 12.0),
        (10.0, HIGHLIGHT_FONT_COLOR, "LEFT"),
    ));
    out
}

/// One `TradeItemTemplate` (153×37): slot art, name plate and item name; the
/// player's filled slots click to take the item back.
fn trade_item(prefix: &str, slot: usize, recipient: bool, item: Option<&TradeItemView>) -> Element {
    let name = format!("{prefix}{}", slot + 1);
    let (x, y) = slot_position(recipient, slot);
    let mut out = texture(
        format!("{name}SlotTexture"),
        EMPTY_SLOT,
        (x - 13.0, y - 13.0, 64.0, 64.0),
        WHITE,
    );
    // NameFrame LEFT at the slot texture's RIGHT -20: x + 51 - 20.
    out.extend(texture(
        format!("{name}NameFrame"),
        NAME_FRAME,
        (x + 31.0, y + ITEM_BUTTON / 2.0 - 32.0, 124.0, 64.0),
        WHITE,
    ));
    if slot == TRADE_SLOTS - 1 && item.is_none() {
        out.extend(texture(
            format!("{name}EnchantIcon"),
            ENCHANT_ICON,
            (x, y, 62.0, 62.0),
            WHITE,
        ));
    }
    if let Some(item) = item {
        out.extend(label(
            format!("{name}Name"),
            &item.name,
            (x + 46.0, y + 3.0, 90.0, 30.0),
            (10.0, &item.name_color, "LEFT"),
        ));
    }
    let action = match (recipient, item) {
        (false, Some(_)) => format!("{ACTION_PLAYER_SLOT_PREFIX}{slot}"),
        _ => String::new(),
    };
    let background = Element::default();
    out.extend(item_slot(
        &format!("{name}ItemButton"),
        (x, y),
        background,
        item.map(|item| &item.item),
        &action,
    ));
    out
}

/// `TradeFrameTradeButton` 85×22 at BOTTOMRIGHT -85,5 and Cancel 77×22 3 right;
/// Trade is disabled once the player has accepted (TF.lua:212).
fn buttons(state: &TradeFrameState) -> Element {
    let top = FRAME_H - 5.0 - 22.0;
    let trade_x = FRAME_W - 85.0 - 85.0;
    let mut out = panel_button(
        "TradeFrameTradeButton".into(),
        "Trade",
        ACTION_TRADE,
        !state.player_accepted,
        (trade_x, top, 85.0, 22.0),
    );
    out.extend(panel_button(
        "TradeFrameCancelButton".into(),
        "Cancel",
        ACTION_CANCEL,
        true,
        (trade_x + 85.0 + 3.0, top, 77.0, 22.0),
    ));
    out
}

#[cfg(all(test, feature = "dev"))]
#[path = "trade_frame_component_tests.rs"]
mod tests;
