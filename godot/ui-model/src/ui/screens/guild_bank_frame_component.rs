//! Retail `GuildBankFrame` (Blizzard_GuildBankUI/Mainline/Blizzard_GuildBankUI.xml /
//! .lua, cited as GB.xml / GB.lua): the 750×428 guild bank with seven 14-slot columns,
//! the bank tabs down its right edge, the Bank / Log / Money Log / Info tabs below it,
//! the tab title and withdrawal limit, the guild money with Deposit / Withdraw, the
//! tab purchase screen and the tab info text. Positions are top-left offsets converted
//! from the anchors; docs/specs/guild-bank-frame.md lists them.

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

use crate::ui::screens::bank_art::{
    HIGHLIGHT_FONT_COLOR, ITEM_BUTTON, MoneyBoxNames, MoneyPrompt, QUICKSLOT, SlotItem, WHITE,
    cropped, item_slot, label, money_display, money_prompt, selected_marker, texture,
};
use crate::ui::screens::merchant_frame_component::{tab, tab_width};
use crate::ui::screens::quest_art::{DynName, NORMAL_FONT_COLOR, panel_button, window_chrome};
use crate::ui::strata::FrameStrata;

pub const FRAME_NAME: &str = "GuildBankFrame";
/// GB.xml:167 `<Size x="750" y="428"/>`.
pub const FRAME_W: f32 = 750.0;
pub const FRAME_H: f32 = 428.0;

pub const ACTION_CLOSE: &str = "guild_bank_close";
/// `guild_bank_mode:<bank|log|moneylog|info>`.
pub const ACTION_MODE_PREFIX: &str = "guild_bank_mode:";
/// `guild_bank_tab:<index>`; the buy tab is `ACTION_BUY_TAB_TAB`.
pub const ACTION_TAB_PREFIX: &str = "guild_bank_tab:";
pub const ACTION_BUY_TAB_TAB: &str = "guild_bank_buy_tab_tab";
/// `guild_bank_slot:<1-based slot id>`.
pub const ACTION_SLOT_PREFIX: &str = "guild_bank_slot:";
pub const ACTION_BUY_TAB: &str = "guild_bank_buy_tab";
pub const ACTION_DEPOSIT_MONEY: &str = "guild_bank_money_deposit";
pub const ACTION_WITHDRAW_MONEY: &str = "guild_bank_money_withdraw";
pub const ACTION_MONEY_ACCEPT: &str = "guild_bank_money_accept";
pub const ACTION_MONEY_CANCEL: &str = "guild_bank_money_cancel";
pub const ACTION_SAVE_INFO: &str = "guild_bank_save_info";

pub const MONEY_BOXES: MoneyBoxNames = MoneyBoxNames {
    gold: "GuildBankMoneyGold",
    silver: "GuildBankMoneySilver",
    copper: "GuildBankMoneyCopper",
};
pub const INFO_BOX: &str = "GuildBankTabInfoEditBox";

/// Retail `MAX_GUILDBANK_SLOTS_PER_TAB`, `NUM_SLOTS_PER_GUILDBANK_GROUP` (GB.lua:1-9).
pub const SLOTS: usize = 98;
const SLOTS_PER_COLUMN: usize = 14;

/// `Interface\GuildBankFrame\GuildVaultBG` (GB.xml:352).
const VAULT_BG: u32 = 590_068;
/// `Interface\GuildBankFrame\UI-GuildBankFrame-Slots` (GB.xml:23).
const COLUMN_BG: u32 = 132_073;
/// `Interface\GuildBankFrame\UI-GuildBankFrame-Tab` (GB.xml:108).
const TAB_BG: u32 = 132_074;
/// `Interface\GuildBankFrame\UI-GuildBankFrame-NewTab`, the buy tab icon (GB.lua:283).
const NEW_TAB: u32 = 132_071;
/// `Interface\GuildBankFrame\UI-TabNameBorder` (GB.xml:171-200).
const TAB_NAME_BORDER: u32 = 132_076;

/// GB.xml:31-68, 477-510: Column1 at 18,-59, columns 100 wide 3 apart; Button1 at
/// 7,-3, 7 below each other, Button8 12 right of Button1. `slot` is 0-based.
pub fn slot_position(slot: usize) -> (f32, f32) {
    let column = slot / SLOTS_PER_COLUMN;
    let within = slot % SLOTS_PER_COLUMN;
    let (sub, row) = (within / 7, within % 7);
    let x = 18.0 + column as f32 * 103.0 + 7.0 + sub as f32 * (ITEM_BUTTON + 12.0);
    let y = 59.0 + 3.0 + row as f32 * (ITEM_BUTTON + 7.0);
    (x, y)
}

/// GB.xml:580-618: Tab1 at the frame's TOPRIGHT -1,-17, each 50 below the last.
pub fn side_tab_position(index: usize) -> (f32, f32) {
    (FRAME_W - 1.0, 17.0 + index as f32 * 50.0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GuildBankModeView {
    #[default]
    Bank,
    Log,
    MoneyLog,
    Info,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct GuildSideTab {
    pub icon_fdid: u32,
    pub selected: bool,
}

/// Tab purchase screen (`BuyInfo`, GB.xml:622-660).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BuyTabView {
    pub cost: u64,
    pub purchased: usize,
    pub can_afford: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct GuildBankFrameState {
    pub visible: bool,
    pub title: String,
    pub mode: GuildBankModeView,
    pub tabs: Vec<GuildSideTab>,
    /// The Guild Master's buy tab: `Some(selected)` while a tab can be bought.
    pub buy_tab: Option<bool>,
    /// Tab title and its access suffix (`GUILDBANK_TAB_FULL_ACCESS` etc.) with its colour.
    pub tab_title: Option<(String, String, &'static str)>,
    /// `GUILDBANK_REMAINING_MONEY` line of the selected tab.
    pub limit_text: Option<String>,
    /// `GUILDBANK_TAB_LOCKED`: the columns are desaturated.
    pub locked: bool,
    /// `SLOTS` entries while the bank mode shows a viewable tab.
    pub slots: Vec<Option<SlotItem>>,
    pub buy: Option<BuyTabView>,
    /// `NO_GUILDBANK_TABS` and similar messages.
    pub error_message: Option<String>,
    /// Log lines, oldest first.
    pub log_lines: Vec<String>,
    /// Tab info text; `Some(editable)` in info mode.
    pub info: Option<(String, bool)>,
    pub money: u64,
    /// `GUILDBANK_AVAILABLE_MONEY` amount; `None` shows `UNLIMITED`.
    pub withdraw_limit: Option<u64>,
    pub can_withdraw: bool,
    pub money_prompt: Option<bool>,
}

pub fn guild_bank_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<GuildBankFrameState>()
        .expect("GuildBankFrameState must be in SharedContext");
    let hide = !state.visible;
    let mut children = window_chrome(FRAME_NAME, (FRAME_W, FRAME_H), &state.title, ACTION_CLOSE);
    // `RedMarbleBG` GuildVaultBG TOPLEFT 2,-20 / BOTTOMRIGHT -2,20, then the black
    // inner fill (GB.xml:350-362).
    children.extend(texture(
        format!("{FRAME_NAME}RedMarbleBG"),
        VAULT_BG,
        (2.0, 20.0, FRAME_W - 4.0, FRAME_H - 40.0),
        WHITE,
    ));
    children.extend(texture(
        format!("{FRAME_NAME}BlackBG"),
        VAULT_BG,
        (16.0, 57.0, FRAME_W - 33.0, 316.0),
        "0.0,0.0,0.0,1.0",
    ));
    children.extend(side_tabs(state));
    if let Some((title, access, color)) = &state.tab_title {
        children.extend(title_plate("TabTitle", title, Some((access, *color)), 30.0));
    }
    match state.mode {
        GuildBankModeView::Bank => children.extend(bank_mode(state)),
        GuildBankModeView::Log | GuildBankModeView::MoneyLog => {
            children.extend(log_lines(&state.log_lines))
        }
        GuildBankModeView::Info => children.extend(info(state.info.as_ref())),
    }
    if let Some(message) = &state.error_message {
        children.extend(label(
            format!("{FRAME_NAME}ErrorMessage"),
            message,
            (0.0, 216.0, FRAME_W, 16.0),
            (12.0, NORMAL_FONT_COLOR, "CENTER"),
        ));
    }
    children.extend(money_bar(state));
    children.extend(mode_tabs(state.mode));
    if let Some(deposit) = state.money_prompt {
        children.extend(money_prompt(
            &MoneyPrompt {
                name: "GuildBankMoneyPopup",
                // GUILDBANK_DEPOSIT / GUILDBANK_WITHDRAW.
                text: if deposit {
                    "Amount to deposit:"
                } else {
                    "Amount to withdraw:"
                },
                boxes: MONEY_BOXES,
                accept_action: ACTION_MONEY_ACCEPT,
                cancel_action: ACTION_MONEY_CANCEL,
            },
            ((FRAME_W - 320.0) / 2.0, 31.0),
        ));
    }
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

/// `TabTitleBG` / `TabLimitBG`: UI-TabNameBorder caps around the text, TOP at `top`.
fn title_plate(key: &str, text: &str, suffix: Option<(&str, &str)>, top: f32) -> Element {
    let full = match suffix {
        Some((access, _)) => format!("{text}  {access}"),
        None => text.to_string(),
    };
    let width = ui_toolkit::text_measure::measure_text(
        &full,
        ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
        12.0,
    )
    .map_or(8.0 * full.len() as f32, |(w, _)| w.ceil())
        + 20.0;
    let x = (FRAME_W - width) / 2.0;
    let name = format!("{FRAME_NAME}{key}");
    let mut children = cropped(
        format!("{name}BGLeft"),
        TAB_NAME_BORDER,
        "0.0,0.0625,0.0,0.5625",
        (x - 8.0, top, 8.0, 18.0),
    );
    children.extend(cropped(
        format!("{name}BG"),
        TAB_NAME_BORDER,
        "0.0625,0.546875,0.0,0.5625",
        (x, top, width, 18.0),
    ));
    children.extend(cropped(
        format!("{name}BGRight"),
        TAB_NAME_BORDER,
        "0.546875,0.609375,0.0,0.5625",
        (x + width, top, 8.0, 18.0),
    ));
    let (main, rest) = match suffix {
        Some((access, color)) => (text.to_string(), Some((access, color))),
        None => (full.clone(), None),
    };
    let main_width = ui_toolkit::text_measure::measure_text(
        &format!("{main}  "),
        ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
        12.0,
    )
    .map_or(8.0 * main.len() as f32, |(w, _)| w.ceil());
    // GB.xml:195-199: `TabTitle` has no width, so it never wraps; the suffix is
    // split off only to colour it, and both halves size to their own text.
    children.extend(title_text(
        name.clone(),
        &main,
        (x + 10.0, top + 2.0),
        NORMAL_FONT_COLOR,
    ));
    if let Some((access, color)) = rest {
        children.extend(title_text(
            format!("{name}Access"),
            access,
            (x + 10.0 + main_width, top + 2.0),
            color,
        ));
    }
    children
}

/// One auto-width line of the title plate (`GameFontNormal`, shadowed).
fn title_text(name: String, text: &str, (x, y): (f32, f32), color: &str) -> Element {
    rsx! {
        fontstring {
            name: {DynName(name)},
            width: "auto",
            height: 14.0,
            text,
            font: GameFont::FrizQuadrata,
            font_size: 12.0,
            font_color: color,
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h: "LEFT",
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

fn bank_mode(state: &GuildBankFrameState) -> Element {
    if let Some(buy) = &state.buy {
        return buy_info(buy);
    }
    let mut children = Element::default();
    if !state.slots.is_empty() {
        children.extend(columns(state));
    }
    if let Some(limit) = &state.limit_text {
        children.extend(title_plate("LimitLabel", limit, None, 370.0));
    }
    children
}

/// Seven `GuildBankFrameColumnTemplate` 100×311 backgrounds and their 14 buttons.
fn columns(state: &GuildBankFrameState) -> Element {
    let color = if state.locked {
        "0.5,0.5,0.5,1.0"
    } else {
        WHITE
    };
    let mut children: Element = (0..SLOTS / SLOTS_PER_COLUMN)
        .flat_map(|column| {
            let (x, y) = (18.0 + column as f32 * 103.0, 59.0);
            rsx! {
                texture {
                    name: {DynName(format!("{FRAME_NAME}Column{}Background", column + 1))},
                    width: 100.0,
                    height: 311.0,
                    texture_fdid: COLUMN_BG,
                    tex_coords: "0.0,0.78125,0.0,0.607421875",
                    vertex_color: color,
                    pos_type: "absolute",
                    left: x,
                    top: y,
                }
            }
        })
        .collect();
    for (slot, item) in state.slots.iter().enumerate() {
        let prefix = format!("{FRAME_NAME}Item{}", slot + 1);
        children.extend(item_slot(
            &prefix,
            slot_position(slot),
            Element::default(),
            item.as_ref(),
            &format!("{ACTION_SLOT_PREFIX}{}", slot + 1),
        ));
    }
    children
}

/// `BuyInfo` at TOP 0,-194: `PURCHASE_TAB_TEXT`, "(n/6 tabs purchased)", "Cost:",
/// the price and the 124×21 Purchase button (GB.xml:622-660).
fn buy_info(buy: &BuyTabView) -> Element {
    let name = format!("{FRAME_NAME}BuyInfo");
    let mut children = label(
        format!("{name}TabText"),
        "Do you wish to purchase this tab?",
        (0.0, 194.0, FRAME_W, 16.0),
        (14.0, HIGHLIGHT_FONT_COLOR, "CENTER"),
    );
    children.extend(label(
        format!("{name}PurchasedText"),
        &format!("({}/6 tabs purchased)", buy.purchased),
        (0.0, 212.0, FRAME_W, 14.0),
        (11.0, HIGHLIGHT_FONT_COLOR, "CENTER"),
    ));
    let cost_x = FRAME_W / 2.0 - 93.0;
    children.extend(label(
        format!("{name}Cost"),
        "Cost:",
        (cost_x - 40.0, 232.0, 80.0, 12.0),
        (12.0, NORMAL_FONT_COLOR, "CENTER"),
    ));
    let money_right = cost_x + 40.0 + 10.0 + 90.0;
    children.extend(money_display(
        &format!("{name}Money"),
        buy.cost,
        (money_right, 246.0),
        !buy.can_afford,
    ));
    children.extend(panel_button(
        format!("{name}PurchaseButton"),
        "Purchase",
        ACTION_BUY_TAB,
        buy.can_afford,
        (money_right + 12.0, 228.0, 124.0, 21.0),
    ));
    children
}

/// `GuildBankMessageFrame` 688×304 at 24,-64 (GB.xml:682-695), oldest line first.
fn log_lines(lines: &[String]) -> Element {
    let line_h = 14.0;
    let visible = (304.0 / line_h) as usize;
    let start = lines.len().saturating_sub(visible);
    lines[start..]
        .iter()
        .enumerate()
        .flat_map(|(index, line)| {
            label(
                format!("{FRAME_NAME}LogLine{}", index + 1),
                line,
                (24.0, 64.0 + index as f32 * line_h, 688.0, line_h),
                (12.0, HIGHLIGHT_FONT_COLOR, "LEFT"),
            )
        })
        .collect()
}

/// Info mode: the 691×306 text area and, for the Guild Master, the 100×22 Save
/// button (GB.xml:708-738).
fn info(info: Option<&(String, bool)>) -> Element {
    let Some((text, editable)) = info else {
        return Element::default();
    };
    if !*editable {
        return label(
            format!("{FRAME_NAME}InfoText"),
            text,
            (24.0, 64.0, 691.0, 306.0),
            (12.0, HIGHLIGHT_FONT_COLOR, "LEFT"),
        );
    }
    let mut children = rsx! {
        editbox {
            name: {DynName(INFO_BOX.to_string())},
            width: 691.0,
            height: 290.0,
            font: ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
            font_size: 12.0,
            font_color: HIGHLIGHT_FONT_COLOR,
            pos_type: "absolute",
            left: 24.0,
            top: 64.0,
        }
    };
    children.extend(panel_button(
        format!("{FRAME_NAME}InfoSaveButton"),
        "Save",
        ACTION_SAVE_INFO,
        true,
        (FRAME_W - 118.0, 360.0, 100.0, 22.0),
    ));
    children
}

/// Money bar (GB.xml:512-570): ThinGoldEdge from BOTTOMLEFT 1,25 to BOTTOMRIGHT -4,2,
/// `GUILDBANK_AVAILABLE_MONEY` with the withdraw limit, the guild money at
/// BOTTOMRIGHT -2,6, and Deposit / Withdraw 100×21 at BOTTOMRIGHT -8,30.
fn money_bar(state: &GuildBankFrameState) -> Element {
    let name = format!("{FRAME_NAME}MoneyFrame");
    let mut children = cropped(
        format!("{name}BG"),
        525_911,
        "0.0,0.9921875,0.3125,0.609375",
        (1.0, FRAME_H - 25.0, FRAME_W - 5.0, 23.0),
    );
    children.extend(label(
        format!("{name}LimitLabel"),
        "Available Amount:",
        (8.0, FRAME_H - 19.0, 120.0, 13.0),
        (11.0, NORMAL_FONT_COLOR, "LEFT"),
    ));
    match state.withdraw_limit {
        Some(limit) => children.extend(money_display(
            &format!("{name}WithdrawMoney"),
            limit,
            (240.0, FRAME_H - 6.0),
            false,
        )),
        None => children.extend(label(
            format!("{name}UnlimitedLabel"),
            "Unlimited",
            (133.0, FRAME_H - 19.0, 100.0, 13.0),
            (11.0, HIGHLIGHT_FONT_COLOR, "LEFT"),
        )),
    }
    children.extend(money_display(
        &format!("{name}Money"),
        state.money,
        (FRAME_W - 2.0 - 6.0, FRAME_H - 6.0),
        false,
    ));
    if state.mode == GuildBankModeView::Bank {
        children.extend(panel_button(
            format!("{FRAME_NAME}DepositButton"),
            "Deposit",
            ACTION_DEPOSIT_MONEY,
            true,
            (FRAME_W - 8.0 - 100.0, FRAME_H - 30.0 - 21.0, 100.0, 21.0),
        ));
        children.extend(panel_button(
            format!("{FRAME_NAME}WithdrawButton"),
            "Withdraw",
            ACTION_WITHDRAW_MONEY,
            state.can_withdraw,
            (FRAME_W - 8.0 - 197.0, FRAME_H - 30.0 - 21.0, 100.0, 21.0),
        ));
    }
    children
}

/// `GuildBankTabTemplate` 42×50: the tab art, a 36×34 button at 2,-8 with the icon,
/// UI-Quickslot2 60×60 and `CheckButtonHilight` when selected (GB.xml:104-156).
fn side_tabs(state: &GuildBankFrameState) -> Element {
    let mut children: Element = state
        .tabs
        .iter()
        .enumerate()
        .flat_map(|(index, side)| {
            side_tab(
                index,
                side.icon_fdid,
                side.selected,
                &format!("{ACTION_TAB_PREFIX}{index}"),
            )
        })
        .collect();
    if let Some(selected) = state.buy_tab {
        children.extend(side_tab(
            state.tabs.len(),
            NEW_TAB,
            selected,
            ACTION_BUY_TAB_TAB,
        ));
    }
    children
}

fn side_tab(index: usize, icon: u32, selected: bool, action: &str) -> Element {
    let name = format!("GuildBankTab{}", index + 1);
    let (x, y) = side_tab_position(index);
    let mut children = texture(
        format!("{name}Background"),
        TAB_BG,
        (0.0, 0.0, 64.0, 64.0),
        WHITE,
    );
    children.extend(texture(
        format!("{name}Icon"),
        icon,
        (2.0, 8.0, 36.0, 34.0),
        WHITE,
    ));
    children.extend(texture(
        format!("{name}NormalTexture"),
        QUICKSLOT,
        (2.0 + 18.0 - 30.0, 8.0 + 17.0 - 30.0 + 1.0, 60.0, 60.0),
        WHITE,
    ));
    if selected {
        children.extend(selected_marker(format!("{name}Checked"), (2.0, 8.0, 35.0)));
    }
    rsx! {
        r#frame {
            name: {DynName(name)},
            width: 42.0,
            height: 50.0,
            onclick: action,
            mouse_enabled: true,
            pos_type: "absolute",
            left: x,
            top: y,
            {children}
        }
    }
}

/// `GuildBankFrameTab1..4` (PanelTabButtonTemplate, maxTabWidth 128) from the frame's
/// BOTTOMLEFT 7,-30 (GB.xml:572-579).
fn mode_tabs(mode: GuildBankModeView) -> Element {
    let tabs = [
        ("Guild Bank", GuildBankModeView::Bank, "bank"),
        ("Log", GuildBankModeView::Log, "log"),
        ("Money Log", GuildBankModeView::MoneyLog, "moneylog"),
        ("Info", GuildBankModeView::Info, "info"),
    ];
    let top = FRAME_H + 30.0 - 32.0;
    let mut x = 7.0;
    let mut selected = Element::default();
    let mut others = Element::default();
    for (index, (label_text, tab_mode, key)) in tabs.into_iter().enumerate() {
        let width = tab_width(label_text).min(128.0);
        let element = tab(
            &format!("{FRAME_NAME}Tab{}", index + 1),
            label_text,
            (x, top, width),
            mode == tab_mode,
            &format!("{ACTION_MODE_PREFIX}{key}"),
        );
        if mode == tab_mode {
            selected.extend(element);
        } else {
            others.extend(element);
        }
        x += width - 16.0;
    }
    others.extend(selected);
    others
}

