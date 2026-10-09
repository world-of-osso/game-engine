//! Retail `MailFrame` (Blizzard_MailFrame/MailFrame.xml / .lua, cited as MF.xml /
//! MF.lua): the `ButtonFrameTemplate` mail window with the Inbox tab (seven
//! `MailItemTemplate` rows, paging, Open All) and the Send Mail tab (To, Subject,
//! the letter, postage, twelve attachment buttons, Send Money / C.O.D. and Send /
//! Cancel), plus `OpenMailFrame` to its right for the open mail (sender, subject,
//! letter, Take Attachments, Reply / Delete or Return / Close). The Inbox and Send
//! frames are 384×512 anchored TOPLEFT, so their bottom anchors use that height.

use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::{GameFont, JustifyV};

use crate::ui::screens::auction_house_frame_component::inset_border;
use crate::ui::screens::bank_art::{
    HIGHLIGHT_FONT_COLOR, ITEM_BUTTON, MoneyBoxNames, RED_FONT_COLOR, SlotItem, WHITE, cropped,
    edit_box, item_slot, label, money_display, money_input, texture,
};
use crate::ui::screens::merchant_frame_component::{tab, tab_width};
use crate::ui::screens::quest_art::{DynName, NORMAL_FONT_COLOR, panel_button, window_chrome};
use crate::ui::strata::FrameStrata;

pub const FRAME_NAME: &str = "MailFrame";
pub const OPEN_MAIL_NAME: &str = "OpenMailFrame";
/// MF.lua:21 `SetPortraitToAsset("Interface\\\\MailFrame\\\\Mail-Icon")`.
pub const MAIL_PORTRAIT_FDID: u32 = 136_382;
/// MF.lua:298 stationery icon; current normal-mail contract uses INV_Misc_Note_01.
pub const OPEN_MAIL_PORTRAIT_FDID: u32 = 134_327;
/// `ButtonFrameTemplate` default size (SharedUIPanelTemplates.xml:548).
pub const FRAME_W: f32 = 338.0;
pub const FRAME_H: f32 = 424.0;
/// `InboxFrame` / `SendMailFrame` size; `OpenMailFrame` sits at their TOPRIGHT.
const TAB_FRAME_W: f32 = 384.0;
const TAB_FRAME_H: f32 = 512.0;

pub const ACTION_CLOSE: &str = "mail_close";
pub const ACTION_TAB_INBOX: &str = "mail_tab_inbox";
pub const ACTION_TAB_SEND: &str = "mail_tab_send";
/// `mail_open:<mail_id>`.
pub const ACTION_OPEN_PREFIX: &str = "mail_open:";
pub const ACTION_PREV: &str = "mail_prev_page";
pub const ACTION_NEXT: &str = "mail_next_page";
pub const ACTION_OPEN_ALL: &str = "mail_open_all";
pub const ACTION_SEND: &str = "mail_send";
pub const ACTION_SEND_CANCEL: &str = "mail_send_cancel";
/// `mail_attachment:<0-based button>`: clicking an attached item takes it off.
pub const ACTION_ATTACHMENT_PREFIX: &str = "mail_attachment:";
pub const ACTION_MODE_MONEY: &str = "mail_mode_money";
pub const ACTION_MODE_COD: &str = "mail_mode_cod";
pub const ACTION_OPEN_CLOSE: &str = "mail_open_close";
pub const ACTION_TAKE_MONEY: &str = "mail_take_money";
/// `mail_take_item:<attachment slot>`.
pub const ACTION_TAKE_ITEM_PREFIX: &str = "mail_take_item:";
pub const ACTION_REPLY: &str = "mail_reply";
pub const ACTION_DELETE: &str = "mail_delete";

/// `SendMailNameEditBox` letters 77, `SendMailSubjectEditBox` 64,
/// `SendMailBodyEditBox` 500 (MF.xml:527, 562, 650).
pub const TO_BOX: &str = "SendMailNameEditBox";
pub const SUBJECT_BOX: &str = "SendMailSubjectEditBox";
pub const BODY_BOX: &str = "SendMailBodyEditBox";

/// RSX lacks vertical justification, EditBoxData's multi_line and max_letters.
pub fn apply_mail_body_postsetup(registry: &mut FrameRegistry) {
    if let Some(id) = registry.get_by_name("OpenMailBodyText")
        && let Some(frame) = registry.get_mut(id)
        && let Some(WidgetData::FontString(body)) = frame.widget_data.as_mut()
    {
        body.justify_v = JustifyV::Top;
    }
    let Some(id) = registry.get_by_name(BODY_BOX) else {
        return;
    };
    let Some(frame) = registry.get_mut(id) else {
        return;
    };
    if let Some(WidgetData::EditBox(edit)) = frame.widget_data.as_mut() {
        edit.multi_line = true;
        edit.max_letters = Some(500);
    }
}
pub const MONEY_BOXES: MoneyBoxNames = MoneyBoxNames {
    gold: "SendMailMoneyGold",
    silver: "SendMailMoneySilver",
    copper: "SendMailMoneyCopper",
};

/// `ATTACHMENTS_MAX_SEND`, `ATTACHMENTS_PER_ROW_SEND` (MF.lua:4-5).
pub const SEND_ATTACHMENTS: usize = 12;
const PER_ROW: usize = 7;

/// `Interface\MailFrame\UI-MailFrameBG` (MF.xml:295).
const INBOX_BG: u32 = 530_419;
/// `Interface\MailFrame\MailItemBorder` (MF.xml:15-26).
const ITEM_BORDER: u32 = 136_383;
/// `Interface\Buttons\UI-EmptySlot-White` (MF.xml:78).
const EMPTY_SLOT_WHITE: u32 = 130_765;
/// `Interface\Buttons\UI-Slot-Background` (MF.xml:177).
const SLOT_BACKGROUND: u32 = 130_862;
/// `Interface\Buttons\UI-SpellbookIcon-PrevPage-*` / `NextPage-*` (MF.xml:401-428).
const PREV_UP: u32 = 130_869;
const PREV_DISABLED: u32 = 130_867;
const NEXT_UP: u32 = 130_866;
const NEXT_DISABLED: u32 = 130_864;
/// `Interface\Stationery\stationerytest1` / `2` (MF.lua:1068-1069).
const STATIONERY_LEFT: u32 = 136_859;
const STATIONERY_RIGHT: u32 = 136_860;
/// `Interface\ClassTrainerFrame\UI-ClassTrainer-HorizontalBar` (MF.xml:463).
const HORIZONTAL_BAR: u32 = 130_968;
/// `Interface\Buttons\UI-RadioButton` (`UIRadioButtonTemplate`).
const RADIO: u32 = 130_843;
/// `Interface\Icons\INV_Misc_Coin_01`, `OpenMailMoneyButton`'s icon.
const COIN_ICON: u32 = 133_784;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MailFrameTab {
    #[default]
    Inbox,
    Send,
}

/// One `MailItemTemplate` row.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct InboxRow {
    pub mail_id: u64,
    pub sender: String,
    pub subject: String,
    /// The package icon (first attachment) or the stationery icon.
    pub icon_fdid: u32,
    /// First attachment's stack size (`SetItemButtonCount`).
    pub count: u32,
    pub read: bool,
    pub cod: bool,
    /// `DAYS_ABBR` in green, or the time left in red under a day.
    pub expires: String,
    pub expires_soon: bool,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct SendView {
    /// `SEND_ATTACHMENTS` buttons.
    pub attachments: Vec<Option<SlotItem>>,
    pub postage: u64,
    /// `GetSendMailPrice() > GetMoney()` turns the postage red.
    pub postage_unaffordable: bool,
    pub cod: bool,
    /// C.O.D. needs an attachment (MF.lua:1013).
    pub cod_enabled: bool,
    /// `SendMailFrame_CanSend`: a recipient, a subject and a C.O.D. within
    /// `MAX_COD_AMOUNT` (MF.lua:1106-1135).
    pub can_send: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct OpenAttachment {
    pub slot: u8,
    pub item: SlotItem,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct OpenMailView {
    pub sender: String,
    pub subject: String,
    pub body: String,
    pub money: u64,
    pub cod: u64,
    pub attachments: Vec<OpenAttachment>,
    pub can_reply: bool,
    /// `InboxItemCanDelete`: Delete, otherwise Return.
    pub can_delete: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct MailFrameState {
    pub visible: bool,
    pub tab: MailFrameTab,
    pub rows: Vec<InboxRow>,
    pub page: usize,
    pub page_count: usize,
    pub send: SendView,
    pub open: Option<OpenMailView>,
    pub money: u64,
    /// A mail request is in flight (`C_Mail.IsCommandPending`): mail buttons wait.
    pub busy: bool,
    /// `OpenAllMailMixin` is taking attachments: "Opening..." and disabled.
    pub opening_all: bool,
}

pub fn mail_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<MailFrameState>()
        .expect("MailFrameState must be in SharedContext");
    build_mail_frame(state)
}

fn build_mail_frame(state: &MailFrameState) -> Element {
    let busy = state.busy;
    let hide = !state.visible;
    let title = match state.tab {
        MailFrameTab::Inbox => "Inbox",
        MailFrameTab::Send => "Send Mail",
    };
    let mut children = window_chrome(FRAME_NAME, (FRAME_W, FRAME_H), title, ACTION_CLOSE);
    children.extend(crate::quest_art::window_portrait_texture(
        &crate::quest_art::window_portrait_slot("MailFramePortrait"),
        MAIL_PORTRAIT_FDID,
    ));
    match state.tab {
        MailFrameTab::Inbox => children.extend(inbox(state, busy)),
        MailFrameTab::Send => children.extend(send_mail(state, busy)),
    }
    children.extend(tabs(state.tab));
    if let Some(open) = &state.open {
        children.extend(open_mail(open, busy));
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

/// `MailFrameTab1` BOTTOMLEFT 14,-30, Tab2 at its RIGHT -8 (MF.xml:840-852).
fn tabs(shown: MailFrameTab) -> Element {
    let inbox_w = tab_width("Inbox");
    let send_w = tab_width("Send Mail");
    let top = FRAME_H + 30.0 - 32.0;
    let inbox = tab(
        "MailFrameTab1",
        "Inbox",
        (14.0, top, inbox_w),
        shown == MailFrameTab::Inbox,
        ACTION_TAB_INBOX,
    );
    let send = tab(
        "MailFrameTab2",
        "Send Mail",
        (14.0 + inbox_w - 8.0, top, send_w),
        shown == MailFrameTab::Send,
        ACTION_TAB_SEND,
    );
    if shown == MailFrameTab::Send {
        inbox.into_iter().chain(send).collect()
    } else {
        send.into_iter().chain(inbox).collect()
    }
}

// --- Inbox ---

/// MailItem1 at TOPLEFT 13,-70, each 45 below the last (MF.xml:346-378).
pub fn row_position(index: usize) -> (f32, f32) {
    (13.0, 70.0 + index as f32 * 45.0)
}

fn inbox(state: &MailFrameState, busy: bool) -> Element {
    let mut out = texture(
        "InboxFrameBg".into(),
        INBOX_BG,
        (7.0, 62.0, 512.0, 512.0),
        WHITE,
    );
    for (index, row) in state.rows.iter().enumerate() {
        out.extend(inbox_row(index, row, busy));
    }
    // Prev CENTER at BOTTOMLEFT 30,114 and Next at 305,114; Open All CENTER at
    // BOTTOM -21,114 (MF.xml:381-434).
    let center_y = TAB_FRAME_H - 114.0;
    let has_prev = state.page > 0;
    let has_next = state.page + 1 < state.page_count;
    out.extend(page_button(
        "InboxPrevPageButton",
        (30.0, center_y),
        (PREV_UP, PREV_DISABLED),
        has_prev.then_some(ACTION_PREV),
    ));
    out.extend(label(
        "InboxPrevPageButtonText".into(),
        "Prev",
        (47.0, center_y - 7.0, 40.0, 14.0),
        (12.0, NORMAL_FONT_COLOR, "LEFT"),
    ));
    out.extend(page_button(
        "InboxNextPageButton",
        (305.0, center_y),
        (NEXT_UP, NEXT_DISABLED),
        has_next.then_some(ACTION_NEXT),
    ));
    out.extend(label(
        "InboxNextPageButtonText".into(),
        "Next",
        (248.0, center_y - 7.0, 40.0, 14.0),
        (12.0, NORMAL_FONT_COLOR, "RIGHT"),
    ));
    let open_all = if state.opening_all {
        "Opening..."
    } else {
        "Open All"
    };
    out.extend(panel_button(
        "OpenAllMail".into(),
        open_all,
        ACTION_OPEN_ALL,
        !busy && !state.opening_all && !state.rows.is_empty(),
        (
            TAB_FRAME_W / 2.0 - 21.0 - 60.0,
            center_y - 12.0,
            120.0,
            24.0,
        ),
    ));
    out
}

fn page_button(
    name: &str,
    (cx, cy): (f32, f32),
    (up, disabled): (u32, u32),
    action: Option<&str>,
) -> Element {
    let fdid = if action.is_some() { up } else { disabled };
    let art = texture(format!("{name}Normal"), fdid, (0.0, 0.0, 32.0, 32.0), WHITE);
    rsx! {
        r#frame {
            name: {DynName(name.into())},
            width: 32.0,
            height: 32.0,
            onclick: {action.unwrap_or("")},
            mouse_enabled: true,
            pos_type: "absolute",
            left: {cx - 16.0},
            top: {cy - 16.0},
            {art}
        }
    }
}

/// `MailItemTemplate` 305×45: border, sender, subject, time left and the package
/// button (MF.xml:11-170, MF.lua:214-290). Read mail is grey with a grey slot;
/// unread has a gold slot.
fn inbox_row(index: usize, row: &InboxRow, busy: bool) -> Element {
    let name = format!("MailItem{}", index + 1);
    let (x, y) = row_position(index);
    let mut out = cropped(
        format!("{name}BorderLeft"),
        ITEM_BORDER,
        "0.0,0.1640625,0.0,0.75",
        (x, y, 42.0, 48.0),
    );
    out.extend(cropped(
        format!("{name}BorderRight"),
        ITEM_BORDER,
        "0.1640625,1.0,0.0,0.75",
        (x + 305.0 - 263.0, y, 263.0, 48.0),
    ));
    let (sender_color, subject_color, slot_color) = if row.read {
        (
            "0.75,0.75,0.75,1.0",
            "0.75,0.75,0.75,1.0",
            "0.5,0.5,0.5,1.0",
        )
    } else {
        (NORMAL_FONT_COLOR, HIGHLIGHT_FONT_COLOR, "1.0,0.82,0.0,1.0")
    };
    out.extend(label(
        format!("{name}Sender"),
        &row.sender,
        (x + 47.0, y + 4.0, 200.0, 16.0),
        (12.0, sender_color, "LEFT"),
    ));
    out.extend(label(
        format!("{name}Subject"),
        &row.subject,
        (x + 47.0, y + 20.0, 248.0, 18.0),
        (10.0, subject_color, "LEFT"),
    ));
    let expires_color = if row.expires_soon {
        RED_FONT_COLOR
    } else {
        "0.1,1.0,0.1,1.0"
    };
    out.extend(label(
        format!("{name}ExpireTime"),
        &row.expires,
        (x + 305.0 - 4.0 - 100.0, y + 4.0, 100.0, 16.0),
        (10.0, expires_color, "RIGHT"),
    ));
    // `$parentButton` 37×37 at 4,-3 over the 64×64 slot (MF.xml:72-81).
    let mut button_bg = texture(
        format!("{name}ButtonSlot"),
        EMPTY_SLOT_WHITE,
        (-13.5, -13.5, 64.0, 64.0),
        slot_color,
    );
    if row.cod {
        button_bg.extend(label(
            format!("{name}ButtonCOD"),
            "COD",
            (0.0, 2.0, ITEM_BUTTON, 12.0),
            (10.0, HIGHLIGHT_FONT_COLOR, "CENTER"),
        ));
    }
    let item = SlotItem {
        icon_fdid: row.icon_fdid,
        count: row.count,
        quality_border: WHITE.into(),
    };
    out.extend(item_slot(
        &format!("{name}Button"),
        (x + 4.0, y + 3.0),
        button_bg,
        Some(&item),
        &if busy {
            String::new()
        } else {
            format!("{ACTION_OPEN_PREFIX}{}", row.mail_id)
        },
    ));
    if row.selected {
        out.extend(texture(
            format!("{name}ButtonChecked"),
            EMPTY_SLOT_WHITE,
            (x + 4.0 - 13.5, y + 3.0 - 13.5, 64.0, 64.0),
            "1.0,1.0,1.0,0.6",
        ));
    }
    out
}

// --- Send Mail ---

/// Attachment button `index` (0-based): two rows of seven from the bottom anchor
/// math of `SendMailFrame_Update` (MF.lua:1045-1086): indentx 15, tabx 45,
/// indenty 215, taby 44.
pub fn send_attachment_position(index: usize) -> (f32, f32) {
    let row = index / PER_ROW;
    let column = index % PER_ROW;
    let cursory = 1 - row as i32;
    let x = 15.0 + 45.0 * column as f32;
    let y = TAB_FRAME_H - (215.0 + 44.0 * cursory as f32);
    (x, y)
}

fn send_mail(state: &MailFrameState, busy: bool) -> Element {
    let send = &state.send;
    // `SendMailNameEditBox` 109×25 at 90,-30; Subject 220×20 below it (MF.xml:562-686).
    let mut out = label(
        format!("{TO_BOX}Label"),
        "To:",
        (18.0, 36.0, 60.0, 14.0),
        (12.0, NORMAL_FONT_COLOR, "RIGHT"),
    );
    out.extend(edit_box(TO_BOX, (90.0, 32.0, 109.0, 20.0)));
    out.extend(label(
        format!("{SUBJECT_BOX}Label"),
        "Subject:",
        (18.0, 58.0, 60.0, 14.0),
        (12.0, NORMAL_FONT_COLOR, "RIGHT"),
    ));
    out.extend(edit_box(SUBJECT_BOX, (90.0, 55.0, 221.0, 20.0)));
    // `SendMailCostMoneyFrame` TOPRIGHT -50,-34 with "Postage:" to its left.
    let postage_right = TAB_FRAME_W - 50.0;
    out.extend(label(
        "SendMailCostMoneyFrameLabel".into(),
        "Postage:",
        (postage_right - 110.0, 34.0, 60.0, 14.0),
        (12.0, NORMAL_FONT_COLOR, "RIGHT"),
    ));
    out.extend(money_display(
        "SendMailCostMoneyFrame",
        send.postage,
        (postage_right.min(FRAME_W - 8.0), 48.0),
        send.postage_unaffordable,
    ));
    // The letter: stationery 252+64 wide at 8,-83, 154 high with two attachment
    // rows (`scrollHeight = 249 - areay`), the body 270 wide at 20,-10 in it.
    let letter_h = 154.0;
    out.extend(cropped(
        "SendStationeryBackgroundLeft".into(),
        STATIONERY_LEFT,
        &format!("0.0,1.0,0.0,{}", letter_h / 256.0),
        (8.0, 83.0, 252.0, letter_h),
    ));
    out.extend(cropped(
        "SendStationeryBackgroundRight".into(),
        STATIONERY_RIGHT,
        &format!("0.0,1.0,0.0,{}", letter_h / 256.0),
        (260.0, 83.0, 64.0, letter_h),
    ));
    out.extend(rsx! {
        editbox {
            name: {DynName(BODY_BOX.into())},
            width: 270.0,
            height: {letter_h - 20.0},
            font: GameFont::FrizQuadrata,
            font_size: 15.0,
            font_color: "0.18,0.12,0.06,1.0",
            pos_type: "absolute",
            left: 28.0,
            top: 93.0,
        }
    });
    out.extend(horizontal_bar("SendMailHorizontalBarLeft2", 233.0));
    for index in 0..SEND_ATTACHMENTS {
        let item = send.attachments.get(index).and_then(Option::as_ref);
        let (x, y) = send_attachment_position(index);
        let background = texture(
            format!("SendMailAttachment{}Background", index + 1),
            SLOT_BACKGROUND,
            (-1.0, -1.0, 39.0, 39.0),
            WHITE,
        );
        let action = if item.is_some() && !busy {
            format!("{ACTION_ATTACHMENT_PREFIX}{index}")
        } else {
            String::new()
        };
        out.extend(item_slot(
            &format!("SendMailAttachment{}", index + 1),
            (x, y),
            background,
            item,
            &action,
        ));
    }
    out.extend(send_money(send));
    out.extend(horizontal_bar("SendMailHorizontalBarLeft", 337.0));
    // `SendMailCancelButton` 80×22 BOTTOMRIGHT -53,92; Send left of it (MF.xml:781-800).
    let top = TAB_FRAME_H - 92.0 - 22.0;
    let cancel_x = (TAB_FRAME_W - 53.0 - 80.0).min(FRAME_W - 8.0 - 80.0);
    out.extend(panel_button(
        "SendMailMailButton".into(),
        "Send",
        ACTION_SEND,
        send.can_send && !busy,
        (cancel_x - 80.0, top, 80.0, 22.0),
    ));
    out.extend(panel_button(
        "SendMailCancelButton".into(),
        "Cancel",
        ACTION_SEND_CANCEL,
        true,
        (cancel_x, top, 80.0, 22.0),
    ));
    out
}

fn horizontal_bar(name: &str, y: f32) -> Element {
    let mut out = cropped(
        name.into(),
        HORIZONTAL_BAR,
        "0.0,1.0,0.0,0.25",
        (2.0, y, 256.0, 16.0),
    );
    out.extend(cropped(
        format!("{name}Right"),
        HORIZONTAL_BAR,
        "0.0,0.29296875,0.25,0.5",
        (258.0, y, 75.0, 16.0),
    ));
    out
}

/// `SendMailMoneyButton` BOTTOMLEFT 15,125: "Send Money:" / "C.O.D.:" over the
/// money entry, with the Send Money / C.O.D. radio buttons 20 right of it
/// (MF.xml:713-760).
fn send_money(send: &SendView) -> Element {
    let top = TAB_FRAME_H - 125.0 - 37.0;
    let caption = if send.cod { "C.O.D.:" } else { "Send Money:" };
    let mut out = label(
        "SendMailMoneyText".into(),
        caption,
        (15.0, top + 5.0, 120.0, 12.0),
        (10.0, NORMAL_FONT_COLOR, "LEFT"),
    );
    out.extend(money_input(MONEY_BOXES, (20.0, top + 20.0)));
    out.extend(inset_border(
        "SendMailMoneyInset",
        (4.0, TAB_FRAME_H - 115.0, 166.0, 23.0),
    ));
    let radio_x: f32 = 20.0 + 176.0 + 20.0;
    out.extend(radio(
        "SendMailSendMoneyButton",
        "Send Money",
        !send.cod,
        ACTION_MODE_MONEY,
        (radio_x.min(FRAME_W - 110.0), top + 8.0),
    ));
    let cod_action = if send.cod_enabled {
        ACTION_MODE_COD
    } else {
        ""
    };
    out.extend(radio(
        "SendMailCODButton",
        "C.O.D.",
        send.cod,
        cod_action,
        (radio_x.min(FRAME_W - 110.0), top + 25.0),
    ));
    out
}

/// `UIRadioButtonTemplate` 16×16 from `UI-RadioButton` (unchecked 0-.25, checked
/// .25-.5) with its label to the right.
fn radio(name: &str, text: &str, checked: bool, action: &str, (x, y): (f32, f32)) -> Element {
    let coords = if checked {
        "0.25,0.5,0.0,1.0"
    } else {
        "0.0,0.25,0.0,1.0"
    };
    let mut children = cropped(
        format!("{name}Texture"),
        RADIO,
        coords,
        (0.0, 0.0, 16.0, 16.0),
    );
    let color = if action.is_empty() && !checked {
        "0.5,0.5,0.5,1.0"
    } else {
        HIGHLIGHT_FONT_COLOR
    };
    children.extend(label(
        format!("{name}Text"),
        text,
        (20.0, 1.0, 90.0, 14.0),
        (10.0, color, "LEFT"),
    ));
    rsx! {
        r#frame {
            name: {DynName(name.into())},
            width: 16.0,
            height: 16.0,
            onclick: action,
            mouse_enabled: true,
            pos_type: "absolute",
            left: x,
            top: y,
            {children}
        }
    }
}

// --- Open Mail ---

/// `OpenMail_Update` (MF.lua:700-790): the money button and the attachments in
/// rows of seven from the bottom (indentx 16, tabx 45, indenty 31, taby 42).
pub fn open_attachment_position(index: usize, rows: usize) -> (f32, f32) {
    let row = index / PER_ROW;
    let column = index % PER_ROW;
    let cursory = (rows - 1 - row) as f32;
    let x = 16.0 + 45.0 * column as f32;
    let y = FRAME_H - (31.0 + 39.0 + 42.0 * cursory);
    (x, y)
}

fn open_mail(open: &OpenMailView, busy: bool) -> Element {
    let prefix = OPEN_MAIL_NAME;
    let mut children = window_chrome(prefix, (FRAME_W, FRAME_H), "Open Mail", ACTION_OPEN_CLOSE);
    // MailFrame.lua:298 uses the stationery icon (INV_Misc_Note_01 for normal mail).
    children.extend(crate::quest_art::window_portrait_texture(
        &crate::quest_art::window_portrait_slot("OpenMailFramePortrait"),
        OPEN_MAIL_PORTRAIT_FDID,
    ));
    // "From:" / "Subject:" right-aligned at 105 (MF.xml:878-896).
    children.extend(label(
        "OpenMailSenderLabel".into(),
        "From:",
        (25.0, 33.0, 80.0, 16.0),
        (12.0, HIGHLIGHT_FONT_COLOR, "RIGHT"),
    ));
    children.extend(label(
        "OpenMailSenderName".into(),
        &open.sender,
        (110.0, 33.0, 210.0, 16.0),
        (12.0, NORMAL_FONT_COLOR, "LEFT"),
    ));
    children.extend(label(
        "OpenMailSubjectLabel".into(),
        "Subject:",
        (25.0, 55.0, 80.0, 16.0),
        (12.0, HIGHLIGHT_FONT_COLOR, "RIGHT"),
    ));
    children.extend(label(
        "OpenMailSubject".into(),
        &open.subject,
        (110.0, 59.0, 225.0, 14.0),
        (10.0, NORMAL_FONT_COLOR, "LEFT"),
    ));
    let buttons = usize::from(open.money > 0) + open.attachments.len();
    let rows = buttons.div_ceil(PER_ROW).max(1);
    let area_h = 3.0 + 12.0 + 3.0 + 39.0 * rows as f32 + 3.0 * (rows as f32 - 1.0) + 3.0;
    let letter_h = (305.0 - area_h).min(256.0);
    children.extend(cropped(
        "OpenStationeryBackgroundLeft".into(),
        STATIONERY_LEFT,
        &format!("0.0,1.0,0.0,{}", letter_h / 256.0),
        (8.0, 84.0, 252.0, letter_h),
    ));
    children.extend(cropped(
        "OpenStationeryBackgroundRight".into(),
        STATIONERY_RIGHT,
        &format!("0.0,1.0,0.0,{}", letter_h / 256.0),
        (260.0, 84.0, 64.0, letter_h),
    ));
    // MF.xml:989-995: the 276px letter wraps from the stationery's top inset.
    children.extend(label(
        "OpenMailBodyText".into(),
        &open.body,
        (18.0, 94.0, 276.0, letter_h - 20.0),
        (12.0, "0.18,0.12,0.06,1.0", "LEFT"),
    ));
    children.extend(horizontal_bar(
        "OpenMailHorizontalBarLeft",
        FRAME_H - 39.0 - area_h,
    ));
    let text_y = FRAME_H - (31.0 + 39.0 * rows as f32 + 3.0 * (rows as f32 - 1.0) + 3.0 + 12.0);
    let (caption, caption_color) = if buttons > 0 {
        ("Take Attachments:", HIGHLIGHT_FONT_COLOR)
    } else {
        ("No Attachments", "0.5,0.5,0.5,1.0")
    };
    children.extend(label(
        "OpenMailAttachmentText".into(),
        caption,
        (16.0, text_y, 200.0, 12.0),
        (10.0, caption_color, "LEFT"),
    ));
    let mut index = 0;
    if open.money > 0 {
        let (x, y) = open_attachment_position(index, rows);
        let coin = SlotItem {
            icon_fdid: COIN_ICON,
            count: 0,
            quality_border: WHITE.into(),
        };
        children.extend(item_slot(
            "OpenMailMoneyButton",
            (x, y),
            Element::default(),
            Some(&coin),
            if busy { "" } else { ACTION_TAKE_MONEY },
        ));
        index += 1;
    }
    for attachment in &open.attachments {
        let (x, y) = open_attachment_position(index, rows);
        children.extend(item_slot(
            &format!("OpenMailAttachmentButton{}", attachment.slot + 1),
            (x, y),
            Element::default(),
            Some(&attachment.item),
            &if busy {
                String::new()
            } else {
                format!("{ACTION_TAKE_ITEM_PREFIX}{}", attachment.slot)
            },
        ));
        index += 1;
    }
    // Retail shows the charge only in the attachment tooltip, which the native client
    // lacks; it sits right of the caption, above the attachment rows.
    if open.cod > 0 {
        let right = FRAME_W - 16.0;
        children.extend(label(
            "OpenMailCODAmountText".into(),
            "C.O.D.:",
            (right - 150.0, text_y, 60.0, 12.0),
            (10.0, NORMAL_FONT_COLOR, "RIGHT"),
        ));
        children.extend(money_display(
            "OpenMailCODAmount",
            open.cod,
            (right, text_y + 12.0),
            false,
        ));
    }
    // Close 80×22 BOTTOMRIGHT -6,4; Delete / Return 82 and Reply 82 to its left
    // (MF.xml:1289-1310).
    let top = FRAME_H - 4.0 - 22.0;
    let close_x = FRAME_W - 6.0 - 80.0;
    let delete_label = if open.can_delete { "Delete" } else { "Return" };
    children.extend(panel_button(
        "OpenMailReplyButton".into(),
        "Reply",
        ACTION_REPLY,
        !busy && open.can_reply,
        (close_x - 164.0, top, 82.0, 22.0),
    ));
    children.extend(panel_button(
        "OpenMailDeleteButton".into(),
        delete_label,
        ACTION_DELETE,
        !busy,
        (close_x - 82.0, top, 82.0, 22.0),
    ));
    children.extend(panel_button(
        "OpenMailCancelButton".into(),
        "Close",
        ACTION_OPEN_CLOSE,
        true,
        (close_x, top, 80.0, 22.0),
    ));
    rsx! {
        r#frame {
            name: {DynName(prefix.into())},
            width: FRAME_W,
            height: FRAME_H,
            mouse_enabled: true,
            pos_type: "absolute",
            left: TAB_FRAME_W,
            top: 0.0,
            {children}
        }
    }
}
