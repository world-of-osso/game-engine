use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::screens::guild_frame_component::ACTION_GUILD_TOGGLE;

use super::inworld_hud_art::{MICRO_BUTTON_BG, SheetCrop, micro_button_icon};
use super::{
    DynName, MICRO_BTN_GAP, MICRO_BTN_H, MICRO_BTN_W, MICRO_BUTTONS, MICRO_MENU_BOTTOM,
    MICRO_MENU_RIGHT,
};

pub(super) fn micro_menu_bar() -> Element {
    let total_w = micro_menu_bar_width();
    let buttons: Element = MICRO_BUTTONS
        .iter()
        .enumerate()
        .flat_map(|(i, name)| micro_button(i, name))
        .collect();
    rsx! {
        r#frame {
            name: "MicroMenuContainer",
            width: {total_w},
            height: {MICRO_BTN_H},
            pos_type: "absolute",
            right: {MICRO_MENU_RIGHT},
            bottom: {MICRO_MENU_BOTTOM},
            {buttons}
        }
    }
}

const MICRO_ART_H: f32 = 41.0;

fn micro_menu_bar_width() -> f32 {
    MICRO_BUTTONS.len() as f32 * MICRO_BTN_W + (MICRO_BUTTONS.len() - 1) as f32 * MICRO_BTN_GAP
}

fn micro_button(index: usize, name: &str) -> Element {
    let btn_name = DynName(name.to_string());
    let onclick = micro_button_action(name);
    let x = index as f32 * (MICRO_BTN_W + MICRO_BTN_GAP);
    let layers: Element = [Some(MICRO_BUTTON_BG), micro_button_icon(name)]
        .into_iter()
        .flatten()
        .enumerate()
        .flat_map(|(layer, crop)| micro_button_layer(name, layer, crop))
        .collect();
    rsx! {
        button {
            name: btn_name,
            width: {MICRO_BTN_W},
            height: {MICRO_BTN_H},
            text: "",
            font_size: 8.0,
            onclick: {onclick},
            pos_type: "absolute",
            left: {x},
            top: -0.0,
            {layers}
        }
    }
}

/// Retail draws the 32x41 atlas art centred on the 32x40 button.
fn micro_button_layer(button: &str, layer: usize, crop: SheetCrop) -> Element {
    let name = DynName(format!("{button}Art{layer}"));
    let coords = crop.tex_coords();
    rsx! {
        texture {
            name,
            width: {MICRO_BTN_W},
            height: {MICRO_ART_H},
            texture_fdid: {crop.fdid},
            tex_coords: {coords.as_str()},
            pos_type: "absolute",
            left: 0.0,
            top: {(MICRO_BTN_H - MICRO_ART_H) / 2.0},
        }
    }
}

fn micro_button_action(name: &str) -> &'static str {
    match name {
        "GuildMicroButton" => ACTION_GUILD_TOGGLE,
        _ => "",
    }
}
