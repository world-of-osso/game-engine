use std::collections::{HashMap, HashSet};
use std::path::Path;

use bevy::prelude::*;
use ui_toolkit::frame::WidgetData;
use ui_toolkit::plugin::{UiState, sync_registry_to_primary_window};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::{TextureData, TextureSource};

use crate::game_state::GameState;
use crate::minimap_render::{
    blit_image, create_arrow_image, create_blank_image, create_border_image, crop_with_circle,
    render_tile_image,
};
use crate::terrain_heightmap::TerrainHeightmap;
use game_engine::ui::screens::inworld_hud_component;

const MINIMAP_TILE_SIZE: u32 = 256;
const MINIMAP_DISPLAY_SIZE: u32 = 200;
const MINIMAP_COMPOSITE_SIZE: u32 = 768; // 3 tiles x 256 pixels

/// Stores generated minimap tile images.
#[derive(Resource, Default)]
pub struct MinimapState {
    /// Generated tile images: (tile_y, tile_x) -> image handle.
    pub tile_images: HashMap<(u32, u32), Handle<Image>>,
    /// Track which tiles we have already generated images for.
    generated: HashSet<(u32, u32)>,
}

/// Tracks last minimap pixel position to skip recomposite when unchanged.
#[derive(Resource)]
struct LastMinimapPixel {
    px_x: usize,
    px_y: usize,
    tile_row: u32,
    tile_col: u32,
    tile_generation: usize,
    composite_buf: Vec<u8>,
}

impl Default for LastMinimapPixel {
    fn default() -> Self {
        Self {
            px_x: usize::MAX,
            px_y: usize::MAX,
            tile_row: u32::MAX,
            tile_col: u32::MAX,
            tile_generation: 0,
            composite_buf: Vec::new(),
        }
    }
}

/// Holds the composite image handle displayed on screen.
#[derive(Resource)]
pub struct MinimapComposite {
    pub handle: Handle<Image>,
}

/// Frame IDs for the minimap UI toolkit frames.
#[derive(Resource)]
struct MinimapFrames {
    cluster: u64,
    header: u64,
    display: u64,
    border: u64,
    arrow: u64,
    zone_name: u64,
    coords: u64,
}

pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MinimapState>()
            .init_resource::<LastMinimapPixel>();
        register_minimap_systems(app);
    }
}

fn register_minimap_systems(app: &mut App) {
    let in_world = in_state(GameState::InWorld);
    app.add_systems(Startup, create_minimap_frames)
        .add_systems(OnEnter(GameState::InWorld), show_minimap_hud)
        .add_systems(OnExit(GameState::InWorld), hide_minimap_hud)
        .add_systems(Update, generate_tile_textures.run_if(in_world.clone()))
        .add_systems(
            Update,
            update_minimap_composite
                .after(generate_tile_textures)
                .run_if(in_world.clone()),
        )
        .add_systems(
            Update,
            update_coord_text.run_if(in_world.clone()),
        )
        .add_systems(Update, update_zone_name.run_if(in_world.clone()))
        .add_systems(Update, rotate_minimap.run_if(in_world));
}

fn resolve_frame_id(registry: &FrameRegistry, name: &str) -> u64 {
    registry
        .get_by_name(name)
        .unwrap_or_else(|| panic!("missing frame {name}"))
}

fn set_initial_visibility(registry: &mut FrameRegistry, id: u64) {
    if let Some(frame) = registry.get_mut(id) {
        frame.hidden = true;
        frame.visible = false;
        frame.effective_alpha = 0.0;
    }
}

/// Create minimap frames in the UI toolkit registry.
fn create_minimap_frames(
    mut commands: Commands,
    mut ui: ResMut<UiState>,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let composite_handle = images.add(create_blank_image(
        MINIMAP_DISPLAY_SIZE,
        MINIMAP_DISPLAY_SIZE,
    ));
    let border_handle = images.add(create_border_image(MINIMAP_DISPLAY_SIZE as usize));
    let arrow_handle = images.add(create_arrow_image());
    let shared = SharedContext::new();
    let mut screen = Screen::new(inworld_hud_component::minimap_screen);
    screen.sync(&shared, &mut ui.registry);

    let cluster = resolve_frame_id(&ui.registry, "MinimapCluster");
    let header = resolve_frame_id(&ui.registry, "MinimapHeader");
    let display = resolve_frame_id(&ui.registry, "MinimapDisplay");
    let border = resolve_frame_id(&ui.registry, "MinimapBorder");
    let arrow = resolve_frame_id(&ui.registry, "MinimapArrow");
    let zone_name = resolve_frame_id(&ui.registry, "MinimapZoneName");
    let coords = resolve_frame_id(&ui.registry, "MinimapCoords");

    if let Some(frame) = ui.registry.get_mut(display) {
        frame.widget_data = Some(WidgetData::Texture(TextureData {
            source: TextureSource::Dynamic(composite_handle.clone()),
            ..TextureData::default()
        }));
    }
    if let Some(frame) = ui.registry.get_mut(border) {
        frame.widget_data = Some(WidgetData::Texture(TextureData {
            source: TextureSource::Dynamic(border_handle),
            ..TextureData::default()
        }));
    }
    if let Some(frame) = ui.registry.get_mut(arrow) {
        frame.widget_data = Some(WidgetData::Texture(TextureData {
            source: TextureSource::Dynamic(arrow_handle),
            ..TextureData::default()
        }));
    }
    for id in [cluster, header, display, border, arrow, zone_name, coords] {
        set_initial_visibility(&mut ui.registry, id);
    }

    commands.insert_resource(MinimapComposite {
        handle: composite_handle,
    });
    commands.insert_resource(MinimapFrames {
        cluster,
        header,
        display,
        border,
        arrow,
        zone_name,
        coords,
    });
}

fn set_hud_visibility(ui: &mut UiState, frames: &MinimapFrames, visible: bool) {
    for &fid in &[
        frames.cluster,
        frames.header,
        frames.display,
        frames.border,
        frames.arrow,
        frames.zone_name,
        frames.coords,
    ] {
        if let Some(frame) = ui.registry.get_mut(fid) {
            frame.hidden = !visible;
            frame.visible = visible;
            frame.effective_alpha = if visible { frame.alpha } else { 0.0 };
        }
    }
}

fn show_minimap_hud(mut ui: ResMut<UiState>, frames: Option<Res<MinimapFrames>>) {
    if let Some(frames) = frames {
        set_hud_visibility(&mut ui, &frames, true);
    }
}

fn hide_minimap_hud(mut ui: ResMut<UiState>, frames: Option<Res<MinimapFrames>>) {
    if let Some(frames) = frames {
        set_hud_visibility(&mut ui, &frames, false);
    }
}

/// Rotate minimap image by camera yaw (WoW-style rotating minimap).
fn rotate_minimap(
    camera_q: Query<&crate::camera::WowCamera>,
    mut ui: ResMut<UiState>,
    frames: Option<Res<MinimapFrames>>,
) {
    let Ok(cam) = camera_q.single() else { return };
    let Some(frames) = frames else { return };
    if let Some(frame) = ui.registry.get_mut(frames.display) {
        if let Some(WidgetData::Texture(tex)) = &mut frame.widget_data {
            tex.rotation = -cam.yaw;
        }
    }
}

/// Generate minimap tile textures for newly loaded terrain tiles.
fn generate_tile_textures(
    heightmap: Res<TerrainHeightmap>,
    mut minimap: ResMut<MinimapState>,
    mut images: ResMut<Assets<Image>>,
) {
    for &(ty, tx) in heightmap.tile_keys() {
        if minimap.generated.contains(&(ty, tx)) {
            continue;
        }
        let image = try_load_minimap_blp(tx, ty).or_else(|| {
            heightmap
                .tile_chunks(ty, tx)
                .map(|chunks| render_tile_image(chunks, MINIMAP_TILE_SIZE as usize))
        });
        if let Some(image) = image {
            let handle = images.add(image);
            minimap.tile_images.insert((ty, tx), handle);
            minimap.generated.insert((ty, tx));
        }
    }
    let loaded: HashSet<(u32, u32)> = heightmap.tile_keys().copied().collect();
    minimap.tile_images.retain(|k, _| loaded.contains(k));
    minimap.generated.retain(|k| loaded.contains(k));
}

/// Try loading a pre-rendered BLP minimap tile from `data/minimap/map{x}_{y}.blp`.
fn try_load_minimap_blp(tile_x: u32, tile_y: u32) -> Option<Image> {
    let path = format!("data/minimap/map{tile_x}_{tile_y}.blp");
    let (pixels, w, h) = crate::asset::blp::load_blp_rgba(Path::new(&path)).ok()?;
    Some(crate::rgba_image(pixels, w, h))
}

/// Composite tile images centered on the player, crop and apply circular mask.
fn update_minimap_composite(
    player_q: Query<&Transform, With<crate::camera::Player>>,
    minimap: Res<MinimapState>,
    composite_res: Option<Res<MinimapComposite>>,
    mut images: ResMut<Assets<Image>>,
    mut last: ResMut<LastMinimapPixel>,
) {
    let Ok(player_tf) = player_q.single() else {
        return;
    };
    let Some(composite_res) = composite_res else {
        return;
    };
    let bx = player_tf.translation.x;
    let bz = player_tf.translation.z;
    let (player_row, player_col) = crate::terrain_tile::bevy_to_tile_coords(bx, bz);
    let comp_size = MINIMAP_COMPOSITE_SIZE as usize;
    let (px_x, px_y) = player_pixel_in_composite(bx, bz, player_row, player_col, comp_size);

    if !composite_needs_update(
        &last,
        px_x,
        px_y,
        player_row,
        player_col,
        minimap.generated.len(),
    ) {
        return;
    }

    recomposite(
        &minimap, &images, &mut last, player_row, player_col, comp_size,
    );
    last.px_x = px_x;
    last.px_y = px_y;
    last.tile_row = player_row;
    last.tile_col = player_col;
    last.tile_generation = minimap.generated.len();

    apply_circular_crop(
        &composite_res,
        &mut images,
        &last.composite_buf,
        comp_size,
        px_x,
        px_y,
    );
}

fn composite_needs_update(
    last: &LastMinimapPixel,
    px_x: usize,
    px_y: usize,
    row: u32,
    col: u32,
    tile_gen: usize,
) -> bool {
    px_x != last.px_x
        || px_y != last.px_y
        || row != last.tile_row
        || col != last.tile_col
        || tile_gen != last.tile_generation
}

fn apply_circular_crop(
    composite_res: &MinimapComposite,
    images: &mut Assets<Image>,
    buf: &[u8],
    comp_size: usize,
    px_x: usize,
    px_y: usize,
) {
    if let Some(img) = images.get_mut(&composite_res.handle) {
        img.data = Some(crop_with_circle(
            buf,
            comp_size,
            px_x,
            px_y,
            MINIMAP_DISPLAY_SIZE,
        ));
    }
}

fn recomposite(
    minimap: &MinimapState,
    images: &Assets<Image>,
    last: &mut LastMinimapPixel,
    player_row: u32,
    player_col: u32,
    comp_size: usize,
) {
    let tile_px = MINIMAP_TILE_SIZE as usize;
    let needed = comp_size * comp_size * 4;
    last.composite_buf.resize(needed, 0);
    fill_dark_background(&mut last.composite_buf, comp_size);
    blit_tiles(
        &mut last.composite_buf,
        comp_size,
        tile_px,
        player_row,
        player_col,
        minimap,
        images,
    );
}

fn fill_dark_background(buf: &mut [u8], comp_size: usize) {
    for i in 0..(comp_size * comp_size) {
        let off = i * 4;
        buf[off] = 20;
        buf[off + 1] = 20;
        buf[off + 2] = 20;
        buf[off + 3] = 255;
    }
}

/// Blit all 3x3 tile images into the composite buffer.
fn blit_tiles(
    composite: &mut [u8],
    comp_size: usize,
    tile_px: usize,
    center_row: u32,
    center_col: u32,
    minimap: &MinimapState,
    images: &Assets<Image>,
) {
    for dy in 0..3i32 {
        for dx in 0..3i32 {
            let row = center_row as i32 + dy - 1;
            let col = center_col as i32 + dx - 1;
            if row < 0 || col < 0 {
                continue;
            }
            blit_single_tile(
                composite,
                comp_size,
                tile_px,
                (row as u32, col as u32),
                dx as usize * tile_px,
                dy as usize * tile_px,
                minimap,
                images,
            );
        }
    }
}

fn blit_single_tile(
    composite: &mut [u8],
    comp_size: usize,
    tile_px: usize,
    key: (u32, u32),
    off_x: usize,
    off_y: usize,
    minimap: &MinimapState,
    images: &Assets<Image>,
) {
    let Some(handle) = minimap.tile_images.get(&key) else {
        return;
    };
    let Some(tile_img) = images.get(handle) else {
        return;
    };
    let Some(tile_data) = tile_img.data.as_ref() else {
        return;
    };
    blit_image(composite, comp_size, tile_data, tile_px, off_x, off_y);
}

/// Compute the player's pixel position within the 3x3 composite image.
fn player_pixel_in_composite(
    bx: f32,
    bz: f32,
    row: u32,
    col: u32,
    comp_size: usize,
) -> (usize, usize) {
    let tile_size = crate::asset::adt::CHUNK_SIZE * 16.0;
    let center = 32.0 * tile_size;
    let frow = (center + bz) / tile_size;
    let fcol = (center - bx) / tile_size;
    let frac_y = frow - row as f32;
    let frac_x = fcol - col as f32;
    let tile_px = MINIMAP_TILE_SIZE as f32;
    let px_x = (tile_px + frac_x * tile_px) as usize;
    let px_y = (tile_px + frac_y * tile_px) as usize;
    (px_x.min(comp_size - 1), px_y.min(comp_size - 1))
}

/// Update zone name text when the current zone changes.
fn update_zone_name(
    zone: Option<Res<crate::networking::CurrentZone>>,
    mut ui: ResMut<UiState>,
    frames: Option<Res<MinimapFrames>>,
) {
    let Some(zone) = zone else { return };
    if !zone.is_changed() {
        return;
    }
    let Some(frames) = frames else { return };
    let name = zone_id_to_name(zone.zone_id);
    if let Some(frame) = ui.registry.get_mut(frames.zone_name) {
        if let Some(WidgetData::FontString(fs)) = &mut frame.widget_data {
            fs.text = name.to_string();
        }
    }
}

/// Update coordinate text with the player's current position.
fn update_coord_text(
    player_q: Query<&Transform, With<crate::camera::Player>>,
    mut ui: ResMut<UiState>,
    frames: Option<Res<MinimapFrames>>,
) {
    let Ok(tf) = player_q.single() else { return };
    let Some(frames) = frames else { return };
    if let Some(frame) = ui.registry.get_mut(frames.coords) {
        if let Some(WidgetData::FontString(fs)) = &mut frame.widget_data {
            fs.text = format!("{:.0}, {:.0}", tf.translation.x, tf.translation.z);
        }
    }
}

/// Map a WoW zone ID to its display name.
fn zone_id_to_name(id: u32) -> &'static str {
    match id {
        10 => "Duskwood",
        12 => "Elwynn Forest",
        14 => "Durotar",
        17 => "The Barrens",
        38 => "Loch Modan",
        40 => "Westfall",
        44 => "Redridge Mountains",
        85 => "Tirisfal Glades",
        215 => "Mulgore",
        331 => "Ashenvale",
        1497 => "Undercity",
        1519 => "Stormwind City",
        1537 => "Ironforge",
        1637 => "Orgrimmar",
        1638 => "Thunder Bluff",
        1657 => "Darnassus",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::adt::ChunkHeightGrid;
    use crate::minimap_render::{
        create_arrow_image, create_border_image, crop_with_circle, draw_dot, find_height_range,
        height_to_color, point_in_triangle, render_tile_image,
    };
    use ui_toolkit::screen::{Screen, SharedContext};

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
        assert!(point_in_triangle(8.0, 8.0, 8.0, 2.0, 3.0, 13.0, 12.0, 13.0));
    }

    #[test]
    fn triangle_outside() {
        assert!(!point_in_triangle(
            0.0, 0.0, 8.0, 2.0, 3.0, 13.0, 12.0, 13.0
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

    // --- Composite update logic tests ---

    #[test]
    fn composite_needs_update_detects_pixel_change() {
        let last = LastMinimapPixel::default();
        assert!(composite_needs_update(&last, 100, 100, 32, 48, 1));
    }

    #[test]
    fn composite_needs_update_same_state_returns_false() {
        let last = LastMinimapPixel {
            px_x: 100,
            px_y: 200,
            tile_row: 32,
            tile_col: 48,
            tile_generation: 5,
            composite_buf: Vec::new(),
        };
        assert!(!composite_needs_update(&last, 100, 200, 32, 48, 5));
    }

    #[test]
    fn composite_needs_update_tile_generation_change() {
        let last = LastMinimapPixel {
            px_x: 100,
            px_y: 200,
            tile_row: 32,
            tile_col: 48,
            tile_generation: 5,
            composite_buf: Vec::new(),
        };
        assert!(composite_needs_update(&last, 100, 200, 32, 48, 6));
    }

    #[test]
    fn player_pixel_for_known_elwynn_position_stays_in_center_tile() {
        let [bx, _, bz] = crate::asset::m2::wow_to_bevy(-8949.0, -132.0, 83.0);
        let (row, col) = crate::terrain_tile::bevy_to_tile_coords(bx, bz);
        let (px_x, px_y) =
            player_pixel_in_composite(bx, bz, row, col, MINIMAP_COMPOSITE_SIZE as usize);

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
    fn fill_dark_background_fills_rgba() {
        let mut buf = vec![0u8; 4 * 4 * 4]; // 4x4
        fill_dark_background(&mut buf, 4);
        // Check first pixel
        assert_eq!(&buf[0..4], &[20, 20, 20, 255]);
        // Check last pixel
        assert_eq!(&buf[60..64], &[20, 20, 20, 255]);
    }

    #[test]
    fn zone_id_to_name_known() {
        assert_eq!(zone_id_to_name(12), "Elwynn Forest");
        assert_eq!(zone_id_to_name(1519), "Stormwind City");
    }

    #[test]
    fn zone_id_to_name_unknown() {
        assert_eq!(zone_id_to_name(99999), "Unknown");
    }
}
