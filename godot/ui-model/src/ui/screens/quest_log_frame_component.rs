//! Retail QuestMapFrame data, list rows and detail content shared by WorldMapFrame.
//! L uses that owner; no standalone QuestLogFrame.

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

use ui_toolkit::atlas::{ActiveSkin, thread_skin};

use crate::ui::screens::quest_art::{
    DynName, NORMAL_FONT_COLOR, POI_IN_PROGRESS, POI_TURN_IN, QUEST_TEXT_COLOR, atlas_texture,
    named_atlas_texture,
};
use crate::ui::screens::quest_frame_component::{Column, RewardView, rewards_section};

pub const QUEST_LOG_DETAILS_SCROLL: &str = "QuestMapDetailsScrollFrame";

const HEADER_H: f32 = 22.0; // QuestLogHeaderTemplate.
// QuestMapFrame.lua:1792,1798,1866: text height +8 top +4 no objectives +6 POI room.
const ROW_H: f32 = ROW_FONT + 18.0;
const ROW_FONT: f32 = 12.0;
const TITLE_FONT: f32 = 18.0;
const BODY_FONT: f32 = 13.0;
const GAP: f32 = 6.0;
/// `QUEST_OBJECTIVE_FONT_COLOR` for finished objective lines on parchment.
const DONE_OBJECTIVE_COLOR: &str = "0.35,0.35,0.35,1.0";

pub const CLOSE_ACTION: &str = "quest_log:close";
pub const ABANDON_ACTION: &str = "quest_log:abandon";
pub const TRACK_ACTION: &str = "quest_log:track";
pub const SHARE_ACTION: &str = "quest_log:share";
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
    /// Owned story text from the authoritative quest entry.
    pub description: Option<String>,
    pub rewards: Option<RewardView>,
    pub watched: bool,
    pub can_share: bool,
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
    let left = pane.x;
    let top = y;
    let list_x = 0.0;
    let y = 0.0;
    let name = format!("QuestLogHeader{}", group.sort_id);
    let action = format!("{HEADER_PREFIX}{}", group.sort_id);
    let marker_atlas = if group.collapsed {
        "common-button-list-plus"
    } else {
        "common-button-list-minus"
    };
    let collapse = named_atlas_texture(
        format!("{name}Marker"),
        marker_atlas,
        (list_x + 263.0, y + 1.0, 20.0, 20.0),
    );
    let mut elements = named_atlas_texture(
        format!("{name}Background"),
        "common-button-list-collapseExpand",
        (list_x, y, 289.0, HEADER_H),
    );
    elements.extend(rsx! {
        {collapse}
        fontstring {
            name: {DynName(format!("{name}Text"))},
            width: {289.0 - 38.0},
            height: HEADER_H,
            text: {group.name.as_str()},
            font: GameFont::FrizQuadrata,
            font_size: 15.0,
            font_color: NORMAL_FONT_COLOR,
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h: "LEFT",
            pos_type: "absolute",
            left: {list_x + 8.0},
            top: y,
        }
    });
    rsx! { button { name: {DynName(name)}, width: 289.0, height: HEADER_H,
    left, top, pos_type: "absolute", button_default_skin: false,
    onclick: {action.as_str()}, {elements} } }
}

/// A quest title in its `QuestDifficultyColors` colour (QuestMapFrame.lua:1750-1751);
/// Forever prefixes `[level] ` (Camelot/QuestMapFrameOverrides.lua:13-16, used at
/// Forever Mainline/QuestMapFrame.lua:1631).
fn quest_row(row: &QuestLogRow, pane: QuestPane, y: f32) -> Element {
    let left = pane.x;
    let top = y;
    let list_x = 0.0;
    let list_w = 290.0;
    let y = 0.0;
    let name = format!("QuestLogTitle{}", row.quest_id);
    let action = format!("{SELECT_PREFIX}{}", row.quest_id);
    let icon = if row.complete {
        POI_TURN_IN
    } else {
        POI_IN_PROGRESS
    };
    let color = row.difficulty.color();
    let title = match thread_skin() {
        ActiveSkin::Forever => format!("[{}] {}", row.level, row.title),
        ActiveSkin::Modern => row.title.clone(),
    };
    let mut elements = Vec::new();
    if row.selected {
        elements.extend(named_atlas_texture(
            format!("{name}Selected"),
            "questlog-quest-glow-yellow",
            (list_x + 31.0, y + 4.0, list_w - 31.0, ROW_H + 8.0),
        ));
    }
    elements.extend(atlas_texture(
        format!("{name}Icon"),
        &icon,
        (list_x + 6.0, y + 4.0, 16.0, 16.0),
    ));
    elements.extend(rsx! {
        fontstring {
            name: {DynName(format!("{name}Text"))},
            width: 241.0,
            height: ROW_FONT,
            text: {title.as_str()},
            font: GameFont::FrizQuadrata,
            font_size: ROW_FONT,
            font_color: color,
            justify_h: "LEFT",
            pos_type: "absolute",
            left: {list_x + 31.0},
            top: {y + 8.0},
        }
    });
    if row.watched {
        elements.extend(named_atlas_texture(
            format!("{name}Check"),
            "questlog-icon-checkmark-yellow",
            (list_x + 277.0, y + 7.0, 14.0, 14.0),
        ));
    }
    rsx! { button { name: {DynName(name)}, width: 290.0, height: ROW_H,
    left, top, pos_type: "absolute", button_default_skin: false,
    onclick: {action.as_str()}, {elements} } }
}

/// The quest's title, objectives, description and rewards, top-down in `pane`.
pub fn quest_details_text(details: &QuestLogDetails, pane: QuestPane) -> Element {
    let elements = quest_details_elements(details, pane);
    rsx! {
        r#frame {
            name: "QuestLogDetailsContent",
            width: {pane.width},
            height: "auto",
            layout: "flex-column",
            align: "start",
            pos_type: "absolute",
            left: {pane.x},
            top: {pane.y},
            {elements}
        }
    }
}

pub(crate) fn quest_details_elements(details: &QuestLogDetails, pane: QuestPane) -> Element {
    let mut elements = details_text(
        pane,
        "QuestLogDetailsTitle",
        &details.title,
        TITLE_FONT,
        QUEST_TEXT_COLOR,
        0.0,
    );
    elements.extend(details_text(
        pane,
        "QuestLogDetailsObjectivesText",
        &details.objectives_text,
        BODY_FONT,
        QUEST_TEXT_COLOR,
        GAP,
    ));
    for (index, objective) in details.objectives.iter().enumerate() {
        let color = if objective.done {
            DONE_OBJECTIVE_COLOR
        } else {
            QUEST_TEXT_COLOR
        };
        elements.extend(details_text(
            pane,
            &format!("QuestLogDetailsObjective{index}"),
            &format!("- {}", objective.text),
            BODY_FONT,
            color,
            2.0,
        ));
    }
    if let Some(description) = &details.description {
        elements.extend(details_text(
            pane,
            "QuestLogDetailsDescriptionHeader",
            "Description",
            TITLE_FONT,
            QUEST_TEXT_COLOR,
            GAP * 2.0,
        ));
        elements.extend(details_text(
            pane,
            "QuestLogDetailsDescription",
            description,
            BODY_FONT,
            QUEST_TEXT_COLOR,
            GAP,
        ));
    }
    if let Some(rewards) = &details.rewards {
        let column = Column {
            x: 0.0,
            width: pane.width,
        };
        let mut rewards_height = 0.0;
        let rewards = rewards_section(rewards, false, column, &mut rewards_height);
        elements.extend(rsx! {
            r#frame {
                width: {pane.width},
                height: rewards_height,
                {rewards}
            }
        });
    }
    elements
}

fn details_text(
    pane: QuestPane,
    name: &str,
    text: &str,
    font_size: f32,
    color: &str,
    gap: f32,
) -> Element {
    rsx! {
        fontstring {
            name: {DynName(name.into())},
            width: {pane.width},
            height: "auto",
            text,
            font: GameFont::FrizQuadrata,
            font_size,
            font_color: color,
            justify_h: "LEFT",
            pos_type: "relative",
            margin_top: gap,
        }
    }
}
