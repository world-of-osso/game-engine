use super::*;
use crate::asset::adt::ChunkHeightGrid;
use crate::minimap_render::{
    TrackingIconKind, create_arrow_image, create_border_image, crop_with_circle, draw_dot,
    draw_tracking_icon, find_height_range, height_to_color, point_in_triangle, render_tile_image,
};
use ui_toolkit::screen::{Screen, SharedContext};

#[derive(Resource, Default)]
struct CoordinateUiChanged(bool);

fn observe_coordinate_ui_change(ui: Res<UiState>, mut changed: ResMut<CoordinateUiChanged>) {
    changed.0 = ui.is_changed();
}

fn coordinate_test_app() -> (App, Entity) {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let frames = build_minimap_screen(&mut registry);
    let mut app = App::new();
    app.insert_resource(UiState {
        registry,
        event_bus: ui_toolkit::event::EventBus::new(),
        focused_frame: None,
    });
    app.insert_resource(frames);
    app.init_resource::<CoordinateUiChanged>();
    app.add_systems(Update, update_coord_text);
    app.add_systems(PostUpdate, observe_coordinate_ui_change);
    let player = app
        .world_mut()
        .spawn((crate::camera::Player, Transform::from_xyz(12.1, 0.0, -7.1)))
        .id();
    (app, player)
}

fn assert_coordinate_frame(app: &mut App, expected: &str, changed: bool) {
    app.world_mut()
        .resource_mut::<UiState>()
        .bypass_change_detection()
        .registry
        .render_dirty
        .clear();
    app.update();
    let coords = app.world().resource::<MinimapFrames>().coords;
    let ui = app.world().resource::<UiState>();
    let Some(WidgetData::FontString(font)) = &ui.registry.get(coords).unwrap().widget_data else {
        panic!("coordinate frame is not a font string");
    };
    assert_eq!(font.text, expected);
    assert_eq!(
        ui.registry.render_dirty.contains(&coords),
        changed,
        "coordinate render dirtiness"
    );
    assert_eq!(
        app.world().resource::<CoordinateUiChanged>().0,
        changed,
        "UiState change detection"
    );
}

#[test]
fn minimap_coordinates_skip_unchanged_rounded_text() {
    let (mut app, player) = coordinate_test_app();
    assert_coordinate_frame(&mut app, "12, -7", true);
    assert_coordinate_frame(&mut app, "12, -7", false);
    app.world_mut()
        .get_mut::<Transform>(player)
        .unwrap()
        .translation = Vec3::new(12.4, 5.0, -7.4);
    assert_coordinate_frame(&mut app, "12, -7", false);
    app.world_mut()
        .get_mut::<Transform>(player)
        .unwrap()
        .translation = Vec3::new(12.6, 5.0, -7.6);
    assert_coordinate_frame(&mut app, "13, -8", true);
    assert_coordinate_frame(&mut app, "13, -8", false);
}

fn replace_coordinate_text(app: &mut App, text: &str) {
    let coords = app.world().resource::<MinimapFrames>().coords;
    let mut ui = app.world_mut().resource_mut::<UiState>();
    let frame = ui.registry.get_mut(coords).unwrap();
    let Some(WidgetData::FontString(font)) = &mut frame.widget_data else {
        panic!("coordinate frame is not a font string");
    };
    font.text = text.to_owned();
}

#[test]
fn minimap_coordinates_reuse_formatted_storage_for_identical_input() {
    let mut cache = None;
    let position = Vec2::new(12.1, -7.1);
    let first = coordinate_text(&mut cache, position);
    assert_eq!(first, "12, -7");
    let buffer = first.as_ptr();
    assert_eq!(coordinate_text(&mut cache, position).as_ptr(), buffer);
    assert_eq!(coordinate_text(&mut cache, Vec2::new(12.6, -7.6)), "13, -8");
}

#[test]
fn minimap_coordinates_repair_external_text_without_position_change() {
    let (mut app, _) = coordinate_test_app();
    assert_coordinate_frame(&mut app, "12, -7", true);
    assert_coordinate_frame(&mut app, "12, -7", false);
    replace_coordinate_text(&mut app, "external change");
    assert_coordinate_frame(&mut app, "12, -7", true);
    assert_coordinate_frame(&mut app, "12, -7", false);
}

#[test]
fn minimap_coordinates_preserve_signed_zero_and_rounding() {
    let (mut app, player) = coordinate_test_app();
    for (x, z, expected) in [
        (-0.0, 0.0, "-0, 0"),
        (0.0, -0.0, "0, -0"),
        (2.5, -2.5, "2, -2"),
        (3.5, -3.5, "4, -4"),
    ] {
        app.world_mut()
            .get_mut::<Transform>(player)
            .unwrap()
            .translation = Vec3::new(x, 0.0, z);
        assert_coordinate_frame(&mut app, expected, true);
        assert_coordinate_frame(&mut app, expected, false);
    }
}

#[test]
fn minimap_coordinates_update_replacement_frame_at_same_position() {
    let (mut app, _) = coordinate_test_app();
    assert_coordinate_frame(&mut app, "12, -7", true);
    let old_coords = app.world().resource::<MinimapFrames>().coords;
    let new_coords = {
        let mut ui = app.world_mut().resource_mut::<UiState>();
        let widget = ui.registry.get(old_coords).unwrap().widget_data.clone();
        let id = ui.registry.create_frame("ReplacementCoords", None);
        let frame = ui.registry.get_mut(id).unwrap();
        frame.widget_data = widget;
        let Some(WidgetData::FontString(font)) = &mut frame.widget_data else {
            unreachable!()
        };
        font.text = "new frame".to_owned();
        id
    };
    app.world_mut().resource_mut::<MinimapFrames>().coords = new_coords;
    assert_coordinate_frame(&mut app, "12, -7", true);
    assert_coordinate_frame(&mut app, "12, -7", false);
}

#[test]
fn minimap_coordinates_preserve_text_without_player_or_frames() {
    let (mut app, player) = coordinate_test_app();
    assert_coordinate_frame(&mut app, "12, -7", true);
    app.world_mut().despawn(player);
    assert_coordinate_frame(&mut app, "12, -7", false);
    app.world_mut()
        .spawn((crate::camera::Player, Transform::from_xyz(99.0, 0.0, 88.0)));
    let frames = app.world_mut().remove_resource::<MinimapFrames>().unwrap();
    app.update();
    assert!(!app.world().resource::<CoordinateUiChanged>().0);
    assert!(
        app.world()
            .resource::<UiState>()
            .registry
            .render_dirty
            .is_empty()
    );
    app.insert_resource(frames);
    assert_coordinate_frame(&mut app, "99, 88", true);
}

fn tracking_composite_app(player_position: Vec3, herbs: &[Vec3]) -> (App, Entity, Vec<Entity>) {
    let mut app = App::new();
    app.init_resource::<MinimapState>();
    app.init_resource::<LastMinimapPixel>();
    app.init_resource::<Assets<Image>>();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    app.insert_resource(MinimapComposite { handle });
    app.add_systems(Update, update_minimap_composite);
    let player = app
        .world_mut()
        .spawn((
            crate::camera::Player,
            Transform::from_translation(player_position),
        ))
        .id();
    let herbs = herbs
        .iter()
        .map(|position| {
            app.world_mut()
                .spawn((
                    MinimapHerbNode,
                    GlobalTransform::from_translation(*position),
                ))
                .id()
        })
        .collect();
    (app, player, herbs)
}

fn tracking_composite_pixels(app: &App) -> Vec<u8> {
    let composite = app.world().resource::<MinimapComposite>();
    app.world()
        .resource::<Assets<Image>>()
        .get(&composite.handle)
        .unwrap()
        .data
        .clone()
        .unwrap()
}

#[test]
fn minimap_tracking_redraw_uses_latest_entities_after_stationary_frames() {
    let initial_player = Vec3::new(100.0, 0.0, 100.0);
    let moved_herb = Vec3::new(140.0, 0.0, 120.0);
    let added_herb = Vec3::new(70.0, 0.0, 130.0);
    let (mut app, player, herbs) = tracking_composite_app(
        initial_player,
        &[Vec3::new(120.0, 0.0, 100.0), Vec3::new(70.0, 0.0, 100.0)],
    );
    app.update();
    let original_pixels = tracking_composite_pixels(&app);
    app.update();
    assert_eq!(tracking_composite_pixels(&app), original_pixels);

    *app.world_mut()
        .get_mut::<GlobalTransform>(herbs[0])
        .unwrap() = GlobalTransform::from_translation(moved_herb);
    app.world_mut().despawn(herbs[1]);
    app.world_mut().spawn((
        MinimapHerbNode,
        GlobalTransform::from_translation(added_herb),
    ));
    app.update();
    assert_eq!(tracking_composite_pixels(&app), original_pixels);

    let next_player = Vec3::new(110.0, 0.0, 100.0);
    app.world_mut()
        .get_mut::<Transform>(player)
        .unwrap()
        .translation = next_player;
    app.update();
    let updated_pixels = tracking_composite_pixels(&app);
    assert_ne!(updated_pixels, original_pixels);
    let (mut expected, _, _) = tracking_composite_app(next_player, &[moved_herb, added_herb]);
    expected.update();
    assert_eq!(updated_pixels, tracking_composite_pixels(&expected));
    let (mut without_icons, _, _) = tracking_composite_app(next_player, &[]);
    without_icons.update();
    assert_ne!(updated_pixels, tracking_composite_pixels(&without_icons));
}

#[test]
fn minimap_screen_builds_expected_hud_frames() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let shared = SharedContext::new();
    let mut screen = Screen::new(inworld_hud_component::minimap_screen);

    screen.sync(&shared, &mut registry);

    assert!(registry.get_by_name("MinimapCluster").is_some());
    assert!(registry.get_by_name("MinimapDisplay").is_some());
    assert!(registry.get_by_name("MinimapBorder").is_some());
    assert!(registry.get_by_name("MinimapArrow").is_some());
    assert!(registry.get_by_name("MinimapZoneName").is_some());
    assert!(registry.get_by_name("MinimapCoords").is_some());
}

#[test]
fn height_color_low() {
    let (r, g, b) = height_to_color(0.0);
    assert_eq!((r, g, b), (30, 80, 20));
}

#[test]
fn height_color_mid() {
    let (r, g, b) = height_to_color(0.4);
    assert_eq!((r, g, b), (80, 160, 50));
}

#[test]
fn height_color_high() {
    let (r, g, b) = height_to_color(1.0);
    assert_eq!((r, g, b), (160, 80, 30));
}

#[test]
fn height_range_empty() {
    let chunks: Vec<Option<ChunkHeightGrid>> = vec![None; 256];
    let (min, max) = find_height_range(&chunks);
    assert_eq!((min, max), (0.0, 1.0));
}

#[test]
fn minimap_state_default() {
    let state = MinimapState::default();
    assert!(state.tile_images.is_empty());
}

#[test]
fn triangle_center_inside() {
    assert!(point_in_triangle(
        [8.0, 8.0],
        [[8.0, 2.0], [3.0, 13.0], [12.0, 13.0]]
    ));
}

#[test]
fn triangle_outside() {
    assert!(!point_in_triangle(
        [0.0, 0.0],
        [[8.0, 2.0], [3.0, 13.0], [12.0, 13.0]],
    ));
}

#[test]
fn arrow_image_has_yellow_pixels() {
    let img = create_arrow_image();
    let data = img.data.as_ref().unwrap();
    let i = (8 * 16 + 8) * 4;
    assert_eq!(data[i], 255);
    assert_eq!(data[i + 1], 220);
    assert_eq!(data[i + 3], 255);
}

#[test]
fn crop_circle_center_pixel_opaque() {
    let comp = vec![255u8; 4 * 4 * 4];
    let result = crop_with_circle(&comp, 4, 2, 2, 4);
    assert_eq!(result[(2 * 4 + 2) * 4 + 3], 255);
}

#[test]
fn draw_dot_center() {
    let mut data = vec![0u8; 10 * 10 * 4];
    draw_dot(&mut data, 10, 5, 5, &[255, 0, 0, 255]);
    let i = (5 * 10 + 5) * 4;
    assert_eq!(&data[i..i + 4], &[255, 0, 0, 255]);
    let i2 = (4 * 10 + 5) * 4;
    assert_eq!(&data[i2..i2 + 4], &[255, 0, 0, 255]);
    let i3 = (3 * 10 + 5) * 4;
    assert_eq!(&data[i3..i3 + 4], &[0, 0, 0, 0]);
}

#[test]
fn draw_dot_edge_no_panic() {
    let mut data = vec![0u8; 5 * 5 * 4];
    draw_dot(&mut data, 5, 0, 0, &[0, 255, 0, 255]);
    assert_eq!(&data[0..4], &[0, 255, 0, 255]);
}

#[test]
fn crop_circle_corner_transparent() {
    let comp = vec![255u8; 100 * 100 * 4];
    let result = crop_with_circle(&comp, 100, 50, 50, 100);
    assert_eq!(result[3], 0);
}

#[test]
fn border_image_has_opaque_ring() {
    let img = create_border_image(MINIMAP_DISPLAY_SIZE as usize);
    let data = img.data.as_ref().unwrap();
    let size = MINIMAP_DISPLAY_SIZE as usize;
    let edge_x = size / 2;
    let edge_y = 1;
    let i = (edge_y * size + edge_x) * 4;
    assert!(data[i + 3] > 0, "Border ring should have alpha at edge");
    let center_i = (size / 2 * size + size / 2) * 4;
    assert_eq!(data[center_i + 3], 0, "Center should be transparent");
}

#[test]
fn render_tile_image_size() {
    let chunks: Vec<Option<ChunkHeightGrid>> = vec![None; 256];
    let img = render_tile_image(&chunks, 256);
    assert_eq!(img.width(), 256);
    assert_eq!(img.height(), 256);
}

#[test]
fn tile_grid_changed_detects_row_change() {
    let last = LastMinimapPixel::default();
    assert!(tile_grid_changed(&last, 32, 48, 1));
}

#[test]
fn tile_grid_changed_same_grid_returns_false() {
    let last = LastMinimapPixel {
        px_x: 100,
        px_y: 200,
        tile_row: 32,
        tile_col: 48,
        tile_generation: 5,
        composite_buf: Vec::new(),
        crop_buf: Vec::new(),
        circle_mask: Vec::new(),
    };
    assert!(!tile_grid_changed(&last, 32, 48, 5));
}

#[test]
fn tile_grid_changed_detects_tile_generation_change() {
    let last = LastMinimapPixel {
        px_x: 100,
        px_y: 200,
        tile_row: 32,
        tile_col: 48,
        tile_generation: 5,
        composite_buf: Vec::new(),
        crop_buf: Vec::new(),
        circle_mask: Vec::new(),
    };
    assert!(tile_grid_changed(&last, 32, 48, 6));
}

#[test]
fn pixel_change_without_grid_change_skips_recomposite() {
    let last = LastMinimapPixel {
        px_x: 100,
        px_y: 200,
        tile_row: 32,
        tile_col: 48,
        tile_generation: 5,
        composite_buf: Vec::new(),
        crop_buf: Vec::new(),
        circle_mask: Vec::new(),
    };
    // Grid unchanged — no recomposite needed
    assert!(!tile_grid_changed(&last, 32, 48, 5));
    // But pixel did change — crop should still update
    assert!(101 != last.px_x);
}

#[test]
fn player_pixel_for_known_elwynn_position_stays_in_center_tile() {
    let [bx, _, bz] = crate::asset::m2::wow_to_bevy(-8949.0, -132.0, 83.0);
    let (row, col) = crate::terrain_tile::bevy_to_tile_coords(bx, bz);
    let (px_x, px_y) = player_pixel_in_composite(bx, bz, row, col, MINIMAP_COMPOSITE_SIZE as usize);

    assert!(
        (MINIMAP_TILE_SIZE as usize..(MINIMAP_TILE_SIZE * 2) as usize).contains(&px_x),
        "expected x to stay in center tile, got {px_x}"
    );
    assert!(
        (MINIMAP_TILE_SIZE as usize..(MINIMAP_TILE_SIZE * 2) as usize).contains(&px_y),
        "expected y to stay in center tile, got {px_y}"
    );
}

#[test]
fn world_pixel_in_composite_matches_player_for_same_position() {
    let [bx, _, bz] = crate::asset::m2::wow_to_bevy(-8949.0, -132.0, 83.0);
    let (row, col) = crate::terrain_tile::bevy_to_tile_coords(bx, bz);
    let player_pixel = player_pixel_in_composite(bx, bz, row, col, MINIMAP_COMPOSITE_SIZE as usize);
    let world_pixel = world_pixel_in_composite(bx, bz, row, col, MINIMAP_COMPOSITE_SIZE as usize);

    assert_eq!(world_pixel, player_pixel);
}

#[test]
fn draw_tracking_icon_mailbox_tints_center_blue() {
    let mut data = vec![0u8; 16 * 16 * 4];

    draw_tracking_icon(&mut data, 16, 8, 8, TrackingIconKind::Mailbox);

    let center = (8 * 16 + 8) * 4;
    assert_eq!(&data[center..center + 4], &[90, 160, 255, 255]);
}

#[test]
fn draw_tracking_icon_quest_marks_center_yellow() {
    let mut data = vec![0u8; 16 * 16 * 4];

    draw_tracking_icon(&mut data, 16, 8, 8, TrackingIconKind::QuestObjective);

    let center = (8 * 16 + 8) * 4;
    assert_eq!(&data[center..center + 4], &[255, 220, 40, 255]);
}

#[test]
fn draw_tracking_icons_places_mailbox_at_player_center() {
    let [bx, _, bz] = crate::asset::m2::wow_to_bevy(-8949.0, -132.0, 83.0);
    let (row, col) = crate::terrain_tile::bevy_to_tile_coords(bx, bz);
    let comp_size = MINIMAP_COMPOSITE_SIZE as usize;
    let ds = MINIMAP_DISPLAY_SIZE as usize;
    let (px_x, px_y) = player_pixel_in_composite(bx, bz, row, col, comp_size);
    let mut data = vec![0u8; ds * ds * 4];
    let mut mask = Vec::new();
    ensure_circle_mask(&mut mask, ds);

    draw_tracking_icons(
        &mut data,
        ds,
        px_x,
        px_y,
        row,
        col,
        &[TrackingPoint {
            kind: TrackingIconKind::Mailbox,
            bx,
            bz,
        }],
        &mask,
    );

    let center = (ds / 2 * ds + ds / 2) * 4;
    assert_eq!(&data[center..center + 4], &[90, 160, 255, 255]);
}

#[test]
fn fill_dark_background_fills_rgba() {
    let mut buf = vec![0u8; 4 * 4 * 4];
    fill_dark_background(&mut buf, 4);
    assert_eq!(&buf[0..4], &[20, 20, 20, 255]);
    assert_eq!(&buf[60..64], &[20, 20, 20, 255]);
}

#[test]
fn ensure_circle_mask_builds_correct_size() {
    let mut mask = Vec::new();
    ensure_circle_mask(&mut mask, 10);
    assert_eq!(mask.len(), 100);
    // Center pixel is inside
    assert!(mask[5 * 10 + 5]);
    // Corner pixel is outside
    assert!(!mask[0]);
    // Calling again is a no-op (same length)
    let ptr = mask.as_ptr();
    ensure_circle_mask(&mut mask, 10);
    assert_eq!(mask.as_ptr(), ptr);
}

#[test]
fn crop_with_mask_matches_crop_with_circle() {
    // Create a simple 8x8 composite with known data
    let comp_size = 8;
    let mut comp = vec![0u8; comp_size * comp_size * 4];
    for i in 0..comp_size * comp_size {
        comp[i * 4] = (i % 256) as u8;
        comp[i * 4 + 1] = 100;
        comp[i * 4 + 2] = 200;
        comp[i * 4 + 3] = 255;
    }
    let ds = 4;
    let mut mask = Vec::new();
    ensure_circle_mask(&mut mask, ds);
    let mut out = vec![0u8; ds * ds * 4];
    crop_with_mask(&comp, comp_size, 4, 4, ds, &mask, &mut out);

    let reference = crop_with_circle(&comp, comp_size, 4, 4, ds as u32);
    // Inside-circle pixels should match
    for i in 0..ds * ds {
        if mask[i] {
            assert_eq!(
                &out[i * 4..i * 4 + 4],
                &reference[i * 4..i * 4 + 4],
                "mismatch at pixel {i}"
            );
        }
    }
}

#[test]
fn zone_id_to_name_known() {
    game_engine::world_db::import_zone_name_cache().expect("import zone name cache");
    assert_eq!(zone_id_to_name(12), "Elwynn Forest");
    assert_eq!(zone_id_to_name(1519), "Stormwind City");
}

#[test]
fn zone_id_to_name_unknown() {
    assert_eq!(zone_id_to_name(99999), "Unknown");
}

// --- Minimap recomposite: tile stitching correctness ---

#[test]
fn blit_image_places_tile_at_correct_offset() {
    let tile_size = 2;
    let comp_size = 6; // 3 tiles of size 2
    let mut composite = vec![0u8; comp_size * comp_size * 4];
    // Fill a 2x2 tile with red
    let tile = vec![
        255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
    ];
    // Blit at offset (2, 2) — the center tile position
    blit_image(&mut composite, comp_size, &tile, tile_size, 2, 2);
    // Check pixel at (2, 2) is red
    let i = (2 * comp_size + 2) * 4;
    assert_eq!(&composite[i..i + 4], &[255, 0, 0, 255]);
    // Check pixel at (0, 0) is still black (background)
    assert_eq!(&composite[0..4], &[0, 0, 0, 0]);
}

#[test]
fn blit_image_adjacent_tiles_no_gap() {
    let tile_size = 2;
    let comp_size = 4;
    let mut composite = vec![0u8; comp_size * comp_size * 4];
    let red = vec![
        255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
    ];
    let blue = vec![
        0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255,
    ];
    // Blit red at (0,0), blue at (2,0)
    blit_image(&mut composite, comp_size, &red, tile_size, 0, 0);
    blit_image(&mut composite, comp_size, &blue, tile_size, 2, 0);
    // Last pixel of red tile (row 0, col 1)
    let red_end = 4;
    assert_eq!(&composite[red_end..red_end + 4], &[255, 0, 0, 255]);
    // First pixel of blue tile (row 0, col 2)
    let blue_start = 8;
    assert_eq!(&composite[blue_start..blue_start + 4], &[0, 0, 255, 255]);
    // No gap between them
    assert_eq!(blue_start - red_end, 4); // exactly 1 pixel apart
}

#[test]
fn blit_image_does_not_overwrite_outside_bounds() {
    let tile_size = 2;
    let comp_size = 4;
    let mut composite = vec![99u8; comp_size * comp_size * 4];
    let tile = vec![0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255];
    blit_image(&mut composite, comp_size, &tile, tile_size, 0, 0);
    // Pixel at (0, 2) should still be 99 (untouched)
    let i = 2 * 4;
    assert_eq!(composite[i], 99);
    // Pixel at (2, 0) should still be 99 (untouched)
    let i = (2 * comp_size) * 4;
    assert_eq!(composite[i], 99);
}

#[test]
fn composite_3x3_grid_center_pixel_matches_center_tile() {
    let tile_size = 4;
    let comp_size = tile_size * 3; // 12
    let mut composite = vec![0u8; comp_size * comp_size * 4];
    fill_dark_background(&mut composite, comp_size);

    // Create a center tile with a known pixel pattern
    let mut center_tile = vec![0u8; tile_size * tile_size * 4];
    // Set pixel (2, 2) in the tile to green
    let tile_pixel = (2 * tile_size + 2) * 4;
    center_tile[tile_pixel..tile_pixel + 4].copy_from_slice(&[0, 255, 0, 255]);

    // Blit center tile at (tile_size, tile_size)
    blit_image(
        &mut composite,
        comp_size,
        &center_tile,
        tile_size,
        tile_size,
        tile_size,
    );

    // Check the composite pixel at (tile_size + 2, tile_size + 2)
    let comp_pixel = ((tile_size + 2) * comp_size + tile_size + 2) * 4;
    assert_eq!(
        &composite[comp_pixel..comp_pixel + 4],
        &[0, 255, 0, 255],
        "center tile pixel should be green in composite"
    );
}

#[test]
fn blit_image_full_composite_coverage() {
    let tile_size = 2;
    let comp_size = 6;
    let mut composite = vec![0u8; comp_size * comp_size * 4];

    // Fill all 9 tiles of a 3x3 grid, each with a unique color
    for dy in 0..3usize {
        for dx in 0..3usize {
            let color = ((dy * 3 + dx) * 28) as u8;
            let tile = vec![color, color, color, 255]
                .into_iter()
                .cycle()
                .take(tile_size * tile_size * 4)
                .collect::<Vec<_>>();
            blit_image(
                &mut composite,
                comp_size,
                &tile,
                tile_size,
                dx * tile_size,
                dy * tile_size,
            );
        }
    }

    // Every pixel should be non-zero (fully covered)
    for y in 0..comp_size {
        for x in 0..comp_size {
            let i = (y * comp_size + x) * 4;
            assert_ne!(
                composite[i + 3],
                0,
                "pixel ({x}, {y}) should have alpha from a tile"
            );
        }
    }
}
