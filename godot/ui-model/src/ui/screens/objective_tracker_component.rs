//! Retail objective tracker (`Blizzard_ObjectiveTracker`): the "All Objectives"
//! container header, the "Quests" module header and one block per watched quest with
//! its POI button, title and objective lines.

use shared::protocol::QuestEntrySnapshot;
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

use crate::hud_layout::{FOREVER_TRACKER_SCALE, HudAnchor, hud_layout};
use crate::quest_runtime::QuestRuntime;
use crate::ui::screens::quest_art::{
    DynName, POI_NUMBER, POI_TURN_IN, TRACKER_CHECK, TRACKER_COLLAPSE_ALL, TRACKER_EXPAND_ALL,
    TRACKER_PRIMARY_HEADER, TRACKER_SECONDARY_COLLAPSE, TRACKER_SECONDARY_EXPAND,
    TRACKER_SECONDARY_HEADER, atlas_texture, line_height, wrapped_text_height,
};

pub const TRACKER_FRAME: &str = "ObjectiveTrackerFrame";

/// Visible encounter frames ahead of the tracker in the right-managed stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct BossFrameCount(pub usize);
/// `ObjectiveTrackerContainerTemplate` width.
pub const TRACKER_W: f32 = 260.0;
/// Container header art (left, top, width, height) in tracker units, from the frame's
/// TOPLEFT, y down; its line fades over the outer 6 units at each end (FlareUI
/// Modules/Minimap.lua:366-371: about 288 of the 300 show).
pub const HEADER_ART: (f32, f32, f32, f32) = (-20.0, -4.0, 300.0, 40.0);
pub const HEADER_LINE_FADE: f32 = 6.0;
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

#[derive(Clone, Debug, PartialEq)]
pub struct DungeonBlock {
    pub name: String,
    pub bosses: Vec<ObjectiveLine>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ObjectiveTrackerState {
    pub collapsed: bool,
    pub quests_collapsed: bool,
    pub quests: Vec<TrackedQuest>,
    pub dungeon: Option<DungeonBlock>,
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
            dungeon: None,
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
    let scale = tracker_scale(
        *ctx.get::<ActiveSkin>()
            .expect("canvas carries the active skin"),
    );
    // Layout runs in the tracker's own units; every emitted length takes `scale`.
    let mut height = CONTAINER_HEADER_H;
    let mut contents = container_header(state.collapsed, scale);
    if !state.collapsed {
        if let Some(dungeon) = &state.dungeon {
            contents.extend(dungeon_module(dungeon, &mut height, scale));
        }
        if !state.quests.is_empty() {
            contents.extend(quests_module(state, &mut height, scale));
        }
    }
    // `SetScale` also scales the frame's own `SetPoint` offsets.
    let anchor = hud_layout(ctx).objective_tracker;
    let boss_count = ctx.get::<BossFrameCount>().copied().unwrap_or_default().0;
    let resting_top = -anchor.y * scale;
    let managed_top =
        super::inworld_unit_frames_component::tracker_top_below_bosses(resting_top, boss_count);
    let at = HudAnchor {
        x: anchor.x * scale,
        y: -managed_top,
        ..anchor
    }
    .place((TRACKER_W * scale, height * scale));
    rsx! {
        r#frame {
            name: {DynName(TRACKER_FRAME.into())},
            width: {TRACKER_W * scale},
            height: {height * scale},
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

pub fn tracker_scale(skin: ActiveSkin) -> f32 {
    match skin {
        ActiveSkin::Modern => 1.0,
        ActiveSkin::Forever => FOREVER_TRACKER_SCALE,
    }
}

/// Screen height of the container in its default position, where Edit Mode's selection
/// covers it: `ObjectiveTrackerContainerMixin:UpdateHeight` sets
/// `max(parentHeight + offsetY, 20)` in the tracker's own units
/// (Blizzard_ObjectiveTrackerContainer.lua:203-209). `top` is the frame's top in screen units.
pub fn default_position_height(skin: ActiveSkin, parent_height: f32, top: f32) -> f32 {
    let scale = tracker_scale(skin);
    (parent_height - top / scale).max(20.0) * scale
}

fn scaled((x, y, width, height): (f32, f32, f32, f32), scale: f32) -> (f32, f32, f32, f32) {
    (x * scale, y * scale, width * scale, height * scale)
}

fn container_header(collapsed: bool, scale: f32) -> Element {
    let button_art = if collapsed {
        TRACKER_EXPAND_ALL
    } else {
        TRACKER_COLLAPSE_ALL
    };
    let mut elements = atlas_texture(
        "ObjectiveTrackerFrameHeaderBackground".into(),
        &TRACKER_PRIMARY_HEADER,
        scaled(HEADER_ART, scale),
    );
    elements.extend(header_text(
        "ObjectiveTrackerFrameHeaderText",
        "All Objectives",
        (CONTAINER_HEADER_H - line_height(HEADER_FONT)) / 2.0,
        scale,
    ));
    elements.extend(header_button(
        "ObjectiveTrackerFrameHeaderMinimizeButton",
        &button_art,
        TOGGLE_ACTION,
        scaled(
            (
                TRACKER_W - 1.0 - 18.0,
                (CONTAINER_HEADER_H - 19.0) / 2.0,
                18.0,
                19.0,
            ),
            scale,
        ),
    ));
    elements
}

fn quests_module(state: &ObjectiveTrackerState, height: &mut f32, scale: f32) -> Element {
    let top = TOP_MODULE_PADDING.max(*height + 6.0);
    let button_art = if state.quests_collapsed {
        TRACKER_SECONDARY_EXPAND
    } else {
        TRACKER_SECONDARY_COLLAPSE
    };
    let mut elements = atlas_texture(
        "QuestObjectiveTrackerHeaderBackground".into(),
        &TRACKER_SECONDARY_HEADER,
        scaled((-20.0, top - 2.0, 300.0, 30.0), scale),
    );
    elements.extend(header_text(
        "QuestObjectiveTrackerHeaderText",
        "Quests",
        top + (MODULE_HEADER_H - line_height(HEADER_FONT)) / 2.0,
        scale,
    ));
    elements.extend(header_button(
        "QuestObjectiveTrackerHeaderMinimizeButton",
        &button_art,
        TOGGLE_QUESTS_ACTION,
        scaled((TRACKER_W + 1.0 - 16.0, top + 5.0, 16.0, 16.0), scale),
    ));
    *height = top + MODULE_HEADER_H;
    if state.quests_collapsed {
        return elements;
    }
    let mut y = top + MODULE_HEADER_HEIGHT + FROM_HEADER_OFFSET_Y;
    for (index, quest) in state.quests.iter().enumerate() {
        elements.extend(quest_block(quest, index + 1, &mut y, scale));
        *height = y;
        y += FROM_BLOCK_OFFSET_Y;
    }
    elements
}

/// Retail's Scenario module precedes the Quest module; the approved container anchor is unchanged.
fn dungeon_module(dungeon: &DungeonBlock, height: &mut f32, scale: f32) -> Element {
    let top = TOP_MODULE_PADDING;
    let mut elements = atlas_texture(
        "DungeonObjectiveTrackerHeaderBackground".into(),
        &TRACKER_SECONDARY_HEADER,
        scaled((-20.0, top - 2.0, 300.0, 30.0), scale),
    );
    elements.extend(header_text(
        "DungeonObjectiveTrackerHeaderText",
        &dungeon.name,
        top + 5.0,
        scale,
    ));
    let mut y = top + MODULE_HEADER_HEIGHT + FROM_HEADER_OFFSET_Y;
    for (index, boss) in dungeon.bosses.iter().enumerate() {
        let name = format!("DungeonBoss{index}");
        elements.extend(objective_line(&name, boss, y, scale, true));
        if boss.style == ObjectiveLineStyle::InProgress {
            // ScenarioObjectiveTracker.lua:404: nub, not a quest dash.
            elements.extend(atlas_texture(
                format!("{name}Nub"),
                &crate::quest_art::TRACKER_NUB,
                scaled((BLOCK_OFFSET_X - 10.0, y - 2.0, 16.0, 16.0), scale),
            ));
        }
        y += wrapped_text_height(&boss.text, BLOCK_W - DASH_W, LINE_FONT) + LINE_SPACING;
    }
    *height = y;
    elements
}

fn header_text(name: &str, text: &str, top: f32, scale: f32) -> Element {
    rsx! {
        fontstring {
            name: {DynName(name.into())},
            width: {208.0 * scale},
            height: {line_height(HEADER_FONT) * scale},
            text,
            font: GameFont::FrizQuadrata,
            font_size: {HEADER_FONT * scale},
            font_color: HEADER_COLOR,
            shadow_color: SHADOW,
            shadow_offset: "1,-1",
            justify_h: "LEFT",
            pos_type: "absolute",
            left: {7.0 * scale},
            top: {top * scale},
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

fn quest_block(quest: &TrackedQuest, number: usize, y: &mut f32, scale: f32) -> Element {
    let block = format!("QuestBlock{}", quest.quest_id);
    let action = format!("{OPEN_QUEST_PREFIX}{}", quest.quest_id);
    let block_top = *y;
    let title_h = wrapped_text_height(&quest.title, BLOCK_W, LINE_FONT);
    let mut elements = poi_button(&block, number, quest.complete, &action, block_top, scale);
    elements.extend(block_title(&block, &quest.title, &action, block_top, scale));
    *y += title_h;
    for (index, line) in quest.lines.iter().enumerate() {
        *y += LINE_SPACING;
        elements.extend(objective_line(
            &format!("{block}Line{index}"),
            line,
            *y,
            scale,
            false,
        ));
        *y += wrapped_text_height(&line.text, BLOCK_W - DASH_W, LINE_FONT);
    }
    elements
}

/// `POIButton` (20×20) TOPRIGHT at the header's TOPLEFT (-7, +5): numbered
/// objective plate, or the turn-in icon for a finished quest.
fn poi_button(
    block: &str,
    number: usize,
    complete: bool,
    action: &str,
    top: f32,
    scale: f32,
) -> Element {
    let x = BLOCK_OFFSET_X - 7.0 - POI_SIZE;
    let y = top - 5.0;
    let centre = |size: f32| {
        scaled(
            (
                x + (POI_SIZE - size) / 2.0,
                y + (POI_SIZE - size) / 2.0,
                size,
                size,
            ),
            scale,
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
        let text = number.to_string();
        plate.extend(rsx! { fontstring {
            name: {DynName(format!("{block}POIButtonNumber"))},
            text: {text.as_str()}, font: GameFont::FrizQuadrata,
            font_size: {11.0 * scale}, font_color: "1,1,1,1",
            shadow_color: SHADOW, shadow_offset: "1,-1",
            justify_h: "CENTER", justify_v: "MIDDLE",
            width: {POI_SIZE * scale}, height: {POI_SIZE * scale},
            pos_type: "absolute", left: {x * scale}, top: {y * scale},
        }});
        plate
    };
    let hit = DynName(format!("{block}POIButton"));
    elements.extend(rsx! {
        r#frame {
            name: hit,
            width: {POI_SIZE * scale},
            height: {POI_SIZE * scale},
            onclick: action,
            pos_type: "absolute",
            left: {x * scale},
            top: {y * scale},
        }
    });
    elements
}

fn block_title(block: &str, title: &str, action: &str, top: f32, scale: f32) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("{block}HeaderText"))},
            width: {BLOCK_W * scale},
            height: {line_height(LINE_FONT) * scale},
            text: title,
            font: GameFont::FrizQuadrata,
            font_size: {LINE_FONT * scale},
            font_color: BLOCK_HEADER_COLOR,
            shadow_color: SHADOW,
            shadow_offset: "1,-1",
            justify_h: "LEFT",
            onclick: action,
            pos_type: "absolute",
            left: {BLOCK_OFFSET_X * scale},
            top: {top * scale},
        }
    }
}

fn objective_line(
    name: &str,
    line: &ObjectiveLine,
    top: f32,
    scale: f32,
    scenario: bool,
) -> Element {
    let (color, dash) = match line.style {
        ObjectiveLineStyle::InProgress => (NORMAL_COLOR, if scenario { "" } else { "- " }),
        ObjectiveLineStyle::Completed => (COMPLETE_COLOR, ""),
        ObjectiveLineStyle::CompletionText => (NORMAL_COLOR, ""),
    };
    let mut elements = rsx! {
        fontstring {
            name: {DynName(format!("{name}Dash"))},
            width: {DASH_W * scale},
            height: {line_height(LINE_FONT) * scale},
            text: dash,
            font: GameFont::FrizQuadrata,
            font_size: {LINE_FONT * scale},
            font_color: color,
            shadow_color: SHADOW,
            shadow_offset: "1,-1",
            justify_h: "LEFT",
            pos_type: "absolute",
            left: {BLOCK_OFFSET_X * scale},
            top: {(top - 1.0) * scale},
        }
        fontstring {
            name: {DynName(format!("{name}Text"))},
            width: {(BLOCK_W - DASH_W) * scale},
            height: {line_height(LINE_FONT) * scale},
            text: {line.text.as_str()},
            font: GameFont::FrizQuadrata,
            font_size: {LINE_FONT * scale},
            font_color: color,
            shadow_color: SHADOW,
            shadow_offset: "1,-1",
            justify_h: "LEFT",
            pos_type: "absolute",
            left: {(BLOCK_OFFSET_X + DASH_W) * scale},
            top: {top * scale},
        }
    };
    if line.style == ObjectiveLineStyle::Completed {
        // ObjectiveTrackerAnimLineTemplate Icon: 16×16 at TOPLEFT (-10, +2).
        elements.extend(atlas_texture(
            format!("{name}Check"),
            &TRACKER_CHECK,
            scaled((BLOCK_OFFSET_X - 10.0, top - 2.0, 16.0, 16.0), scale),
        ));
    }
    elements
}

// Bevy layout support; the Godot UI model builds without it.
