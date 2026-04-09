use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;

const OPTIONS_CONTENT_W: f32 = 716.0;
const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub fn controls_body() -> Element {
    section_stack(
        [
            info_row(
                "controls_turn",
                "Mouse Turn Style",
                "Classic hold-to-turn behavior",
            ),
            ghost_button_row(
                "controls_bindings",
                "Binding Groups",
                "Full binding editor lives in Keybindings",
            ),
        ]
        .into_iter()
        .flatten()
        .collect(),
    )
}

pub fn macros_body() -> Element {
    section_stack(
        [
            ghost_button_row(
                "macros_general",
                "General Macros",
                "Macro list editor planned",
            ),
            ghost_button_row(
                "macros_character",
                "Character Macros",
                "Per-character macro storage planned",
            ),
            info_row(
                "macros_exec",
                "Execution",
                "Command parser hooks will live here",
            ),
        ]
        .into_iter()
        .flatten()
        .collect(),
    )
}

pub fn social_addons_body() -> Element {
    section_stack(
        [
            info_row(
                "social_addons",
                "Addon Directory",
                "Load `.js` addons from `addons/` and hot-reload on save",
            ),
            info_row(
                "social_api",
                "Addon API",
                "`addon.createFrame`, `createFontString`, `setPoint`, `setSize`, `setText`, `show`, `hide`, and color helpers are live",
            ),
            info_row(
                "social_compat",
                "Compatibility",
                "The legacy `.wasm` host is still stubbed; live UI customization uses QuickJS-backed `.js` addons today",
            ),
        ]
        .into_iter()
        .flatten()
        .collect(),
    )
}

pub fn support_body() -> Element {
    section_stack(
        [
            info_row(
                "support_about",
                "About",
                "Standalone WoW-style client renderer on Bevy 0.18",
            ),
            info_row(
                "support_help",
                "Help",
                "Support flow will point to project docs and issue links",
            ),
            ghost_button_row(
                "support_contact",
                "Open Support",
                "External support links are not wired in-client",
            ),
        ]
        .into_iter()
        .flatten()
        .collect(),
    )
}

pub fn info_row(key: &str, label: &str, detail: &str) -> Element {
    rsx! {
        r#frame {
            name: {DynName(format!("InfoRow{key}"))},
            width: {OPTIONS_CONTENT_W - 30.0},
            height: 34.0,
            {row_label(key, label)}
            {info_detail(key, detail)}
        }
    }
}

pub fn ghost_button_row(key: &str, label: &str, detail: &str) -> Element {
    rsx! {
        r#frame {
            name: {DynName(format!("GhostRow{key}"))},
            width: {OPTIONS_CONTENT_W - 30.0},
            height: 34.0,
            {row_label(key, label)}
            {ghost_detail(key, detail)}
            {disabled_button(key)}
        }
    }
}

fn section_stack(rows: Element) -> Element {
    rsx! {
        r#frame {
            width: {OPTIONS_CONTENT_W - 30.0},
            height: 0.0,
            layout: "flex-column",
            gap: 12.0,
            {rows}
        }
    }
}

fn row_label(key: &str, text: &str) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("RowLabel{key}"))},
            width: 236.0,
            height: 20.0,
            text: {text},
            font_size: 16.0,
            color: "0.95,0.90,0.74,1.0",
            justify_h: "LEFT",
            anchor { point: AnchorPoint::Left, relative_point: AnchorPoint::Left }
        }
    }
}

fn info_detail(key: &str, detail: &str) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("InfoDetail{key}"))},
            width: 370.0,
            height: 28.0,
            text: {detail},
            font_size: 13.0,
            color: "0.72,0.72,0.72,1.0",
            justify_h: "RIGHT",
            anchor {
                point: AnchorPoint::Right,
                relative_point: AnchorPoint::Right,
                x: "-8",
            }
        }
    }
}

fn ghost_detail(key: &str, detail: &str) -> Element {
    let name = DynName(format!("GhostDetail{key}"));
    let button_name = DynName(format!("GhostButton{key}"));

    rsx! {
        fontstring {
            name: {name},
            width: 270.0,
            height: 28.0,
            text: {detail},
            font_size: 13.0,
            color: "0.72,0.72,0.72,1.0",
            justify_h: "RIGHT",
            anchor {
                point: AnchorPoint::Right,
                relative_to: {button_name},
                relative_point: AnchorPoint::Left,
                x: "-8",
            }
        }
    }
}

fn disabled_button(key: &str) -> Element {
    rsx! {
        button {
            name: {DynName(format!("GhostButton{key}"))},
            width: 84.0,
            height: 30.0,
            text: "Open",
            font_size: 14.0,
            disabled: true,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor {
                point: AnchorPoint::Right,
                relative_point: AnchorPoint::Right,
                x: "-4",
            }
        }
    }
}
