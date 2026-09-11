use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::FrameName;
use crate::ui::screens::bag_frame_component::bag_toggle_action;
use crate::ui::screens::calendar_frame_component::ACTION_CALENDAR_TOGGLE;
use crate::ui::strata::FrameStrata;
use inworld_hud_micro::micro_menu_bar;

#[path = "inworld_hud_micro.rs"]
mod inworld_hud_micro;

const SLOT_COUNT: usize = 12;
const SLOT_W: f32 = 45.0;
const SLOT_H: f32 = 45.0;
const MINIMAP_DISPLAY_SIZE: f32 = 200.0;

const BAR_BG: &str = "0.03,0.02,0.01,0.18";
const SLOT_BG: &str = "0.06,0.05,0.04,0.82";
const SLOT_HOTKEY: &str = "0.82,0.88,1.0,0.95";
const SLOT_COUNT_COLOR: &str = "1.0,1.0,1.0,0.95";
const MAIN_BUTTON_ATLAS: &str = "ui-hud-actionbar-iconframe";
const EXTRA_BUTTON_ATLAS: &str = "ui-hud-actionbar-iconframe-addrow";
const MAIN_BUTTON_PRESSED_ATLAS: &str = "ui-hud-actionbar-iconframe-down";
const EXTRA_BUTTON_PRESSED_ATLAS: &str = "ui-hud-actionbar-iconframe-addrow-down";
const HIGHLIGHT_BUTTON_ATLAS: &str = "ui-hud-actionbar-iconframe-mouseover";
const BORDER_ATLAS: &str = "ui-hud-actionbar-iconframe-border";
const FLASH_ATLAS: &str = "ui-hud-actionbar-iconframe-flash";
const GUIDE_COLOR: &str = "0.95,0.78,0.25,0.95";
const EDIT_BANNER_BG: &str = "0.03,0.04,0.06,0.9";
const EDIT_BANNER_TEXT: &str = "1.0,0.86,0.25,1.0";
const MOVER_LABEL_TEXT: &str = "1.0,0.9,0.45,1.0";
const MICRO_BTN_W: f32 = 28.0;
const MICRO_BTN_H: f32 = 36.0;
const MICRO_BTN_GAP: f32 = 2.0;
const MICRO_BTN_BG: &str = "0.08,0.07,0.06,0.88";
const BAG_SLOT_SIZE: f32 = 30.0;
const BAG_SLOT_GAP: f32 = 4.0;
const BAG_SLOT_BG: &str = "0.06,0.05,0.04,0.82";
const BAG_COUNT: usize = 4;
const MONEY_DISPLAY_W: f32 = 160.0;
const MONEY_DISPLAY_H: f32 = 14.0;
const MONEY_TEXT_COLOR: &str = "1.0,0.82,0.0,1.0";
const MINIMAP_ZONE_COLOR: &str = "1.0,0.82,0.0,1.0";
const MINIMAP_COORDS_COLOR: &str = "1.0,1.0,1.0,1.0";
const MINIMAP_HEADER_BG: &str = "0.06,0.05,0.04,0.92";
const MINIMAP_CLUSTER_SHADE: &str = "0.0,0.0,0.0,0.2";
pub(super) const MICRO_BUTTONS: &[&str] = &[
    "CharacterMicroButton",
    "SpellbookMicroButton",
    "TalentMicroButton",
    "AchievementMicroButton",
    "QuestLogMicroButton",
    "GuildMicroButton",
    "LFDMicroButton",
    "CollectionsMicroButton",
    "EJMicroButton",
    "StoreMicroButton",
    "MainMenuMicroButton",
];

pub const MINIMAP_DISPLAY: FrameName = FrameName("MinimapDisplay");
pub const MINIMAP_BORDER: FrameName = FrameName("MinimapBorder");
pub const MINIMAP_ARROW: FrameName = FrameName("MinimapArrow");
pub const MINIMAP_ZONE_NAME: FrameName = FrameName("MinimapZoneName");
pub const MINIMAP_COORDS: FrameName = FrameName("MinimapCoords");

struct DynName(String);

fn dyn_name(name: String) -> DynName {
    DynName(name)
}

fn slot_label(index: usize) -> &'static str {
    match index {
        0 => "1",
        1 => "2",
        2 => "3",
        3 => "4",
        4 => "5",
        5 => "6",
        6 => "7",
        7 => "8",
        8 => "9",
        9 => "0",
        10 => "-",
        _ => "=",
    }
}

fn slot_hotkey(hotkey_name: DynName, text: &str) -> Element {
    rsx! {
        fontstring {
            name: hotkey_name,
            width: 32.0,
            height: 15.0,
            text,
            font: "ArialNarrow",
            font_size: 12.0,
            font_color: SLOT_HOTKEY,
            justify_h: "RIGHT",
            pos_type: "absolute",
            right: 5.0,
            pos_y: 5.0,
        }
    }
}

fn slot_count(count_name: DynName) -> Element {
    rsx! {
        fontstring {
            name: count_name,
            width: 18.0,
            height: 14.0,
            text: "",
            font: "ArialNarrow",
            font_size: 14.0,
            font_color: SLOT_COUNT_COLOR,
            justify_h: "RIGHT",
            pos_type: "absolute",
            right: 5.0,
            bottom: 5.0,
        }
    }
}

fn slot_frame_texture(texture_name: DynName, atlas: &str, hidden: bool, size: f32) -> Element {
    rsx! {
        texture {
            name: texture_name,
            width: size,
            height: SLOT_H,
            texture_atlas: atlas,
            hidden,
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: 0.0,
        }
    }
}

fn slot_button_layers(button_name: &DynName, frame_atlas: &str) -> Element {
    let normal_name = dyn_name(format!("{}NormalTexture", button_name.0));
    let border_name = dyn_name(format!("{}Border", button_name.0));
    let flash_name = dyn_name(format!("{}Flash", button_name.0));
    rsx! {
        {slot_frame_texture(normal_name, frame_atlas, false, SLOT_W)}
        {slot_frame_texture(border_name, BORDER_ATLAS, true, SLOT_W + 1.0)}
        {slot_frame_texture(flash_name, FLASH_ATLAS, true, SLOT_W + 1.0)}
    }
}

fn slot_button_widget(
    button_name: DynName,
    hotkey_text: &str,
    frame_atlas: &str,
    pressed_atlas: &str,
) -> Element {
    let hotkey_name = dyn_name(format!("{}HotKey", button_name.0));
    let count_name = dyn_name(format!("{}Count", button_name.0));
    rsx! {
        button {
            name: button_name,
            width: SLOT_W,
            height: SLOT_H,
            text: "",
            font_size: 12.0,
            background_color: SLOT_BG,
            button_atlas_up: frame_atlas,
            button_atlas_pressed: pressed_atlas,
            button_atlas_highlight: HIGHLIGHT_BUTTON_ATLAS,
            button_atlas_disabled: frame_atlas,
            pos_type: "absolute",
            left: "50%",
            top: "50%",
            translate_x: "-50%",
            translate_y: "-50%",
            {slot_button_layers(&button_name, frame_atlas)}
            {slot_hotkey(hotkey_name, hotkey_text)}
            {slot_count(count_name)}
        }
    }
}

fn action_button(
    container_prefix: &str,
    button_prefix: &str,
    index: usize,
    hotkey: &str,
    frame_atlas: &str,
    pressed_atlas: &str,
) -> Element {
    let container_name = dyn_name(format!("{container_prefix}{}", index + 1));
    let button_name = dyn_name(format!("{button_prefix}{}", index + 1));
    rsx! {
        r#frame {
            name: container_name,
            width: SLOT_W,
            height: SLOT_H,
            {slot_button_widget(button_name, hotkey, frame_atlas, pressed_atlas)}
        }
    }
}

fn slot_buttons(
    container_prefix: &str,
    button_prefix: &str,
    show_hotkeys: bool,
    frame_atlas: &str,
    pressed_atlas: &str,
) -> Element {
    (0..SLOT_COUNT)
        .flat_map(|index| {
            let hotkey = if show_hotkeys { slot_label(index) } else { "" };
            action_button(
                container_prefix,
                button_prefix,
                index,
                hotkey,
                frame_atlas,
                pressed_atlas,
            )
        })
        .collect()
}

fn action_bar_root(
    name: FrameName,
    label_name: FrameName,
    label_text: &str,
    hidden: bool,
    buttons: Element,
) -> Element {
    rsx! {
        r#frame {
            name,
            width: 1.0,
            height: 1.0,
            background_color: BAR_BG,
            strata: FrameStrata::Dialog,
            hidden,
            {buttons}
            fontstring {
                name: label_name,
                width: 220.0,
                height: 16.0,
                text: label_text,
                font_size: 13.0,
                font_color: MOVER_LABEL_TEXT,
                justify_h: "LEFT",
            }
        }
    }
}

fn main_action_bar() -> Element {
    action_bar_root(
        FrameName("MainActionBar"),
        FrameName("MainActionBarMoverLabel"),
        "Main Action Bar",
        false,
        slot_buttons(
            "MainActionBarButtonContainer",
            "ActionButton",
            true,
            MAIN_BUTTON_ATLAS,
            MAIN_BUTTON_PRESSED_ATLAS,
        ),
    )
}

fn bottom_action_bars() -> Element {
    [bottom_left_action_bar(), bottom_right_action_bar()]
        .into_iter()
        .flatten()
        .collect()
}

fn bottom_left_action_bar() -> Element {
    action_bar_root(
        FrameName("MultiBarBottomLeft"),
        FrameName("MultiBarBottomLeftMoverLabel"),
        "Bottom Left Action Bar",
        true,
        slot_buttons(
            "MultiBarBottomLeftButtonContainer",
            "MultiBarBottomLeftButton",
            false,
            EXTRA_BUTTON_ATLAS,
            EXTRA_BUTTON_PRESSED_ATLAS,
        ),
    )
}

fn bottom_right_action_bar() -> Element {
    action_bar_root(
        FrameName("MultiBarBottomRight"),
        FrameName("MultiBarBottomRightMoverLabel"),
        "Bottom Right Action Bar",
        true,
        slot_buttons(
            "MultiBarBottomRightButtonContainer",
            "MultiBarBottomRightButton",
            false,
            EXTRA_BUTTON_ATLAS,
            EXTRA_BUTTON_PRESSED_ATLAS,
        ),
    )
}

fn side_action_bars() -> Element {
    [right_action_bar(), left_action_bar()]
        .into_iter()
        .flatten()
        .collect()
}

fn right_action_bar() -> Element {
    action_bar_root(
        FrameName("MultiBarRight"),
        FrameName("MultiBarRightMoverLabel"),
        "Right Action Bar",
        true,
        slot_buttons(
            "MultiBarRightButtonContainer",
            "MultiBarRightButton",
            false,
            EXTRA_BUTTON_ATLAS,
            EXTRA_BUTTON_PRESSED_ATLAS,
        ),
    )
}

fn left_action_bar() -> Element {
    action_bar_root(
        FrameName("MultiBarLeft"),
        FrameName("MultiBarLeftMoverLabel"),
        "Left Action Bar",
        true,
        slot_buttons(
            "MultiBarLeftButtonContainer",
            "MultiBarLeftButton",
            false,
            EXTRA_BUTTON_ATLAS,
            EXTRA_BUTTON_PRESSED_ATLAS,
        ),
    )
}

fn action_bar_overlays() -> Element {
    rsx! {
        r#frame {
            name: "ActionBarGuideVertical",
            width: 2.0,
            height: 1.0,
            background_color: GUIDE_COLOR,
            strata: FrameStrata::Tooltip,
            hidden: true,
        }
        r#frame {
            name: "ActionBarGuideHorizontal",
            width: 1.0,
            height: 2.0,
            background_color: GUIDE_COLOR,
            strata: FrameStrata::Tooltip,
            hidden: true,
        }
        r#frame {
            name: "ActionBarEditBanner",
            width: 760.0,
            height: 34.0,
            background_color: EDIT_BANNER_BG,
            strata: FrameStrata::Tooltip,
            hidden: true,
            fontstring {
                name: "ActionBarEditBannerText",
                width: 760.0,
                height: 34.0,
                text: "Action Bar Edit Mode",
                font_size: 15.0,
                font_color: EDIT_BANNER_TEXT,
            }
        }
    }
}

pub fn action_bar_screen(_ctx: &SharedContext) -> Element {
    [
        main_action_bar(),
        bottom_action_bars(),
        side_action_bars(),
        action_bar_overlays(),
        micro_menu_bar(),
        bag_bar(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn bag_bar() -> Element {
    let backpack = bag_slot("MainMenuBarBackpackButton", 0);
    let bags: Element = (0..BAG_COUNT)
        .flat_map(|i| {
            let name = format!("CharacterBag{i}Slot");
            let slot_index = i + 1;
            bag_slot(&name, slot_index)
        })
        .collect();
    let total_w = (BAG_COUNT as f32 + 1.0) * BAG_SLOT_SIZE + BAG_COUNT as f32 * BAG_SLOT_GAP;
    let bar_h = BAG_SLOT_SIZE + MONEY_DISPLAY_H + 4.0;
    rsx! {
        r#frame {
            name: "BagsBar",
            width: {total_w},
            height: {bar_h},
            pos_type: "absolute",
            right: 4.0,
            bottom: 8.0,
            {backpack}
            {bags}
            {money_display()}
        }
    }
}

fn money_display() -> Element {
    rsx! {
        fontstring {
            name: "BagsBarMoneyDisplay",
            width: {MONEY_DISPLAY_W},
            height: {MONEY_DISPLAY_H},
            text: "0g 0s 0c",
            font: "ArialNarrow",
            font_size: 11.0,
            font_color: MONEY_TEXT_COLOR,
            justify_h: "RIGHT",
            pos_type: "absolute",
            right: 0.0,
            pos_y: {BAG_SLOT_SIZE + 4.0},
        }
    }
}

fn bag_slot(name: &str, index: usize) -> Element {
    let slot_name = DynName(name.to_string());
    let x = index as f32 * (BAG_SLOT_SIZE + BAG_SLOT_GAP);
    let action = bag_toggle_action(index);
    rsx! {
        button {
            name: slot_name,
            width: {BAG_SLOT_SIZE},
            height: {BAG_SLOT_SIZE},
            text: "",
            font_size: 8.0,
            background_color: BAG_SLOT_BG,
            onclick: {action.as_str()},
            pos_type: "absolute",
            pos_x: x,
            pos_y: 0.0,
        }
    }
}

fn minimap_header() -> Element {
    rsx! {
        r#frame {
            name: "MinimapHeader",
            width: 175.0,
            height: 16.0,
            background_color: MINIMAP_HEADER_BG,
            pos_type: "absolute",
            left: "50%",
            margin_left: 15.0,
            pos_y: 4.0,
            translate_x: "-50%",
            {minimap_zone_name()}
        }
    }
}

fn minimap_zone_name() -> Element {
    rsx! {
        fontstring {
            name: MINIMAP_ZONE_NAME,
            width: 135.0,
            height: 12.0,
            text: "Elwynn Forest",
            font_size: 16.0,
            font_color: MINIMAP_ZONE_COLOR,
            justify_h: "LEFT",
            hidden: true,
            pos_type: "absolute",
            pos_x: 6.0,
            top: "50%",
            translate_y: "-50%",
        }
    }
}

fn minimap_display() -> Element {
    rsx! {
        r#frame {
            name: "MinimapShade",
            width: 215.0,
            height: 226.0,
            background_color: MINIMAP_CLUSTER_SHADE,
            pos_type: "absolute",
            left: "50%",
            margin_left: 10.0,
            pos_y: 30.0,
            translate_x: "-50%",
        }
        r#frame {
            name: "MinimapMapArea",
            width: MINIMAP_DISPLAY_SIZE,
            height: MINIMAP_DISPLAY_SIZE,
            pos_type: "absolute",
            left: "50%",
            margin_left: 10.0,
            pos_y: 42.0,
            translate_x: "-50%",
            texture {
                name: MINIMAP_DISPLAY,
                width: MINIMAP_DISPLAY_SIZE,
                height: MINIMAP_DISPLAY_SIZE,
                strata: FrameStrata::High,
                hidden: true,
                pos_type: "absolute",
                pos_x: 0.0,
                pos_y: 0.0,
            }
            {minimap_border()}
            {minimap_overlay()}
            {minimap_buttons()}
        }
    }
}

fn minimap_border() -> Element {
    rsx! {
        texture {
            name: MINIMAP_BORDER,
            width: MINIMAP_DISPLAY_SIZE,
            height: MINIMAP_DISPLAY_SIZE,
            strata: FrameStrata::High,
            frame_level: 10.0,
            hidden: true,
            pos_type: "absolute",
            left: "50%",
            top: "50%",
            translate_x: "-50%",
            translate_y: "-50%",
        }
    }
}

fn minimap_overlay() -> Element {
    rsx! {
        texture {
            name: MINIMAP_ARROW,
            width: 16.0,
            height: 16.0,
            strata: FrameStrata::High,
            frame_level: 11.0,
            hidden: true,
            pos_type: "absolute",
            left: "50%",
            top: "50%",
            translate_x: "-50%",
            translate_y: "-50%",
        }
        fontstring {
            name: MINIMAP_COORDS,
            width: MINIMAP_DISPLAY_SIZE,
            height: 18.0,
            text: "0, 0",
            font_size: 14.0,
            font_color: MINIMAP_COORDS_COLOR,
            justify_h: "RIGHT",
            hidden: true,
            pos_type: "absolute",
            right: 0.0,
            top: "100%",
            margin_top: 6.0,
        }
    }
}

fn minimap_cluster() -> Element {
    rsx! {
        r#frame {
            name: "MinimapCluster",
            width: 215.0,
            height: 242.0,
            strata: FrameStrata::High,
            hidden: true,
            pos_type: "absolute",
            right: 12.0,
            pos_y: 8.0,
            {minimap_header()}
            {minimap_display()}
        }
    }
}

const MINIMAP_BTN_SIZE: f32 = 24.0;

macro_rules! minimap_btn_frame {
    ($name:expr, $txt_name:expr, $text:expr, $x_off:expr, $y_off:expr) => {
        rsx! {
            r#frame {
                name: $name,
                width: {MINIMAP_BTN_SIZE},
                height: {MINIMAP_BTN_SIZE},
                background_color: MINIMAP_HEADER_BG,
                strata: FrameStrata::High,
                frame_level: 12.0,
                pos_type: "absolute",
                left: "50%",
                top: "50%",
                margin_left: {$x_off},
                margin_top: {-$y_off},
                translate_x: "-50%",
                translate_y: "-50%",
                {minimap_btn_label($txt_name, $text)}
            }
        }
    };
    ($name:expr, $txt_name:expr, $text:expr, $x_off:expr, $y_off:expr, $action:expr) => {
        rsx! {
            r#frame {
                name: $name,
                width: {MINIMAP_BTN_SIZE},
                height: {MINIMAP_BTN_SIZE},
                background_color: MINIMAP_HEADER_BG,
                strata: FrameStrata::High,
                frame_level: 12.0,
                onclick: $action,
                pos_type: "absolute",
                left: "50%",
                top: "50%",
                margin_left: {$x_off},
                margin_top: {-$y_off},
                translate_x: "-50%",
                translate_y: "-50%",
                {minimap_btn_label($txt_name, $text)}
            }
        }
    };
}

fn minimap_buttons() -> Element {
    let btns = [
        ("MinimapZoomIn", "+", 90.0, -10.0),
        ("MinimapZoomOut", "-", 90.0, 14.0),
        ("MinimapCalendarButton", "Cal", -80.0, -10.0),
        ("MinimapMailButton", "Mail", -80.0, 14.0),
        ("MinimapLFGButton", "LFG", -80.0, 38.0),
    ];
    btns.iter()
        .flat_map(|(name, text, x_off, y_off)| minimap_btn(name, text, *x_off, *y_off))
        .collect()
}

fn minimap_btn(name: &str, text: &str, x_off: f32, y_off: f32) -> Element {
    let btn_name = DynName(name.to_string());
    let txt_name = DynName(format!("{name}Text"));
    if let Some(action) = minimap_btn_action(name) {
        minimap_btn_frame!(btn_name, txt_name, text, x_off, y_off, action)
    } else {
        minimap_btn_frame!(btn_name, txt_name, text, x_off, y_off)
    }
}

fn minimap_btn_action(name: &str) -> Option<&'static str> {
    match name {
        "MinimapCalendarButton" => Some(ACTION_CALENDAR_TOGGLE),
        _ => None,
    }
}

fn minimap_btn_label(name: DynName, text: &str) -> Element {
    rsx! {
        fontstring {
            name,
            width: {MINIMAP_BTN_SIZE},
            height: {MINIMAP_BTN_SIZE},
            text,
            font_size: 8.0,
            font_color: MINIMAP_ZONE_COLOR,
            justify_h: "CENTER",
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: 0.0,
        }
    }
}

pub fn minimap_screen(_ctx: &SharedContext) -> Element {
    minimap_cluster()
}

#[cfg(test)]
#[path = "inworld_hud_component_tests.rs"]
mod tests;
