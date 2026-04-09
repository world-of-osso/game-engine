use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::screens::guild_frame_component::ACTION_GUILD_TOGGLE;

use super::{
    AnchorPoint, DynName, MICRO_BTN_BG, MICRO_BTN_GAP, MICRO_BTN_H, MICRO_BTN_W, MICRO_BUTTONS,
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
            anchor {
                point: AnchorPoint::BottomRight,
                relative_point: AnchorPoint::BottomRight,
                x: "-230",
                y: "55",
            }
            {buttons}
        }
    }
}

fn micro_menu_bar_width() -> f32 {
    MICRO_BUTTONS.len() as f32 * MICRO_BTN_W + (MICRO_BUTTONS.len() - 1) as f32 * MICRO_BTN_GAP
}

fn micro_button(index: usize, name: &str) -> Element {
    let btn_name = DynName(name.to_string());
    let onclick = micro_button_action(name);
    let x = index as f32 * (MICRO_BTN_W + MICRO_BTN_GAP);
    rsx! {
        button {
            name: btn_name,
            width: {MICRO_BTN_W},
            height: {MICRO_BTN_H},
            text: "",
            font_size: 8.0,
            background_color: MICRO_BTN_BG,
            onclick: {onclick},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: "0",
            }
        }
    }
}

fn micro_button_action(name: &str) -> &'static str {
    match name {
        "GuildMicroButton" => ACTION_GUILD_TOGGLE,
        _ => "",
    }
}
