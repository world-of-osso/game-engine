use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont};

use super::char_select_component::CharSelectAction;

pub const DELETE_CONFIRM_DIALOG: FrameName = FrameName("DeleteCharacterDialog");
pub const DELETE_CONFIRM_INPUT: FrameName = FrameName("DeleteCharacterConfirmInput");
pub const DELETE_CONFIRM_BUTTON: FrameName = FrameName("DeleteCharacterConfirmButton");
pub const DELETE_CANCEL_BUTTON: FrameName = FrameName("DeleteCharacterCancelButton");

const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";
const COLOR_GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const COLOR_SUBTITLE: FontColor = FontColor::new(0.92, 0.88, 0.74, 1.0);
const DELETE_DIALOG_BG: &str = "0.04,0.03,0.02,0.98";
const DELETE_DIALOG_OVERLAY: &str = "0.0,0.0,0.0,0.65";
const DELETE_HELPER: FontColor = FontColor::new(0.96, 0.92, 0.8, 1.0);
const DELETE_WARNING: FontColor = FontColor::new(0.93, 0.4, 0.35, 1.0);
const INPUT_BORDER_TEXTURES: [&str; 9] = [
    "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-TL.blp",
    "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-T.blp",
    "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-TR.blp",
    "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-L.blp",
    "data/textures/editbox-white-fill.ktx2",
    "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-R.blp",
    "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-BL.blp",
    "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-B.blp",
    "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-BR.blp",
];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeleteConfirmUiState {
    pub visible: bool,
    pub character_name: String,
    pub typed_text: String,
    pub countdown_text: String,
    pub confirm_enabled: bool,
}

pub fn delete_confirmation_modal(state: &DeleteConfirmUiState) -> Element {
    if !state.visible {
        return Vec::new();
    }

    rsx! {
        r#frame {
            name: "DeleteCharacterOverlay",
            stretch: true,
            strata: FrameStrata::Dialog,
            mouse_enabled: true,
            background_color: DELETE_DIALOG_OVERLAY,
        }
        r#frame {
            name: DELETE_CONFIRM_DIALOG,
            width: 420.0,
            height: 278.0,
            strata: FrameStrata::Dialog,
            mouse_enabled: true,
            background_color: DELETE_DIALOG_BG,
            border: "1,0.82,0,0.35",
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
                y: "-14",
            }
            {dialog_title()}
            {dialog_warning(state)}
            {dialog_helper()}
            {delete_confirm_editbox(state)}
            {dialog_countdown(state)}
            {delete_cancel_button()}
            {delete_confirm_button(state)}
        }
    }
}

fn dialog_title() -> Element {
    rsx! {
        fontstring {
            name: "DeleteCharacterDialogTitle",
            width: 340.0,
            height: 28.0,
            text: "Delete Character",
            font: GameFont::FrizQuadrata,
            font_size: 22.0,
            font_color: COLOR_GOLD,
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                y: "-22",
            }
        }
    }
}

fn dialog_warning(state: &DeleteConfirmUiState) -> Element {
    rsx! {
        fontstring {
            name: "DeleteCharacterDialogWarning",
            width: 340.0,
            height: 40.0,
            text: {format!("This will permanently delete {}.", state.character_name)},
            font: GameFont::FrizQuadrata,
            font_size: 16.0,
            font_color: DELETE_WARNING,
            anchor {
                point: AnchorPoint::Top,
                relative_to: FrameName("DeleteCharacterDialogTitle"),
                relative_point: AnchorPoint::Bottom,
                y: "-18",
            }
        }
    }
}

fn dialog_helper() -> Element {
    rsx! {
        fontstring {
            name: "DeleteCharacterDialogHelper",
            width: 340.0,
            height: 22.0,
            text: "Type DELETE to confirm",
            font: GameFont::FrizQuadrata,
            font_size: 14.0,
            font_color: DELETE_HELPER,
            anchor {
                point: AnchorPoint::Top,
                relative_to: FrameName("DeleteCharacterDialogWarning"),
                relative_point: AnchorPoint::Bottom,
                y: "-14",
            }
        }
    }
}

fn delete_confirm_editbox(state: &DeleteConfirmUiState) -> Element {
    rsx! {
        editbox {
            name: DELETE_CONFIRM_INPUT,
            width: 240.0,
            height: 38.0,
            text: state.typed_text.clone(),
            font: GameFont::ArialNarrow,
            font_size: 18.0,
            font_color: COLOR_GOLD,
            max_letters: 6,
            text_insets: "12,5,8,8",
            background_color: "0.14,0.10,0.07,0.5",
            nine_slice {
                edge_size: 8,
                bg_color: "0.14,0.10,0.07,0.5",
                border_color: "1.0,0.82,0.0,1.0",
                textures: {INPUT_BORDER_TEXTURES.map(str::to_string)},
            }
            anchor {
                point: AnchorPoint::Top,
                relative_to: DELETE_CONFIRM_DIALOG,
                relative_point: AnchorPoint::Top,
                y: "-154",
            }
        }
    }
}

fn dialog_countdown(state: &DeleteConfirmUiState) -> Element {
    rsx! {
        fontstring {
            name: "DeleteCharacterDialogCountdown",
            width: 300.0,
            height: 22.0,
            text: state.countdown_text.clone(),
            font: GameFont::FrizQuadrata,
            font_size: 13.0,
            font_color: COLOR_SUBTITLE,
            anchor {
                point: AnchorPoint::Top,
                relative_to: DELETE_CONFIRM_INPUT,
                relative_point: AnchorPoint::Bottom,
                y: "-12",
            }
        }
    }
}

fn delete_confirm_button(state: &DeleteConfirmUiState) -> Element {
    if state.confirm_enabled {
        modal_button(
            DELETE_CONFIRM_BUTTON,
            "Delete Forever",
            Some(CharSelectAction::ConfirmDeleteChar),
            AnchorPoint::BottomRight,
            -12.0,
        )
    } else {
        modal_button(
            DELETE_CONFIRM_BUTTON,
            "Delete Forever",
            None,
            AnchorPoint::BottomRight,
            -12.0,
        )
    }
}

fn delete_cancel_button() -> Element {
    modal_button(
        DELETE_CANCEL_BUTTON,
        "Cancel",
        Some(CharSelectAction::CancelDeleteChar),
        AnchorPoint::BottomLeft,
        12.0,
    )
}

fn modal_button(
    name: FrameName,
    text: &str,
    action: Option<CharSelectAction>,
    point: AnchorPoint,
    x: f32,
) -> Element {
    let disabled = action.is_none();
    let onclick = action.map(|action| action.to_string()).unwrap_or_default();
    rsx! {
        button {
            name,
            width: 168.0,
            height: 40.0,
            text,
            font_size: 14.0,
            onclick,
            disabled: disabled,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor {
                point,
                relative_to: DELETE_CONFIRM_DIALOG,
                relative_point: AnchorPoint::Bottom,
                x: {x.to_string()},
                y: "18",
            }
        }
    }
}
