use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;

const SLOT_COUNT: usize = 12;
const SLOT_W: f32 = 36.0;
const SLOT_H: f32 = 36.0;
const MINIMAP_DISPLAY_SIZE: f32 = 200.0;

const BAR_BG: &str = "0.07,0.06,0.05,0.92";
const SLOT_BG: &str = "0.15,0.12,0.08,0.95";
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

fn slot_buttons(prefix: &str, show_labels: bool) -> Element {
    (0..SLOT_COUNT)
        .flat_map(|index| {
            let name = dyn_name(format!("{prefix}{}", index + 1));
            let text = if show_labels { slot_label(index) } else { "" };
            rsx! {
                button {
                    name,
                    width: SLOT_W,
                    height: SLOT_H,
                    text,
                    font_size: 13.0,
                    background_color: SLOT_BG,
                }
            }
        })
        .collect()
}

fn action_bar_root(
    name: FrameName,
    label_name: FrameName,
    label_text: &str,
    buttons: Element,
) -> Element {
    rsx! {
        r#frame {
            name,
            width: 1.0,
            height: 1.0,
            background_color: BAR_BG,
            strata: FrameStrata::Dialog,
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

pub fn action_bar_screen(_ctx: &SharedContext) -> Element {
    let main = action_bar_root(
        FrameName("MainActionBar"),
        FrameName("MainActionBarMoverLabel"),
        "Main Action Bar",
        slot_buttons("ActionButton", true),
    );
    let right = action_bar_root(
        FrameName("MultiBarRight"),
        FrameName("MultiBarRightMoverLabel"),
        "Right Action Bar",
        slot_buttons("MultiBarRightButton", false),
    );
    let left = action_bar_root(
        FrameName("MultiBarLeft"),
        FrameName("MultiBarLeftMoverLabel"),
        "Left Action Bar",
        slot_buttons("MultiBarLeftButton", false),
    );
    let overlays = rsx! {
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
    };

    [main, right, left, overlays]
        .into_iter()
        .flatten()
        .collect()
}

pub fn minimap_screen(_ctx: &SharedContext) -> Element {
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
}
