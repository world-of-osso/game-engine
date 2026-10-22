use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::FrameName;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont};

use super::char_select_component::CharSelectAction;

pub const DELETE_CONFIRM_DIALOG: FrameName = FrameName("DeleteCharacterDialog");
pub const DELETE_CONFIRM_INPUT: FrameName = FrameName("DeleteCharacterConfirmInput");
pub const DELETE_CONFIRM_BUTTON: FrameName = FrameName("DeleteCharacterConfirmButton");
pub const DELETE_CANCEL_BUTTON: FrameName = FrameName("DeleteCharacterCancelButton");

const COLOR_GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeleteConfirmUiState {
    pub visible: bool,
    pub character_name: String,
    pub typed_text: String,
    pub countdown_text: String,
    pub confirm_enabled: bool,
}

pub const DELETE_CONFIRM_TOKEN: &str = "DELETE";
pub const DELETE_CONFIRM_DELAY_SECS: f32 = 3.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeleteCharacterTarget {
    pub character_id: u64,
    pub name: String,
}

/// Delay and typed-token gate a player must pass before a character is deleted.
#[derive(Clone, Debug, Default)]
pub struct DeleteConfirmation {
    pub target: Option<DeleteCharacterTarget>,
    pub typed_text: String,
    pub elapsed_secs: f32,
}

impl DeleteConfirmation {
    pub fn open(&mut self, target: DeleteCharacterTarget) {
        *self = Self {
            target: Some(target),
            ..Self::default()
        };
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn tick(&mut self, dt: f32) {
        if self.target.is_some() {
            self.elapsed_secs += dt.max(0.0);
        }
    }

    pub fn ready(&self) -> bool {
        self.target.is_some()
            && self.elapsed_secs >= DELETE_CONFIRM_DELAY_SECS
            && self.typed_text.eq_ignore_ascii_case(DELETE_CONFIRM_TOKEN)
    }

    pub fn ui_state(&self) -> DeleteConfirmUiState {
        let Some(target) = self.target.as_ref() else {
            return DeleteConfirmUiState::default();
        };
        let remaining = if self.elapsed_secs >= DELETE_CONFIRM_DELAY_SECS {
            0
        } else {
            (DELETE_CONFIRM_DELAY_SECS - self.elapsed_secs).ceil() as u8
        };
        let countdown_text = if remaining > 0 {
            format!("Delete unlocks in {remaining}s")
        } else if self.typed_text.eq_ignore_ascii_case(DELETE_CONFIRM_TOKEN) {
            "Ready. Press Delete Forever to remove this character.".to_string()
        } else {
            format!("Type {DELETE_CONFIRM_TOKEN} to enable deletion")
        };
        DeleteConfirmUiState {
            visible: true,
            character_name: target.name.clone(),
            typed_text: self.typed_text.clone(),
            countdown_text,
            confirm_enabled: self.ready(),
        }
    }
}

pub fn delete_confirmation_modal(state: &DeleteConfirmUiState) -> Element {
    if !state.visible {
        return Vec::new();
    }
    // GlueDialog.xml Container512x256; GlueDialogMixin uses CENTER and23px text inset.
    rsx! {
        r#frame { name: "DeleteCharacterOverlay", stretch: true, strata: FrameStrata::Dialog,
            mouse_enabled: true, background_color: "0,0,0,0.7", }
        r#frame {
            name: DELETE_CONFIRM_DIALOG, width: 512.0, height: 256.0, strata: FrameStrata::Dialog,
            mouse_enabled: true, pos_type: "absolute", left: "50%", top: "50%",
            translate_x: "-50%", translate_y: "-50%",
            texture { name: "DeleteCharacterDialogBackground", texture_atlas: "UI-Frame-DialogBox-BackgroundTile",
                width: "auto", height: "auto", pos_type: "absolute", left: 8.0, right: 8.0, top: 9.0, bottom: 9.0, }
            {crate::game_menu_main::sliced_atlas("DeleteCharacterDialogBorder", "UI-DiamondDialogBox-Border", [512.0,256.0], [32.0;4])}
            r#frame {
                name: "DeleteCharacterDialogText", width: 450.0, height: "auto", layout: "flex-column", gap: 2.0,
                pos_type: "absolute", left: "50%", top: 23.0, translate_x: "-50%",
                {dialog_label("DeleteCharacterDialogTitle", "Delete Character", 18.0)}
                {dialog_label("DeleteCharacterDialogWarning", &format!("This will permanently delete {}.", state.character_name), 18.0)}
                {dialog_label("DeleteCharacterDialogCountdown", &state.countdown_text, 12.0)}
            }
            {delete_confirm_editbox(state)}
            {modal_button(DELETE_CONFIRM_BUTTON, "Delete Forever", state.confirm_enabled.then_some(CharSelectAction::ConfirmDeleteChar), -206.0)}
            {modal_button(DELETE_CANCEL_BUTTON, "Cancel", Some(CharSelectAction::CancelDeleteChar), 9.0)}
        }
    }
}

struct DynName(String);
fn dialog_label(name: &str, text: &str, size: f32) -> Element {
    rsx! { fontstring { name: DynName(name.into()), width: 450.0, height: "auto", text,
    font: GameFont::FrizQuadrata, font_size: size, font_color: COLOR_GOLD, } }
}

fn delete_confirm_editbox(state: &DeleteConfirmUiState) -> Element {
    // GlueDialog editbox base150x32 CENTER, Instructions BOTTOM->TOP+3.
    rsx! {
        editbox {
            name: DELETE_CONFIRM_INPUT, width: 150.0, height: 32.0, text: state.typed_text.clone(),
            font: GameFont::FrizQuadrata, font_size: 15.0, font_color: COLOR_GOLD,
            max_letters: 6, text_insets: "12,5,0,5", background_color: "0,0,0,0", pos_type: "absolute",
            left: "50%", top: "50%", translate_x: "-50%", translate_y: "-50%",
            {tooltip_input_border()}
            fontstring { name: "DeleteCharacterDialogHelper", width: 150.0, height: "auto", text: "Type DELETE to confirm",
                font: GameFont::FrizQuadrata, font_size: 12.0, font_color: COLOR_GOLD,
                pos_type: "absolute", left: 0.0, bottom: "100%", margin_bottom: 3.0, }
        }
    }
}

fn tooltip_input_border() -> Element {
    // TooltipDefaultLayout in NineSliceLayouts.lua; source corner7px.
    use crate::quest_art::named_atlas_texture;
    const W: f32 = 150.0;
    const H: f32 = 32.0;
    const C: f32 = 7.0;
    [
        (
            "Center",
            "Tooltip-NineSlice-Center",
            (3.0, 3.0, W - 6.0, H - 6.0),
        ),
        (
            "TopLeft",
            "Tooltip-NineSlice-CornerTopLeft",
            (0.0, 0.0, C, C),
        ),
        (
            "TopRight",
            "Tooltip-NineSlice-CornerTopRight",
            (W - C, 0.0, C, C),
        ),
        (
            "BottomLeft",
            "Tooltip-NineSlice-CornerBottomLeft",
            (0.0, H - C, C, C),
        ),
        (
            "BottomRight",
            "Tooltip-NineSlice-CornerBottomRight",
            (W - C, H - C, C, C),
        ),
        (
            "Top",
            "_Tooltip-NineSlice-EdgeTop",
            (C, 0.0, W - 2.0 * C, C),
        ),
        (
            "Bottom",
            "_Tooltip-NineSlice-EdgeBottom",
            (C, H - C, W - 2.0 * C, C),
        ),
        (
            "Left",
            "!Tooltip-NineSlice-EdgeLeft",
            (0.0, C, C, H - 2.0 * C),
        ),
        (
            "Right",
            "!Tooltip-NineSlice-EdgeRight",
            (W - C, C, C, H - 2.0 * C),
        ),
    ]
    .into_iter()
    .flat_map(|(part, atlas, rect)| {
        named_atlas_texture(format!("DeleteCharacterInput{part}"), atlas, rect)
    })
    .collect()
}

fn modal_button(name: FrameName, text: &str, action: Option<CharSelectAction>, x: f32) -> Element {
    let disabled = action.is_none();
    let onclick = action.map(|action| action.to_string()).unwrap_or_default();
    rsx! {
        button {
            name, width: 200.0, height: 30.0, text, font_size: 15.0, onclick, disabled,
            button_template: "SharedButtonTemplate",
            pos_type: "absolute", left: "50%", top: "100%", translate_y: "-100%",
            margin_left: x, margin_top: -18.0,
        }
    }
}

pub(super) fn sync_glue_dialog_art(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::frame::WidgetData;
    for name in [
        "DeleteCharacterInputCenter",
        "DeleteCharacterDialogBackground",
    ] {
        if let Some(id) = registry.get_by_name(name)
            && let Some(frame) = registry.get_mut(id)
            && let Some(WidgetData::Texture(texture)) = frame.widget_data.as_mut()
        {
            if name == "DeleteCharacterInputCenter" {
                // TooltipBackdropTemplateMixin: TOOLTIP_DEFAULT_BACKGROUND_COLOR.
                texture.vertex_color = [0.09, 0.09, 0.188, 1.0];
            }
            texture.horiz_tile = true;
            texture.vert_tile = true;
        }
    }
}
