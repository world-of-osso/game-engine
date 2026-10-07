//! HUD edit mode: per-element selection boxes and the layout manager panel
//! (Retail `EditModeManagerFrame`-like).

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

/// `Interface/EditMode/EditModeUIHighlightBackground.blp`: unselected element.
const HIGHLIGHT_FDID: u32 = 4_554_383;
/// `Interface/EditMode/EditModeUISelectedBackground.blp`: selected element.
const SELECTED_FDID: u32 = 4_554_386;
/// `Interface/DialogFrame/UIFrameDialogBoxBackgroundDark`, as the static popups.
const PANEL_BACKGROUND_FDID: u32 = 6_839_810;
const PANEL_STYLE: &str = crate::static_popup_component::STATIC_POPUP_PANEL_STYLE;

pub const PANEL_W: f32 = 380.0;
pub const PANEL_H: f32 = 228.0;
/// Retail EditModeManager.xml:6: TOP of UIParent at y=-100.
pub const PANEL_TOP: f32 = 100.0;
pub const PANEL_CLEARANCE: f32 = 8.0;
pub const PANEL_TITLE_H: f32 = 32.0;
const PANEL_INSET: f32 = 7.0;
const BUTTON_W: f32 = 104.0;
const BUTTON_H: f32 = 26.0;
const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";
const COLOR_TITLE: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const COLOR_TEXT: FontColor = FontColor::new(1.0, 1.0, 1.0, 1.0);
const COLOR_LABEL: FontColor = FontColor::new(0.9, 0.9, 0.9, 1.0);

struct DynName(String);

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
    pub layout_name: String,
    pub name_draft: String,
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
    let label = if entry.selected {
        entry.label.as_str()
    } else {
        "Click to edit"
    };
    let fdid = if entry.selected {
        SELECTED_FDID
    } else {
        HIGHLIGHT_FDID
    };
    rsx! {
        r#frame {
            name: {DynName(name.clone())},
            width: {w},
            height: {h},
            strata: FrameStrata::Fullscreen,
            pos_type: "absolute",
            left: {x},
            top: {y},
            texture {
                name: {DynName(format!("{name}Background"))},
                width: {w},
                height: {h},
                texture_fdid: fdid,
                // Pinned rsx LitFloat truncates decimals; retire expression form once fixed.
                alpha: {0.7},
                strata: FrameStrata::Fullscreen,
                pos_type: "absolute",
                left: 0.0,
                top: 0.0,
            }
            r#frame {
                name: {DynName(format!("{name}LabelBacking"))},
                hidden: hide_label,
                width: {w}, height: {label_height},
                background_color: "0.0,0.0,0.0,1.0",
                strata: FrameStrata::Fullscreen,
                pos_type: "absolute", left: 0.0,
                top: "50%", translate_y: "-50%",
            }
            fontstring {
                name: {DynName(format!("{name}Label"))},
                hidden: hide_label,
                width: {w},
                height: {label_height},
                text: label,
                font: GameFont::FrizQuadrata,
                font_size: 13.0,
                font_color: COLOR_TEXT,
                justify_h: "CENTER",
                strata: FrameStrata::Fullscreen,
                pos_type: "absolute",
                left: 0.0,
                top: "50%",
                translate_y: "-50%",
            }
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
    let user_layout = !state.preset;
    let left = state
        .position
        .map_or_else(|| "50%".to_string(), |at| at[0].to_string());
    let translate_x = if state.position.is_some() {
        "0"
    } else {
        "-50%"
    };
    let top = state.position.map_or(PANEL_TOP, |at| at[1]);
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
            {panel_button("EditModeManagerFramePrev", "<", ACTION_EDIT_MODE_PREV_LAYOUT, 20.0, 36.0, 30.0, true)}
            {panel_button("EditModeManagerFrameNext", ">", ACTION_EDIT_MODE_NEXT_LAYOUT, PANEL_W - 50.0, 36.0, 30.0, true)}
            {name_input(state)}
            {panel_button("EditModeManagerFrameNew", "New", ACTION_EDIT_MODE_NEW, 22.0, 108.0, BUTTON_W, true)}
            {panel_button("EditModeManagerFrameRename", "Rename", ACTION_EDIT_MODE_RENAME, 138.0, 108.0, BUTTON_W, user_layout)}
            {panel_button("EditModeManagerFrameDelete", "Delete", ACTION_EDIT_MODE_DELETE, 254.0, 108.0, BUTTON_W, user_layout)}
            {panel_button("EditModeManagerFrameRevert", "Revert", ACTION_EDIT_MODE_REVERT, 22.0, 140.0, BUTTON_W, state.dirty)}
            {panel_button("EditModeManagerFrameSave", "Save", ACTION_EDIT_MODE_SAVE, 138.0, 140.0, BUTTON_W, state.dirty)}
            {panel_button("EditModeManagerFrameExit", "Exit", ACTION_EDIT_MODE_EXIT, 254.0, 140.0, BUTTON_W, true)}
            {panel_button("EditModeManagerFrameReset", "Reset Selected", ACTION_EDIT_MODE_RESET, 22.0, 172.0, 150.0, true)}
            {panel_text("EditModeManagerFrameStatus", &state.status, 12.0, COLOR_LABEL, 204.0)}
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
            top: 70.0,
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

/// Bars 4/5 have no native gameplay consumer yet. Their empty, non-interactive preview
/// roots exist only in the editor, so the same mover registry can position them without
/// adding gameplay bars or changing either preset's visibility.
pub fn edit_mode_side_bar_previews(ctx: &SharedContext) -> Element {
    let skin = ctx
        .get::<ui_toolkit::atlas::ActiveSkin>()
        .copied()
        .unwrap_or_default();
    let scale = if skin == ui_toolkit::atlas::ActiveSkin::Forever {
        crate::hud_layout::FOREVER_ACTION_BUTTON_SCALE
    } else {
        1.0
    };
    [
        side_bar_preview("MultiBarRight", 6.0, scale),
        side_bar_preview("MultiBarLeft", 53.0, scale),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn side_bar_preview(name: &str, right: f32, scale: f32) -> Element {
    use crate::main_action_bar_component::{BUTTON_PADDING, BUTTON_SIZE, MAIN_BAR_BUTTONS};
    let size = BUTTON_SIZE * scale;
    let pitch = (BUTTON_SIZE + BUTTON_PADDING) * scale;
    let height = MAIN_BAR_BUTTONS as f32 * pitch - BUTTON_PADDING * scale;
    let slots: Element = (0..MAIN_BAR_BUTTONS)
        .flat_map(|index| {
            rsx! {
                texture {
                    name: {DynName(format!("{name}PreviewSlot{index}"))},
                    width: {size}, height: {size},
                    texture_atlas: "UI-HUD-ActionBar-IconFrame",
                    pos_type: "absolute", left: 0.0, top: {index as f32 * pitch},
                    strata: FrameStrata::FullscreenDialog,
                }
            }
        })
        .collect();
    rsx! {
        r#frame {
            name: {DynName(name.into())},
            width: {size}, height,
            pos_type: "absolute", right, top: "50%", translate_y: "-50%",
            strata: FrameStrata::FullscreenDialog,
            {slots}
        }
    }
}
