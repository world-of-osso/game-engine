//! HUD edit mode: per-element selection boxes and the layout manager panel
//! (Retail `EditModeManagerFrame`-like).

use std::collections::BTreeMap;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use ui_toolkit::anchor::FrameName;
use ui_toolkit::strata::FrameStrata;
use ui_toolkit::widgets::font_string::{FontColor, GameFont};

pub const EDIT_MODE_OVERLAY_ROOT: FrameName = FrameName("EditModeOverlay");
pub const EDIT_MODE_PANEL: FrameName = FrameName("EditModeManagerFrame");
pub const EDIT_MODE_NAME_INPUT: FrameName = FrameName("EditModeManagerFrameLayoutName");

pub const ACTION_EDIT_MODE_PREV_LAYOUT: &str = "edit_mode_prev_layout";
pub const ACTION_EDIT_MODE_NEXT_LAYOUT: &str = "edit_mode_next_layout";
pub const ACTION_EDIT_MODE_NEW: &str = "edit_mode_new";
pub const ACTION_EDIT_MODE_RENAME: &str = "edit_mode_rename";
pub const ACTION_EDIT_MODE_DELETE: &str = "edit_mode_delete";
pub const ACTION_EDIT_MODE_REVERT: &str = "edit_mode_revert";
pub const ACTION_EDIT_MODE_SAVE: &str = "edit_mode_save";
pub const ACTION_EDIT_MODE_EXIT: &str = "edit_mode_exit";
pub const ACTION_EDIT_MODE_RESET: &str = "edit_mode_reset";
pub const ACTION_EDIT_MODE_CONFIRM_DELETE: &str = "edit_mode_confirm_delete";
pub const ACTION_EDIT_MODE_CANCEL_DELETE: &str = "edit_mode_cancel_delete";

/// Retail EditModeSystemSelectionLayout uses 16-unit atlas pieces offset by 8.
/// Map the authored pieces to 8-unit inset corners, preserving mover bounds.
const SELECTION_CORNER: f32 = 8.0;
/// `Interface/DialogFrame/UIFrameDialogBoxBackgroundDark`, as the static popups.
const PANEL_BACKGROUND_FDID: u32 = 6_839_810;
const PANEL_STYLE: &str = crate::static_popup_component::STATIC_POPUP_PANEL_STYLE;

/// Retail's 510-wide manager cannot clear our authored 1366×768 HUD; only the panel shrinks.
pub const PANEL_W: f32 = 460.0;
pub const PANEL_H: f32 = 260.0;
/// Retail EditModeManager.xml:6: TOP of UIParent at y=-100.
pub const PANEL_TOP: f32 = 100.0;
pub const PANEL_CLEARANCE: f32 = 8.0;
pub const PANEL_TITLE_H: f32 = 32.0;
const PANEL_INSET: f32 = 7.0;
const BUTTON_W: f32 = 104.0;
const BUTTON_H: f32 = 26.0;
/// Retail EditModeManager.xml:526-534 anchors footer buttons 15 in, 16 up.
const BUTTON_SIDE_INSET: f32 = 15.0;
const BUTTON_BOTTOM_INSET: f32 = 16.0;
const BUTTON_GAP: f32 = 8.0;
const RESET_BUTTON_W: f32 = 118.0;
const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";
const COLOR_TITLE: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const COLOR_TEXT: FontColor = FontColor::new(1.0, 1.0, 1.0, 1.0);
const COLOR_LABEL: FontColor = FontColor::new(0.9, 0.9, 0.9, 1.0);

struct DynName(String);

/// Retail basicLayoutIndex order (EditModeManager.xml:244-323), registered systems only.
pub const SHOW_SYSTEMS: &[(&str, &str, &[&str])] = &[
    (
        "target_and_focus",
        "Target and Focus",
        &["target_frame", "target_of_target", "focus_frame"],
    ),
    ("raid_frames", "Raid Frames", &["raid_frames"]),
    ("party_frames", "Party Frames", &["party_frames"]),
    (
        "buffs_and_debuffs",
        "Buffs and Debuffs",
        &["buffs", "debuffs"],
    ),
    ("cast_bar", "Cast Bar", &["cast_bar"]),
];

pub fn system_is_shown(settings: &BTreeMap<String, bool>, key: &str) -> bool {
    settings.get(key).copied().unwrap_or(true)
}

pub fn mover_is_shown(settings: &BTreeMap<String, bool>, mover: &str) -> bool {
    SHOW_SYSTEMS
        .iter()
        .all(|(key, _, movers)| !movers.contains(&mover) || system_is_shown(settings, key))
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditModeSelectionBox {
    pub key: String,
    pub label: String,
    /// Top-left and size in UI units.
    pub rect: [f32; 4],
    pub selected: bool,
    pub hovered: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditModeOverlayState {
    pub boxes: Vec<EditModeSelectionBox>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditModePanelState {
    pub show_systems: BTreeMap<String, bool>,
    pub layout_name: String,
    pub name_draft: String,
    pub layout_names: Vec<String>,
    pub pending_delete: Option<String>,
    /// The active layout is the preset: rename/delete are disabled, save copies it.
    pub preset: bool,
    pub dirty: bool,
    pub status: String,
    /// Computed clearance position, or the user's transient title drag position.
    pub position: Option<[f32; 2]>,
}

pub fn selection_box_name(key: &str) -> String {
    format!("EditModeSelection_{key}")
}

pub fn edit_mode_overlay_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<EditModeOverlayState>()
        .expect("EditModeOverlayState must be in SharedContext");
    // Paint idle highlights first, instructions next, selected system last.
    // Overlapping authored roots cannot cover the selected label with a later highlight.
    let mut ordered: Vec<_> = state.boxes.iter().collect();
    ordered.sort_by_key(|entry| (entry.selected, entry.hovered));
    let boxes: Element = ordered.into_iter().flat_map(selection_box).collect();
    rsx! {
        r#frame {
            name: EDIT_MODE_OVERLAY_ROOT,
            stretch: true,
            strata: FrameStrata::Fullscreen,
            {boxes}
        }
    }
}

fn selection_box(entry: &EditModeSelectionBox) -> Element {
    let name = selection_box_name(&entry.key);
    let [x, y, w, h] = entry.rect;
    // Vertical bars need several text lines; the one-line label clipped their names.
    let label_height = if w < 100.0 { h.min(54.0) } else { h.min(18.0) };
    let hide_label = !(entry.selected || entry.hovered);
    let level = if entry.selected {
        20.0
    } else if entry.hovered {
        10.0
    } else {
        0.0
    };
    let font_size = (label_height - 2.0).clamp(8.0, 13.0);
    let label = if entry.selected {
        entry.label.as_str()
    } else {
        "Click to edit"
    };
    let background = selection_background(&name, [w, h], entry.selected, level);
    rsx! {
        r#frame {
            name: {DynName(name.clone())},
            width: {w},
            height: {h},
            strata: FrameStrata::Fullscreen,
            frame_level: {level},
            pos_type: "absolute",
            left: {x},
            top: {y},
            {background}
            fontstring {
                name: {DynName(format!("{name}Label"))},
                hidden: hide_label,
                width: {w},
                height: {label_height},
                text: label,
                font: GameFont::FrizQuadrata,
                font_size,
                font_color: COLOR_TEXT,
                justify_h: "CENTER",
                strata: FrameStrata::Fullscreen,
                frame_level: {level + 2.0},
                pos_type: "absolute",
                left: 0.0,
                top: "50%",
                translate_y: "-50%",
            }
        }
    }
}

fn selection_background_pieces(name: &str, size: [f32; 2], selected: bool, level: f32) -> Element {
    let [w, h] = size;
    let c = SELECTION_CORNER;
    let kit = if selected { "selected" } else { "highlight" };
    // Fit Retail's full atlas pieces into 8 UI units, not its outward padding.
    // Corner texcoords mirror the same authored top-left member as NineSlice.lua.
    [
        ([0.0, 0.0, c, c], "", "corner", "0,1,0,1"),
        ([c, 0.0, w - 2.0 * c, c], "_", "edgetop", "0,1,0,1"),
        ([w - c, 0.0, c, c], "", "corner", "1,0,0,1"),
        ([0.0, c, c, h - 2.0 * c], "!", "edgeleft", "0,1,0,1"),
        ([c, c, w - 2.0 * c, h - 2.0 * c], "", "center", "0,1,0,1"),
        ([w - c, c, c, h - 2.0 * c], "!", "edgeright", "0,1,0,1"),
        ([0.0, h - c, c, c], "", "corner", "0,1,1,0"),
        ([c, h - c, w - 2.0 * c, c], "_", "edgebottom", "0,1,0,1"),
        ([w - c, h - c, c, c], "", "corner", "1,0,1,0"),
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(index, ([x, y, width, height], prefix, member, coords))| {
        let atlas = format!("{prefix}editmode-actionbar-{kit}-nineslice-{member}");
        rsx! {
            texture {
                name: {DynName(format!("{name}BackgroundPart{index}"))},
                width, height, texture_atlas: {atlas.as_str()}, tex_coords: coords,
                strata: FrameStrata::Fullscreen, frame_level: level,
                pos_type: "absolute", left: x, top: y,
            }
        }
    })
    .collect()
}

fn selection_background(name: &str, size: [f32; 2], selected: bool, level: f32) -> Element {
    let [w, h] = size;
    let pieces = selection_background_pieces(name, size, selected, level);
    rsx! {
        r#frame {
            name: {DynName(format!("{name}Background"))},
            width: w, height: h,
            // Pinned rsx LitFloat truncates decimals; retire expression form once fixed.
            alpha: {0.7},
            strata: FrameStrata::Fullscreen, frame_level: level,
            pos_type: "absolute", left: 0.0, top: 0.0,
            {pieces}
        }
    }
}

pub fn edit_mode_panel_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<EditModePanelState>()
        .expect("EditModePanelState must be in SharedContext");
    let layout_line = if state.dirty {
        format!("Layout: {} (unsaved)", state.layout_name)
    } else {
        format!("Layout: {}", state.layout_name)
    };
    let interactive = state.pending_delete.is_none();
    let user_layout = !state.preset && interactive;
    let name = state.name_draft.trim();
    let new_enabled = interactive
        && !name.is_empty()
        && !game_engine_core::ui_layout_data::SYSTEM_PRESETS
            .iter()
            .any(|(preset, _)| *preset == name)
        && !state.layout_names.iter().any(|existing| existing == name);
    let left = state
        .position
        .map_or_else(|| "50%".to_string(), |at| at[0].to_string());
    let translate_x = if state.position.is_some() {
        "0"
    } else {
        "-50%"
    };
    let top = state.position.map_or(PANEL_TOP, |at| at[1]);
    let row_width = PANEL_W - 2.0 * BUTTON_SIDE_INSET;
    let action_width = (row_width - RESET_BUTTON_W - 3.0 * BUTTON_GAP) / 3.0;
    let action_step = action_width + BUTTON_GAP;
    let footer_width = (row_width - 2.0 * BUTTON_GAP) / 3.0;
    let footer_step = footer_width + BUTTON_GAP;
    let footer_top = PANEL_H - BUTTON_H - BUTTON_BOTTOM_INSET;
    rsx! {
        r#frame {
            name: EDIT_MODE_PANEL,
            width: PANEL_W,
            height: PANEL_H,
            strata: FrameStrata::FullscreenDialog,
            mouse_enabled: true,
            pos_type: "absolute",
            left: {left.as_str()},
            translate_x,
            top,
            texture {
                name: "EditModeManagerFrameBackground",
                width: {PANEL_W - 2.0 * PANEL_INSET},
                height: {PANEL_H - 2.0 * PANEL_INSET},
                texture_fdid: PANEL_BACKGROUND_FDID,
                strata: FrameStrata::FullscreenDialog,
                pos_type: "absolute",
                left: PANEL_INSET,
                top: PANEL_INSET,
            }
            r#frame {
                name: "EditModeManagerFrameBorder",
                width: PANEL_W,
                height: PANEL_H,
                style: PANEL_STYLE,
                strata: FrameStrata::FullscreenDialog,
                pos_type: "absolute",
                left: 0.0,
                top: 0.0,
            }
            {panel_text("EditModeManagerFrameTitle", "Edit Mode", 16.0, COLOR_TITLE, 14.0)}
            {panel_text("EditModeManagerFrameLayout", &layout_line, 13.0, COLOR_LABEL, 40.0)}
            {panel_button("EditModeManagerFramePrev", "<", ACTION_EDIT_MODE_PREV_LAYOUT, 20.0, 36.0, 30.0, interactive)}
            {panel_button("EditModeManagerFrameNext", ">", ACTION_EDIT_MODE_NEXT_LAYOUT, PANEL_W - 50.0, 36.0, 30.0, interactive)}
            {name_input(state)}
            {panel_button("EditModeManagerFrameNew", "New", ACTION_EDIT_MODE_NEW, BUTTON_SIDE_INSET, 188.0, action_width, new_enabled)}
            {panel_button("EditModeManagerFrameRename", "Rename", ACTION_EDIT_MODE_RENAME, BUTTON_SIDE_INSET + action_step, 188.0, action_width, user_layout)}
            {panel_button("EditModeManagerFrameDelete", "Delete", ACTION_EDIT_MODE_DELETE, BUTTON_SIDE_INSET + 2.0 * action_step, 188.0, action_width, user_layout)}
            {panel_button("EditModeManagerFrameRevert", "Revert", ACTION_EDIT_MODE_REVERT, BUTTON_SIDE_INSET, footer_top, footer_width, state.dirty && interactive)}
            {panel_button("EditModeManagerFrameSave", "Save", ACTION_EDIT_MODE_SAVE, BUTTON_SIDE_INSET + footer_step, footer_top, footer_width, state.dirty && interactive)}
            {panel_button("EditModeManagerFrameExit", "Exit", ACTION_EDIT_MODE_EXIT, BUTTON_SIDE_INSET + 2.0 * footer_step, footer_top, footer_width, interactive)}
            {panel_button("EditModeManagerFrameReset", "Reset Selected", ACTION_EDIT_MODE_RESET, BUTTON_SIDE_INSET + 3.0 * action_step, 188.0, RESET_BUTTON_W, interactive)}
            fontstring {
                name: "EditModeManagerFrameStatus", width: 210.0, height: 32.0,
                text: {state.status.as_str()}, font: GameFont::FrizQuadrata,
                font_size: 12.0, font_color: COLOR_LABEL, justify_h: "LEFT",
                strata: FrameStrata::FullscreenDialog,
                pos_type: "absolute", left: 230.0, top: 154.0,
            }
            {system_checkboxes(state, interactive)}
        }
        {delete_confirmation(state.pending_delete.as_deref())}
    }
}

fn system_checkboxes(state: &EditModePanelState, enabled: bool) -> Element {
    SHOW_SYSTEMS
        .iter()
        .enumerate()
        .flat_map(|(index, (key, label, _))| {
            let name = format!("EditModeManagerShow_{key}");
            let action = format!("edit_mode_show_{key}");
            let unchecked = !system_is_shown(&state.show_systems, key);
            let x = 20.0 + (index % 2) as f32 * 210.0;
            let y = 90.0 + (index / 2) as f32 * 32.0;
            rsx! {
                r#frame {
                    name: {DynName(name.clone())}, width: 210.0, height: 32.0,
                    onclick: {action.as_str()}, mouse_enabled: enabled,
                    strata: FrameStrata::FullscreenDialog,
                    pos_type: "absolute", left: x, top: y,
                    texture {
                        name: {DynName(format!("{name}Up"))}, width: 32.0, height: 32.0,
                        texture_fdid: 130_755,
                        strata: FrameStrata::FullscreenDialog,
                        pos_type: "absolute", left: 0.0, top: 0.0,
                    }
                    texture {
                        name: {DynName(format!("{name}Check"))}, width: 32.0, height: 32.0,
                        texture_fdid: 130_751, hidden: unchecked,
                        strata: FrameStrata::FullscreenDialog,
                        pos_type: "absolute", left: 0.0, top: 0.0,
                    }
                    fontstring {
                        name: {DynName(format!("{name}Label"))}, width: 173.0, height: 32.0,
                        text: {*label}, font: GameFont::FrizQuadrata, font_size: 12.0,
                        font_color: COLOR_TEXT, justify_h: "LEFT",
                        strata: FrameStrata::FullscreenDialog,
                        pos_type: "absolute", left: 37.0, top: 0.0,
                    }
                }
            }
        })
        .collect()
}

fn delete_confirmation(name: Option<&str>) -> Element {
    let Some(name) = name else {
        return Element::default();
    };
    // Retail GlobalStrings HUD_EDIT_MODE_DELETE_LAYOUT_DIALOG_TITLE, Yes / No.
    let text = format!("Are you sure you want to delete the layout\n{name}?");
    rsx! {
        r#frame {
            name: "EditModeDeleteLayoutDialogBlocker",
            stretch: true,
            mouse_enabled: true,
            strata: FrameStrata::FullscreenDialog,
            frame_level: 100.0,
            r#frame {
                name: "EditModeDeleteLayoutDialog",
                width: PANEL_W, height: 136.0,
                mouse_enabled: true,
                strata: FrameStrata::FullscreenDialog,
                frame_level: 101.0,
                pos_type: "absolute", left: "50%", top: "50%",
                translate_x: "-50%", translate_y: "-50%",
                texture {
                    name: "EditModeDeleteLayoutDialogBackground",
                    width: {PANEL_W - 14.0}, height: 122.0,
                    texture_fdid: PANEL_BACKGROUND_FDID,
                    strata: FrameStrata::FullscreenDialog,
                    pos_type: "absolute", left: PANEL_INSET, top: PANEL_INSET,
                }
                r#frame {
                    name: "EditModeDeleteLayoutDialogBorder",
                    width: PANEL_W, height: 136.0, style: PANEL_STYLE,
                    strata: FrameStrata::FullscreenDialog,
                    pos_type: "absolute", left: 0.0, top: 0.0,
                }
                fontstring {
                    name: "EditModeDeleteLayoutDialogText",
                    width: {PANEL_W - 28.0}, height: 54.0,
                    text: {text.as_str()}, font: GameFont::FrizQuadrata,
                    font_size: 14.0, font_color: COLOR_TEXT, justify_h: "CENTER",
                    strata: FrameStrata::FullscreenDialog,
                    pos_type: "absolute", left: 14.0, top: 18.0,
                }
                {panel_button("EditModeDeleteLayoutDialogYes", "Yes", ACTION_EDIT_MODE_CONFIRM_DELETE, 72.0, 90.0, BUTTON_W, true)}
                {panel_button("EditModeDeleteLayoutDialogNo", "No", ACTION_EDIT_MODE_CANCEL_DELETE, 204.0, 90.0, BUTTON_W, true)}
            }
        }
    }
}

/// Prefer Retail's TOP -100; move only the manager to the nearest clear edge candidate.
/// Authored HUD positions, including user-chosen chat/tracker/party defaults, stay untouched.
pub fn find_panel_position(screen: [f32; 2], boxes: &[EditModeSelectionBox]) -> Option<[f32; 2]> {
    let preferred = [(screen[0] - PANEL_W) / 2.0, PANEL_TOP];
    let xs = panel_axis_candidates(preferred[0], screen[0], PANEL_W, boxes, 0);
    let ys = panel_axis_candidates(preferred[1], screen[1], PANEL_H, boxes, 1);
    let mut candidates: Vec<_> = xs
        .iter()
        .flat_map(|x| ys.iter().map(move |y| [*x, *y]))
        .collect();
    let distance = |at: [f32; 2]| (at[0] - preferred[0]).powi(2) + (at[1] - preferred[1]).powi(2);
    candidates.sort_by(|a, b| distance(*a).total_cmp(&distance(*b)));
    candidates.into_iter().find(|at| {
        boxes
            .iter()
            .all(|entry| !rects_overlap([at[0], at[1], PANEL_W, PANEL_H], entry.rect))
    })
}

fn panel_axis_candidates(
    preferred: f32,
    screen: f32,
    size: f32,
    boxes: &[EditModeSelectionBox],
    axis: usize,
) -> Vec<f32> {
    let mut values = vec![preferred, 0.0, screen - size];
    for entry in boxes {
        values.extend([
            entry.rect[axis] - size - PANEL_CLEARANCE,
            entry.rect[axis] + entry.rect[axis + 2] + PANEL_CLEARANCE,
        ]);
    }
    values.retain(|value| *value >= 0.0 && *value + size <= screen);
    values
}

pub fn rects_overlap(a: [f32; 4], b: [f32; 4]) -> bool {
    let horizontal = a[0] < b[0] + b[2] && b[0] < a[0] + a[2];
    let vertical = a[1] < b[1] + b[3] && b[1] < a[1] + a[3];
    horizontal && vertical
}

fn panel_text(name: &'static str, text: &str, size: f32, color: FontColor, top: f32) -> Element {
    rsx! {
        fontstring {
            name: {DynName(name.to_string())},
            width: {PANEL_W - 100.0},
            height: 20.0,
            text,
            font: GameFont::FrizQuadrata,
            font_size: size,
            font_color: color,
            justify_h: "CENTER",
            strata: FrameStrata::FullscreenDialog,
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            top: {top},
        }
    }
}

fn name_input(state: &EditModePanelState) -> Element {
    rsx! {
        editbox {
            name: EDIT_MODE_NAME_INPUT,
            width: 240.0,
            height: 28.0,
            text: state.name_draft.clone(),
            font: GameFont::ArialNarrow,
            font_size: 14.0,
            font_color: COLOR_TEXT,
            max_letters: 32,
            text_insets: "8,4,6,6",
            background_color: "0.10,0.08,0.06,0.8",
            strata: FrameStrata::FullscreenDialog,
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            top: 62.0,
        }
    }
}

fn panel_button(
    name: &'static str,
    text: &str,
    onclick: &str,
    left: f32,
    top: f32,
    width: f32,
    enabled: bool,
) -> Element {
    let disabled = !enabled;
    rsx! {
        button {
            name: {DynName(name.to_string())},
            width: {width},
            height: BUTTON_H,
            text,
            font_size: 13.0,
            onclick,
            disabled,
            strata: FrameStrata::FullscreenDialog,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            pos_type: "absolute",
            left: {left},
            top: {top},
        }
    }
}
