//! Retail `BankFrame` (Blizzard_UIPanels_Game/Mainline/BankFrame.xml / .lua, cited as
//! BF.xml / BF.lua): the 738×460 combined bank with the Bank and Warband Bank tabs
//! below it, purchased bank tabs down its right edge, the 98-slot grid of the
//! selected tab, the tab purchase prompt, the Warband money frame, Deposit All and
//! the tab settings menu. Positions are top-left offsets converted from the anchors;
//! docs/specs/bank-frame.md lists them.

use ui_toolkit::atlas::{ActiveSkin, AtlasSource, active_skin, resolve_region};
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::strata::DrawLayer;
use ui_toolkit::widget_def::Element;

use crate::ui::screens::bank_art::{
    HIGHLIGHT_FONT_COLOR, ITEM_BUTTON, MoneyBoxNames, MoneyPrompt, SlotItem, WHITE, checkbox,
    cropped, edit_box, item_slot, label, money_display, money_prompt, selected_marker, texture,
};
use crate::ui::screens::merchant_frame_component::{tab, tab_width};
use crate::ui::screens::quest_art::{DynName, NORMAL_FONT_COLOR, panel_button, window_chrome};
use crate::ui::strata::FrameStrata;

pub const FRAME_NAME: &str = "BankFrame";
/// BF.xml:674 `<Size x="738" y="460"/>`.
pub const FRAME_W: f32 = 738.0;
pub const FRAME_H: f32 = 460.0;

pub const ACTION_CLOSE: &str = "bank_close";
pub const ACTION_SHOW_CHARACTER: &str = "bank_show:character";
pub const ACTION_SHOW_ACCOUNT: &str = "bank_show:account";
/// `bank_tab:<index>`; the purchase tab is `ACTION_PURCHASE_TAB`.
pub const ACTION_TAB_PREFIX: &str = "bank_tab:";
pub const ACTION_PURCHASE_TAB: &str = "bank_purchase_tab";
/// `bank_slot:<index>` of the shown tab.
pub const ACTION_SLOT_PREFIX: &str = "bank_slot:";
pub const ACTION_PURCHASE: &str = "bank_purchase";
pub const ACTION_DEPOSIT_MONEY: &str = "bank_money_deposit";
pub const ACTION_WITHDRAW_MONEY: &str = "bank_money_withdraw";
pub const ACTION_MONEY_ACCEPT: &str = "bank_money_accept";
pub const ACTION_MONEY_CANCEL: &str = "bank_money_cancel";
pub const ACTION_AUTO_DEPOSIT: &str = "bank_auto_deposit";
pub const ACTION_INCLUDE_REAGENTS: &str = "bank_include_reagents";
/// `bank_settings_flag:<Enum.BagSlotFlags bit>`.
pub const ACTION_SETTINGS_FLAG_PREFIX: &str = "bank_settings_flag:";
pub const ACTION_SETTINGS_ACCEPT: &str = "bank_settings_accept";
pub const ACTION_SETTINGS_CANCEL: &str = "bank_settings_cancel";

pub const MONEY_BOXES: MoneyBoxNames = MoneyBoxNames {
    gold: "BankMoneyGold",
    silver: "BankMoneySilver",
    copper: "BankMoneyCopper",
};
pub const TAB_NAME_BOX: &str = "BankTabSettingsName";

/// BF.xml:677; Retail UiTextureAtlasMember.csv:11422, Forever :17833.
const BACKGROUND: &str = "bank-frame-background";
/// BF.xml:571 / BF.lua:586; Retail members :7767, Forever :18117.
const SLOT: &str = "bags-item-slot64";
/// BF.lua:582; Retail members :11547 (same set-0 member under Forever).
const WARBAND_SLOT: &str = "warband-bank-slot";
/// Forever Camelot/BankFrame.xml:34,36; Forever members :18121,:17832.
const FOREVER_SLOT_FRAME: &str = "bank-frame-item-slotframe";
const FOREVER_SLOT: &str = "bags-item-bankslot64";
/// Camelot/BankFrame.xml:76-79: useAtlasSize, scale .48, BOTTOM +220.
/// Forever member :18118 (864×32 on a 1024×128 sheet).
const FOREVER_DIVIDER: &str = "bank-divider";
const FOREVER_DIVIDER_SCALE: f32 = 0.48;
const FOREVER_DIVIDER_BOTTOM: f32 = 220.0;
/// `Interface\SpellBook\SpellBook-SkillLineTab`, the side tab border (BF.xml:353).
const SKILL_LINE_TAB: u32 = 136_831;
/// Retail and Forever UiTextureAtlasMember.csv:2883: the existing purchase-tab crop.
const ADD_SLOTS: &str = "bags-icon-addslots";
/// `Interface\GuildBankFrame\Corners`, the purchase prompt corners (BF.xml:410-437).
const PROMPT_CORNERS: u32 = 590_067;

/// BF.lua:941-968: 7 rows; columns pair up 8 apart, pairs 19 apart; first slot 26,-63.
pub fn slot_position(index: usize) -> (f32, f32) {
    let (column, row) = (index / 7, index % 7);
    let x = 26.0 + column as f32 * (ITEM_BUTTON + 8.0) + (column / 2) as f32 * 11.0;
    let y = 63.0 + row as f32 * (ITEM_BUTTON + 10.0);
    (x, y)
}

/// BF.lua:907-924: first tab at the panel's TOPRIGHT +2,-25, then 17 below the last.
pub fn side_tab_position(index: usize) -> (f32, f32) {
    (FRAME_W + 2.0, 25.0 + index as f32 * (32.0 + 17.0))
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct SideTab {
    pub icon_fdid: u32,
    pub selected: bool,
}

/// Tab purchase prompt (BF.xml:477-560, `C_Bank.FetchNextPurchasableBankTabData`).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PurchasePromptView {
    pub title: String,
    pub text: String,
    pub cost: u64,
    pub can_afford: bool,
}

/// Warband money frame (BF.xml:186-226).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MoneyFrameView {
    pub money: u64,
    pub can_withdraw: bool,
    pub can_deposit: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BankPromptView {
    Money {
        deposit: bool,
    },
    /// Deposit assignment flags being edited.
    TabSettings {
        flags: u32,
        name_prompt: String,
    },
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct BankFrameState {
    pub visible: bool,
    pub title: String,
    /// The Warband bank is shown.
    pub account: bool,
    /// Selected tab name (`BankPanelHeaderFrameTemplate`).
    pub header: String,
    pub tabs: Vec<SideTab>,
    /// The purchase tab: `Some(selected)` while a tab can be bought.
    pub purchase_tab: Option<bool>,
    /// `BANK_TAB_SLOTS` slots of the selected tab; empty while the prompt shows.
    pub slots: Vec<Option<SlotItem>>,
    pub purchase: Option<PurchasePromptView>,
    pub money: Option<MoneyFrameView>,
    /// `CHARACTER_BANK_DEPOSIT_BUTTON_LABEL` / `ACCOUNT_BANK_DEPOSIT_BUTTON_LABEL`.
    pub deposit_all_label: String,
    /// "Include tradeable reagents" checkbox (Warband bank only).
    pub include_reagents: Option<bool>,
    pub prompt: Option<BankPromptView>,
}

pub fn bank_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<BankFrameState>()
        .expect("BankFrameState must be in SharedContext");
    let hide = !state.visible;
    let skin = active_skin();
    let mut children = window_chrome(FRAME_NAME, (FRAME_W, FRAME_H), &state.title, ACTION_CLOSE);
    // `Background` TOPLEFT 0,-20 / BOTTOMRIGHT 0,30 (BF.xml:677-682).
    children.extend(bank_atlas(
        format!("{FRAME_NAME}Background"),
        BACKGROUND,
        skin,
        (2.0, 20.0, FRAME_W - 4.0, FRAME_H - 50.0),
        (WHITE, DrawLayer::Artwork),
    ));
    match skin {
        ActiveSkin::Modern => {}
        ActiveSkin::Forever => children.extend(bank_divider(skin)),
    }
    children.extend(side_tabs(state, skin));
    match &state.purchase {
        Some(prompt) => children.extend(purchase_prompt(prompt, skin)),
        None => {
            children.extend(header(&state.header));
            children.extend(slots(state, skin));
            children.extend(deposit_all(state));
        }
    }
    if let Some(money) = &state.money {
        children.extend(money_frame(money));
    }
    children.extend(bank_tabs(state.account));
    if let Some(prompt) = &state.prompt {
        children.extend(prompt_frame(prompt));
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

/// Resolve names before mounting so Modern retains its exact FDID/UV tree contract.
/// Missing local c60 BLPs are intentionally not checked here.
fn bank_atlas(
    name: String,
    atlas_name: &str,
    skin: ActiveSkin,
    (x, y, width, height): (f32, f32, f32, f32),
    (color, layer): (&str, DrawLayer),
) -> Element {
    let (fdid, coords) = resolve_bank_texture(atlas_name, skin);
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            texture_fdid: fdid,
            tex_coords: {coords.as_str()},
            vertex_color: color,
            draw_layer: {layer.as_str()},
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

fn resolve_bank_texture(name: &str, skin: ActiveSkin) -> (u32, String) {
    let region = resolve_region(name, skin)
        .unwrap_or_else(|| panic!("bank atlas {name} missing under {skin:?}"));
    let AtlasSource::FileDataId(fdid) = region.source else {
        panic!("bank atlas {name} is not a DB2 texture")
    };
    let coords = format!(
        "{},{},{},{}",
        region.left, region.right, region.top, region.bottom
    );
    (fdid, coords)
}

fn bank_divider(skin: ActiveSkin) -> Element {
    let region = resolve_region(FOREVER_DIVIDER, skin).expect("Forever bank-divider atlas");
    let width = region.width * FOREVER_DIVIDER_SCALE;
    let height = region.height * FOREVER_DIVIDER_SCALE;
    bank_atlas(
        format!("{FRAME_NAME}Divider"),
        FOREVER_DIVIDER,
        skin,
        (
            (FRAME_W - width) / 2.0,
            FRAME_H - FOREVER_DIVIDER_BOTTOM - height,
            width,
            height,
        ),
        (WHITE, DrawLayer::Artwork),
    )
}

/// Camelot/BankFrame.xml:34-36 changes only the slot art, not the existing grid.
fn slot_chrome(prefix: &str, account: bool, skin: ActiveSkin) -> Element {
    let mut children = slot_background(prefix, account, skin);
    match skin {
        ActiveSkin::Modern => {}
        ActiveSkin::Forever => children.extend(bank_atlas(
            format!("{prefix}NormalTexture"),
            FOREVER_SLOT_FRAME,
            skin,
            (0.0, 0.0, ITEM_BUTTON, ITEM_BUTTON),
            (WHITE, DrawLayer::Overlay),
        )),
    }
    children
}

fn slot_background(prefix: &str, account: bool, skin: ActiveSkin) -> Element {
    let (name, rect) = match (skin, account) {
        // BF.lua:579-585: Warband backgrounds grow -6,5 / 6,-7.
        (_, true) => (
            WARBAND_SLOT,
            (-6.0, -5.0, ITEM_BUTTON + 12.0, ITEM_BUTTON + 12.0),
        ),
        (ActiveSkin::Modern, false) => (SLOT, (0.0, 0.0, ITEM_BUTTON, ITEM_BUTTON)),
        (ActiveSkin::Forever, false) => (FOREVER_SLOT, (0.0, 0.0, ITEM_BUTTON, ITEM_BUTTON)),
    };
    bank_atlas(
        format!("{prefix}Background"),
        name,
        skin,
        rect,
        (WHITE, DrawLayer::Artwork),
    )
}

/// `BankPanelHeaderFrameTemplate` 300×20 at TOP 0,-36 (BF.xml:166-176, 605-608).
fn header(text: &str) -> Element {
    label(
        format!("{FRAME_NAME}Header"),
        text,
        ((FRAME_W - 300.0) / 2.0, 36.0, 300.0, 20.0),
        (14.0, HIGHLIGHT_FONT_COLOR, "CENTER"),
    )
}

fn slots(state: &BankFrameState, skin: ActiveSkin) -> Element {
    state
        .slots
        .iter()
        .enumerate()
        .flat_map(|(index, item)| {
            let prefix = format!("{FRAME_NAME}Item{}", index + 1);
            let background = slot_chrome(&prefix, state.account, skin);
            item_slot(
                &prefix,
                slot_position(index),
                background,
                item.as_ref(),
                &format!("{ACTION_SLOT_PREFIX}{index}"),
            )
        })
        .collect()
}

/// `BankPanelTabTemplate` 32×32: SkillLineTab border 64×64 at -3,11, the icon, and
/// `CheckButtonHilight` when selected (BF.xml:349-376).
fn side_tabs(state: &BankFrameState, skin: ActiveSkin) -> Element {
    let mut children: Element = state
        .tabs
        .iter()
        .enumerate()
        .flat_map(|(index, side)| {
            side_tab(
                &format!("{FRAME_NAME}Tab{}", index + 1),
                side_tab_position(index),
                SideTabIcon::Texture(side.icon_fdid),
                skin,
                side.selected,
                &format!("{ACTION_TAB_PREFIX}{index}"),
            )
        })
        .collect();
    if let Some(selected) = state.purchase_tab {
        children.extend(side_tab(
            &format!("{FRAME_NAME}PurchaseTab"),
            side_tab_position(state.tabs.len()),
            SideTabIcon::Atlas(ADD_SLOTS),
            skin,
            selected,
            ACTION_PURCHASE_TAB,
        ));
    }
    children
}

enum SideTabIcon<'a> {
    Texture(u32),
    Atlas(&'a str),
}

fn side_tab(
    name: &str,
    (x, y): (f32, f32),
    icon: SideTabIcon,
    skin: ActiveSkin,
    selected: bool,
    action: &str,
) -> Element {
    let mut children = texture(
        format!("{name}Border"),
        SKILL_LINE_TAB,
        (-3.0, -11.0, 64.0, 64.0),
        WHITE,
    );
    children.extend(match icon {
        SideTabIcon::Texture(fdid) => {
            texture(format!("{name}Icon"), fdid, (0.0, 0.0, 32.0, 32.0), WHITE)
        }
        SideTabIcon::Atlas(art) => bank_atlas(
            format!("{name}Icon"),
            art,
            skin,
            (0.0, 0.0, 32.0, 32.0),
            (WHITE, DrawLayer::Artwork),
        ),
    });
    if selected {
        children.extend(selected_marker(format!("{name}Selected"), (0.0, 0.0, 32.0)));
    }
    rsx! {
        r#frame {
            name: {DynName(name.to_string())},
            width: 32.0,
            height: 32.0,
            onclick: action,
            mouse_enabled: true,
            pos_type: "absolute",
            left: x,
            top: y,
            {children}
        }
    }
}

/// `BankPanelPurchasePromptTemplate` inset 10,-60 / -10,70 (BF.xml:477-560, 645-649):
/// title, `PurchasePromptBody`, "Cost:" with the price (red when unaffordable) and the
/// 105×21 Purchase button.
fn purchase_prompt(prompt: &PurchasePromptView, skin: ActiveSkin) -> Element {
    let (x, y) = (10.0, 60.0);
    let (width, height) = (FRAME_W - 20.0, FRAME_H - 130.0);
    let name = format!("{FRAME_NAME}PurchasePrompt");
    let mut children = bank_atlas(
        format!("{name}Background"),
        SLOT,
        skin,
        (x + 4.0, y + 4.0, width - 8.0, height - 7.0),
        ("0.0,0.0,0.0,0.75", DrawLayer::Artwork),
    );
    // Corners: Interface\GuildBankFrame\Corners (BF.xml:410-437).
    let corners = [
        (
            "TopLeft",
            "0.015625,0.515625,0.40234375,0.52734375",
            (x + 4.0, y),
        ),
        (
            "TopRight",
            "0.015625,0.515625,0.26953125,0.39453125",
            (x + width - 36.0, y),
        ),
        (
            "BottomLeft",
            "0.015625,0.515625,0.00390625,0.12890625",
            (x + 4.0, y + height - 32.0),
        ),
        (
            "BottomRight",
            "0.015625,0.515625,0.13671875,0.26171875",
            (x + width - 36.0, y + height - 32.0),
        ),
    ];
    for (corner, coords, (cx, cy)) in corners {
        children.extend(cropped(
            format!("{name}{corner}"),
            PROMPT_CORNERS,
            coords,
            (cx, cy, 32.0, 32.0),
        ));
    }
    let mid = y + height / 2.0;
    // `Title` QuestFont_Enormous 18 above `PromptText` (Game16Font, 300 wide, CENTER).
    children.extend(label(
        format!("{name}Title"),
        &prompt.title,
        ((FRAME_W - 384.0) / 2.0, mid - 90.0, 384.0, 30.0),
        (26.0, NORMAL_FONT_COLOR, "CENTER"),
    ));
    children.extend(label(
        format!("{name}Text"),
        &prompt.text,
        ((FRAME_W - 300.0) / 2.0, mid - 50.0, 300.0, 100.0),
        (16.0, HIGHLIGHT_FONT_COLOR, "CENTER"),
    ));
    // `TabCostFrame` 15 below the text: money CENTER -30, "Cost:" 10 left of it, the
    // Purchase button 12 right of it.
    let cost_y = mid + 70.0;
    let money_right = FRAME_W / 2.0 + 10.0;
    children.extend(label(
        format!("{name}Cost"),
        "Cost:",
        (money_right - 170.0, cost_y, 80.0, 14.0),
        (14.0, NORMAL_FONT_COLOR, "RIGHT"),
    ));
    // Retail reds the price the player can't afford (BF.lua:1078-1087); the money
    // frame greys it.
    children.extend(money_display(
        &format!("{name}Money"),
        prompt.cost,
        (money_right, cost_y + 14.0),
        !prompt.can_afford,
    ));
    children.extend(panel_button(
        format!("{name}Button"),
        "Purchase",
        ACTION_PURCHASE,
        prompt.can_afford,
        (money_right + 12.0, cost_y - 4.0, 105.0, 21.0),
    ));
    children
}

/// `BankPanelMoneyFrameTemplate` 394×25 at BOTTOMRIGHT -3,3: ThinGoldEdge 178×19 with
/// the money right-aligned in it, then Withdraw and Deposit 105×21 (BF.xml:186-226).
fn money_frame(view: &MoneyFrameView) -> Element {
    let (x, y) = (FRAME_W - 3.0 - 394.0, FRAME_H - 3.0 - 25.0);
    let name = format!("{FRAME_NAME}MoneyFrame");
    let mut children = cropped(
        format!("{name}Border"),
        525_911,
        "0.0,0.9921875,0.3125,0.609375",
        (x, y + 3.0, 178.0, 19.0),
    );
    children.extend(money_display(
        &format!("{name}Money"),
        view.money,
        (x + 178.0 - 6.0, y + 3.0 + 16.0),
        false,
    ));
    children.extend(panel_button(
        format!("{name}WithdrawButton"),
        "Withdraw",
        ACTION_WITHDRAW_MONEY,
        view.can_withdraw,
        (x + 183.0, y + 2.0, 105.0, 21.0),
    ));
    children.extend(panel_button(
        format!("{name}DepositButton"),
        "Deposit",
        ACTION_DEPOSIT_MONEY,
        view.can_deposit,
        (x + 289.0, y + 2.0, 105.0, 21.0),
    ));
    children
}

/// `BankPanelAutoDepositFrameTemplate` 300×26 at the NineSlice BOTTOM +10: the 256×24
/// button centred and the 24×23 checkbox 10 right of it (BF.xml:228-259, 639-642).
fn deposit_all(state: &BankFrameState) -> Element {
    let (x, y) = ((FRAME_W - 256.0) / 2.0, FRAME_H - 30.0 - 10.0 - 25.0);
    let mut children = panel_button(
        format!("{FRAME_NAME}DepositAllButton"),
        &state.deposit_all_label,
        ACTION_AUTO_DEPOSIT,
        true,
        (x, y, 256.0, 24.0),
    );
    if let Some(checked) = state.include_reagents {
        children.extend(checkbox(
            &format!("{FRAME_NAME}IncludeReagents"),
            "Include tradeable reagents",
            checked,
            ACTION_INCLUDE_REAGENTS,
            (x + 256.0 + 10.0, y),
        ));
    }
    children
}

/// The Bank / Warband Bank `TabSystem` at the frame's BOTTOMLEFT 22,2 (BF.xml:694-701).
fn bank_tabs(account: bool) -> Element {
    let top = FRAME_H - 2.0;
    let bank_w = tab_width("Bank").max(100.0);
    let warband_w = tab_width("Warband Bank").max(100.0);
    let character = tab(
        &format!("{FRAME_NAME}TabSystemTab1"),
        "Bank",
        (22.0, top, bank_w),
        !account,
        ACTION_SHOW_CHARACTER,
    );
    let warband = tab(
        &format!("{FRAME_NAME}TabSystemTab2"),
        "Warband Bank",
        (22.0 + bank_w - 16.0, top, warband_w),
        account,
        ACTION_SHOW_ACCOUNT,
    );
    if account {
        character.into_iter().chain(warband).collect()
    } else {
        warband.into_iter().chain(character).collect()
    }
}

/// Deposit assignments of the settings menu (BF.xml:93-139, `Enum.BagSlotFlags`).
pub const SETTINGS_FLAGS: [(&str, u32); 5] = [
    ("Equipment", 0x2),
    ("Consumables", 0x4),
    ("Profession Goods", 0x8),
    ("Reagents", 0x80),
    ("Junk", 0x10),
];

fn prompt_frame(prompt: &BankPromptView) -> Element {
    match prompt {
        BankPromptView::Money { deposit } => money_prompt(
            &MoneyPrompt {
                name: "BankMoneyPopup",
                // BANK_MONEY_DEPOSIT_PROMPT / BANK_MONEY_WITHDRAW_PROMPT.
                text: if *deposit {
                    "Amount to deposit:"
                } else {
                    "Amount to withdraw:"
                },
                boxes: MONEY_BOXES,
                accept_action: ACTION_MONEY_ACCEPT,
                cancel_action: ACTION_MONEY_CANCEL,
            },
            ((FRAME_W - 320.0) / 2.0, 31.0),
        ),
        BankPromptView::TabSettings { flags, name_prompt } => tab_settings(*flags, name_prompt),
    }
}

/// `BankPanelTabSettingsMenuTemplate` at the panel's TOPRIGHT +40,+5 (BF.xml:659-662):
/// the tab name box and the deposit assignment checkboxes. The icon picker and the
/// expansion filter are not built.
fn tab_settings(flags: u32, name_prompt: &str) -> Element {
    let name = format!("{FRAME_NAME}TabSettings");
    let (width, height) = (300.0, 250.0);
    let mut children = label(
        format!("{name}NamePrompt"),
        name_prompt,
        (16.0, 18.0, 268.0, 14.0),
        (12.0, NORMAL_FONT_COLOR, "LEFT"),
    );
    children.extend(edit_box(TAB_NAME_BOX, (22.0, 38.0, 180.0, 20.0)));
    children.extend(label(
        format!("{name}AssignHeader"),
        "Assign to tab:",
        (16.0, 70.0, 268.0, 14.0),
        (12.0, NORMAL_FONT_COLOR, "LEFT"),
    ));
    for (index, (text, bit)) in SETTINGS_FLAGS.iter().enumerate() {
        children.extend(checkbox(
            &format!("{name}Assign{index}"),
            text,
            flags & bit != 0,
            &format!("{ACTION_SETTINGS_FLAG_PREFIX}{bit}"),
            (16.0, 90.0 + index as f32 * 24.0),
        ));
    }
    children.extend(panel_button(
        format!("{name}Okay"),
        "Okay",
        ACTION_SETTINGS_ACCEPT,
        true,
        (30.0, height - 34.0, 110.0, 22.0),
    ));
    children.extend(panel_button(
        format!("{name}Cancel"),
        "Cancel",
        ACTION_SETTINGS_CANCEL,
        true,
        (160.0, height - 34.0, 110.0, 22.0),
    ));
    rsx! {
        r#frame {
            name: {DynName(name)},
            width,
            height,
            style: crate::ui::screens::static_popup_component::STATIC_POPUP_PANEL_STYLE,
            strata: FrameStrata::Dialog,
            frame_level: 200.0,
            pos_type: "absolute",
            left: {FRAME_W + 40.0},
            top: -5.0,
            {children}
        }
    }
}
