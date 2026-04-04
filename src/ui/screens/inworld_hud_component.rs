use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;

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
const MINIMAP_ZONE_COLOR: &str = "1.0,0.82,0.0,1.0";
const MINIMAP_COORDS_COLOR: &str = "1.0,1.0,1.0,1.0";
const MINIMAP_HEADER_BG: &str = "0.06,0.05,0.04,0.92";
const MINIMAP_CLUSTER_SHADE: &str = "0.0,0.0,0.0,0.2";

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

fn slot_hotkey(button_name: &DynName, hotkey_name: DynName, text: &str) -> Element {
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
            anchor {
                point: AnchorPoint::TopRight,
                relative_to: button_name.0.as_str(),
                relative_point: AnchorPoint::TopRight,
                x: "-5",
                y: "-5",
            }
        }
    }
}

fn slot_count(button_name: &DynName, count_name: DynName) -> Element {
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
            anchor {
                point: AnchorPoint::BottomRight,
                relative_to: button_name.0.as_str(),
                relative_point: AnchorPoint::BottomRight,
                x: "-5",
                y: "5",
            }
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
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
            }
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
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
            }
            {slot_button_layers(&button_name, frame_atlas)}
            {slot_hotkey(&button_name, hotkey_name, hotkey_text)}
            {slot_count(&button_name, count_name)}
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
    [
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
        ),
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
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
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
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn minimap_header() -> Element {
    rsx! {
        r#frame {
            name: "MinimapHeader",
            width: 175.0,
            height: 16.0,
            background_color: MINIMAP_HEADER_BG,
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                x: "15",
                y: "-4",
            }
        }
        fontstring {
            name: MINIMAP_ZONE_NAME,
            width: 135.0,
            height: 12.0,
            text: "Elwynn Forest",
            font_size: 16.0,
            font_color: MINIMAP_ZONE_COLOR,
            justify_h: "LEFT",
            hidden: true,
            anchor {
                point: AnchorPoint::Left,
                relative_to: FrameName("MinimapHeader"),
                relative_point: AnchorPoint::Left,
                x: "6",
            }
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
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                x: "10",
                y: "-30",
            }
        }
        texture {
            name: MINIMAP_DISPLAY,
            width: MINIMAP_DISPLAY_SIZE,
            height: MINIMAP_DISPLAY_SIZE,
            strata: FrameStrata::High,
            hidden: true,
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                x: "10",
                y: "-42",
            }
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
            anchor {
                point: AnchorPoint::Center,
                relative_to: MINIMAP_DISPLAY,
                relative_point: AnchorPoint::Center,
            }
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
            anchor {
                point: AnchorPoint::Center,
                relative_to: MINIMAP_DISPLAY,
                relative_point: AnchorPoint::Center,
            }
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
            anchor {
                point: AnchorPoint::TopRight,
                relative_to: MINIMAP_DISPLAY,
                relative_point: AnchorPoint::BottomRight,
                y: "-6",
            }
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
            anchor {
                point: AnchorPoint::TopRight,
                relative_point: AnchorPoint::TopRight,
                x: "-12",
                y: "-8",
            }
            {minimap_header()}
            {minimap_display()}
            {minimap_border()}
            {minimap_overlay()}
        }
    }
}

pub fn minimap_screen(_ctx: &SharedContext) -> Element {
    minimap_cluster()
}
