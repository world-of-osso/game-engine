//! GroupLootFrame.xml: four 277×67 prompts; LootHistory.xml: 261×590 panel,
//! 218×58 entries. Both skins resolve the same Retail atlas names and geometry.
use crate::group_loot::GroupLootState;
use crate::item_icons::item_icon_fdid;
use crate::merchant_data::quality_color;
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::quest_art::{DynName, flat_panel_chrome};
use crate::ui::strata::FrameStrata;
use shared::loot::ROLL_WINDOW_TIMEOUT;
use shared::protocol::{GroupLootRoll, LootContent};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const HISTORY_CLOSE: &str = "group_loot_history_close";
pub const HISTORY_LIST: &str = "GroupLootHistoryFrameScrollBox";
const PROMPT_W: f32 = 277.0;
const PROMPT_H: f32 = 67.0;
const HISTORY_H: f32 = 590.0;
const HISTORY_ROW_H: f32 = 58.0;

#[derive(Default, Clone, PartialEq)]
pub struct GroupLootUiState {
    pub prompts: Vec<GroupLootRoll>,
    pub history: Vec<GroupLootRoll>,
    pub viewer: u64,
    pub history_open: bool,
}

impl GroupLootUiState {
    pub fn from_state(state: &GroupLootState, viewer: u64) -> Self {
        let prompts = state
            .visible(viewer)
            .into_iter()
            .map(|roll| {
                let mut roll = roll.clone();
                roll.seconds_remaining = (roll.seconds_remaining * 10.0).floor() / 10.0;
                roll
            })
            .collect();
        Self {
            prompts,
            history: state.history().to_vec(),
            viewer,
            history_open: state.history_open,
        }
    }
}

pub fn group_loot_screen(ctx: &SharedContext) -> Element {
    let state = ctx.get::<GroupLootUiState>().expect("GroupLootUiState");
    let mut prompts = Element::default();
    for (index, roll) in state.prompts.iter().enumerate() {
        prompts.extend(prompt(index, roll, state.viewer));
    }
    let hide = state.prompts.is_empty();
    let mut root = rsx! { r#frame {
        name: "GroupLootContainer", width: PROMPT_W, height: 400.0,
        hidden: hide, strata: FrameStrata::Dialog,
        pos_type: "absolute", left: "50%", top: "60%",
        translate_x: "-50%", translate_y: "-100%", {prompts}
    }};
    if state.history_open {
        root.extend(history_panel(ctx, state));
    }
    root
}

fn prompt(index: usize, roll: &GroupLootRoll, viewer: u64) -> Element {
    let prefix = format!("GroupLootFrame{}", index + 1);
    let player = roll
        .players
        .iter()
        .find(|player| player.character_id == viewer)
        .expect("visible participant");
    // XML comments name these regions, but they are not named DB2 atlas members.
    let mut children = toast_texture(
        format!("{prefix}Background"),
        "0.28222656,0.55273438,0.30859375,0.57031250",
        [0.0, 0.0, PROMPT_W, PROMPT_H],
    );
    children.extend(toast_texture(
        format!("{prefix}Border"),
        "0.00097656,0.28027344,0.43750000,0.73437500",
        [-4.5, -4.5, 286.0, 76.0],
    ));
    children.extend(ui_toolkit::tooltip::bind_provider(
        item_art(&prefix, roll, [10.0, 13.0, 34.0, 34.0]),
        "group_loot",
    ));
    children.extend(item_label(&prefix, roll, [60.0, 15.0, 125.0, 30.0]));
    children.extend(roll_button(
        &prefix,
        roll.roll_id,
        "Need",
        "need",
        [201.0, 6.0],
        player.can_need,
    ));
    children.extend(roll_button(
        &prefix,
        roll.roll_id,
        "Pass",
        "pass",
        [239.0, 4.0],
        true,
    ));
    let (suffix, choice) = if player.can_transmog {
        ("Transmog", "transmog")
    } else {
        ("Greed", "greed")
    };
    children.extend(roll_button(
        &prefix,
        roll.roll_id,
        suffix,
        choice,
        [201.0, 33.0],
        true,
    ));
    let fraction = (roll.seconds_remaining / ROLL_WINDOW_TIMEOUT).clamp(0.0, 1.0);
    children.extend(rsx! {
        texture { name: {DynName(format!("{prefix}TimerBackground"))}, width: 190.0, height: 8.0,
            vertex_color: "0,0,0,1", pos_type: "absolute", left: 3.0, top: 57.0, }
        texture { name: {DynName(format!("{prefix}Timer"))}, width: {190.0 * fraction}, height: 8.0,
            texture_fdid: 136570u32, vertex_color: "1,1,0,1", pos_type: "absolute", left: 3.0, top: 57.0, }
    });
    rsx! { r#frame { name: {DynName(prefix)}, width: PROMPT_W, height: PROMPT_H,
        strata: FrameStrata::Dialog, mouse_enabled: true,
        pos_type: "absolute", left: 0.0, top: {316.5 - index as f32 * 100.0}, {children}
    }}
}

fn roll_button(
    prefix: &str,
    id: u64,
    suffix: &str,
    choice: &str,
    [left, top]: [f32; 2],
    enabled: bool,
) -> Element {
    let action = format!("group_loot:{id}:{choice}");
    let up = format!("lootroll-toast-icon-{choice}-up");
    let down = format!("lootroll-toast-icon-{choice}-down");
    let highlight = format!("lootroll-toast-icon-{choice}-highlight");
    let alpha = if enabled { 1.0 } else { 0.35 };
    rsx! { button { name: {DynName(format!("{prefix}{suffix}Button"))}, width: 32.0, height: 32.0,
        enabled, alpha, onclick: {action.as_str()}, tooltip_provider: "group_loot",
        button_atlas_up: {up.as_str()}, button_atlas_pressed: {down.as_str()},
        button_atlas_highlight: {highlight.as_str()}, button_atlas_disabled: {up.as_str()},
        pos_type: "absolute", left, top,
    }}
}

fn item_art(prefix: &str, roll: &GroupLootRoll, [left, top, width, height]: [f32; 4]) -> Element {
    let LootContent::Item { item_id, count, .. } = roll.item else {
        return Element::default();
    };
    let mut children = Element::default();
    if let Some(fdid) = item_icon_fdid(item_id) {
        children.extend(
            rsx! { texture { name: {DynName(format!("{prefix}Icon"))}, width, height,
                texture_fdid: fdid, pos_type: "absolute", left: 0.0, top: 0.0,
            }},
        );
    }
    let hide_count = count <= 1;
    let count = count.to_string();
    children.extend(
        rsx! { fontstring { name: {DynName(format!("{prefix}Count"))}, width, height: 16.0,
            hidden: hide_count, text: {count.as_str()}, font_size: 12.0, justify_h: "RIGHT",
            pos_type: "absolute", left: 0.0, top: {height - 16.0},
        }},
    );
    rsx! { button { name: {DynName(format!("{prefix}IconFrame"))}, width, height,
        pos_type: "absolute", left, top, {children}
    }}
}

fn item_label(prefix: &str, roll: &GroupLootRoll, rect: [f32; 4]) -> Element {
    let LootContent::Item {
        ref name, quality, ..
    } = roll.item
    else {
        return Element::default();
    };
    label(format!("{prefix}Name"), name, rect, quality_color(quality))
}

fn label(name: String, text: &str, [left, top, width, height]: [f32; 4], color: &str) -> Element {
    rsx! { fontstring { name: {DynName(name)}, width, height, text,
        font_object: "GameFontNormal", font_color: color,
        justify_h: "LEFT", pos_type: "absolute", left, top,
    }}
}

fn toast_texture(name: String, coords: &str, [left, top, width, height]: [f32; 4]) -> Element {
    rsx! { texture { name: {DynName(name)}, width, height, texture_fdid: 609618u32,
        tex_coords: coords, pos_type: "absolute", left, top,
    }}
}

fn atlas(name: String, atlas: &str, [left, top, width, height]: [f32; 4]) -> Element {
    rsx! { texture { name: {DynName(name)}, width, height, texture_atlas: atlas,
        pos_type: "absolute", left, top,
    }}
}

fn history_panel(ctx: &SharedContext, state: &GroupLootUiState) -> Element {
    let mut children = flat_panel_chrome(
        "GroupLootHistoryFrame",
        (261.0, HISTORY_H),
        "Loot",
        HISTORY_CLOSE,
    );
    let content_h = state.history.len() as f32 * HISTORY_ROW_H;
    let visible_h = HISTORY_H - 94.0;
    let geometry = pixel_geometry(visible_h, content_h, visible_h);
    let offset = geometry.clamp(ctx.scroll_first_row(HISTORY_LIST));
    let config = scroll_list_attr(&geometry);
    let bar = MinimalScrollBar {
        list: HISTORY_LIST,
        left: 235.0,
        top: 0.0,
        height: visible_h,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    let mut entries = Element::default();
    for (index, roll) in state.history.iter().rev().enumerate() {
        entries.extend(history_entry(index, roll));
    }
    children.extend(label(
        "GroupLootHistoryFrameEncounterDropdownText".into(),
        "Recent Loot",
        [12.0, 32.0, 230.0, 24.0],
        "1,0.82,0,1",
    ));
    children.extend(
        rsx! { r#frame { name: {DynName(HISTORY_LIST.into())}, width: 250.0, height: visible_h,
            mouse_enabled: true, scroll_list: config, pos_type: "absolute", left: 6.0, top: 90.0,
            r#frame { name: "GroupLootHistoryFrameScrollChild", width: 232.0, height: content_h,
                pos_type: "absolute", left: 0.0, top: {-(offset as f32)}, {entries}
            }
            {bar.element()}
        }},
    );
    rsx! { r#frame { name: "GroupLootHistoryFrame", width: 261.0, height: HISTORY_H,
        strata: FrameStrata::High, mouse_enabled: true,
        pos_type: "absolute", left: 150.0, top: 150.0, {children}
    }}
}

fn history_entry(index: usize, roll: &GroupLootRoll) -> Element {
    let prefix = format!("LootHistoryElement{}", index + 1);
    let mut children = atlas(
        format!("{prefix}NameFrame"),
        "looting_itemcard_bg",
        [0.0, 0.0, 218.0, HISTORY_ROW_H],
    );
    children.extend(item_art(&prefix, roll, [6.0, 5.0, 48.0, 48.0]));
    children.extend(item_label(&prefix, roll, [60.0, 5.0, 151.0, 24.0]));
    let text = if !roll.complete {
        "Waiting for rolls".into()
    } else if let Some(winner) = roll.winner {
        roll.players
            .iter()
            .find(|player| player.character_id == winner)
            .map(|player| player.name.clone())
            .unwrap_or_default()
    } else {
        "Everyone passed".into()
    };
    children.extend(label(
        format!("{prefix}WinningRoll"),
        &text,
        [60.0, 31.0, 151.0, 20.0],
        "1,1,1,1",
    ));
    rsx! { r#frame { name: {DynName(prefix)}, width: 218.0, height: HISTORY_ROW_H,
        tooltip_provider: "group_loot", mouse_enabled: true, pos_type: "absolute", left: 0.0, top: {index as f32 * HISTORY_ROW_H}, {children}
    }}
}
