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
const PIN_QUEST_BG: &str = "1.0,0.82,0.0,0.9";
const PIN_FP_BG: &str = "0.3,0.8,0.3,0.9";
const PIN_POI_BG: &str = "0.6,0.6,0.6,0.9";

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

#[derive(Clone, Debug, PartialEq, Default)]
pub struct WorldMapFrameState {
    pub visible: bool,
    pub zone_name: String,
    pub player_x: f32,
    pub player_y: f32,
    pub continent_name: String,
    pub zone_overlays: Vec<ZoneOverlay>,
    pub pins: Vec<MapPin>,
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
            {zone_overlays(&state.zone_overlays)}
            {map_pins(&state.pins)}
            {close_button()}
        }
    }
}

// --- Header bar ---

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
            fontstring {
                name: "WorldMapZoneName",
                width: {ZONE_NAME_W},
                height: {HEADER_H},
                text: zone_name,
                font_size: 16.0,
                font_color: ZONE_NAME_COLOR,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {HEADER_INSET},
                    y: "0",
                }
            }
            fontstring {
                name: "WorldMapCoords",
                width: {COORD_W},
                height: {COORD_H},
                text: coord_text,
                font_size: 11.0,
                font_color: COORD_COLOR,
                justify_h: "RIGHT",
                anchor {
                    point: AnchorPoint::TopRight,
                    relative_point: AnchorPoint::TopRight,
                    x: {-(CLOSE_BTN_SIZE + CLOSE_BTN_INSET + 8.0)},
                    y: {-(HEADER_H - COORD_H) / 2.0},
                }
            }
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

fn dropdown_nav(continent: &str, zone: &str) -> Element {
    let dropdown_y = (HEADER_H - DROPDOWN_H) / 2.0;
    let zone_x = DROPDOWN_X + DROPDOWN_W + DROPDOWN_GAP;
    rsx! {
        r#frame {
            name: "WorldMapContinentDropdown",
            width: {DROPDOWN_W},
            height: {DROPDOWN_H},
            background_color: DROPDOWN_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {DROPDOWN_X},
                y: {-dropdown_y},
            }
            fontstring {
                name: "WorldMapContinentLabel",
                width: {DROPDOWN_W - 20.0},
                height: {DROPDOWN_H},
                text: continent,
                font_size: 10.0,
                font_color: DROPDOWN_TEXT_COLOR,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "6",
                    y: "0",
                }
            }
            fontstring {
                name: "WorldMapContinentArrow",
                width: 14.0,
                height: {DROPDOWN_H},
                text: "▼",
                font_size: 9.0,
                font_color: DROPDOWN_ARROW_COLOR,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopRight,
                    relative_point: AnchorPoint::TopRight,
                    x: "-4",
                    y: "0",
                }
            }
        }
        r#frame {
            name: "WorldMapZoneDropdown",
            width: {DROPDOWN_W},
            height: {DROPDOWN_H},
            background_color: DROPDOWN_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {zone_x},
                y: {-dropdown_y},
            }
            fontstring {
                name: "WorldMapZoneDropLabel",
                width: {DROPDOWN_W - 20.0},
                height: {DROPDOWN_H},
                text: zone,
                font_size: 10.0,
                font_color: DROPDOWN_TEXT_COLOR,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "6",
                    y: "0",
                }
            }
            fontstring {
                name: "WorldMapZoneDropArrow",
                width: 14.0,
                height: {DROPDOWN_H},
                text: "▼",
                font_size: 9.0,
                font_color: DROPDOWN_ARROW_COLOR,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopRight,
                    relative_point: AnchorPoint::TopRight,
                    x: "-4",
                    y: "0",
                }
            }
        }
    }
}

// --- Zone overlay buttons ---

fn zone_overlays(overlays: &[ZoneOverlay]) -> Element {
    overlays
        .iter()
        .enumerate()
        .take(ZONE_OVERLAY_MAX)
        .flat_map(|(i, ov)| {
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
        })
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

// --- Close button ---

fn close_button() -> Element {
    rsx! {
        r#frame {
            name: "WorldMapCloseBtn",
            width: {CLOSE_BTN_SIZE},
            height: {CLOSE_BTN_SIZE},
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
mod tests {
    use super::*;
    use ui_toolkit::layout::{LayoutRect, recompute_layouts};
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn sample_state() -> WorldMapFrameState {
        WorldMapFrameState {
            visible: true,
            zone_name: "Elwynn Forest".into(),
            player_x: 0.425,
            player_y: 0.637,
            continent_name: "Eastern Kingdoms".into(),
            zone_overlays: vec![
                ZoneOverlay {
                    name: "Goldshire".into(),
                    x: 0.3,
                    y: 0.5,
                    w: 0.2,
                    h: 0.15,
                },
                ZoneOverlay {
                    name: "Northshire".into(),
                    x: 0.5,
                    y: 0.2,
                    w: 0.15,
                    h: 0.1,
                },
            ],
            pins: vec![
                MapPin {
                    pin_type: MapPinType::Quest,
                    label: "Quest Hub".into(),
                    x: 0.35,
                    y: 0.55,
                },
                MapPin {
                    pin_type: MapPinType::FlightPath,
                    label: "Goldshire FP".into(),
                    x: 0.32,
                    y: 0.52,
                },
                MapPin {
                    pin_type: MapPinType::PointOfInterest,
                    label: "Lion's Pride Inn".into(),
                    x: 0.34,
                    y: 0.54,
                },
            ],
        }
    }

    fn build_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(sample_state());
        Screen::new(world_map_frame_screen).sync(&shared, &mut reg);
        reg
    }

    fn layout_registry() -> FrameRegistry {
        let mut reg = build_registry();
        recompute_layouts(&mut reg);
        reg
    }

    fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
        reg.get(reg.get_by_name(name).expect(name))
            .and_then(|f| f.layout_rect.clone())
            .unwrap_or_else(|| panic!("{name} has no layout_rect"))
    }

    // --- Structure tests ---

    #[test]
    fn builds_frame_and_elements() {
        let reg = build_registry();
        assert!(reg.get_by_name("WorldMapFrame").is_some());
        assert!(reg.get_by_name("WorldMapHeader").is_some());
        assert!(reg.get_by_name("WorldMapZoneName").is_some());
        assert!(reg.get_by_name("WorldMapCoords").is_some());
        assert!(reg.get_by_name("WorldMapCanvas").is_some());
        assert!(reg.get_by_name("WorldMapCloseBtn").is_some());
        assert!(reg.get_by_name("WorldMapCloseBtnText").is_some());
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(WorldMapFrameState::default());
        Screen::new(world_map_frame_screen).sync(&shared, &mut reg);
        let id = reg.get_by_name("WorldMapFrame").expect("frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    // --- Data model tests ---

    #[test]
    fn coord_text_formatting() {
        let state = sample_state();
        assert_eq!(state.coord_text(), "42.5, 63.7");
    }

    #[test]
    fn coord_text_zero() {
        let state = WorldMapFrameState::default();
        assert_eq!(state.coord_text(), "0.0, 0.0");
    }

    // --- Coord validation ---

    #[test]
    fn coord_frame_fullscreen() {
        let reg = layout_registry();
        let r = rect(&reg, "WorldMapFrame");
        assert!((r.x).abs() < 1.0);
        assert!((r.y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
        assert!((r.height - FRAME_H).abs() < 1.0);
    }

    #[test]
    fn coord_header_top() {
        let reg = layout_registry();
        let r = rect(&reg, "WorldMapHeader");
        assert!((r.x).abs() < 1.0);
        assert!((r.y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
        assert!((r.height - HEADER_H).abs() < 1.0);
    }

    #[test]
    fn coord_zone_name_left_aligned() {
        let reg = layout_registry();
        let header_r = rect(&reg, "WorldMapHeader");
        let name_r = rect(&reg, "WorldMapZoneName");
        assert!((name_r.x - (header_r.x + HEADER_INSET)).abs() < 1.0);
        assert!((name_r.y - header_r.y).abs() < 1.0);
        assert!((name_r.width - ZONE_NAME_W).abs() < 1.0);
    }

    #[test]
    fn coord_canvas_below_header() {
        let reg = layout_registry();
        let r = rect(&reg, "WorldMapCanvas");
        assert!((r.x - CANVAS_INSET).abs() < 1.0);
        assert!((r.y - CANVAS_TOP).abs() < 1.0);
        assert!((r.width - CANVAS_W).abs() < 1.0);
        assert!((r.height - CANVAS_H).abs() < 1.0);
    }

    #[test]
    fn coord_close_button_top_right() {
        let reg = layout_registry();
        let frame_r = rect(&reg, "WorldMapFrame");
        let btn_r = rect(&reg, "WorldMapCloseBtn");
        let expected_right = frame_r.x + frame_r.width;
        assert!((btn_r.x + btn_r.width + CLOSE_BTN_INSET - expected_right).abs() < 1.0);
        assert!((btn_r.width - CLOSE_BTN_SIZE).abs() < 1.0);
        assert!((btn_r.height - CLOSE_BTN_SIZE).abs() < 1.0);
    }

    // --- Dropdown tests ---

    #[test]
    fn builds_dropdowns() {
        let reg = build_registry();
        assert!(reg.get_by_name("WorldMapContinentDropdown").is_some());
        assert!(reg.get_by_name("WorldMapContinentLabel").is_some());
        assert!(reg.get_by_name("WorldMapContinentArrow").is_some());
        assert!(reg.get_by_name("WorldMapZoneDropdown").is_some());
        assert!(reg.get_by_name("WorldMapZoneDropLabel").is_some());
        assert!(reg.get_by_name("WorldMapZoneDropArrow").is_some());
    }

    #[test]
    fn coord_dropdowns_in_header() {
        let reg = layout_registry();
        let cont_r = rect(&reg, "WorldMapContinentDropdown");
        let zone_r = rect(&reg, "WorldMapZoneDropdown");
        assert!((cont_r.x - DROPDOWN_X).abs() < 1.0);
        assert!((cont_r.width - DROPDOWN_W).abs() < 1.0);
        // Zone dropdown to the right of continent
        let expected_zone_x = DROPDOWN_X + DROPDOWN_W + DROPDOWN_GAP;
        assert!((zone_r.x - expected_zone_x).abs() < 1.0);
    }

    // --- Zone overlay tests ---

    #[test]
    fn builds_zone_overlays() {
        let reg = build_registry();
        assert!(reg.get_by_name("WorldMapZoneOv0").is_some());
        assert!(reg.get_by_name("WorldMapZoneOv0Label").is_some());
        assert!(reg.get_by_name("WorldMapZoneOv1").is_some());
        assert!(reg.get_by_name("WorldMapZoneOv2").is_none());
    }

    #[test]
    fn coord_zone_overlay_positioned_on_canvas() {
        let reg = layout_registry();
        let ov_r = rect(&reg, "WorldMapZoneOv0");
        // Goldshire: x=0.3, y=0.5, w=0.2, h=0.15
        let expected_x = CANVAS_INSET + 0.3 * CANVAS_W;
        let expected_y = CANVAS_TOP + 0.5 * CANVAS_H;
        let expected_w = 0.2 * CANVAS_W;
        let expected_h = 0.15 * CANVAS_H;
        assert!((ov_r.x - expected_x).abs() < 1.0);
        assert!((ov_r.y - expected_y).abs() < 1.0);
        assert!((ov_r.width - expected_w).abs() < 1.0);
        assert!((ov_r.height - expected_h).abs() < 1.0);
    }

    // --- Pin tests ---

    #[test]
    fn builds_map_pins() {
        let reg = build_registry();
        for i in 0..3 {
            assert!(
                reg.get_by_name(&format!("WorldMapPin{i}")).is_some(),
                "WorldMapPin{i} missing"
            );
            assert!(
                reg.get_by_name(&format!("WorldMapPin{i}Symbol")).is_some(),
                "WorldMapPin{i}Symbol missing"
            );
        }
        assert!(reg.get_by_name("WorldMapPin3").is_none());
    }

    #[test]
    fn coord_pin_centered_on_position() {
        let reg = layout_registry();
        let pin_r = rect(&reg, "WorldMapPin0");
        // Quest Hub at x=0.35, y=0.55, pin centered
        let expected_x = CANVAS_INSET + 0.35 * CANVAS_W - PIN_SIZE / 2.0;
        let expected_y = CANVAS_TOP + 0.55 * CANVAS_H - PIN_SIZE / 2.0;
        assert!((pin_r.x - expected_x).abs() < 1.0);
        assert!((pin_r.y - expected_y).abs() < 1.0);
        assert!((pin_r.width - PIN_SIZE).abs() < 1.0);
    }

    // --- Data model tests ---

    #[test]
    fn pin_type_symbols() {
        assert_eq!(MapPinType::Quest.symbol(), "!");
        assert_eq!(MapPinType::FlightPath.symbol(), "⚑");
        assert_eq!(MapPinType::PointOfInterest.symbol(), "●");
    }

    #[test]
    fn pin_type_colors_non_empty() {
        assert!(!MapPinType::Quest.color().is_empty());
        assert!(!MapPinType::FlightPath.color().is_empty());
        assert!(!MapPinType::PointOfInterest.color().is_empty());
    }
}
