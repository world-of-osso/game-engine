use std::collections::{HashMap, HashSet};

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::asset::adt::ChunkHeightGrid;
use crate::terrain::TerrainHeightmap;

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

/// Holds the composite image handle displayed on screen.
#[derive(Resource)]
pub struct MinimapComposite {
    pub handle: Handle<Image>,
}

/// Marker for the minimap UI node.
#[derive(Component)]
pub struct MinimapDisplay;

/// Marker for player arrow.
#[derive(Component)]
pub struct MinimapArrow;

/// Marker for minimap coordinate text.
#[derive(Component)]
pub struct MinimapCoords;

/// Marker for entity dots.
#[derive(Component)]
pub struct MinimapDot {
    pub is_player: bool,
}

pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MinimapState>()
            .add_systems(Startup, (spawn_minimap_display, spawn_minimap_arrow, spawn_coord_text))
            .add_systems(Update, generate_tile_textures)
            .add_systems(Update, update_minimap_composite.after(generate_tile_textures))
            .add_systems(Update, draw_entity_dots.after(update_minimap_composite))
            .add_systems(Update, update_coord_text)
            .add_systems(Update, rotate_minimap);
    }
}

/// Rotate minimap image by camera yaw (WoW-style rotating minimap).
fn rotate_minimap(
    camera_q: Query<&crate::camera::WowCamera>,
    mut minimap_q: Query<&mut Transform, With<MinimapDisplay>>,
) {
    let Ok(cam) = camera_q.single() else { return };
    for mut tf in &mut minimap_q {
        tf.rotation = Quat::from_rotation_z(-cam.yaw);
    }
}

/// Create a blank RGBA image of given dimensions.
fn create_blank_image(w: u32, h: u32) -> Image {
    let data = vec![0u8; (w * h * 4) as usize];
    Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

/// Spawn the minimap UI node in the top-right corner.
fn spawn_minimap_display(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let blank = create_blank_image(MINIMAP_DISPLAY_SIZE, MINIMAP_DISPLAY_SIZE);
    let handle = images.add(blank);
    commands.insert_resource(MinimapComposite { handle: handle.clone() });

    commands.spawn((
        MinimapDisplay,
        ImageNode::new(handle),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            width: Val::Px(MINIMAP_DISPLAY_SIZE as f32),
            height: Val::Px(MINIMAP_DISPLAY_SIZE as f32),
            ..default()
        },
    ));
}

/// Spawn a small arrow indicator at the center of the minimap.
fn spawn_minimap_arrow(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let arrow_img = create_arrow_image();
    let handle = images.add(arrow_img);
    let half = MINIMAP_DISPLAY_SIZE as f32 / 2.0;
    let arrow_half = 8.0;

    commands.spawn((
        MinimapArrow,
        ImageNode::new(handle),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0 + half - arrow_half),
            right: Val::Px(10.0 + half - arrow_half),
            width: Val::Px(16.0),
            height: Val::Px(16.0),
            ..default()
        },
    ));
}

/// Create a 16x16 RGBA image with an upward-pointing yellow triangle.
fn create_arrow_image() -> Image {
    let size = 16usize;
    let mut data = vec![0u8; size * size * 4];

    for y in 0..size {
        for x in 0..size {
            if point_in_triangle(x as f32, y as f32, 8.0, 2.0, 3.0, 13.0, 12.0, 13.0) {
                let i = (y * size + x) * 4;
                data[i] = 255;     // R
                data[i + 1] = 220; // G
                data[i + 2] = 0;   // B
                data[i + 3] = 255; // A
            }
        }
    }

    Image::new(
        Extent3d { width: size as u32, height: size as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

/// Check if point (px, py) is inside triangle defined by three vertices.
fn point_in_triangle(
    px: f32, py: f32,
    x1: f32, y1: f32,
    x2: f32, y2: f32,
    x3: f32, y3: f32,
) -> bool {
    let d1 = (px - x2) * (y1 - y2) - (x1 - x2) * (py - y2);
    let d2 = (px - x3) * (y2 - y3) - (x2 - x3) * (py - y3);
    let d3 = (px - x1) * (y3 - y1) - (x3 - x1) * (py - y1);
    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);
    !(has_neg && has_pos)
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
        if let Some(image) = render_tile_image(&heightmap, ty, tx) {
            let handle = images.add(image);
            minimap.tile_images.insert((ty, tx), handle);
            minimap.generated.insert((ty, tx));
        }
    }
    // Remove textures for unloaded tiles.
    let loaded: HashSet<(u32, u32)> = heightmap.tile_keys().copied().collect();
    minimap.tile_images.retain(|k, _| loaded.contains(k));
    minimap.generated.retain(|k| loaded.contains(k));
}

/// Composite tile images centered on the player, crop and apply circular mask.
fn update_minimap_composite(
    player_q: Query<&Transform, With<crate::camera::Player>>,
    minimap: Res<MinimapState>,
    composite_res: Option<Res<MinimapComposite>>,
    images: Res<Assets<Image>>,
    mut images_mut: ResMut<Assets<Image>>,
) {
    let Ok(player_tf) = player_q.single() else { return };
    let Some(composite_res) = composite_res else { return };

    let bx = player_tf.translation.x;
    let bz = player_tf.translation.z;
    let (player_row, player_col) = crate::terrain::bevy_to_tile_coords(bx, bz);

    let comp_size = MINIMAP_COMPOSITE_SIZE as usize;
    let tile_px = MINIMAP_TILE_SIZE as usize;
    let mut composite = build_dark_composite(comp_size);

    blit_tiles(&mut composite, comp_size, tile_px, player_row, player_col, &minimap, &images);

    let (px_x, px_y) = player_pixel_in_composite(bx, bz, player_row, player_col, comp_size);
    let display = crop_with_circle(&composite, comp_size, px_x, px_y, MINIMAP_DISPLAY_SIZE);

    if let Some(img) = images_mut.get_mut(&composite_res.handle) {
        img.data = Some(display);
    }
}

/// Build a dark-background composite buffer (comp_size x comp_size RGBA).
fn build_dark_composite(comp_size: usize) -> Vec<u8> {
    let mut composite = vec![0u8; comp_size * comp_size * 4];
    for i in 0..(comp_size * comp_size) {
        let off = i * 4;
        composite[off] = 20;
        composite[off + 1] = 20;
        composite[off + 2] = 20;
        composite[off + 3] = 255;
    }
    composite
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
            if row < 0 || col < 0 { continue; }
            let key = (row as u32, col as u32);
            let Some(handle) = minimap.tile_images.get(&key) else { continue };
            let Some(tile_img) = images.get(handle) else { continue };
            let Some(tile_data) = tile_img.data.as_ref() else { continue };

            let off_x = dx as usize * tile_px;
            let off_y = dy as usize * tile_px;
            blit_image(composite, comp_size, tile_data, tile_px, off_x, off_y);
        }
    }
}

/// Copy one tile image (src_w x src_w RGBA) into the composite at (off_x, off_y).
fn blit_image(dst: &mut [u8], dst_w: usize, src: &[u8], src_w: usize, off_x: usize, off_y: usize) {
    for y in 0..src_w {
        let si_start = y * src_w * 4;
        let di_start = ((off_y + y) * dst_w + off_x) * 4;
        let row_bytes = src_w * 4;
        if si_start + row_bytes <= src.len() && di_start + row_bytes <= dst.len() {
            dst[di_start..di_start + row_bytes].copy_from_slice(&src[si_start..si_start + row_bytes]);
        }
    }
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

    // Continuous tile coordinates (fractional)
    let frow = (center - bx) / tile_size;
    let fcol = (center + bz) / tile_size;

    // Fraction within the center tile
    let frac_y = frow - row as f32;
    let frac_x = fcol - col as f32;

    let tile_px = MINIMAP_TILE_SIZE as f32;
    // Center tile starts at (tile_px, tile_px) in the 3x3 composite
    let px_x = (tile_px + frac_x * tile_px) as usize;
    let px_y = (tile_px + frac_y * tile_px) as usize;

    (px_x.min(comp_size - 1), px_y.min(comp_size - 1))
}

/// Crop a display_size window centered on (cx, cy) and apply a circular alpha mask.
fn crop_with_circle(
    composite: &[u8],
    comp_size: usize,
    cx: usize,
    cy: usize,
    display_size: u32,
) -> Vec<u8> {
    let ds = display_size as usize;
    let radius = ds as f32 / 2.0;
    let mut out = vec![0u8; ds * ds * 4];

    for y in 0..ds {
        for x in 0..ds {
            let dx = x as f32 - radius + 0.5;
            let dy = y as f32 - radius + 0.5;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > radius { continue; }

            let di = (y * ds + x) * 4;
            let sx = cx as i32 - ds as i32 / 2 + x as i32;
            let sy = cy as i32 - ds as i32 / 2 + y as i32;

            if sx >= 0 && (sx as usize) < comp_size && sy >= 0 && (sy as usize) < comp_size {
                let si = (sy as usize * comp_size + sx as usize) * 4;
                out[di..di + 4].copy_from_slice(&composite[si..si + 4]);
            } else {
                out[di] = 20;
                out[di + 1] = 20;
                out[di + 2] = 20;
                out[di + 3] = 255;
            }
        }
    }
    out
}

/// Render a 256x256 RGBA image for one terrain tile from heightmap data.
fn render_tile_image(heightmap: &TerrainHeightmap, tile_y: u32, tile_x: u32) -> Option<Image> {
    let chunks = heightmap.tile_chunks(tile_y, tile_x)?;
    let size = MINIMAP_TILE_SIZE as usize;
    let mut data = vec![0u8; size * size * 4];

    let (min_h, max_h) = find_height_range(chunks);
    let range = (max_h - min_h).max(1.0);

    for chunk in chunks.iter().flatten() {
        fill_chunk_pixels(&mut data, size, chunk, min_h, range);
    }

    Some(Image::new(
        Extent3d { width: size as u32, height: size as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    ))
}

/// Fill pixels for a single chunk within the tile image buffer.
fn fill_chunk_pixels(
    data: &mut [u8],
    size: usize,
    chunk: &ChunkHeightGrid,
    min_h: f32,
    range: f32,
) {
    let cx = chunk.index_x as usize;
    let cy = chunk.index_y as usize;
    let ppc = size / 16; // pixels per chunk

    for py in 0..ppc {
        for px in 0..ppc {
            // Sample from 9x9 outer grid (stride 17 in the 145-element array).
            let gx = (px * 8 / ppc).min(8);
            let gy = (py * 8 / ppc).min(8);
            let idx = gy * 17 + gx;
            let h = chunk.heights[idx];

            let t = ((h - min_h) / range).clamp(0.0, 1.0);
            let (r, g, b) = height_to_color(t);

            let img_x = cx * ppc + px;
            let img_y = cy * ppc + py;
            let offset = (img_y * size + img_x) * 4;
            if offset + 3 < data.len() {
                data[offset] = r;
                data[offset + 1] = g;
                data[offset + 2] = b;
                data[offset + 3] = 255;
            }
        }
    }
}

/// Map a normalized height (0..1) to an RGB color.
/// Low = dark green (forest), mid = light green, high = brown (mountains).
fn height_to_color(t: f32) -> (u8, u8, u8) {
    if t < 0.4 {
        let s = t / 0.4;
        let r = (30.0 + s * 50.0) as u8;
        let g = (80.0 + s * 80.0) as u8;
        let b = (20.0 + s * 30.0) as u8;
        (r, g, b)
    } else {
        let s = (t - 0.4) / 0.6;
        let r = (80.0 + s * 80.0) as u8;
        let g = (160.0 - s * 80.0) as u8;
        let b = (50.0 - s * 20.0) as u8;
        (r, g, b)
    }
}

/// Find min/max height across all chunks in a tile.
fn find_height_range(chunks: &[Option<ChunkHeightGrid>]) -> (f32, f32) {
    let mut min_h = f32::MAX;
    let mut max_h = f32::MIN;
    for chunk in chunks.iter().flatten() {
        for &h in &chunk.heights {
            min_h = min_h.min(h);
            max_h = max_h.max(h);
        }
    }
    if min_h > max_h { (0.0, 1.0) } else { (min_h, max_h) }
}

/// Spawn coordinate text below the minimap.
fn spawn_coord_text(mut commands: Commands) {
    commands.spawn((
        MinimapCoords,
        Text::new("0, 0"),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(215.0), // below minimap (10 + 200 + 5)
            right: Val::Px(10.0),
            width: Val::Px(200.0),
            ..default()
        },
    ));
}

/// Update coordinate text with the player's current position.
fn update_coord_text(
    player_q: Query<&Transform, With<crate::camera::Player>>,
    mut text_q: Query<&mut Text, With<MinimapCoords>>,
) {
    let Ok(tf) = player_q.single() else { return };
    let x = tf.translation.x;
    let z = tf.translation.z;
    for mut text in &mut text_q {
        **text = format!("{x:.0}, {z:.0}");
    }
}

/// Draw colored dots for nearby remote entities onto the minimap image.
fn draw_entity_dots(
    player_q: Query<&Transform, With<crate::camera::Player>>,
    remote_q: Query<
        (&Transform, Option<&shared::components::Npc>),
        With<crate::networking::RemoteEntity>,
    >,
    composite_res: Option<Res<MinimapComposite>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok(player_tf) = player_q.single() else { return };
    let Some(composite_res) = composite_res else { return };
    let Some(img) = images.get_mut(&composite_res.handle) else { return };
    let Some(data) = img.data.as_mut() else { return };

    let ds = MINIMAP_DISPLAY_SIZE as usize;
    let center = ds as f32 / 2.0;
    let tile_size = crate::asset::adt::CHUNK_SIZE * 16.0;
    let yards_per_pixel = tile_size / MINIMAP_TILE_SIZE as f32;

    for (tf, npc) in &remote_q {
        let dx = tf.translation.x - player_tf.translation.x;
        let dz = tf.translation.z - player_tf.translation.z;

        // Minimap axes: col = +bz, row = -bx (matches player_pixel_in_composite)
        let px = center + dz / yards_per_pixel;
        let py = center - dx / yards_per_pixel;

        let dist = ((px - center).powi(2) + (py - center).powi(2)).sqrt();
        if dist > center - 3.0 { continue; }

        let color = if npc.is_some() { [255, 200, 0, 255] } else { [0, 255, 0, 255] };
        draw_dot(data, ds, px as i32, py as i32, &color);
    }
}

/// Draw a 3x3 colored dot at (cx, cy) in an RGBA buffer.
fn draw_dot(data: &mut [u8], size: usize, cx: i32, cy: i32, color: &[u8; 4]) {
    for dy in -1..=1i32 {
        for dx in -1..=1i32 {
            let x = cx + dx;
            let y = cy + dy;
            if x >= 0 && y >= 0 && (x as usize) < size && (y as usize) < size {
                let i = (y as usize * size + x as usize) * 4;
                if i + 3 < data.len() {
                    data[i..i + 4].copy_from_slice(color);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(!point_in_triangle(0.0, 0.0, 8.0, 2.0, 3.0, 13.0, 12.0, 13.0));
    }

    #[test]
    fn arrow_image_has_yellow_pixels() {
        let img = create_arrow_image();
        let data = img.data.as_ref().unwrap();
        // Center pixel (8,8) should be yellow (inside triangle)
        let i = (8 * 16 + 8) * 4;
        assert_eq!(data[i], 255);
        assert_eq!(data[i + 1], 220);
        assert_eq!(data[i + 3], 255);
    }

    #[test]
    fn crop_circle_center_pixel_opaque() {
        // 4x4 white composite
        let comp = vec![255u8; 4 * 4 * 4];
        let result = crop_with_circle(&comp, 4, 2, 2, 4);
        // Center pixel (2,2) should be opaque
        assert_eq!(result[(2 * 4 + 2) * 4 + 3], 255);
    }

    #[test]
    fn draw_dot_center() {
        let mut data = vec![0u8; 10 * 10 * 4];
        draw_dot(&mut data, 10, 5, 5, &[255, 0, 0, 255]);
        // Center pixel should be red
        let i = (5 * 10 + 5) * 4;
        assert_eq!(&data[i..i + 4], &[255, 0, 0, 255]);
        // Adjacent pixel should also be red (3x3 dot)
        let i2 = (4 * 10 + 5) * 4;
        assert_eq!(&data[i2..i2 + 4], &[255, 0, 0, 255]);
        // Pixel 2 away should be untouched
        let i3 = (3 * 10 + 5) * 4;
        assert_eq!(&data[i3..i3 + 4], &[0, 0, 0, 0]);
    }

    #[test]
    fn draw_dot_edge_no_panic() {
        let mut data = vec![0u8; 5 * 5 * 4];
        draw_dot(&mut data, 5, 0, 0, &[0, 255, 0, 255]);
        // (0,0) should be colored, (-1,-1) clipped without panic
        assert_eq!(&data[0..4], &[0, 255, 0, 255]);
    }

    #[test]
    fn crop_circle_corner_transparent() {
        let comp = vec![255u8; 100 * 100 * 4];
        let result = crop_with_circle(&comp, 100, 50, 50, 100);
        // Corner (0,0) should be transparent (outside circle)
        assert_eq!(result[3], 0);
    }
}
