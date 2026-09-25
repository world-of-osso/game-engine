//! Retail `FlightMapFrame` (Blizzard_FlightMap/Blizzard_FlightMap.xml, cited as FM.xml,
//! and FM_FlightPathDataProvider.lua, FPD.lua): a 1004×689 `PortraitFrameTemplate`
//! titled `FLIGHT_MAP` over the continent's map art, with a pin per flight point and
//! route lines. The scene computes every position; this only lays them out.

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;
use crate::ui::screens::merchant_frame_component::{MoneyAlign, money};
use crate::ui::screens::quest_art::{DynName, NORMAL_FONT_COLOR, atlas_texture, portrait_border};
use crate::ui::strata::FrameStrata;

pub const FRAME_NAME: &str = "FlightMapFrame";
/// FM.xml:5-6 `<Size x="1004" y="689"/>`.
pub const FRAME_W: f32 = 1004.0;
pub const FRAME_H: f32 = 689.0;
/// `ScrollContainer` TOPLEFT 1,-20 / BOTTOMRIGHT -1,1 (MapCanvas.xml:21-25).
pub const CANVAS_X: f32 = 1.0;
pub const CANVAS_Y: f32 = 20.0;
pub const CANVAS_W: f32 = FRAME_W - 2.0;
pub const CANVAS_H: f32 = FRAME_H - 21.0;

pub const ACTION_CLOSE: &str = "flight_map_close";
/// `taxi_node:<TaxiNodes.ID>`.
pub const ACTION_NODE_PREFIX: &str = "taxi_node:";

/// UiTextureAtlas 853 `interface/taxiframe/taxiassets.blp` (1455734) 128×512.
const TAXI_ATLAS: (u32, (f32, f32)) = (1_455_734, (128.0, 512.0));
const fn taxi_art(rect: (f32, f32, f32, f32)) -> AtlasArt {
    AtlasArt {
        fdid: TAXI_ATLAS.0,
        atlas: TAXI_ATLAS.1,
        rect,
    }
}
/// `taxi_frame_green` (6156), `taxi_frame_gray` (6155), `taxi_frame_yellow` (6225),
/// `ui-taxi-icon-nub` (6157) (FPD.lua:330-349).
const PIN_CURRENT: AtlasArt = taxi_art((1.0, 75.0, 111.0, 185.0));
const PIN_REACHABLE: AtlasArt = taxi_art((1.0, 75.0, 35.0, 109.0));
const PIN_HOVERED: AtlasArt = taxi_art((1.0, 75.0, 187.0, 261.0));
const PIN_UNREACHABLE: AtlasArt = taxi_art((77.0, 93.0, 35.0, 51.0));
/// `AdventureMap_TopBorder` (5349), atlas 722 (1270719) 1024×1024, TOPLEFT 2,-22 /
/// BOTTOMRIGHT -3,2 (FM.xml:17-24).
const TOP_BORDER: AtlasArt = AtlasArt {
    fdid: 1_270_719,
    atlas: (1024.0, 1024.0),
    rect: (1.0, 1003.0, 1.0, 669.0),
};

/// `TAXINODEYOUAREHERE`, `TAXI_PATH_UNREACHABLE`.
const YOU_ARE_HERE: &str = "You are here";
const NOT_DISCOVERED: &str = "Not Discovered";
const HIGHLIGHT_FONT_COLOR: &str = "1.0,1.0,1.0,1.0";
const RED_FONT_COLOR: &str = "1.0,0.125,0.125,1.0";
/// Route dots stand in for the rotated `_UI-Taxi-Line-horizontal` line texture.
const LINE_COLOR: &str = "1.0,0.82,0.0,1.0";
const BACKGROUND_LINE_COLOR: &str = "1.0,0.82,0.0,0.45";
const LINE_DOT: f32 = 3.0;
const LINE_STEP: f32 = 7.0;

/// `Enum.FlightPathState` plus the hovered-reachable look.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PinLook {
    Current,
    #[default]
    Reachable,
    Hovered,
    Unreachable,
}

impl PinLook {
    /// `UpdatePinSize` (FPD.lua:287-313).
    pub fn size(self) -> f32 {
        match self {
            Self::Current => 28.0,
            Self::Reachable | Self::Hovered => 20.0,
            Self::Unreachable => 14.0,
        }
    }

    fn art(self) -> &'static AtlasArt {
        match self {
            Self::Current => &PIN_CURRENT,
            Self::Reachable => &PIN_REACHABLE,
            Self::Hovered => &PIN_HOVERED,
            Self::Unreachable => &PIN_UNREACHABLE,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct FlightMapPin {
    pub node: u32,
    /// Centre in frame space.
    pub x: f32,
    pub y: f32,
    pub look: PinLook,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TooltipDetail {
    YouAreHere,
    Cost(u64),
    NotDiscovered,
}

/// GameTooltip BOTTOMLEFT at the pin's TOPRIGHT (FPD.lua:240-269).
#[derive(Clone, Debug, PartialEq)]
pub struct FlightMapTooltip {
    /// The hovered pin's top-right corner in frame space.
    pub x: f32,
    pub y: f32,
    pub name: String,
    pub detail: Option<TooltipDetail>,
}

/// A route segment in frame space; `highlight` for the hovered route.
#[derive(Clone, Debug, PartialEq)]
pub struct FlightMapLine {
    pub from: (f32, f32),
    pub to: (f32, f32),
    pub highlight: bool,
}

/// A map art tile in frame space.
#[derive(Clone, Debug, PartialEq)]
pub struct FlightMapTile {
    pub fdid: u32,
    pub rect: (f32, f32, f32, f32),
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct FlightMapFrameState {
    pub visible: bool,
    pub left: f32,
    pub top: f32,
    pub tiles: Vec<FlightMapTile>,
    pub lines: Vec<FlightMapLine>,
    pub pins: Vec<FlightMapPin>,
    pub tooltip: Option<FlightMapTooltip>,
}

pub fn flight_map_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<FlightMapFrameState>()
        .expect("FlightMapFrameState must be in SharedContext");
    let hide = !state.visible;
    // `SetupTitle`: the background is plain black (Blizzard_FlightMap.lua:5-10).
    let mut children = rsx! {
        r#frame {
            name: {DynName(format!("{FRAME_NAME}Bg"))},
            width: FRAME_W,
            height: FRAME_H,
            background_color: "0.0,0.0,0.0,1.0",
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
        }
    };
    for (index, tile) in state.tiles.iter().enumerate() {
        children.extend(tile_texture(index, tile));
    }
    children.extend(atlas_texture(
        format!("{FRAME_NAME}TopBorder"),
        &TOP_BORDER,
        (2.0, 22.0, FRAME_W - 5.0, FRAME_H - 24.0),
    ));
    for (index, line) in state.lines.iter().enumerate() {
        children.extend(route_line(index, line));
    }
    for pin in &state.pins {
        children.extend(pin_button(pin));
    }
    children.extend(portrait_border(
        FRAME_NAME,
        (FRAME_W, FRAME_H),
        "Flight Map",
        ACTION_CLOSE,
    ));
    if let Some(tooltip) = &state.tooltip {
        children.extend(tooltip_frame(tooltip));
    }
    rsx! {
        r#frame {
            name: {DynName(FRAME_NAME.into())},
            width: FRAME_W,
            height: FRAME_H,
            strata: FrameStrata::High,
            hidden: hide,
            mouse_enabled: true,
            pos_type: "absolute",
            left: {state.left},
            top: {state.top},
            {children}
        }
    }
}

fn tile_texture(index: usize, tile: &FlightMapTile) -> Element {
    let (x, y, width, height) = tile.rect;
    rsx! {
        texture {
            name: {DynName(format!("{FRAME_NAME}Tile{index}"))},
            width,
            height,
            texture_fdid: {tile.fdid},
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

fn route_line(index: usize, line: &FlightMapLine) -> Element {
    let (dx, dy) = (line.to.0 - line.from.0, line.to.1 - line.from.1);
    let length = (dx * dx + dy * dy).sqrt();
    let steps = (length / LINE_STEP).ceil().max(1.0) as usize;
    let color = if line.highlight {
        LINE_COLOR
    } else {
        BACKGROUND_LINE_COLOR
    };
    (0..=steps)
        .flat_map(|step| {
            let t = step as f32 / steps as f32;
            let x = line.from.0 + dx * t - LINE_DOT / 2.0;
            let y = line.from.1 + dy * t - LINE_DOT / 2.0;
            rsx! {
                r#frame {
                    name: {DynName(format!("{FRAME_NAME}Line{index}Dot{step}"))},
                    width: LINE_DOT,
                    height: LINE_DOT,
                    background_color: color,
                    pos_type: "absolute",
                    left: x,
                    top: y,
                }
            }
        })
        .collect()
}

/// `FlightMap_FlightPointPinTemplate`: the state icon, clickable (`TakeTaxiNode`).
fn pin_button(pin: &FlightMapPin) -> Element {
    let size = pin.look.size();
    let art = pin.look.art();
    let coords = art.tex_coords(1.0);
    let name = format!("{FRAME_NAME}Pin{}", pin.node);
    let action = format!("{ACTION_NODE_PREFIX}{}", pin.node);
    rsx! {
        button {
            name: {DynName(name.clone())},
            width: size,
            height: size,
            onclick: {action.as_str()},
            pos_type: "absolute",
            left: {pin.x - size / 2.0},
            top: {pin.y - size / 2.0},
            texture {
                name: {DynName(format!("{name}Icon"))},
                width: size,
                height: size,
                texture_fdid: {art.fdid},
                tex_coords: {coords.as_str()},
                pos_type: "absolute",
                left: 0.0,
                top: 0.0,
            }
        }
    }
}

const TOOLTIP_W: f32 = 200.0;
const TOOLTIP_PAD: f32 = 8.0;
const TOOLTIP_LINE: f32 = 16.0;

fn tooltip_frame(tooltip: &FlightMapTooltip) -> Element {
    let lines = if tooltip.detail.is_some() { 2.0 } else { 1.0 };
    let height = 2.0 * TOOLTIP_PAD + lines * TOOLTIP_LINE;
    let prefix = format!("{FRAME_NAME}Tooltip");
    let mut children = tooltip_text(
        &format!("{prefix}Name"),
        &tooltip.name,
        0.0,
        NORMAL_FONT_COLOR,
    );
    match &tooltip.detail {
        Some(TooltipDetail::YouAreHere) => children.extend(tooltip_text(
            &format!("{prefix}Detail"),
            YOU_ARE_HERE,
            TOOLTIP_LINE,
            HIGHLIGHT_FONT_COLOR,
        )),
        Some(TooltipDetail::NotDiscovered) => children.extend(tooltip_text(
            &format!("{prefix}Detail"),
            NOT_DISCOVERED,
            TOOLTIP_LINE,
            RED_FONT_COLOR,
        )),
        Some(TooltipDetail::Cost(copper)) => children.extend(money(
            &format!("{prefix}Money"),
            *copper,
            (TOOLTIP_PAD, TOOLTIP_PAD + 2.0 * TOOLTIP_LINE),
            MoneyAlign::Left,
            false,
        )),
        None => {}
    }
    rsx! {
        r#frame {
            name: {DynName(prefix)},
            width: TOOLTIP_W,
            height,
            background_color: "0.0,0.0,0.0,0.85",
            strata: FrameStrata::Tooltip,
            pos_type: "absolute",
            left: {tooltip.x},
            top: {tooltip.y - height},
            {children}
        }
    }
}

fn tooltip_text(name: &str, text: &str, y: f32, color: &str) -> Element {
    rsx! {
        fontstring {
            name: {DynName(name.into())},
            width: {TOOLTIP_W - 2.0 * TOOLTIP_PAD},
            height: TOOLTIP_LINE,
            text,
            font: GameFont::FrizQuadrata,
            font_size: 12.0,
            font_color: color,
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h: "LEFT",
            pos_type: "absolute",
            left: TOOLTIP_PAD,
            top: {TOOLTIP_PAD + y},
        }
    }
}

#[cfg(test)]
#[path = "flight_map_component_tests.rs"]
mod tests;
