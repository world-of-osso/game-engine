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

// --- Colors ---

const FRAME_BG: &str = "0.04,0.03,0.02,0.95";
const HEADER_BG: &str = "0.08,0.06,0.04,0.95";
const ZONE_NAME_COLOR: &str = "1.0,0.82,0.0,1.0";
const COORD_COLOR: &str = "0.8,0.8,0.8,1.0";
const CANVAS_BG: &str = "0.02,0.02,0.02,0.9";
const CLOSE_BTN_BG: &str = "0.3,0.08,0.08,0.9";
const CLOSE_BTN_TEXT: &str = "1.0,0.3,0.3,1.0";

// --- Data types ---

#[derive(Clone, Debug, PartialEq, Default)]
pub struct WorldMapFrameState {
    pub visible: bool,
    pub zone_name: String,
    pub player_x: f32,
    pub player_y: f32,
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
            {map_canvas()}
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
}
