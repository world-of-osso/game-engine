use super::*;
use ui_toolkit::frame::WidgetData;
use ui_toolkit::layout::{LayoutRect, recompute_layouts};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn ov(name: &str, x: f32, y: f32, w: f32, h: f32) -> ZoneOverlay {
    ZoneOverlay {
        name: name.into(),
        x,
        y,
        w,
        h,
    }
}

fn pin(pt: MapPinType, label: &str, x: f32, y: f32) -> MapPin {
    MapPin {
        pin_type: pt,
        label: label.into(),
        x,
        y,
    }
}

fn sample_state() -> WorldMapFrameState {
    WorldMapFrameState {
        visible: true,
        zone_name: "Elwynn Forest".into(),
        player_x: 0.425,
        player_y: 0.637,
        continent_name: "Eastern Kingdoms".into(),
        map_texture_fdid: 123456,
        zone_overlays: vec![
            ov("Goldshire", 0.3, 0.5, 0.2, 0.15),
            ov("Northshire", 0.5, 0.2, 0.15, 0.1),
        ],
        fog_overlays: Vec::new(),
        pins: vec![
            pin(MapPinType::Quest, "Quest Hub", 0.35, 0.55),
            pin(MapPinType::FlightPath, "Goldshire FP", 0.32, 0.52),
            pin(MapPinType::PointOfInterest, "Lion's Pride Inn", 0.34, 0.54),
        ],
        flight_paths: vec![FlightPathSegment {
            x1: 0.32,
            y1: 0.52,
            x2: 0.6,
            y2: 0.3,
        }],
        hovered_pin: None,
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
fn builds_map_canvas_texture_when_fdid_present() {
    let reg = build_registry();
    let texture_id = reg
        .get_by_name("WorldMapCanvasTexture")
        .expect("canvas texture frame");
    let frame = reg.get(texture_id).expect("canvas texture data");
    let widget = frame.widget_data.as_ref().expect("texture widget");
    let WidgetData::Texture(texture) = widget else {
        panic!("expected texture widget");
    };
    assert!(matches!(
        texture.source,
        crate::ui::widgets::texture::TextureSource::FileDataId(123456)
    ));
}

#[test]
fn hides_map_canvas_texture_when_fdid_is_zero() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    let mut state = sample_state();
    state.map_texture_fdid = 0;
    shared.insert(state);
    Screen::new(world_map_frame_screen).sync(&shared, &mut reg);

    assert!(reg.get_by_name("WorldMapCanvasTexture").is_none());
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

// --- Legend tests ---

#[test]
fn builds_legend() {
    let reg = build_registry();
    assert!(reg.get_by_name("WorldMapLegend").is_some());
    assert!(reg.get_by_name("WorldMapLegendTitle").is_some());
    for i in 0..3 {
        assert!(reg.get_by_name(&format!("WorldMapLegendIcon{i}")).is_some());
        assert!(reg.get_by_name(&format!("WorldMapLegendText{i}")).is_some());
    }
}

#[test]
fn coord_legend_dimensions() {
    let reg = layout_registry();
    let r = rect(&reg, "WorldMapLegend");
    assert!((r.width - LEGEND_W).abs() < 1.0);
}

// --- Flight path tests ---

#[test]
fn builds_flight_path_elements() {
    let reg = build_registry();
    assert!(reg.get_by_name("WorldMapFP0Line").is_some());
    assert!(reg.get_by_name("WorldMapFP0Dot1").is_some());
    assert!(reg.get_by_name("WorldMapFP0Dot2").is_some());
    assert!(reg.get_by_name("WorldMapFP1Line").is_none());
}

#[test]
fn coord_flight_path_dots() {
    let reg = layout_registry();
    let dot1 = rect(&reg, "WorldMapFP0Dot1");
    let dot2 = rect(&reg, "WorldMapFP0Dot2");
    assert!((dot1.width - FP_DOT_SIZE).abs() < 1.0);
    assert!((dot2.width - FP_DOT_SIZE).abs() < 1.0);
    // Dot1 at (0.32, 0.52), dot2 at (0.6, 0.3) — dot2 is to the right
    assert!(dot2.x > dot1.x);
}

// --- Tooltip tests ---

#[test]
fn tooltip_hidden_when_no_hover() {
    let reg = build_registry();
    let id = reg.get_by_name("WorldMapPinTooltip").expect("tooltip");
    assert!(reg.get(id).expect("data").hidden);
}

#[test]
fn tooltip_visible_when_hovered() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    let mut state = sample_state();
    state.hovered_pin = Some(0);
    shared.insert(state);
    Screen::new(world_map_frame_screen).sync(&shared, &mut reg);
    let id = reg.get_by_name("WorldMapPinTooltip").expect("tooltip");
    assert!(!reg.get(id).expect("data").hidden);
    assert!(reg.get_by_name("WorldMapPinTooltipTitle").is_some());
    assert!(reg.get_by_name("WorldMapPinTooltipType").is_some());
}

// --- Additional coord validation ---

#[test]
fn coord_dropdown_dimensions() {
    let reg = layout_registry();
    let cont_r = rect(&reg, "WorldMapContinentDropdown");
    let zone_r = rect(&reg, "WorldMapZoneDropdown");
    assert!((cont_r.width - DROPDOWN_W).abs() < 1.0);
    assert!((cont_r.height - DROPDOWN_H).abs() < 1.0);
    assert!((zone_r.width - DROPDOWN_W).abs() < 1.0);
    assert!((zone_r.height - DROPDOWN_H).abs() < 1.0);
}

#[test]
fn coord_legend_bottom_left_of_canvas() {
    let reg = layout_registry();
    let legend_r = rect(&reg, "WorldMapLegend");
    // Legend in lower-left area of canvas
    let canvas_bottom = CANVAS_TOP + CANVAS_H;
    assert!(legend_r.y + legend_r.height < canvas_bottom + 1.0);
    assert!((legend_r.x - (CANVAS_INSET + 8.0)).abs() < 1.0);
}

#[test]
fn coord_second_zone_overlay() {
    let reg = layout_registry();
    let ov_r = rect(&reg, "WorldMapZoneOv1");
    // Northshire: x=0.5, y=0.2, w=0.15, h=0.1
    let expected_x = CANVAS_INSET + 0.5 * CANVAS_W;
    let expected_y = CANVAS_TOP + 0.2 * CANVAS_H;
    assert!((ov_r.x - expected_x).abs() < 1.0);
    assert!((ov_r.y - expected_y).abs() < 1.0);
}

#[test]
fn builds_fog_overlay_when_present() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    let mut state = sample_state();
    state.fog_overlays = vec![ov("Unexplored: Elwynn Forest", 0.0, 0.0, 1.0, 1.0)];
    shared.insert(state);
    Screen::new(world_map_frame_screen).sync(&shared, &mut reg);

    assert!(reg.get_by_name("WorldMapFogOv0").is_some());
    assert!(reg.get_by_name("WorldMapFogOv0Label").is_some());
}

#[test]
fn coord_tooltip_width() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    let mut state = sample_state();
    state.hovered_pin = Some(0);
    shared.insert(state);
    Screen::new(world_map_frame_screen).sync(&shared, &mut reg);
    recompute_layouts(&mut reg);
    let r = rect(&reg, "WorldMapPinTooltip");
    assert!((r.width - TOOLTIP_W).abs() < 1.0);
}

#[test]
fn coord_flight_path_line_bounds() {
    let reg = layout_registry();
    let line_r = rect(&reg, "WorldMapFP0Line");
    // Line between (0.32, 0.52) and (0.6, 0.3)
    let x1 = CANVAS_INSET + 0.32 * CANVAS_W;
    let x2 = CANVAS_INSET + 0.6 * CANVAS_W;
    let expected_x = x1.min(x2);
    let expected_w = (x2 - x1).abs().max(FP_LINE_H);
    assert!((line_r.x - expected_x).abs() < 1.0);
    assert!((line_r.width - expected_w).abs() < 1.0);
}
