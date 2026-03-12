use std::collections::{HashMap, HashSet};
use std::path::Path;

use bevy::prelude::*;

use crate::game_state::GameState;
use crate::minimap_render::{
    blit_image, create_arrow_image, create_blank_image, create_border_image, crop_with_circle,
    draw_dot, render_tile_image,
};
use crate::terrain_heightmap::TerrainHeightmap;

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

/// Marker for the minimap UI node.
#[derive(Component)]
pub struct MinimapDisplay;

/// Marker for player arrow.
#[derive(Component)]
pub struct MinimapArrow;

/// Marker for minimap border ring overlay.
#[derive(Component)]
pub struct MinimapBorder;

/// Marker for minimap coordinate text.
#[derive(Component)]
pub struct MinimapCoords;

/// Marker for zone name text above coordinates.
#[derive(Component)]
pub struct MinimapZoneName;

pub struct MinimapPlugin;

/// Marker for all minimap HUD nodes, used to toggle visibility by game state.
#[derive(Component)]
pub struct MinimapHud;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MinimapState>()
            .init_resource::<LastMinimapPixel>();
        register_minimap_systems(app);
    }
}

fn register_minimap_systems(app: &mut App) {
    let in_world = in_state(GameState::InWorld);
    app.add_systems(
        Startup,
        (
            spawn_minimap_display,
            spawn_minimap_border,
            spawn_minimap_arrow,
            spawn_zone_name,
            spawn_coord_text,
        ),
    )
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
        draw_entity_dots
            .after(update_minimap_composite)
            .run_if(in_world.clone()),
    )
    .add_systems(Update, update_coord_text.run_if(in_world.clone()))
    .add_systems(Update, update_zone_name.run_if(in_world.clone()))
    .add_systems(Update, rotate_minimap.run_if(in_world));
}

fn show_minimap_hud(mut query: Query<&mut Visibility, With<MinimapHud>>) {
    for mut vis in &mut query {
        *vis = Visibility::Visible;
    }
}

fn hide_minimap_hud(mut query: Query<&mut Visibility, With<MinimapHud>>) {
    for mut vis in &mut query {
        *vis = Visibility::Hidden;
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

/// Spawn the minimap UI node in the top-right corner.
fn spawn_minimap_display(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let blank = create_blank_image(MINIMAP_DISPLAY_SIZE, MINIMAP_DISPLAY_SIZE);
    let handle = images.add(blank);
    commands.insert_resource(MinimapComposite {
        handle: handle.clone(),
    });

    commands.spawn((
        MinimapDisplay,
        MinimapHud,
        Visibility::Hidden,
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

/// Spawn the minimap border ring overlay on top of the minimap.
fn spawn_minimap_border(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let handle = images.add(create_border_image(MINIMAP_DISPLAY_SIZE as usize));
    commands.spawn((
        MinimapBorder,
        MinimapHud,
        Visibility::Hidden,
        ImageNode::new(handle),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            width: Val::Px(MINIMAP_DISPLAY_SIZE as f32),
            height: Val::Px(MINIMAP_DISPLAY_SIZE as f32),
            ..default()
        },
        ZIndex(10),
    ));
}

/// Spawn a small arrow indicator at the center of the minimap.
fn spawn_minimap_arrow(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let handle = images.add(create_arrow_image());
    let half = MINIMAP_DISPLAY_SIZE as f32 / 2.0;
    let arrow_half = 8.0;
    commands.spawn((
        MinimapArrow,
        MinimapHud,
        Visibility::Hidden,
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
/// Skips recomposite when the player's pixel position and tile set haven't changed.
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

    let tile_gen = minimap.generated.len();
    if px_x == last.px_x
        && px_y == last.px_y
        && player_row == last.tile_row
        && player_col == last.tile_col
        && tile_gen == last.tile_generation
    {
        return;
    }

    recomposite(
        &minimap, &images, &mut last, player_row, player_col, comp_size, px_x, px_y,
    );
    last.px_x = px_x;
    last.px_y = px_y;
    last.tile_row = player_row;
    last.tile_col = player_col;
    last.tile_generation = tile_gen;

    if let Some(img) = images.get_mut(&composite_res.handle) {
        img.data = Some(crop_with_circle(
            &last.composite_buf,
            comp_size,
            px_x,
            px_y,
            MINIMAP_DISPLAY_SIZE,
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn recomposite(
    minimap: &MinimapState,
    images: &Assets<Image>,
    last: &mut LastMinimapPixel,
    player_row: u32,
    player_col: u32,
    comp_size: usize,
    _px_x: usize,
    _px_y: usize,
) {
    let tile_px = MINIMAP_TILE_SIZE as usize;
    let needed = comp_size * comp_size * 4;
    last.composite_buf.resize(needed, 0);
    // Fill dark background
    for i in 0..(comp_size * comp_size) {
        let off = i * 4;
        last.composite_buf[off] = 20;
        last.composite_buf[off + 1] = 20;
        last.composite_buf[off + 2] = 20;
        last.composite_buf[off + 3] = 255;
    }
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
            let key = (row as u32, col as u32);
            let Some(handle) = minimap.tile_images.get(&key) else {
                continue;
            };
            let Some(tile_img) = images.get(handle) else {
                continue;
            };
            let Some(tile_data) = tile_img.data.as_ref() else {
                continue;
            };
            let off_x = dx as usize * tile_px;
            let off_y = dy as usize * tile_px;
            blit_image(composite, comp_size, tile_data, tile_px, off_x, off_y);
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
    let frow = (center - bx) / tile_size;
    let fcol = (center + bz) / tile_size;
    let frac_y = frow - row as f32;
    let frac_x = fcol - col as f32;
    let tile_px = MINIMAP_TILE_SIZE as f32;
    let px_x = (tile_px + frac_x * tile_px) as usize;
    let px_y = (tile_px + frac_y * tile_px) as usize;
    (px_x.min(comp_size - 1), px_y.min(comp_size - 1))
}

/// Spawn zone name text below the minimap.
fn spawn_zone_name(mut commands: Commands) {
    commands.spawn((
        MinimapZoneName,
        MinimapHud,
        Visibility::Hidden,
        Text::new("Elwynn Forest"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 0.82, 0.0, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(215.0),
            right: Val::Px(10.0),
            width: Val::Px(200.0),
            ..default()
        },
    ));
}

/// Spawn coordinate text below the minimap.
fn spawn_coord_text(mut commands: Commands) {
    commands.spawn((
        MinimapCoords,
        MinimapHud,
        Visibility::Hidden,
        Text::new("0, 0"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(235.0),
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

/// Update the zone name text when the current zone changes.
fn update_zone_name(
    zone: Option<Res<crate::networking::CurrentZone>>,
    mut text_q: Query<&mut Text, With<MinimapZoneName>>,
) {
    let Some(zone) = zone else { return };
    if !zone.is_changed() {
        return;
    }
    let name = zone_id_to_name(zone.zone_id);
    for mut text in &mut text_q {
        **text = name.to_string();
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
    let Ok(player_tf) = player_q.single() else {
        return;
    };
    let Some(composite_res) = composite_res else {
        return;
    };
    let Some(img) = images.get_mut(&composite_res.handle) else {
        return;
    };
    let Some(data) = img.data.as_mut() else {
        return;
    };

    let ds = MINIMAP_DISPLAY_SIZE as usize;
    let center = ds as f32 / 2.0;
    let tile_size = crate::asset::adt::CHUNK_SIZE * 16.0;
    let yards_per_pixel = tile_size / MINIMAP_TILE_SIZE as f32;

    for (tf, npc) in &remote_q {
        let dx = tf.translation.x - player_tf.translation.x;
        let dz = tf.translation.z - player_tf.translation.z;
        let px = center + dz / yards_per_pixel;
        let py = center - dx / yards_per_pixel;
        if ((px - center).powi(2) + (py - center).powi(2)).sqrt() > center - 3.0 {
            continue;
        }
        let color = if npc.is_some() {
            [255, 200, 0, 255]
        } else {
            [0, 255, 0, 255]
        };
        draw_dot(data, ds, px as i32, py as i32, &color);
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
}
