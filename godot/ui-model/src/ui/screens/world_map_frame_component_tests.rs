use super::*;
use crate::ui::screens::menu_character_layout_test_support::compute_layout;
use ui_toolkit::layout::LayoutRect;
use ui_toolkit::screen::Screen;

const VIEWPORT: [f32; 2] = [1280.0, 720.0];

fn sample_state() -> WorldMapFrameState {
    WorldMapFrameState {
        visible: true,
        viewport: VIEWPORT,
        map_id: 37,
        map_name: "Elwynn Forest".into(),
        breadcrumbs: vec![
            MapBreadcrumb {
                map_id: 947,
                name: "Azeroth".into(),
            },
            MapBreadcrumb {
                map_id: 13,
                name: "Eastern Kingdoms".into(),
            },
            MapBreadcrumb {
                map_id: 37,
                name: "Elwynn Forest".into(),
            },
        ],
        tiles: vec![MapTile {
            fdid: 271_579,
            rect: [0.0, 0.0, 256.0 / 1002.0, 256.0 / 668.0],
            tex_coords: [0.0, 1.0, 0.0, 1.0],
        }],
        highlight: None,
        pins: vec![MapPin {
            pin_type: MapPinType::QuestObjective,
            label: "Kobold Camp Cleanup".into(),
            badge: "1".into(),
            x: 0.5,
            y: 0.25,
        }],
        quest_areas: Vec::new(),
        player: Some(MapPlayerMarker {
            x: 0.4178,
            y: 0.6456,
            rotation: 1.25,
        }),
    }
}

fn laid_out(state: WorldMapFrameState) -> FrameRegistry {
    let mut registry = FrameRegistry::new(VIEWPORT[0], VIEWPORT[1]);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    Screen::new(world_map_frame_screen).sync(&shared, &mut registry);
    apply_world_map_postsetup(&state, &mut registry);
    compute_layout(&mut registry);
    registry
}

fn rect(registry: &FrameRegistry, name: &str) -> LayoutRect {
    registry
        .get(registry.get_by_name(name).expect(name))
        .and_then(|frame| frame.layout_rect.clone())
        .unwrap_or_else(|| panic!("{name} has no layout_rect"))
}

fn center(rect: &LayoutRect) -> [f32; 2] {
    [rect.x + rect.width / 2.0, rect.y + rect.height / 2.0]
}

#[test]
fn frame_fits_the_viewport_with_the_retail_canvas_aspect() {
    let registry = laid_out(sample_state());
    let canvas = rect(&registry, WORLD_MAP_CANVAS.0);
    let expected = WorldMapLayout::for_viewport(VIEWPORT).canvas_rect();
    assert!((canvas.x - expected[0]).abs() < 0.5 && (canvas.y - expected[1]).abs() < 0.5);
    assert!((canvas.width / canvas.height - CANVAS_W / CANVAS_H).abs() < 0.01);
    let border = rect(&registry, "WorldMapBorderFrame");
    assert!(border.y > 0.0 && border.y + border.height < VIEWPORT[1]);
    assert!(border.x > 0.0 && border.x + border.width < VIEWPORT[0]);
}

#[test]
fn player_arrow_and_pins_sit_at_their_map_positions() {
    let registry = laid_out(sample_state());
    let canvas = rect(&registry, WORLD_MAP_CANVAS.0);
    let arrow = center(&rect(&registry, WORLD_MAP_PLAYER_ARROW));
    assert!((arrow[0] - (canvas.x + 0.4178 * canvas.width)).abs() <= 1.0);
    assert!((arrow[1] - (canvas.y + 0.6456 * canvas.height)).abs() <= 1.0);
    let pin = center(&rect(&registry, "WorldMapPin0"));
    // Layout snaps sizes to whole pixels.
    assert!((pin[0] - (canvas.x + 0.5 * canvas.width)).abs() <= 1.0);
    assert!((pin[1] - (canvas.y + 0.25 * canvas.height)).abs() <= 1.0);
    let arrow_frame = registry
        .get(registry.get_by_name(WORLD_MAP_PLAYER_ARROW).unwrap())
        .unwrap();
    let Some(WidgetData::Texture(texture)) = &arrow_frame.widget_data else {
        panic!("arrow is a texture");
    };
    assert_eq!(texture.rotation, 1.25);
}

#[test]
fn breadcrumbs_navigate_to_their_maps_and_close_closes() {
    let mut registry = laid_out(sample_state());
    let crumb = registry.get_by_name("WorldMapNav1").unwrap();
    assert_eq!(
        registry.click_frame(crumb).as_deref(),
        Some("world_map_nav:13")
    );
    let close = registry.get_by_name("WorldMapCloseButton").unwrap();
    assert_eq!(
        registry.click_frame(close).as_deref(),
        Some(ACTION_WORLD_MAP_CLOSE)
    );
    let nav0 = rect(&registry, "WorldMapNav0");
    let nav1 = rect(&registry, "WorldMapNav1");
    assert!(nav1.x > nav0.x + nav0.width, "crumbs read left to right");
}

#[test]
fn hidden_state_hides_the_frame_and_no_player_draws_no_arrow() {
    let state = WorldMapFrameState {
        visible: false,
        player: None,
        ..sample_state()
    };
    let registry = laid_out(state);
    let root = registry.get(registry.get_by_name(WORLD_MAP_ROOT.0).unwrap());
    assert!(root.unwrap().hidden);
    assert!(registry.get_by_name(WORLD_MAP_PLAYER_ARROW).is_none());
}

#[test]
fn texture_list_covers_tiles_and_chrome() {
    let fdids = world_map_texture_fdids(&sample_state());
    assert!(fdids.contains(&271_579));
    assert!(fdids.contains(&art::PLAYER_ARROW.fdid));
    assert!(!fdids.contains(&0));
}
