//! Quest log window (L): quest list grouped by zone header on the left, the selected
//! quest's details on `questlogbackground` parchment on the right, with Abandon and
//! Track/Untrack. Retail `QuestMapFrame` list and details content in a Panel window.

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

use ui_toolkit::atlas::{ActiveSkin, active_skin};

use crate::ui::screens::quest_art::{
    DynName, HIGHLIGHT_FONT_COLOR, NORMAL_FONT_COLOR, POI_IN_PROGRESS, POI_TURN_IN,
    QUEST_LOG_DIVIDER, QUEST_PARCHMENT, QUEST_TEXT_COLOR, TRACKER_CHECK, atlas_texture,
    named_atlas_texture, panel_button, window_chrome, wrapped_text_height,
};
use crate::ui::screens::quest_frame_component::{Column, RewardView, rewards_section};
use crate::ui::strata::FrameStrata;

pub const QUEST_LOG_FRAME: &str = "QuestLogFrame";
pub const FRAME_W: f32 = 640.0;
pub const FRAME_H: f32 = 496.0;
const PANE_TOP: f32 = 62.0;
const PANE_BOTTOM: f32 = FRAME_H - 32.0;
const LIST_X: f32 = 12.0;
const LIST_W: f32 = 300.0;
/// Details pane on `QuestBG-Parchment`, like the quest giver frame.
const DETAILS_X: f32 = 330.0;
const DETAILS_W: f32 = 287.0;
const DETAILS_INSET: f32 = 12.0;
const DETAILS_TEXT_W: f32 = DETAILS_W - 2.0 * DETAILS_INSET;
const HEADER_H: f32 = 26.0;
const ROW_H: f32 = 18.0;
const ROW_FONT: f32 = 12.0;
const TITLE_FONT: f32 = 18.0;
const BODY_FONT: f32 = 13.0;
const GAP: f32 = 6.0;
/// `Interface\QuestFrame\UI-QuestLog-BookIcon` in the portrait ring.
const BOOK_ICON: u32 = 136_797;
/// `QUEST_OBJECTIVE_FONT_COLOR` for finished objective lines on parchment.
const DONE_OBJECTIVE_COLOR: &str = "0.35,0.35,0.35,1.0";

pub const CLOSE_ACTION: &str = "quest_log:close";
pub const ABANDON_ACTION: &str = "quest_log:abandon";
pub const TRACK_ACTION: &str = "quest_log:track";
pub const SELECT_PREFIX: &str = "quest_log:select:";
pub const HEADER_PREFIX: &str = "quest_log:header:";

/// `Enum.RelativeContentDifficulty` of a quest for the player, picked like
/// `DifficultyUtil.GetRelativeDifficultyColor` (Blizzard_FrameXMLUtil/Mainline/
/// DifficultyUtil.lua:33-46) and coloured with `QuestDifficultyColors`
/// (Blizzard_FrameXMLBase/Constants.lua:210-218).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestDifficulty {
    Impossible,
    VeryDifficult,
    Difficult,
    Standard,
    Trivial,
}

/// `UnitQuestTrivialLevelRange`: the server greys quests more than
/// `CONFIG_QUEST_LOW_LEVEL_HIDE_DIFF` (4) levels below the player
/// (game-server quest_rules.rs:13-14, 365), so the log agrees with the `!` markers.
pub const QUEST_TRIVIAL_LEVEL_RANGE: i32 = 4;

impl QuestDifficulty {
    pub fn relative(player_level: i32, quest_level: i32) -> Self {
        let diff = quest_level - player_level;
        if diff >= 5 {
            Self::Impossible
        } else if diff >= 3 {
            Self::VeryDifficult
        } else if diff >= -4 {
            Self::Difficult
        } else if -diff <= QUEST_TRIVIAL_LEVEL_RANGE {
            Self::Standard
        } else {
            Self::Trivial
        }
    }

    pub fn color(self) -> &'static str {
        match self {
            Self::Impossible => "1.0,0.1,0.1,1.0",
            Self::VeryDifficult => "1.0,0.5,0.25,1.0",
            Self::Difficult => "1.0,0.82,0.0,1.0",
            Self::Standard => "0.25,0.75,0.25,1.0",
            Self::Trivial => "0.5,0.5,0.5,1.0",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestLogRow {
    pub quest_id: u32,
    pub title: String,
    /// Displayed quest level (`difficultyLevel`): scaling quests show the player's.
    pub level: i32,
    pub difficulty: QuestDifficulty,
    pub complete: bool,
    pub watched: bool,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestLogGroup {
    pub sort_id: i32,
    pub name: String,
    pub collapsed: bool,
    pub quests: Vec<QuestLogRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestLogObjectiveLine {
    pub text: String,
    pub done: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestLogDetails {
    pub quest_id: u32,
    pub title: String,
    pub objectives_text: String,
    pub objectives: Vec<QuestLogObjectiveLine>,
    /// Story text; known once the quest giver showed it this session.
    pub description: Option<String>,
    pub rewards: Option<RewardView>,
    pub watched: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct QuestLogFrameState {
    pub visible: bool,
    pub quest_count: usize,
    pub max_quests: usize,
    pub groups: Vec<QuestLogGroup>,
    pub details: Option<QuestLogDetails>,
}

/// Where a quest list or details column sits inside its window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuestPane {
    pub x: f32,
    pub y: f32,
    pub width: f32,
}

const LIST_PANE: QuestPane = QuestPane {
    x: LIST_X,
    y: PANE_TOP,
    width: LIST_W,
};

pub fn quest_log_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<QuestLogFrameState>()
        .expect("QuestLogFrameState must be in SharedContext");
    let hide = !state.visible;
    let chrome = window_chrome(
        QUEST_LOG_FRAME,
        (FRAME_W, FRAME_H),
        "Quest Log",
        CLOSE_ACTION,
    );
    let list = quest_list(state, LIST_PANE);
    let details = details_pane(state.details.as_ref());
    let count_text = format!("Quests: {}/{}", state.quest_count, state.max_quests);
    rsx! {
        r#frame {
            name: {DynName(QUEST_LOG_FRAME.into())},
            width: FRAME_W,
            height: FRAME_H,
            strata: FrameStrata::Dialog,
            hidden: hide,
            mouse_enabled: true,
            pos_type: "absolute",
            left: 16.0,
            top: 104.0,
            {chrome}
            texture {
                name: "QuestLogFramePortrait",
                width: 60.0,
                height: 60.0,
                texture_fdid: BOOK_ICON,
                pos_type: "absolute",
                left: -4.0,
                top: -6.0,
            }
            fontstring {
                name: "QuestLogCount",
                width: LIST_W,
                height: 16.0,
                text: {count_text.as_str()},
                font: GameFont::FrizQuadrata,
                font_size: ROW_FONT,
                font_color: HIGHLIGHT_FONT_COLOR,
                justify_h: "RIGHT",
                pos_type: "absolute",
                left: LIST_X,
                top: 36.0,
            }
            {list}
            {details}
        }
    }
}

/// The zone headers and quest titles of `state`, from the top of `pane`.
pub fn quest_list(state: &QuestLogFrameState, pane: QuestPane) -> Element {
    if state.groups.is_empty() {
        return rsx! {
            fontstring {
                name: "QuestLogNoQuestsText",
                width: {pane.width},
                height: 60.0,
                text: "No quests available\n\nAccept quests by talking to characters with a ! above their head.",
                font: GameFont::FrizQuadrata,
                font_size: ROW_FONT,
                font_color: NORMAL_FONT_COLOR,
                justify_h: "CENTER",
                pos_type: "absolute",
                left: {pane.x},
                top: {pane.y + 40.0},
            }
        };
    }
    let mut y = pane.y;
    let mut elements = Vec::new();
    for group in &state.groups {
        elements.extend(group_header(group, pane, y));
        y += HEADER_H;
        if group.collapsed {
            continue;
        }
        for row in &group.quests {
            elements.extend(quest_row(row, pane, y));
            y += ROW_H;
        }
    }
    elements
}

/// Zone header on the `questlog_divider` plate with a +/- collapse marker.
fn group_header(group: &QuestLogGroup, pane: QuestPane, y: f32) -> Element {
    let QuestPane {
        x: list_x,
        width: list_w,
        ..
    } = pane;
    let name = format!("QuestLogHeader{}", group.sort_id);
    let action = format!("{HEADER_PREFIX}{}", group.sort_id);
    let marker = if group.collapsed { "+" } else { "-" };
    let mut elements = named_atlas_texture(
        format!("{name}Background"),
        QUEST_LOG_DIVIDER,
        (list_x, y - 5.0, list_w, 37.0),
    );
    elements.extend(rsx! {
        fontstring {
            name: {DynName(format!("{name}Marker"))},
            width: 14.0,
            height: HEADER_H,
            text: marker,
            font: GameFont::FrizQuadrata,
            font_size: 14.0,
            font_color: NORMAL_FONT_COLOR,
            justify_h: "CENTER",
            pos_type: "absolute",
            left: {list_x + 8.0},
            top: y,
        }
        fontstring {
            name: {DynName(format!("{name}Text"))},
            width: {list_w - 30.0},
            height: HEADER_H,
            text: {group.name.as_str()},
            font: GameFont::FrizQuadrata,
            font_size: 13.0,
            font_color: NORMAL_FONT_COLOR,
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h: "LEFT",
            onclick: {action.as_str()},
            pos_type: "absolute",
            left: {list_x + 26.0},
            top: y,
        }
    });
    elements
}

/// A quest title in its `QuestDifficultyColors` colour (QuestMapFrame.lua:1750-1751);
/// Forever prefixes `[level] ` (Camelot/QuestMapFrameOverrides.lua:13-16, used at
/// Forever Mainline/QuestMapFrame.lua:1631).
fn quest_row(row: &QuestLogRow, pane: QuestPane, y: f32) -> Element {
    let QuestPane {
        x: list_x,
        width: list_w,
        ..
    } = pane;
    let name = format!("QuestLogTitle{}", row.quest_id);
    let action = format!("{SELECT_PREFIX}{}", row.quest_id);
    let icon = if row.complete {
        POI_TURN_IN
    } else {
        POI_IN_PROGRESS
    };
    let color = row.difficulty.color();
    let title = match active_skin() {
        ActiveSkin::Forever => format!("[{}] {}", row.level, row.title),
        ActiveSkin::Modern => row.title.clone(),
    };
    let mut elements = Vec::new();
    if row.selected {
        elements.extend(rsx! {
            r#frame {
                name: {DynName(format!("{name}Selected"))},
                width: list_w,
                height: ROW_H,
                background_color: "1.0,0.82,0.0,0.2",
                pos_type: "absolute",
                left: list_x,
                top: y,
            }
        });
    }
    elements.extend(atlas_texture(
        format!("{name}Icon"),
        &icon,
        (list_x + 22.0, y + 1.0, 16.0, 16.0),
    ));
    elements.extend(rsx! {
        fontstring {
            name: {DynName(format!("{name}Text"))},
            width: {list_w - 64.0},
            height: ROW_H,
            text: {title.as_str()},
            font: GameFont::FrizQuadrata,
            font_size: ROW_FONT,
            font_color: color,
            justify_h: "LEFT",
            onclick: {action.as_str()},
            pos_type: "absolute",
            left: {list_x + 42.0},
            top: {y + 2.0},
        }
    });
    if row.watched {
        elements.extend(atlas_texture(
            format!("{name}Check"),
            &TRACKER_CHECK,
            (list_x + list_w - 20.0, y + 1.0, 16.0, 16.0),
        ));
    }
    elements
}

const DETAILS_TEXT_PANE: QuestPane = QuestPane {
    x: DETAILS_X + DETAILS_INSET,
    y: PANE_TOP + DETAILS_INSET,
    width: DETAILS_TEXT_W,
};

fn details_pane(details: Option<&QuestLogDetails>) -> Element {
    let mut elements = named_atlas_texture(
        "QuestLogDetailsBackground".into(),
        QUEST_PARCHMENT,
        (DETAILS_X, PANE_TOP, DETAILS_W, PANE_BOTTOM - PANE_TOP),
    );
    let Some(details) = details else {
        return elements;
    };
    elements.extend(quest_details_text(details, DETAILS_TEXT_PANE));
    let track_label = if details.watched { "Untrack" } else { "Track" };
    let button_y = FRAME_H - 4.0 - 22.0;
    elements.extend(panel_button(
        "QuestLogAbandonButton".into(),
        "Abandon",
        ABANDON_ACTION,
        true,
        (DETAILS_X, button_y, 100.0, 22.0),
    ));
    elements.extend(panel_button(
        "QuestLogTrackButton".into(),
        track_label,
        TRACK_ACTION,
        true,
        (DETAILS_X + DETAILS_W - 100.0, button_y, 100.0, 22.0),
    ));
    elements
}

/// The quest's title, objectives, description and rewards, top-down in `pane`.
pub fn quest_details_text(details: &QuestLogDetails, pane: QuestPane) -> Element {
    let mut y = pane.y;
    let mut elements = details_text(
        pane,
        "QuestLogDetailsTitle",
        &details.title,
        TITLE_FONT,
        QUEST_TEXT_COLOR,
        &mut y,
    );
    y += GAP;
    elements.extend(details_text(
        pane,
        "QuestLogDetailsObjectivesText",
        &details.objectives_text,
        BODY_FONT,
        QUEST_TEXT_COLOR,
        &mut y,
    ));
    for (index, objective) in details.objectives.iter().enumerate() {
        let color = if objective.done {
            DONE_OBJECTIVE_COLOR
        } else {
            QUEST_TEXT_COLOR
        };
        y += 2.0;
        elements.extend(details_text(
            pane,
            &format!("QuestLogDetailsObjective{index}"),
            &format!("- {}", objective.text),
            BODY_FONT,
            color,
            &mut y,
        ));
    }
    if let Some(description) = &details.description {
        y += GAP * 2.0;
        elements.extend(details_text(
            pane,
            "QuestLogDetailsDescriptionHeader",
            "Description",
            TITLE_FONT,
            QUEST_TEXT_COLOR,
            &mut y,
        ));
        y += GAP;
        elements.extend(details_text(
            pane,
            "QuestLogDetailsDescription",
            description,
            BODY_FONT,
            QUEST_TEXT_COLOR,
            &mut y,
        ));
    }
    if let Some(rewards) = &details.rewards {
        let column = Column {
            x: pane.x,
            width: pane.width,
        };
        elements.extend(rewards_section(rewards, false, column, &mut y));
    }
    elements
}

fn details_text(
    pane: QuestPane,
    name: &str,
    text: &str,
    font_size: f32,
    color: &str,
    y: &mut f32,
) -> Element {
    let top = *y;
    let height = wrapped_text_height(text, pane.width, font_size);
    *y += height;
    rsx! {
        fontstring {
            name: {DynName(name.into())},
            width: {pane.width},
            height,
            text,
            font: GameFont::FrizQuadrata,
            font_size,
            font_color: color,
            justify_h: "LEFT",
            pos_type: "absolute",
            left: {pane.x},
            top,
        }
    }
}
