//! Retail quest giver frame (`QuestFrame`, `ButtonFrameTemplate` 338×496): greeting with
//! gossip options and the giver's quests, then the detail (Accept/Decline), progress
//! (Continue/Cancel) and reward (Complete Quest) panels on quest parchment.

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

use crate::ui::screens::quest_art::{
    DynName, GOSSIP_ACTIVE_ICON, GOSSIP_AVAILABLE_ICON, GOSSIP_IN_PROGRESS_ICON,
    HIGHLIGHT_FONT_COLOR, QUEST_PARCHMENT, QUEST_SMALL_HEADER_COLOR, QUEST_TEXT_COLOR,
    atlas_texture, named_atlas_texture, panel_button, window_chrome, wrapped_text_height,
};
use crate::ui::strata::FrameStrata;

pub const QUEST_FRAME: &str = "QuestFrame";
pub const FRAME_W: f32 = 338.0;
pub const FRAME_H: f32 = 496.0;
/// `QuestFramePanelTemplate` Bg: `QuestBG-Parchment` at (7, -62).
const PARCHMENT_X: f32 = 7.0;
const PARCHMENT_Y: f32 = 62.0;
const PARCHMENT_W: f32 = 299.0;
const PARCHMENT_H: f32 = 407.0;
/// Scroll child content inset (`QuestProgressTitleText` at (10, -10)), 285 wide.
const CONTENT_X: f32 = PARCHMENT_X + 10.0;
const CONTENT_TOP: f32 = PARCHMENT_Y + 10.0;
const CONTENT_W: f32 = 280.0;
/// `QuestTitleFont` (Morpheus 18 in Retail) and `QuestFont` (13).
const TITLE_FONT: f32 = 18.0;
const BODY_FONT: f32 = 13.0;
const SMALL_FONT: f32 = 12.0;
const TITLE_GAP: f32 = 5.0;
const SECTION_GAP: f32 = 10.0;
/// `LargeItemButtonTemplate` 147×41, two per row.
const ITEM_W: f32 = 147.0;
const ITEM_H: f32 = 41.0;
const ITEM_GAP: f32 = 2.0;
const ICON: f32 = 39.0;
const LIST_ROW_H: f32 = 18.0;
/// `Interface\QuestFrame\UI-QuestItemNameFrame`, `UI-QuestItemHighlight`.
const ITEM_NAME_FRAME: u32 = 136_796;
const ITEM_HIGHLIGHT: u32 = 136_795;
/// `Interface\Icons\INV_Misc_QuestionMark`: Retail icon for an item without data.
const UNKNOWN_ITEM_ICON: u32 = 134_400;
/// `Interface\GossipFrame\GossipGossipIcon`.
const GOSSIP_CHAT_ICON: u32 = 132_053;
/// `Interface\QuestFrame\UI-HorizontalBreak` (256×32 on the greeting).
const HORIZONTAL_BREAK: u32 = 136_783;

pub const CLOSE_ACTION: &str = "quest_frame:close";
pub const ACCEPT_ACTION: &str = "quest_frame:accept";
pub const DECLINE_ACTION: &str = "quest_frame:decline";
pub const CONTINUE_ACTION: &str = "quest_frame:continue";
pub const COMPLETE_ACTION: &str = "quest_frame:complete";
pub const QUEST_ACTION_PREFIX: &str = "quest_frame:quest:";
pub const GOSSIP_ACTION_PREFIX: &str = "quest_frame:gossip:";
pub const CHOICE_ACTION_PREFIX: &str = "quest_frame:choice:";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewardItemView {
    pub name: String,
    pub count: u32,
    pub icon_fdid: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RewardView {
    /// Copper; shown as Retail money text.
    pub money: u32,
    pub items: Vec<RewardItemView>,
    pub choices: Vec<RewardItemView>,
    pub selected_choice: Option<usize>,
}

impl RewardView {
    pub fn is_empty(&self) -> bool {
        self.money == 0 && self.items.is_empty() && self.choices.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GreetingQuestKind {
    Available,
    Complete,
    Incomplete,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GreetingQuest {
    /// Index into the giver's quest list (the click action).
    pub index: usize,
    pub title: String,
    pub kind: GreetingQuestKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GossipOptionView {
    pub option_id: u32,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuestFramePage {
    Greeting {
        text: String,
        options: Vec<GossipOptionView>,
        quests: Vec<GreetingQuest>,
    },
    Detail {
        title: String,
        description: String,
        objectives_text: String,
        rewards: RewardView,
    },
    Progress {
        title: String,
        text: String,
        required: Vec<RewardItemView>,
        can_complete: bool,
    },
    Reward {
        title: String,
        text: String,
        rewards: RewardView,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestFrameState {
    pub visible: bool,
    pub npc_name: String,
    pub page: QuestFramePage,
}

impl Default for QuestFrameState {
    fn default() -> Self {
        Self {
            visible: false,
            npc_name: String::new(),
            page: QuestFramePage::Greeting {
                text: String::new(),
                options: Vec::new(),
                quests: Vec::new(),
            },
        }
    }
}

pub fn quest_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<QuestFrameState>()
        .expect("QuestFrameState must be in SharedContext");
    let hide = !state.visible;
    let chrome = window_chrome(
        QUEST_FRAME,
        (FRAME_W, FRAME_H),
        &state.npc_name,
        CLOSE_ACTION,
    );
    let parchment = named_atlas_texture(
        "QuestFrameParchment".into(),
        QUEST_PARCHMENT,
        (PARCHMENT_X, PARCHMENT_Y, PARCHMENT_W, PARCHMENT_H),
    );
    let page = page_elements(&state.page);
    rsx! {
        r#frame {
            name: {DynName(QUEST_FRAME.into())},
            width: FRAME_W,
            height: FRAME_H,
            strata: FrameStrata::Dialog,
            hidden: hide,
            mouse_enabled: true,
            pos_type: "absolute",
            left: 16.0,
            top: 104.0,
            {chrome}
            {parchment}
            {page}
        }
    }
}

fn page_elements(page: &QuestFramePage) -> Element {
    match page {
        QuestFramePage::Greeting {
            text,
            options,
            quests,
        } => greeting_panel(text, options, quests),
        QuestFramePage::Detail {
            title,
            description,
            objectives_text,
            rewards,
        } => detail_panel(title, description, objectives_text, rewards),
        QuestFramePage::Progress {
            title,
            text,
            required,
            can_complete,
        } => progress_panel(title, text, required, *can_complete),
        QuestFramePage::Reward {
            title,
            text,
            rewards,
        } => reward_panel(title, text, rewards),
    }
}

/// Bottom buttons: `BOTTOMLEFT (6, 4)` and `BOTTOMRIGHT (-6, 4)`, 22 high.
fn left_button(name: &str, text: &str, action: &str, enabled: bool, width: f32) -> Element {
    panel_button(
        name.into(),
        text,
        action,
        enabled,
        (6.0, FRAME_H - 4.0 - 22.0, width, 22.0),
    )
}

fn right_button(name: &str, text: &str, action: &str) -> Element {
    panel_button(
        name.into(),
        text,
        action,
        true,
        (FRAME_W - 6.0 - 78.0, FRAME_H - 4.0 - 22.0, 78.0, 22.0),
    )
}

/// Left edge and width of a text column.
#[derive(Clone, Copy)]
pub struct Column {
    pub x: f32,
    pub width: f32,
}

const CONTENT: Column = Column {
    x: CONTENT_X,
    width: CONTENT_W,
};

fn text_block(
    name: String,
    text: &str,
    font_size: f32,
    color: &str,
    column: Column,
    y: &mut f32,
) -> Element {
    let top = *y;
    // The block is as tall as its wrapped lines, so a multi-line text is not squeezed
    // into one line's height.
    let height = wrapped_text_height(text, column.width, font_size);
    *y += height;
    rsx! {
        fontstring {
            name: {DynName(name)},
            width: {column.width},
            height,
            text,
            font: GameFont::FrizQuadrata,
            font_size,
            font_color: color,
            justify_h: "LEFT",
            pos_type: "absolute",
            left: {column.x},
            top,
        }
    }
}

fn greeting_panel(text: &str, options: &[GossipOptionView], quests: &[GreetingQuest]) -> Element {
    let mut y = CONTENT_TOP;
    let mut elements = text_block(
        "GreetingText".into(),
        text,
        BODY_FONT,
        QUEST_TEXT_COLOR,
        CONTENT,
        &mut y,
    );
    let current: Vec<&GreetingQuest> = quests
        .iter()
        .filter(|quest| quest.kind != GreetingQuestKind::Available)
        .collect();
    let available: Vec<&GreetingQuest> = quests
        .iter()
        .filter(|quest| quest.kind == GreetingQuestKind::Available)
        .collect();
    for (header, name, list) in [
        ("Current Quests", "CurrentQuestsText", current),
        ("Available Quests", "AvailableQuestsText", available),
    ] {
        if list.is_empty() {
            continue;
        }
        y += SECTION_GAP;
        elements.extend(text_block(
            name.into(),
            header,
            TITLE_FONT,
            QUEST_TEXT_COLOR,
            CONTENT,
            &mut y,
        ));
        for quest in list {
            elements.extend(greeting_quest_row(quest, &mut y));
        }
    }
    if !options.is_empty() {
        y += SECTION_GAP;
        elements.extend(rsx! {
            texture {
                name: "QuestGreetingFrameHorizontalBreak",
                width: 256.0,
                height: 32.0,
                texture_fdid: HORIZONTAL_BREAK,
                pos_type: "absolute",
                left: {CONTENT_X + 12.0},
                top: {y - 12.0},
            }
        });
        y += SECTION_GAP;
        for option in options {
            elements.extend(gossip_option_row(option, &mut y));
        }
    }
    elements.extend(right_button(
        "QuestFrameGreetingGoodbyeButton",
        "Goodbye",
        CLOSE_ACTION,
    ));
    elements
}

fn greeting_quest_row(quest: &GreetingQuest, y: &mut f32) -> Element {
    let name = format!("QuestTitleButton{}", quest.index + 1);
    let action = format!("{QUEST_ACTION_PREFIX}{}", quest.index);
    let icon = match quest.kind {
        GreetingQuestKind::Available => {
            plain_icon(format!("{name}Icon"), GOSSIP_AVAILABLE_ICON, *y)
        }
        GreetingQuestKind::Complete => plain_icon(format!("{name}Icon"), GOSSIP_ACTIVE_ICON, *y),
        GreetingQuestKind::Incomplete => atlas_texture(
            format!("{name}Icon"),
            &GOSSIP_IN_PROGRESS_ICON,
            (CONTENT_X, *y, 16.0, 18.0),
        ),
    };
    let row = list_row(name, &quest.title, &action, *y);
    *y += LIST_ROW_H;
    icon.into_iter().chain(row).collect()
}

fn gossip_option_row(option: &GossipOptionView, y: &mut f32) -> Element {
    let name = format!("GossipOption{}", option.option_id);
    let action = format!("{GOSSIP_ACTION_PREFIX}{}", option.option_id);
    let icon = plain_icon(format!("{name}Icon"), GOSSIP_CHAT_ICON, *y);
    let row = list_row(name, &option.text, &action, *y);
    *y += LIST_ROW_H;
    icon.into_iter().chain(row).collect()
}

fn plain_icon(name: String, fdid: u32, top: f32) -> Element {
    rsx! {
        texture {
            name: {DynName(name)},
            width: 16.0,
            height: 16.0,
            texture_fdid: fdid,
            pos_type: "absolute",
            left: CONTENT_X,
            top,
        }
    }
}

fn list_row(name: String, text: &str, action: &str, top: f32) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("{name}Text"))},
            width: {CONTENT_W - 20.0},
            height: LIST_ROW_H,
            text,
            font: GameFont::FrizQuadrata,
            font_size: BODY_FONT,
            font_color: QUEST_TEXT_COLOR,
            justify_h: "LEFT",
            onclick: action,
            pos_type: "absolute",
            left: {CONTENT_X + 20.0},
            top: {top + 1.0},
        }
    }
}

fn detail_panel(
    title: &str,
    description: &str,
    objectives_text: &str,
    rewards: &RewardView,
) -> Element {
    let mut y = CONTENT_TOP;
    let mut elements = text_block(
        "QuestInfoTitleHeader".into(),
        title,
        TITLE_FONT,
        QUEST_TEXT_COLOR,
        CONTENT,
        &mut y,
    );
    y += TITLE_GAP;
    elements.extend(text_block(
        "QuestInfoDescriptionText".into(),
        description,
        BODY_FONT,
        QUEST_TEXT_COLOR,
        CONTENT,
        &mut y,
    ));
    y += SECTION_GAP;
    elements.extend(text_block(
        "QuestInfoObjectivesHeader".into(),
        "Quest Objectives",
        TITLE_FONT,
        QUEST_TEXT_COLOR,
        CONTENT,
        &mut y,
    ));
    y += TITLE_GAP;
    elements.extend(text_block(
        "QuestInfoObjectivesText".into(),
        objectives_text,
        BODY_FONT,
        QUEST_TEXT_COLOR,
        CONTENT,
        &mut y,
    ));
    elements.extend(rewards_section(rewards, false, CONTENT, &mut y));
    elements.extend(left_button(
        "QuestFrameAcceptButton",
        "Accept",
        ACCEPT_ACTION,
        true,
        77.0,
    ));
    elements.extend(right_button(
        "QuestFrameDeclineButton",
        "Decline",
        DECLINE_ACTION,
    ));
    elements
}

fn progress_panel(
    title: &str,
    text: &str,
    required: &[RewardItemView],
    can_complete: bool,
) -> Element {
    let mut y = CONTENT_TOP;
    let mut elements = text_block(
        "QuestProgressTitleText".into(),
        title,
        TITLE_FONT,
        QUEST_TEXT_COLOR,
        CONTENT,
        &mut y,
    );
    y += TITLE_GAP;
    elements.extend(text_block(
        "QuestProgressText".into(),
        text,
        BODY_FONT,
        QUEST_TEXT_COLOR,
        CONTENT,
        &mut y,
    ));
    if !required.is_empty() {
        y += SECTION_GAP;
        elements.extend(text_block(
            "QuestProgressRequiredItemsText".into(),
            "Required items:",
            TITLE_FONT,
            QUEST_TEXT_COLOR,
            CONTENT,
            &mut y,
        ));
        y += TITLE_GAP;
        elements.extend(item_grid(
            "QuestProgressItem",
            required,
            None,
            false,
            CONTENT,
            &mut y,
        ));
    }
    elements.extend(left_button(
        "QuestFrameCompleteButton",
        "Continue",
        CONTINUE_ACTION,
        can_complete,
        120.0,
    ));
    elements.extend(right_button(
        "QuestFrameGoodbyeButton",
        "Cancel",
        CLOSE_ACTION,
    ));
    elements
}

fn reward_panel(title: &str, text: &str, rewards: &RewardView) -> Element {
    let mut y = CONTENT_TOP;
    let mut elements = text_block(
        "QuestInfoTitleHeader".into(),
        title,
        TITLE_FONT,
        QUEST_TEXT_COLOR,
        CONTENT,
        &mut y,
    );
    y += TITLE_GAP;
    elements.extend(text_block(
        "QuestInfoRewardText".into(),
        text,
        BODY_FONT,
        QUEST_TEXT_COLOR,
        CONTENT,
        &mut y,
    ));
    elements.extend(rewards_section(rewards, true, CONTENT, &mut y));
    // Always enabled; QuestRewardCompleteButton_OnClick reports a missing choice.
    elements.extend(left_button(
        "QuestFrameCompleteQuestButton",
        "Complete Quest",
        COMPLETE_ACTION,
        true,
        120.0,
    ));
    elements
}

/// `QuestInfoRewardsFrame`: "Rewards" header, `REWARD_CHOOSE` choices, then fixed items
/// (`REWARD_ITEMS` / `REWARD_ITEMS_ONLY`) and money.
pub fn rewards_section(
    rewards: &RewardView,
    choosable: bool,
    column: Column,
    y: &mut f32,
) -> Element {
    if rewards.is_empty() {
        return Vec::new();
    }
    *y += SECTION_GAP;
    let mut elements = text_block(
        "QuestInfoRewardsFrameHeader".into(),
        "Rewards",
        TITLE_FONT,
        QUEST_TEXT_COLOR,
        column,
        y,
    );
    let mut index = 0;
    if !rewards.choices.is_empty() {
        *y += TITLE_GAP;
        elements.extend(text_block(
            "QuestInfoRewardsFrameItemChooseText".into(),
            "Choose your reward:",
            SMALL_FONT,
            QUEST_SMALL_HEADER_COLOR,
            column,
            y,
        ));
        *y += TITLE_GAP;
        elements.extend(item_grid(
            "QuestInfoRewardsFrameQuestInfoItem",
            &rewards.choices,
            rewards.selected_choice,
            choosable,
            column,
            y,
        ));
        index = rewards.choices.len();
    }
    if !rewards.items.is_empty() {
        let label = if rewards.choices.is_empty() {
            "You will receive:"
        } else {
            "You will also receive:"
        };
        *y += TITLE_GAP;
        elements.extend(text_block(
            "QuestInfoRewardsFrameItemReceiveText".into(),
            label,
            SMALL_FONT,
            QUEST_SMALL_HEADER_COLOR,
            column,
            y,
        ));
        *y += TITLE_GAP;
        elements.extend(item_grid_from(
            "QuestInfoRewardsFrameQuestInfoItem",
            &rewards.items,
            index,
            (None, false),
            column,
            y,
        ));
    }
    if rewards.money > 0 {
        *y += TITLE_GAP;
        elements.extend(text_block(
            "QuestInfoMoneyText".into(),
            &format!("Money: {}", crate::quest_runtime::money_text(rewards.money)),
            SMALL_FONT,
            QUEST_SMALL_HEADER_COLOR,
            column,
            y,
        ));
    }
    elements
}

fn item_grid(
    prefix: &str,
    items: &[RewardItemView],
    selected: Option<usize>,
    choosable: bool,
    column: Column,
    y: &mut f32,
) -> Element {
    item_grid_from(prefix, items, 0, (selected, choosable), column, y)
}

/// Item buttons two per row; `first` numbers them after earlier buttons (Retail
/// `QuestInfoItem1..n` covers choices then fixed items).
fn item_grid_from(
    prefix: &str,
    items: &[RewardItemView],
    first: usize,
    (selected, choosable): (Option<usize>, bool),
    column: Column,
    y: &mut f32,
) -> Element {
    let top = *y;
    let mut elements = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let x = column.x + (i % 2) as f32 * (ITEM_W + ITEM_GAP);
        let row_y = top + (i / 2) as f32 * (ITEM_H + ITEM_GAP);
        let name = format!("{prefix}{}", first + i + 1);
        let action = choosable.then(|| format!("{CHOICE_ACTION_PREFIX}{i}"));
        elements.extend(item_button(
            &name,
            item,
            selected == Some(i),
            action,
            (x, row_y),
        ));
    }
    *y = top + items.len().div_ceil(2) as f32 * (ITEM_H + ITEM_GAP);
    elements
}

/// `LargeItemButtonTemplate`: 39×39 icon, name frame at icon RIGHT (-10), name 90×36.
fn item_button(
    name: &str,
    item: &RewardItemView,
    selected: bool,
    action: Option<String>,
    (x, y): (f32, f32),
) -> Element {
    let label = if item.count > 1 {
        format!("{} x{}", item.name, item.count)
    } else {
        item.name.clone()
    };
    let icon = item.icon_fdid.unwrap_or(UNKNOWN_ITEM_ICON);
    let mut elements = Vec::new();
    if selected {
        elements.extend(rsx! {
            texture {
                name: {DynName(format!("{name}Highlight"))},
                width: 256.0,
                height: 64.0,
                texture_fdid: ITEM_HIGHLIGHT,
                pos_type: "absolute",
                left: {x - 8.0},
                top: {y - 11.0},
            }
        });
    }
    elements.extend(rsx! {
        texture {
            name: {DynName(format!("{name}NameFrame"))},
            width: 128.0,
            height: 64.0,
            texture_fdid: ITEM_NAME_FRAME,
            pos_type: "absolute",
            left: {x + ICON - 10.0},
            top: {y + (ITEM_H - 64.0) / 2.0},
        }
        texture {
            name: {DynName(format!("{name}IconTexture"))},
            width: ICON,
            height: ICON,
            texture_fdid: icon,
            pos_type: "absolute",
            left: x,
            top: y,
        }
        fontstring {
            name: {DynName(format!("{name}Name"))},
            width: 90.0,
            height: 36.0,
            text: {label.as_str()},
            font: GameFont::FrizQuadrata,
            font_size: SMALL_FONT,
            font_color: HIGHLIGHT_FONT_COLOR,
            justify_h: "LEFT",
            pos_type: "absolute",
            left: {x + ICON - 10.0 + 15.0},
            top: {y + (ITEM_H - 36.0) / 2.0},
        }
    });
    if let Some(action) = action {
        elements.extend(rsx! {
            r#frame {
                name: {DynName(name.to_string())},
                width: ITEM_W,
                height: ITEM_H,
                onclick: {action.as_str()},
                pos_type: "absolute",
                left: x,
                top: y,
            }
        });
    }
    elements
}
