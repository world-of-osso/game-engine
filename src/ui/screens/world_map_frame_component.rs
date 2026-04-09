use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::strata::FrameStrata;

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// --- Layout constants ---

/// Full-screen dimensions (1920×1080 reference).
pub const FRAME_W: f32 = 1920.0;
pub const FRAME_H: f32 = 1080.0;

const HEADER_H: f32 = 36.0;
const HEADER_INSET: f32 = 12.0;
const ZONE_NAME_W: f32 = 400.0;
const COORD_W: f32 = 140.0;
const COORD_H: f32 = 20.0;

const CANVAS_TOP: f32 = HEADER_H + 4.0;
const CANVAS_INSET: f32 = 8.0;
const CANVAS_W: f32 = FRAME_W - 2.0 * CANVAS_INSET;
const CANVAS_H: f32 = FRAME_H - CANVAS_TOP - CANVAS_INSET;

const CLOSE_BTN_SIZE: f32 = 24.0;
const CLOSE_BTN_INSET: f32 = 8.0;

const DROPDOWN_W: f32 = 180.0;
const DROPDOWN_H: f32 = 24.0;
const DROPDOWN_GAP: f32 = 8.0;
/// Continent dropdown starts after zone name.
const DROPDOWN_X: f32 = HEADER_INSET + ZONE_NAME_W + 16.0;

const ZONE_OVERLAY_MAX: usize = 8;

const PIN_SIZE: f32 = 16.0;
const MAX_PINS: usize = 20;

const LEGEND_W: f32 = 140.0;
const LEGEND_ROW_H: f32 = 18.0;
const LEGEND_ICON_SIZE: f32 = 12.0;
const LEGEND_INSET: f32 = 6.0;
const LEGEND_HEADER_H: f32 = 16.0;

const FP_LINE_H: f32 = 2.0;
const FP_DOT_SIZE: f32 = 6.0;
const MAX_FP_SEGMENTS: usize = 16;

const TOOLTIP_W: f32 = 180.0;
const TOOLTIP_LINE_H: f32 = 16.0;
const TOOLTIP_INSET: f32 = 6.0;

pub const ACTION_WORLD_MAP_CLOSE: &str = "world_map_close";

// --- Colors ---

const FRAME_BG: &str = "0.04,0.03,0.02,0.95";
const HEADER_BG: &str = "0.08,0.06,0.04,0.95";
const ZONE_NAME_COLOR: &str = "1.0,0.82,0.0,1.0";
const COORD_COLOR: &str = "0.8,0.8,0.8,1.0";
const CANVAS_BG: &str = "0.02,0.02,0.02,0.9";
const CLOSE_BTN_BG: &str = "0.3,0.08,0.08,0.9";
const CLOSE_BTN_TEXT: &str = "1.0,0.3,0.3,1.0";
const DROPDOWN_BG: &str = "0.1,0.08,0.06,0.95";
const DROPDOWN_TEXT_COLOR: &str = "1.0,1.0,1.0,1.0";
const DROPDOWN_ARROW_COLOR: &str = "0.8,0.8,0.8,1.0";
const ZONE_OVERLAY_BG: &str = "0.3,0.25,0.1,0.25";
const ZONE_OVERLAY_TEXT: &str = "1.0,0.82,0.0,0.8";
const FOG_OVERLAY_BG: &str = "0.0,0.0,0.0,0.72";
const FOG_OVERLAY_TEXT: &str = "0.85,0.85,0.85,0.92";
const PIN_QUEST_BG: &str = "1.0,0.82,0.0,0.9";
const PIN_FP_BG: &str = "0.3,0.8,0.3,0.9";
const PIN_POI_BG: &str = "0.6,0.6,0.6,0.9";
const LEGEND_BG: &str = "0.06,0.05,0.04,0.9";
const LEGEND_HEADER_COLOR: &str = "1.0,0.82,0.0,1.0";
const LEGEND_TEXT_COLOR: &str = "0.85,0.85,0.85,1.0";
const FP_LINE_COLOR: &str = "0.3,0.8,0.3,0.6";
const FP_DOT_COLOR: &str = "0.3,0.8,0.3,0.9";
const TOOLTIP_BG: &str = "0.08,0.06,0.04,0.95";
const TOOLTIP_TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const TOOLTIP_TEXT_COLOR: &str = "0.85,0.85,0.85,1.0";

// --- Data types ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MapPinType {
    #[default]
    Quest,
    FlightPath,
    PointOfInterest,
}

impl MapPinType {
    pub fn color(self) -> &'static str {
        match self {
            Self::Quest => PIN_QUEST_BG,
            Self::FlightPath => PIN_FP_BG,
            Self::PointOfInterest => PIN_POI_BG,
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Quest => "!",
            Self::FlightPath => "⚑",
            Self::PointOfInterest => "●",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapPin {
    pub pin_type: MapPinType,
    pub label: String,
    /// Position on canvas as fraction (0.0–1.0).
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ZoneOverlay {
    pub name: String,
    /// Bounding box on canvas as fractions (0.0–1.0).
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// A flight path line segment between two points on the canvas (fractions 0.0–1.0).
#[derive(Clone, Debug, PartialEq)]
pub struct FlightPathSegment {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct WorldMapFrameState {
    pub visible: bool,
    pub zone_name: String,
    pub player_x: f32,
    pub player_y: f32,
    pub continent_name: String,
    pub zone_overlays: Vec<ZoneOverlay>,
    pub fog_overlays: Vec<ZoneOverlay>,
    pub pins: Vec<MapPin>,
    pub flight_paths: Vec<FlightPathSegment>,
    /// Index of hovered pin for tooltip display.
    pub hovered_pin: Option<usize>,
}

impl WorldMapFrameState {
    pub fn coord_text(&self) -> String {
        format!("{:.1}, {:.1}", self.player_x * 100.0, self.player_y * 100.0)
    }
}

// --- Screen entry ---

pub fn world_map_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<WorldMapFrameState>()
        .expect("WorldMapFrameState must be in SharedContext");
    let hide = !state.visible;
    let coords = state.coord_text();
    rsx! {
        r#frame {
            name: "WorldMapFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            strata: FrameStrata::Fullscreen,
            hidden: hide,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: "0",
            }
            {header_bar(&state.zone_name, &coords)}
            {dropdown_nav(&state.continent_name, &state.zone_name)}
            {map_canvas()}
            {flight_path_lines(&state.flight_paths)}
            {zone_overlays(&state.zone_overlays)}
            {map_pins(&state.pins)}
            {fog_overlays(&state.fog_overlays)}
            {map_legend()}
            {pin_tooltip(&state.pins, state.hovered_pin)}
            {close_button()}
        }
    }
}

// --- Header bar ---

fn header_zone_label(text: &str) -> Element {
    rsx! {
        fontstring {
            name: "WorldMapZoneName",
            width: {ZONE_NAME_W},
            height: {HEADER_H},
            text: text,
            font_size: 16.0,
            font_color: ZONE_NAME_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {HEADER_INSET}, y: "0" }
        }
    }
}

fn header_coord_label(text: &str) -> Element {
    rsx! {
        fontstring {
            name: "WorldMapCoords",
            width: {COORD_W},
            height: {COORD_H},
            text: text,
            font_size: 11.0,
            font_color: COORD_COLOR,
            justify_h: "RIGHT",
            anchor { point: AnchorPoint::TopRight, relative_point: AnchorPoint::TopRight, x: {-(CLOSE_BTN_SIZE + CLOSE_BTN_INSET + 8.0)}, y: {-(HEADER_H - COORD_H) / 2.0} }
        }
    }
}

fn header_bar(zone_name: &str, coord_text: &str) -> Element {
    rsx! {
        r#frame {
            name: "WorldMapHeader",
            width: {FRAME_W},
            height: {HEADER_H},
            background_color: HEADER_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: "0",
            }
            {header_zone_label(zone_name)}
            {header_coord_label(coord_text)}
        }
    }
}

// --- Map canvas ---

fn map_canvas() -> Element {
    rsx! {
        r#frame {
            name: "WorldMapCanvas",
            width: {CANVAS_W},
            height: {CANVAS_H},
            background_color: CANVAS_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {CANVAS_INSET},
                y: {-CANVAS_TOP},
            }
        }
    }
}

// --- Dropdown navigation ---

fn nav_dropdown(
    frame_name: &str,
    label_name: &str,
    arrow_name: &str,
    text: &str,
    x: f32,
    y: f32,
) -> Element {
    rsx! {
        r#frame {
            name: DynName(frame_name.into()),
            width: {DROPDOWN_W},
            height: {DROPDOWN_H},
            background_color: DROPDOWN_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            fontstring {
                name: DynName(label_name.into()),
                width: {DROPDOWN_W - 20.0},
                height: {DROPDOWN_H},
                text: text,
                font_size: 10.0,
                font_color: DROPDOWN_TEXT_COLOR,
                justify_h: "LEFT",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "6", y: "0" }
            }
            fontstring {
                name: DynName(arrow_name.into()),
                width: 14.0,
                height: {DROPDOWN_H},
                text: "▼",
                font_size: 9.0,
                font_color: DROPDOWN_ARROW_COLOR,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopRight, relative_point: AnchorPoint::TopRight, x: "-4", y: "0" }
            }
        }
    }
}

fn dropdown_nav(continent: &str, zone: &str) -> Element {
    let y = -((HEADER_H - DROPDOWN_H) / 2.0);
    rsx! {
        {nav_dropdown("WorldMapContinentDropdown", "WorldMapContinentLabel", "WorldMapContinentArrow", continent, DROPDOWN_X, y)}
        {nav_dropdown("WorldMapZoneDropdown", "WorldMapZoneDropLabel", "WorldMapZoneDropArrow", zone, DROPDOWN_X + DROPDOWN_W + DROPDOWN_GAP, y)}
    }
}

// --- Zone overlay buttons ---

fn zone_overlay_frame(i: usize, ov: &ZoneOverlay) -> Element {
    let id = DynName(format!("WorldMapZoneOv{i}"));
    let label_id = DynName(format!("WorldMapZoneOv{i}Label"));
    let x = CANVAS_INSET + ov.x * CANVAS_W;
    let y = CANVAS_TOP + ov.y * CANVAS_H;
    let w = ov.w * CANVAS_W;
    let h = ov.h * CANVAS_H;
    rsx! {
        r#frame {
            name: id,
            width: {w},
            height: {h},
            background_color: ZONE_OVERLAY_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {-y},
            }
            fontstring {
                name: label_id,
                width: {w},
                height: {h},
                text: {ov.name.as_str()},
                font_size: 10.0,
                font_color: ZONE_OVERLAY_TEXT,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
    }
}

fn zone_overlays(overlays: &[ZoneOverlay]) -> Element {
    overlays
        .iter()
        .enumerate()
        .take(ZONE_OVERLAY_MAX)
        .flat_map(|(i, ov)| zone_overlay_frame(i, ov))
        .collect()
}

fn fog_overlay_frame(i: usize, ov: &ZoneOverlay) -> Element {
    let id = DynName(format!("WorldMapFogOv{i}"));
    let label_id = DynName(format!("WorldMapFogOv{i}Label"));
    let x = CANVAS_INSET + ov.x * CANVAS_W;
    let y = CANVAS_TOP + ov.y * CANVAS_H;
    let w = ov.w * CANVAS_W;
    let h = ov.h * CANVAS_H;
    rsx! {
        r#frame {
            name: id,
            width: {w},
            height: {h},
            background_color: FOG_OVERLAY_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {-y},
            }
            fontstring {
                name: label_id,
                width: {w},
                height: {h},
                text: {ov.name.as_str()},
                font_size: 12.0,
                font_color: FOG_OVERLAY_TEXT,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
    }
}

fn fog_overlays(overlays: &[ZoneOverlay]) -> Element {
    overlays
        .iter()
        .enumerate()
        .take(ZONE_OVERLAY_MAX)
        .flat_map(|(i, ov)| fog_overlay_frame(i, ov))
        .collect()
}

// --- Map pins ---

fn map_pins(pins: &[MapPin]) -> Element {
    pins.iter()
        .enumerate()
        .take(MAX_PINS)
        .flat_map(|(i, pin)| {
            let id = DynName(format!("WorldMapPin{i}"));
            let label_id = DynName(format!("WorldMapPin{i}Symbol"));
            let x = CANVAS_INSET + pin.x * CANVAS_W - PIN_SIZE / 2.0;
            let y = CANVAS_TOP + pin.y * CANVAS_H - PIN_SIZE / 2.0;
            rsx! {
                r#frame {
                    name: id,
                    width: {PIN_SIZE},
                    height: {PIN_SIZE},
                    background_color: {pin.pin_type.color()},
                    anchor {
                        point: AnchorPoint::TopLeft,
                        relative_point: AnchorPoint::TopLeft,
                        x: {x},
                        y: {-y},
                    }
                    fontstring {
                        name: label_id,
                        width: {PIN_SIZE},
                        height: {PIN_SIZE},
                        text: {pin.pin_type.symbol()},
                        font_size: 10.0,
                        font_color: "1.0,1.0,1.0,1.0",
                        justify_h: "CENTER",
                        anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
                    }
                }
            }
        })
        .collect()
}

// --- Flight path lines (dot at each endpoint) ---

fn fp_dot(id: DynName, cx: f32, cy: f32) -> Element {
    rsx! {
        r#frame {
            name: id,
            width: {FP_DOT_SIZE},
            height: {FP_DOT_SIZE},
            background_color: FP_DOT_COLOR,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {cx - FP_DOT_SIZE / 2.0}, y: {-(cy - FP_DOT_SIZE / 2.0)} }
        }
    }
}

fn fp_segment(i: usize, seg: &FlightPathSegment) -> Element {
    let x1 = CANVAS_INSET + seg.x1 * CANVAS_W;
    let y1 = CANVAS_TOP + seg.y1 * CANVAS_H;
    let x2 = CANVAS_INSET + seg.x2 * CANVAS_W;
    let y2 = CANVAS_TOP + seg.y2 * CANVAS_H;
    let line_x = x1.min(x2);
    let line_y = y1.min(y2);
    let line_w = (x2 - x1).abs().max(FP_LINE_H);
    let line_h = (y2 - y1).abs().max(FP_LINE_H);
    rsx! {
        r#frame {
            name: DynName(format!("WorldMapFP{i}Line")),
            width: {line_w},
            height: {line_h},
            background_color: FP_LINE_COLOR,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {line_x}, y: {-line_y} }
        }
        {fp_dot(DynName(format!("WorldMapFP{i}Dot1")), x1, y1)}
        {fp_dot(DynName(format!("WorldMapFP{i}Dot2")), x2, y2)}
    }
}

fn flight_path_lines(segments: &[FlightPathSegment]) -> Element {
    segments
        .iter()
        .enumerate()
        .take(MAX_FP_SEGMENTS)
        .flat_map(|(i, seg)| fp_segment(i, seg))
        .collect()
}

// --- Map legend (bottom-left corner of canvas) ---

fn legend_title() -> Element {
    rsx! {
        fontstring {
            name: "WorldMapLegendTitle",
            width: {LEGEND_W - 2.0 * LEGEND_INSET},
            height: {LEGEND_HEADER_H},
            text: "Legend",
            font_size: 10.0,
            font_color: LEGEND_HEADER_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {LEGEND_INSET}, y: {-LEGEND_INSET} }
        }
    }
}

fn build_legend_rows() -> Element {
    [
        (MapPinType::Quest, "Quests"),
        (MapPinType::FlightPath, "Flight Paths"),
        (MapPinType::PointOfInterest, "Points of Interest"),
    ]
    .iter()
    .enumerate()
    .flat_map(|(i, (pt, label))| legend_row(i, *pt, label))
    .collect()
}

fn map_legend() -> Element {
    let legend_h = LEGEND_HEADER_H + 3.0 * LEGEND_ROW_H + 2.0 * LEGEND_INSET;
    let legend_x = CANVAS_INSET + 8.0;
    let legend_y = CANVAS_TOP + CANVAS_H - legend_h - 8.0;
    let rows = build_legend_rows();
    rsx! {
        r#frame {
            name: "WorldMapLegend",
            width: {LEGEND_W},
            height: {legend_h},
            background_color: LEGEND_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {legend_x},
                y: {-legend_y},
            }
            {legend_title()}
            {rows}
        }
    }
}

fn legend_row(idx: usize, pin_type: MapPinType, label: &str) -> Element {
    let icon_id = DynName(format!("WorldMapLegendIcon{idx}"));
    let text_id = DynName(format!("WorldMapLegendText{idx}"));
    let y = LEGEND_INSET + LEGEND_HEADER_H + idx as f32 * LEGEND_ROW_H;
    rsx! {
        r#frame {
            name: icon_id,
            width: {LEGEND_ICON_SIZE},
            height: {LEGEND_ICON_SIZE},
            background_color: {pin_type.color()},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {LEGEND_INSET},
                y: {-y},
            }
        }
        fontstring {
            name: text_id,
            width: {LEGEND_W - LEGEND_ICON_SIZE - 3.0 * LEGEND_INSET},
            height: {LEGEND_ROW_H},
            text: label,
            font_size: 9.0,
            font_color: LEGEND_TEXT_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {LEGEND_INSET + LEGEND_ICON_SIZE + LEGEND_INSET},
                y: {-y},
            }
        }
    }
}

// --- Pin tooltip ---

fn pin_tooltip_line(name: &str, text: &str, font_size: f32, color: &str, y: f32) -> Element {
    rsx! {
        fontstring {
            name: DynName(name.into()),
            width: {TOOLTIP_W - 2.0 * TOOLTIP_INSET},
            height: {TOOLTIP_LINE_H},
            text: text,
            font_size: font_size,
            font_color: color,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {TOOLTIP_INSET}, y: {y} }
        }
    }
}

fn pin_tooltip(pins: &[MapPin], hovered: Option<usize>) -> Element {
    let hide = hovered.is_none();
    let (title, subtitle) = match hovered {
        Some(idx) if idx < pins.len() => {
            let pin = &pins[idx];
            (pin.label.as_str(), pin.pin_type.symbol())
        }
        _ => ("", ""),
    };
    let tooltip_h = 2.0 * TOOLTIP_INSET + TOOLTIP_LINE_H * 2.0;
    rsx! {
        r#frame {
            name: "WorldMapPinTooltip",
            width: {TOOLTIP_W},
            height: {tooltip_h},
            hidden: hide,
            background_color: TOOLTIP_BG,
            anchor {
                point: AnchorPoint::BottomRight,
                relative_point: AnchorPoint::BottomRight,
                x: {-CANVAS_INSET - 8.0},
                y: {CANVAS_INSET + 8.0},
            }
            {pin_tooltip_line("WorldMapPinTooltipTitle", title, 11.0, TOOLTIP_TITLE_COLOR, -TOOLTIP_INSET)}
            {pin_tooltip_line("WorldMapPinTooltipType", subtitle, 9.0, TOOLTIP_TEXT_COLOR, -(TOOLTIP_INSET + TOOLTIP_LINE_H))}
        }
    }
}

// --- Close button ---

fn close_button() -> Element {
    rsx! {
        r#frame {
            name: "WorldMapCloseBtn",
            width: {CLOSE_BTN_SIZE},
            height: {CLOSE_BTN_SIZE},
            onclick: ACTION_WORLD_MAP_CLOSE,
            background_color: CLOSE_BTN_BG,
            anchor {
                point: AnchorPoint::TopRight,
                relative_point: AnchorPoint::TopRight,
                x: {-CLOSE_BTN_INSET},
                y: {-(HEADER_H - CLOSE_BTN_SIZE) / 2.0},
            }
            fontstring {
                name: "WorldMapCloseBtnText",
                width: {CLOSE_BTN_SIZE},
                height: {CLOSE_BTN_SIZE},
                text: "X",
                font_size: 12.0,
                font_color: CLOSE_BTN_TEXT,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
    }
}

#[cfg(test)]
#[path = "world_map_frame_component_tests.rs"]
mod tests;
