//! Retail objective tracker (`Blizzard_ObjectiveTracker`): the "All Objectives"
//! container header, the "Quests" module header and one block per watched quest with
//! its POI button, title and objective lines.

use shared::protocol::QuestEntrySnapshot;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

use crate::hud_layout::hud_layout;
use crate::quest_runtime::QuestRuntime;
use crate::ui::screens::quest_art::{
    DynName, POI_IN_PROGRESS, POI_NUMBER, POI_TURN_IN, TRACKER_CHECK, TRACKER_COLLAPSE_ALL,
    TRACKER_EXPAND_ALL, TRACKER_PRIMARY_HEADER, TRACKER_SECONDARY_COLLAPSE,
    TRACKER_SECONDARY_EXPAND, TRACKER_SECONDARY_HEADER, atlas_texture, line_height,
    wrapped_text_height,
};

pub const TRACKER_FRAME: &str = "ObjectiveTrackerFrame";
/// `ObjectiveTrackerContainerTemplate` width.
pub const TRACKER_W: f32 = 260.0;
const CONTAINER_HEADER_H: f32 = 32.0;
/// `ObjectiveTrackerFrame.topModulePadding`.
const TOP_MODULE_PADDING: f32 = 38.0;
const MODULE_HEADER_H: f32 = 26.0;
/// `ObjectiveTrackerModuleMixin` defaults: headerHeight 25, fromHeaderOffsetY -10,
/// blockOffsetX 20, fromBlockOffsetY -10, lineSpacing 4.
const MODULE_HEADER_HEIGHT: f32 = 25.0;
const FROM_HEADER_OFFSET_Y: f32 = 10.0;
const BLOCK_OFFSET_X: f32 = 20.0;
const FROM_BLOCK_OFFSET_Y: f32 = 10.0;
const LINE_SPACING: f32 = 4.0;
const BLOCK_W: f32 = TRACKER_W - BLOCK_OFFSET_X;
/// `ObjectiveTrackerLineFont` (12) and `ObjectiveTrackerHeaderFont` (14).
const LINE_FONT: f32 = 12.0;
const HEADER_FONT: f32 = 14.0;
/// Width of `QUEST_DASH` ("- ") in the line font.
const DASH_W: f32 = 9.0;
const POI_SIZE: f32 = 20.0;

/// `NORMAL_FONT_COLOR` for the container and module headers.
const HEADER_COLOR: &str = "1.0,0.82,0.0,1.0";
/// `OBJECTIVE_TRACKER_COLOR`: Header (block title), Normal, Complete.
const BLOCK_HEADER_COLOR: &str = "0.75,0.61,0.0,1.0";
const NORMAL_COLOR: &str = "0.8,0.8,0.8,1.0";
const COMPLETE_COLOR: &str = "0.6,0.6,0.6,1.0";
const SHADOW: &str = "0.0,0.0,0.0,1.0";

pub const TOGGLE_ACTION: &str = "quest_tracker:toggle";
pub const TOGGLE_QUESTS_ACTION: &str = "quest_tracker:toggle_quests";
pub const OPEN_QUEST_PREFIX: &str = "quest_tracker:open:";

/// `QUEST_WATCH_QUEST_READY`.
pub const READY_FOR_TURN_IN: &str = "Ready for turn-in";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectiveLineStyle {
    /// Dash, `OBJECTIVE_TRACKER_COLOR.Normal`.
    InProgress,
    /// Check icon, no dash, `Complete` colour.
    Completed,
    /// Completion text of a finished quest: no dash, `Normal` colour.
    CompletionText,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectiveLine {
    pub text: String,
    pub style: ObjectiveLineStyle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrackedQuest {
    pub quest_id: u32,
    pub title: String,
    pub complete: bool,
    pub lines: Vec<ObjectiveLine>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ObjectiveTrackerState {
    pub collapsed: bool,
    pub quests_collapsed: bool,
    pub quests: Vec<TrackedQuest>,
}

impl ObjectiveTrackerState {
    pub fn from_runtime(runtime: &QuestRuntime, collapsed: bool, quests_collapsed: bool) -> Self {
        Self::from_watched(runtime.watched_entries(), collapsed, quests_collapsed)
    }

    /// Blocks for `watched` entries in watch order.
    pub fn from_watched<'a>(
        watched: impl IntoIterator<Item = &'a QuestEntrySnapshot>,
        collapsed: bool,
        quests_collapsed: bool,
    ) -> Self {
        Self {
            collapsed,
            quests_collapsed,
            quests: watched.into_iter().map(tracked_quest).collect(),
        }
    }
}

/// `QuestObjectiveTrackerMixin:UpdateSingle`: a finished quest shows only its
/// completion text (or "Ready for turn-in"); otherwise one line per objective,
/// finished ones checked.
fn tracked_quest(entry: &QuestEntrySnapshot) -> TrackedQuest {
    let lines = if entry.completed {
        vec![completion_line(entry)]
    } else if entry.objectives.is_empty() {
        vec![ObjectiveLine {
            text: entry.objectives_text.clone(),
            style: ObjectiveLineStyle::InProgress,
        }]
    } else {
        entry
            .objectives
            .iter()
            .map(|objective| ObjectiveLine {
                text: format!(
                    "{}/{} {}",
                    objective.current, objective.required, objective.text
                ),
                style: if objective.completed {
                    ObjectiveLineStyle::Completed
                } else {
                    ObjectiveLineStyle::InProgress
                },
            })
            .collect()
    };
    TrackedQuest {
        quest_id: entry.quest_id,
        title: entry.title.clone(),
        complete: entry.completed,
        lines,
    }
}

fn completion_line(entry: &QuestEntrySnapshot) -> ObjectiveLine {
    if entry.completion_text.is_empty() {
        ObjectiveLine {
            text: READY_FOR_TURN_IN.into(),
            style: ObjectiveLineStyle::Completed,
        }
    } else {
        ObjectiveLine {
            text: entry.completion_text.clone(),
            style: ObjectiveLineStyle::CompletionText,
        }
    }
}

pub fn objective_tracker_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<ObjectiveTrackerState>()
        .expect("ObjectiveTrackerState must be in SharedContext");
    // Retail hides an empty container outside Edit Mode (Blizzard_ObjectiveTrackerContainer.lua:99-107);
    // here the "All Objectives" header always shows, as Retail's Edit Mode draws it with nothing tracked.
    let mut height = CONTAINER_HEADER_H;
    let mut contents = container_header(state.collapsed);
    if !state.collapsed && !state.quests.is_empty() {
        contents.extend(quests_module(state, &mut height));
    }
    let at = hud_layout(ctx).objective_tracker.place((TRACKER_W, height));
    rsx! {
        r#frame {
            name: {DynName(TRACKER_FRAME.into())},
            width: TRACKER_W,
            height: {height},
            pos_type: "absolute",
            left: {at.left.as_str()},
            right: {at.right.as_str()},
            top: {at.top.as_str()},
            bottom: {at.bottom.as_str()},
            margin_left: {at.margin_left},
            margin_top: {at.margin_top},
            {contents}
        }
    }
}

fn container_header(collapsed: bool) -> Element {
    let button_art = if collapsed {
        TRACKER_EXPAND_ALL
    } else {
        TRACKER_COLLAPSE_ALL
    };
    let mut elements = atlas_texture(
        "ObjectiveTrackerFrameHeaderBackground".into(),
        &TRACKER_PRIMARY_HEADER,
        (-20.0, -4.0, 300.0, 40.0),
    );
    elements.extend(header_text(
        "ObjectiveTrackerFrameHeaderText",
        "All Objectives",
        (CONTAINER_HEADER_H - line_height(HEADER_FONT)) / 2.0,
    ));
    elements.extend(header_button(
        "ObjectiveTrackerFrameHeaderMinimizeButton",
        &button_art,
        TOGGLE_ACTION,
        (
            TRACKER_W - 1.0 - 18.0,
            (CONTAINER_HEADER_H - 19.0) / 2.0,
            18.0,
            19.0,
        ),
    ));
    elements
}

fn quests_module(state: &ObjectiveTrackerState, height: &mut f32) -> Element {
    let top = TOP_MODULE_PADDING;
    let button_art = if state.quests_collapsed {
        TRACKER_SECONDARY_EXPAND
    } else {
        TRACKER_SECONDARY_COLLAPSE
    };
    let mut elements = atlas_texture(
        "QuestObjectiveTrackerHeaderBackground".into(),
        &TRACKER_SECONDARY_HEADER,
        (-20.0, top - 2.0, 300.0, 30.0),
    );
    elements.extend(header_text(
        "QuestObjectiveTrackerHeaderText",
        "Quests",
        top + (MODULE_HEADER_H - line_height(HEADER_FONT)) / 2.0,
    ));
    elements.extend(header_button(
        "QuestObjectiveTrackerHeaderMinimizeButton",
        &button_art,
        TOGGLE_QUESTS_ACTION,
        (TRACKER_W + 1.0 - 16.0, top + 5.0, 16.0, 16.0),
    ));
    *height = top + MODULE_HEADER_H;
    if state.quests_collapsed {
        return elements;
    }
    let mut y = top + MODULE_HEADER_HEIGHT + FROM_HEADER_OFFSET_Y;
    for quest in &state.quests {
        elements.extend(quest_block(quest, &mut y));
        *height = y;
        y += FROM_BLOCK_OFFSET_Y;
    }
    elements
}

fn header_text(name: &str, text: &str, top: f32) -> Element {
    rsx! {
        fontstring {
            name: {DynName(name.into())},
            width: 208.0,
            height: {line_height(HEADER_FONT)},
            text,
            font: GameFont::FrizQuadrata,
            font_size: HEADER_FONT,
            font_color: HEADER_COLOR,
            shadow_color: SHADOW,
            shadow_offset: "1,-1",
            justify_h: "LEFT",
            pos_type: "absolute",
            left: 7.0,
            top,
        }
    }
}

fn header_button(
    name: &str,
    art: &crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt,
    action: &str,
    (x, y, width, height): (f32, f32, f32, f32),
) -> Element {
    let coords = art.tex_coords(1.0);
    rsx! {
        texture {
            name: {DynName(name.into())},
            width,
            height,
            texture_fdid: {art.fdid},
            tex_coords: {coords.as_str()},
            onclick: action,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

fn quest_block(quest: &TrackedQuest, y: &mut f32) -> Element {
    let block = format!("QuestBlock{}", quest.quest_id);
    let action = format!("{OPEN_QUEST_PREFIX}{}", quest.quest_id);
    let block_top = *y;
    let title_h = wrapped_text_height(&quest.title, BLOCK_W, LINE_FONT);
    let mut elements = poi_button(&block, quest.complete, &action, block_top);
    elements.extend(block_title(&block, &quest.title, &action, block_top));
    *y += title_h;
    for (index, line) in quest.lines.iter().enumerate() {
        *y += LINE_SPACING;
        elements.extend(objective_line(&format!("{block}Line{index}"), line, *y));
        *y += wrapped_text_height(&line.text, BLOCK_W - DASH_W, LINE_FONT);
    }
    elements
}

/// `POIButton` (20×20) TOPRIGHT at the header's TOPLEFT (-7, +5): in-progress icon on
/// the quest number plate, or the turn-in icon for a finished quest.
fn poi_button(block: &str, complete: bool, action: &str, top: f32) -> Element {
    let x = BLOCK_OFFSET_X - 7.0 - POI_SIZE;
    let y = top - 5.0;
    let centre = |size: f32| {
        (
            x + (POI_SIZE - size) / 2.0,
            y + (POI_SIZE - size) / 2.0,
            size,
            size,
        )
    };
    let mut elements = if complete {
        atlas_texture(
            format!("{block}POIButtonTurnIn"),
            &POI_TURN_IN,
            centre(32.0),
        )
    } else {
        let mut plate = atlas_texture(format!("{block}POIButtonNormal"), &POI_NUMBER, centre(32.0));
        plate.extend(atlas_texture(
            format!("{block}POIButtonInProgress"),
            &POI_IN_PROGRESS,
            centre(20.0),
        ));
        plate
    };
    let hit = DynName(format!("{block}POIButton"));
    elements.extend(rsx! {
        r#frame {
            name: hit,
            width: POI_SIZE,
            height: POI_SIZE,
            onclick: action,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    });
    elements
}

fn block_title(block: &str, title: &str, action: &str, top: f32) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("{block}HeaderText"))},
            width: BLOCK_W,
            height: {line_height(LINE_FONT)},
            text: title,
            font: GameFont::FrizQuadrata,
            font_size: LINE_FONT,
            font_color: BLOCK_HEADER_COLOR,
            shadow_color: SHADOW,
            shadow_offset: "1,-1",
            justify_h: "LEFT",
            onclick: action,
            pos_type: "absolute",
            left: BLOCK_OFFSET_X,
            top,
        }
    }
}

fn objective_line(name: &str, line: &ObjectiveLine, top: f32) -> Element {
    let (color, dash) = match line.style {
        ObjectiveLineStyle::InProgress => (NORMAL_COLOR, "- "),
        ObjectiveLineStyle::Completed => (COMPLETE_COLOR, ""),
        ObjectiveLineStyle::CompletionText => (NORMAL_COLOR, ""),
    };
    let mut elements = rsx! {
        fontstring {
            name: {DynName(format!("{name}Dash"))},
            width: DASH_W,
            height: {line_height(LINE_FONT)},
            text: dash,
            font: GameFont::FrizQuadrata,
            font_size: LINE_FONT,
            font_color: color,
            shadow_color: SHADOW,
            shadow_offset: "1,-1",
            justify_h: "LEFT",
            pos_type: "absolute",
            left: BLOCK_OFFSET_X,
            top: {top - 1.0},
        }
        fontstring {
            name: {DynName(format!("{name}Text"))},
            width: {BLOCK_W - DASH_W},
            height: {line_height(LINE_FONT)},
            text: {line.text.as_str()},
            font: GameFont::FrizQuadrata,
            font_size: LINE_FONT,
            font_color: color,
            shadow_color: SHADOW,
            shadow_offset: "1,-1",
            justify_h: "LEFT",
            pos_type: "absolute",
            left: {BLOCK_OFFSET_X + DASH_W},
            top,
        }
    };
    if line.style == ObjectiveLineStyle::Completed {
        // ObjectiveTrackerAnimLineTemplate Icon: 16×16 at TOPLEFT (-10, +2).
        elements.extend(atlas_texture(
            format!("{name}Check"),
            &TRACKER_CHECK,
            (BLOCK_OFFSET_X - 10.0, top - 2.0, 16.0, 16.0),
        ));
    }
    elements
}

// Bevy layout support; the Godot UI model builds without it.
#[cfg(all(test, feature = "dev"))]
#[path = "objective_tracker_component_tests.rs"]
mod tests;
