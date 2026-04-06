use std::collections::{HashMap, HashSet};
use std::path::Path;

use bevy::prelude::*;
use ui_toolkit::frame::WidgetData;
use ui_toolkit::plugin::{UiState, sync_registry_to_primary_window};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::{TextureData, TextureSource};

use crate::client_options::HudVisibilityToggles;
use crate::game_state::GameState;
use crate::minimap_render::{
    blit_image, create_arrow_image, create_blank_image, create_border_image, render_tile_image,
};
use crate::terrain_heightmap::TerrainHeightmap;
use crate::zone_names::zone_id_to_name;
use game_engine::ui::screens::inworld_hud_component;

const MINIMAP_TILE_SIZE: u32 = 256;
const MINIMAP_DISPLAY_SIZE: u32 = 200;
const MINIMAP_COMPOSITE_SIZE: u32 = 768; // 3 tiles x 256 pixels
const MINIMAP_BG_COLOR: [u8; 4] = [20, 20, 20, 255];

/// Stores generated minimap tile images.
#[derive(Resource, Default)]
pub struct MinimapState {
    /// Generated tile images: (tile_y, tile_x) -> image handle.
    pub tile_images: HashMap<(u32, u32), Handle<Image>>,
    /// Track which tiles we have already generated images for.
    generated: HashSet<(u32, u32)>,
}

/// Tracks last minimap state to skip unnecessary work.
///
/// The 768×768 composite buffer only changes when the tile grid or loaded
/// tiles change.  When just the player pixel moves within the same grid,
/// we skip the expensive full recomposite and only redo the cheap 200×200
/// circular crop.
#[derive(Resource)]
struct LastMinimapPixel {
    px_x: usize,
    px_y: usize,
    tile_row: u32,
    tile_col: u32,
    tile_generation: usize,
    composite_buf: Vec<u8>,
    /// Reusable output buffer for the circular crop (avoids per-frame allocation).
    crop_buf: Vec<u8>,
    /// Precomputed circular mask: true = inside circle. Computed once on first use.
    circle_mask: Vec<bool>,
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
            crop_buf: Vec::new(),
            circle_mask: Vec::new(),
        }
    }
}

/// Holds the composite image handle displayed on screen.
#[derive(Resource, Clone)]
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

struct MinimapCompositeState {
    composite: MinimapComposite,
    px_x: usize,
    px_y: usize,
    player_row: u32,
    player_col: u32,
    comp_size: usize,
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
        .add_systems(Update, sync_minimap_visibility)
        .add_systems(Update, generate_tile_textures.run_if(in_world.clone()))
        .add_systems(
            Update,
            update_minimap_composite
                .after(generate_tile_textures)
                .run_if(in_world.clone()),
        )
        .add_systems(Update, update_coord_text.run_if(in_world.clone()))
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
    let frames = build_minimap_screen(&mut ui.registry);
    assign_minimap_textures(
        &mut ui.registry,
        &frames,
        composite_handle.clone(),
        border_handle,
        arrow_handle,
    );
    for id in [
        frames.cluster,
        frames.header,
        frames.display,
        frames.border,
        frames.arrow,
        frames.zone_name,
        frames.coords,
    ] {
        set_initial_visibility(&mut ui.registry, id);
    }

    commands.insert_resource(MinimapComposite {
        handle: composite_handle,
    });
    commands.insert_resource(frames);
}

fn build_minimap_screen(registry: &mut FrameRegistry) -> MinimapFrames {
    let shared = SharedContext::new();
    let mut screen = Screen::new(inworld_hud_component::minimap_screen);
    screen.sync(&shared, registry);
    MinimapFrames {
        cluster: resolve_frame_id(registry, "MinimapCluster"),
        header: resolve_frame_id(registry, "MinimapHeader"),
        display: resolve_frame_id(registry, "MinimapDisplay"),
        border: resolve_frame_id(registry, "MinimapBorder"),
        arrow: resolve_frame_id(registry, "MinimapArrow"),
        zone_name: resolve_frame_id(registry, "MinimapZoneName"),
        coords: resolve_frame_id(registry, "MinimapCoords"),
    }
}

fn assign_minimap_textures(
    registry: &mut FrameRegistry,
    frames: &MinimapFrames,
    composite_handle: Handle<Image>,
    border_handle: Handle<Image>,
    arrow_handle: Handle<Image>,
) {
    set_minimap_texture(registry, frames.display, composite_handle);
    set_minimap_texture(registry, frames.border, border_handle);
    set_minimap_texture(registry, frames.arrow, arrow_handle);
}

fn set_minimap_texture(registry: &mut FrameRegistry, frame_id: u64, handle: Handle<Image>) {
    if let Some(frame) = registry.get_mut(frame_id) {
        frame.widget_data = Some(WidgetData::Texture(TextureData {
            source: TextureSource::Dynamic(handle),
            ..TextureData::default()
        }));
    }
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

fn sync_minimap_visibility(
    mut ui: ResMut<UiState>,
    frames: Option<Res<MinimapFrames>>,
    game_state: Res<State<GameState>>,
    hud_visibility: Option<Res<HudVisibilityToggles>>,
) {
    let Some(frames) = frames else { return };
    let visible = *game_state.get() == GameState::InWorld
        && hud_visibility.is_none_or(|toggles| toggles.show_minimap);
    set_hud_visibility(&mut ui, &frames, visible);
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
///
/// The 768×768 composite is only rebuilt when the tile grid or loaded tile set
/// changes.  When just the player pixel shifts (same grid), we skip the heavy
/// blit and only redo the cheap circular crop.
fn update_minimap_composite(
    player_q: Query<&Transform, With<crate::camera::Player>>,
    minimap: Res<MinimapState>,
    composite_res: Option<Res<MinimapComposite>>,
    mut images: ResMut<Assets<Image>>,
    mut last: ResMut<LastMinimapPixel>,
) {
    let Some(state) = current_minimap_composite_state(&player_q, composite_res) else {
        return;
    };

    let tile_gen = minimap.generated.len();
    let grid_changed = tile_grid_changed(&last, state.player_row, state.player_col, tile_gen);
    let pixel_changed = state.px_x != last.px_x || state.px_y != last.px_y;

    if !grid_changed && !pixel_changed {
        return;
    }

    if grid_changed {
        recomposite(
            &minimap,
            &images,
            &mut last,
            state.player_row,
            state.player_col,
            state.comp_size,
        );
    }
    update_last_minimap_pixel(&mut last, &state, tile_gen);
    apply_circular_crop(
        &state.composite,
        &mut images,
        &mut last,
        state.comp_size,
        state.px_x,
        state.px_y,
    );
}

fn current_minimap_composite_state(
    player_q: &Query<&Transform, With<crate::camera::Player>>,
    composite_res: Option<Res<MinimapComposite>>,
) -> Option<MinimapCompositeState> {
    let player_tf = player_q.single().ok()?;
    let composite = composite_res?.into_inner().clone();
    let bx = player_tf.translation.x;
    let bz = player_tf.translation.z;
    let (player_row, player_col) = crate::terrain_tile::bevy_to_tile_coords(bx, bz);
    let comp_size = MINIMAP_COMPOSITE_SIZE as usize;
    let (px_x, px_y) = player_pixel_in_composite(bx, bz, player_row, player_col, comp_size);
    Some(MinimapCompositeState {
        composite,
        px_x,
        px_y,
        player_row,
        player_col,
        comp_size,
    })
}

fn update_last_minimap_pixel(
    last: &mut LastMinimapPixel,
    state: &MinimapCompositeState,
    tile_generation: usize,
) {
    last.px_x = state.px_x;
    last.px_y = state.px_y;
    last.tile_row = state.player_row;
    last.tile_col = state.player_col;
    last.tile_generation = tile_generation;
}

/// Returns true when the 768×768 composite buffer needs a full rebuild
/// (tile grid shifted or new tiles loaded).  Pixel-only movement is handled
/// by re-cropping without rebuilding the composite.
fn tile_grid_changed(last: &LastMinimapPixel, row: u32, col: u32, tile_gen: usize) -> bool {
    row != last.tile_row || col != last.tile_col || tile_gen != last.tile_generation
}

fn apply_circular_crop(
    composite_res: &MinimapComposite,
    images: &mut Assets<Image>,
    last: &mut LastMinimapPixel,
    comp_size: usize,
    px_x: usize,
    px_y: usize,
) {
    let ds = MINIMAP_DISPLAY_SIZE as usize;
    ensure_circle_mask(&mut last.circle_mask, ds);
    let crop_len = ds * ds * 4;
    last.crop_buf.resize(crop_len, 0);

    crop_with_mask(
        &last.composite_buf,
        comp_size,
        px_x,
        px_y,
        ds,
        &last.circle_mask,
        &mut last.crop_buf,
    );

    if let Some(img) = images.get_mut(&composite_res.handle) {
        img.data = Some(last.crop_buf.clone());
    }
}

/// Build the circular mask once (true = inside circle).
fn ensure_circle_mask(mask: &mut Vec<bool>, ds: usize) {
    if mask.len() == ds * ds {
        return;
    }
    let radius = ds as f32 / 2.0;
    let r2 = radius * radius;
    *mask = (0..ds * ds)
        .map(|i| {
            let x = (i % ds) as f32 - radius + 0.5;
            let y = (i / ds) as f32 - radius + 0.5;
            x * x + y * y <= r2
        })
        .collect();
}

/// Crop a display-sized window from the composite using a precomputed mask.
/// Writes into `out` (must be pre-sized to ds*ds*4).
fn crop_with_mask(
    composite: &[u8],
    comp_size: usize,
    cx: usize,
    cy: usize,
    ds: usize,
    mask: &[bool],
    out: &mut [u8],
) {
    let half = ds / 2;
    let bg = MINIMAP_BG_COLOR;

    for y in 0..ds {
        let sy = cy as i32 - half as i32 + y as i32;
        for x in 0..ds {
            let di = (y * ds + x) * 4;
            if !mask[y * ds + x] {
                out[di..di + 4].fill(0);
                continue;
            }
            let sx = cx as i32 - half as i32 + x as i32;
            let in_bounds =
                sx >= 0 && (sx as usize) < comp_size && sy >= 0 && (sy as usize) < comp_size;
            if in_bounds {
                let si = (sy as usize * comp_size + sx as usize) * 4;
                out[di..di + 4].copy_from_slice(&composite[si..si + 4]);
            } else {
                out[di..di + 4].copy_from_slice(&bg);
            }
        }
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
    for pixel in buf[..comp_size * comp_size * 4].chunks_exact_mut(4) {
        pixel.copy_from_slice(&MINIMAP_BG_COLOR);
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
            fs.text = name;
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

#[cfg(test)]
#[path = "minimap_tests.rs"]
mod tests;
