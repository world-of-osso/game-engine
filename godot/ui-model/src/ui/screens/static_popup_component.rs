use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::FrameName;
use crate::ui::popup::{PopupEntry, PopupId, PopupOutcome};
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont};

pub const STATIC_POPUP_ROOT: FrameName = FrameName("StaticPopupRoot");
/// Registered in `panel_styles`: Retail `UI-DiamondDialogBox-Border`.
pub const STATIC_POPUP_PANEL_STYLE: &str = "static_popup";

pub const POPUP_WIDTH: f32 = 320.0;
/// Retail anchors `StaticPopup1` TOP at y -135 of UIParent.
pub const FIRST_POPUP_TOP: f32 = 135.0;

const ACCEPT_ACTION_PREFIX: &str = "static_popup_accept:";
const CANCEL_ACTION_PREFIX: &str = "static_popup_cancel:";

/// `Interface/DialogFrame/UIFrameDialogBoxBackgroundDark` (atlas `UI-DialogBox-Background-Dark`).
const BACKGROUND_FDID: u32 = 6_839_810;
const BACKGROUND_INSET: f32 = 7.0;
const PAD_TOP: f32 = 16.0;
const PAD_BOTTOM: f32 = 16.0;
const TEXT_GAP: f32 = 12.0;
const TEXT_WIDTH: f32 = 290.0;
const FONT_SIZE: f32 = 14.0;
const LINE_HEIGHT: f32 = 18.0;
/// Rough FrizQuadrata advance at 14px, used only to size the popup height.
const AVG_GLYPH_WIDTH: f32 = 7.0;
const BUTTON_W: f32 = 128.0;
const BUTTON_H: f32 = 26.0;
const BUTTON_GAP: f32 = 12.0;
const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";
const COLOR_TEXT: FontColor = FontColor::new(1.0, 1.0, 1.0, 1.0);
/// `StaticPopup1EditBox` (InputBoxTemplate) 130×32 under the text.
const EDITBOX_W: f32 = 130.0;
const EDITBOX_H: f32 = 20.0;
const EDITBOX_GAP: f32 = 8.0;

struct DynName(String);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StaticPopupState {
    pub popups: Vec<PopupEntry>,
}

pub fn popup_frame_name(slot: usize) -> String {
    format!("StaticPopup{}", slot + 1)
}

pub fn popup_action(id: PopupId, outcome: PopupOutcome) -> String {
    let prefix = match outcome {
        PopupOutcome::Accepted => ACCEPT_ACTION_PREFIX,
        PopupOutcome::Cancelled | PopupOutcome::TimedOut => CANCEL_ACTION_PREFIX,
    };
    format!("{prefix}{}", id.0)
}

pub fn parse_popup_action(action: &str) -> Option<(PopupId, PopupOutcome)> {
    let (raw, outcome) = if let Some(raw) = action.strip_prefix(ACCEPT_ACTION_PREFIX) {
        (raw, PopupOutcome::Accepted)
    } else {
        (
            action.strip_prefix(CANCEL_ACTION_PREFIX)?,
            PopupOutcome::Cancelled,
        )
    };
    Some((PopupId(raw.parse().ok()?), outcome))
}

pub fn popup_height(entry: &PopupEntry) -> f32 {
    let editbox = if entry.spec.confirm_text.is_some() {
        EDITBOX_GAP + EDITBOX_H
    } else {
        0.0
    };
    PAD_TOP + text_height(&entry.spec.text) + editbox + TEXT_GAP + BUTTON_H + PAD_BOTTOM
}

fn text_height(text: &str) -> f32 {
    let chars_per_line = (TEXT_WIDTH / AVG_GLYPH_WIDTH).floor().max(1.0) as usize;
    let lines: usize = text
        .split('\n')
        .map(|line| line.chars().count().div_ceil(chars_per_line).max(1))
        .sum();
    lines as f32 * LINE_HEIGHT
}

pub fn static_popup_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<StaticPopupState>()
        .expect("StaticPopupState must be in SharedContext");
    let mut top = FIRST_POPUP_TOP;
    let popups: Element = state
        .popups
        .iter()
        .enumerate()
        .flat_map(|(slot, entry)| {
            let popup = static_popup(slot, entry, top);
            top += popup_height(entry);
            popup
        })
        .collect();
    rsx! {
        r#frame {
            name: STATIC_POPUP_ROOT,
            stretch: true,
            strata: FrameStrata::Dialog,
            {popups}
        }
    }
}

fn static_popup(slot: usize, entry: &PopupEntry, top: f32) -> Element {
    let name = popup_frame_name(slot);
    let height = popup_height(entry);
    rsx! {
        r#frame {
            name: {DynName(name.clone())},
            width: POPUP_WIDTH,
            height: {height},
            strata: FrameStrata::Dialog,
            mouse_enabled: true,
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            top: {top},
            {popup_background(&name, height)}
            {popup_border(&name, height)}
            {popup_text(&name, entry)}
            {popup_editbox(&name, entry)}
            {popup_buttons(&name, entry)}
        }
    }
}

fn popup_background(name: &str, height: f32) -> Element {
    rsx! {
        texture {
            name: {DynName(format!("{name}Background"))},
            width: {POPUP_WIDTH - 2.0 * BACKGROUND_INSET},
            height: {height - 2.0 * BACKGROUND_INSET},
            texture_fdid: BACKGROUND_FDID,
            strata: FrameStrata::Dialog,
            pos_type: "absolute",
            left: BACKGROUND_INSET,
            top: BACKGROUND_INSET,
        }
    }
}

fn popup_border(name: &str, height: f32) -> Element {
    rsx! {
        r#frame {
            name: {DynName(format!("{name}Border"))},
            width: POPUP_WIDTH,
            height: {height},
            style: STATIC_POPUP_PANEL_STYLE,
            strata: FrameStrata::Dialog,
            frame_level: 5.0,
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
        }
    }
}

fn popup_text(name: &str, entry: &PopupEntry) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("{name}Text"))},
            width: TEXT_WIDTH,
            height: {text_height(&entry.spec.text)},
            text: entry.spec.text.as_str(),
            font: GameFont::FrizQuadrata,
            font_size: FONT_SIZE,
            font_color: COLOR_TEXT,
            justify_h: "CENTER",
            strata: FrameStrata::Dialog,
            frame_level: 6.0,
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            top: PAD_TOP,
        }
    }
}

/// The typed word of a `confirm_text` popup, in a bordered box under the text.
fn popup_editbox(name: &str, entry: &PopupEntry) -> Element {
    if entry.spec.confirm_text.is_none() {
        return Element::default();
    }
    let name = format!("{name}EditBox");
    rsx! {
        r#frame {
            name: {DynName(name.clone())},
            width: EDITBOX_W,
            height: EDITBOX_H,
            background_color: "0.0,0.0,0.0,0.8",
            border: "1px solid 0.6,0.6,0.6,1.0",
            strata: FrameStrata::Dialog,
            frame_level: 8.0,
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            top: {PAD_TOP + text_height(&entry.spec.text) + EDITBOX_GAP},
            fontstring {
                name: {DynName(format!("{name}Text"))},
                width: {EDITBOX_W - 8.0},
                height: EDITBOX_H,
                text: entry.typed.as_str(),
                font: GameFont::FrizQuadrata,
                font_size: 13.0,
                font_color: COLOR_TEXT,
                justify_h: "LEFT",
                strata: FrameStrata::Dialog,
                frame_level: 9.0,
                pos_type: "absolute",
                left: 4.0,
                top: 0.0,
            }
        }
    }
}

fn popup_buttons(name: &str, entry: &PopupEntry) -> Element {
    // Button1 stays disabled until the confirm word is typed (`OnTextChanged`).
    let accept_action = if entry.can_accept() {
        popup_action(entry.id, PopupOutcome::Accepted)
    } else {
        String::new()
    };
    let Some(cancel_label) = &entry.spec.cancel_label else {
        return popup_button(
            format!("{name}Button1"),
            &entry.spec.accept_label,
            accept_action,
            -BUTTON_W / 2.0,
            entry.can_accept(),
        );
    };
    let cancel_action = popup_action(entry.id, PopupOutcome::Cancelled);
    let mut buttons = popup_button(
        format!("{name}Button1"),
        &entry.spec.accept_label,
        accept_action,
        -BUTTON_W - BUTTON_GAP / 2.0,
        entry.can_accept(),
    );
    buttons.extend(popup_button(
        format!("{name}Button2"),
        cancel_label,
        cancel_action,
        BUTTON_GAP / 2.0,
        true,
    ));
    buttons
}

fn popup_button(
    name: String,
    text: &str,
    onclick: String,
    offset_x: f32,
    enabled: bool,
) -> Element {
    let disabled = !enabled;
    rsx! {
        button {
            name: {DynName(name)},
            width: BUTTON_W,
            height: BUTTON_H,
            text,
            font_size: 13.0,
            onclick,
            disabled,
            strata: FrameStrata::Dialog,
            frame_level: 10.0,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            pos_type: "absolute",
            left: "50%",
            top: "100%",
            translate_y: "-100%",
            margin_left: offset_x,
            margin_top: {-PAD_BOTTOM},
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_action_round_trips() {
        let id = PopupId(42);
        for outcome in [PopupOutcome::Accepted, PopupOutcome::Cancelled] {
            assert_eq!(
                parse_popup_action(&popup_action(id, outcome)),
                Some((id, outcome))
            );
        }
        assert_eq!(parse_popup_action("menu_resume"), None);
    }

    #[test]
    fn a_confirm_word_popup_shows_the_typed_text_and_enables_yes_once_it_matches() {
        use crate::ui::popup::{PopupSpec, PopupStack};
        use ui_toolkit::registry::FrameRegistry;
        use ui_toolkit::screen::Screen;

        let mut stack = PopupStack::default();
        stack.push(PopupSpec {
            key: "DELETE_GOOD_ITEM".into(),
            text: "Do you want to destroy Martin Fury?".into(),
            accept_label: "Yes".into(),
            cancel_label: Some("No".into()),
            timeout: None,
            confirm_text: Some("DELETE".into()),
        });
        let build = |stack: &PopupStack| {
            let mut registry = FrameRegistry::new(1920.0, 1080.0);
            let mut shared = SharedContext::new();
            shared.insert(StaticPopupState {
                popups: stack.visible(),
            });
            Screen::new(static_popup_screen).sync(&shared, &mut registry);
            registry
        };
        let onclick = |registry: &FrameRegistry, name: &str| {
            registry
                .get(registry.get_by_name(name).unwrap())
                .unwrap()
                .onclick
                .clone()
                .unwrap_or_default()
        };
        let registry = build(&stack);
        assert!(registry.get_by_name("StaticPopup1EditBox").is_some());
        assert_eq!(onclick(&registry, "StaticPopup1Button1"), "");

        stack.typing_target().unwrap().typed.push_str("delete");
        let registry = build(&stack);
        assert_eq!(
            crate::ui::screens::screen_test_helpers::fontstring_text(
                &registry,
                "StaticPopup1EditBoxText"
            ),
            "delete"
        );
        assert_ne!(onclick(&registry, "StaticPopup1Button1"), "");
    }
}
