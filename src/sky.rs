use std::collections::BTreeMap;
use std::f32::consts::{FRAC_PI_2, TAU};

use bevy::asset::RenderAssetUsages;
use bevy::light::GeneratedEnvironmentMapLight;
use bevy::pbr::{DistanceFog, FogFalloff, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
};
use serde::Deserialize;

use crate::game_state::GameState;

pub use crate::sky_material::{SkyMaterial, SkyUniforms};

// ---------------------------------------------------------------------------
// Time display UI
// ---------------------------------------------------------------------------

#[derive(Component)]
struct TimeDisplay;

// ---------------------------------------------------------------------------
// GameTime resource
// ---------------------------------------------------------------------------

/// In-game time of day. 0=midnight, 720=dawn, 1440=noon, 2160=dusk, 2880=midnight.
#[derive(Resource)]
pub struct GameTime {
    pub minutes: f32,
    pub speed: f32,
}

impl Default for GameTime {
    fn default() -> Self {
        Self {
            minutes: 1440.0,
            speed: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// LightData parsing (RON + CSV fallback)
// ---------------------------------------------------------------------------

/// One keyframe row filtered to a single LightParamID.
#[derive(Debug, Clone)]
struct LightDataRow {
    time: f32,
    direct_color: Color,
    ambient_color: Color,
    sky_top: Color,
    sky_middle: Color,
    sky_band1: Color,
    sky_band2: Color,
    sky_smog: Color,
    #[allow(dead_code)]
    fog_color: Color,
}

#[derive(Debug, Deserialize)]
struct LightDataFile {
    by_param: BTreeMap<u32, Vec<LightDataSerializedRow>>,
}

#[derive(Debug, Deserialize)]
struct LightDataSerializedRow {
    time: f32,
    direct_color: u32,
    ambient_color: u32,
    sky_top: u32,
    sky_middle: u32,
    sky_band1: u32,
    sky_band2: u32,
    sky_smog: u32,
    #[serde(default)]
    fog_color: u32,
}

/// Decode a BGR32 integer (as stored in LightData exports) to linear Color.
fn decode_bgr32(val: u32) -> Color {
    let r = (val & 0xFF) as f32 / 255.0;
    let g = ((val >> 8) & 0xFF) as f32 / 255.0;
    let b = ((val >> 16) & 0xFF) as f32 / 255.0;
    Color::linear_rgb(r, g, b)
}

/// Interpolated sky color set for the current time of day.
#[derive(Debug, Clone)]
pub struct SkyColorSet {
    pub sky_top: Color,
    pub sky_middle: Color,
    pub sky_band1: Color,
    pub sky_band2: Color,
    pub sky_smog: Color,
    pub direct_color: Color,
    pub ambient_color: Color,
    #[allow(dead_code)]
    pub fog_color: Color,
}

fn deserialize_light_row(row: LightDataSerializedRow) -> LightDataRow {
    LightDataRow {
        time: row.time,
        direct_color: decode_bgr32(row.direct_color),
        ambient_color: decode_bgr32(row.ambient_color),
        sky_top: decode_bgr32(row.sky_top),
        sky_middle: decode_bgr32(row.sky_middle),
        sky_band1: decode_bgr32(row.sky_band1),
        sky_band2: decode_bgr32(row.sky_band2),
        sky_smog: decode_bgr32(row.sky_smog),
        fog_color: decode_bgr32(row.fog_color),
    }
}

fn load_light_data_ron(path: &str, param_id: u32) -> Result<Vec<LightDataRow>, String> {
    let contents = std::fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    let mut file: LightDataFile =
        ron::from_str(&contents).map_err(|e| format!("parse {path}: {e}"))?;
    let mut rows: Vec<LightDataRow> = file
        .by_param
        .remove(&param_id)
        .unwrap_or_default()
        .into_iter()
        .map(deserialize_light_row)
        .collect();
    rows.sort_by(|a, b| a.time.total_cmp(&b.time));
    Ok(rows)
}

/// Resolve CSV column indices for legacy LightData.csv fallback.
fn resolve_csv_fallback_column_indices(header: &str) -> [usize; 10] {
    let cols: Vec<&str> = header.split(',').collect();
    let idx =
        |name: &str, fallback: usize| cols.iter().position(|c| *c == name).unwrap_or(fallback);
    [
        idx("LightParamID", 1),
        idx("Time", 2),
        idx("DirectColor", 3),
        idx("AmbientColor", 4),
        idx("SkyTopColor", 5),
        idx("SkyMiddleColor", 6),
        idx("SkyBand1Color", 7),
        idx("SkyBand2Color", 8),
        idx("SkySmogColor", 9),
        idx("SkyFogColor", 10),
    ]
}

/// Parse a single CSV line into a LightDataRow using pre-resolved column indices.
fn parse_csv_fallback_light_row(
    line: &str,
    ci: &[usize; 10],
    param_id: u32,
) -> Option<LightDataRow> {
    let fields: Vec<&str> = line.split(',').collect();
    let pid: u32 = fields.get(ci[0])?.parse().ok()?;
    if pid != param_id {
        return None;
    }
    let p = |i: usize| -> u32 { fields.get(ci[i]).and_then(|s| s.parse().ok()).unwrap_or(0) };
    Some(LightDataRow {
        time: p(1) as f32,
        direct_color: decode_bgr32(p(2)),
        ambient_color: decode_bgr32(p(3)),
        sky_top: decode_bgr32(p(4)),
        sky_middle: decode_bgr32(p(5)),
        sky_band1: decode_bgr32(p(6)),
        sky_band2: decode_bgr32(p(7)),
        sky_smog: decode_bgr32(p(8)),
        fog_color: decode_bgr32(p(9)),
    })
}

/// Load legacy LightData.csv rows for a specific LightParamID, sorted by time.
fn load_light_data_csv_fallback(path: &str, param_id: u32) -> Vec<LightDataRow> {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read fallback {path}: {e}");
            return Vec::new();
        }
    };
    let mut lines = contents.lines();
    let header = match lines.next() {
        Some(h) => h,
        None => return Vec::new(),
    };
    let ci = resolve_csv_fallback_column_indices(header);
    let mut rows: Vec<LightDataRow> = lines
        .filter_map(|l| parse_csv_fallback_light_row(l, &ci, param_id))
        .collect();
    rows.sort_by(|a, b| a.time.total_cmp(&b.time));
    rows
}

/// Load LightData.ron rows for a specific LightParamID, with CSV fallback.
fn load_light_data(path: &str, param_id: u32) -> Vec<LightDataRow> {
    match load_light_data_ron(path, param_id) {
        Ok(rows) => rows,
        Err(err) => {
            eprintln!("{err}");
            if let Some(base) = path.strip_suffix(".ron") {
                let csv_path = format!("{base}.csv");
                eprintln!("Falling back to legacy CSV: {csv_path}");
                return load_light_data_csv_fallback(&csv_path, param_id);
            }
            Vec::new()
        }
    }
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let a = a.to_linear();
    let b = b.to_linear();
    Color::linear_rgba(
        a.red + (b.red - a.red) * t,
        a.green + (b.green - a.green) * t,
        a.blue + (b.blue - a.blue) * t,
        1.0,
    )
}

/// Default sky colors when no LightData is available.
fn default_sky_colors() -> SkyColorSet {
    SkyColorSet {
        sky_top: Color::linear_rgb(0.2, 0.4, 0.8),
        sky_middle: Color::linear_rgb(0.4, 0.6, 0.9),
        sky_band1: Color::linear_rgb(0.5, 0.7, 0.9),
        sky_band2: Color::linear_rgb(0.6, 0.75, 0.9),
        sky_smog: Color::linear_rgb(0.7, 0.8, 0.85),
        direct_color: Color::WHITE,
        ambient_color: Color::linear_rgb(0.3, 0.3, 0.4),
        fog_color: Color::linear_rgb(0.7, 0.8, 0.9),
    }
}

/// Interpolate a SkyColorSet from two rows at factor t.
fn lerp_rows(a: &LightDataRow, b: &LightDataRow, t: f32) -> SkyColorSet {
    SkyColorSet {
        sky_top: lerp_color(a.sky_top, b.sky_top, t),
        sky_middle: lerp_color(a.sky_middle, b.sky_middle, t),
        sky_band1: lerp_color(a.sky_band1, b.sky_band1, t),
        sky_band2: lerp_color(a.sky_band2, b.sky_band2, t),
        sky_smog: lerp_color(a.sky_smog, b.sky_smog, t),
        direct_color: lerp_color(a.direct_color, b.direct_color, t),
        ambient_color: lerp_color(a.ambient_color, b.ambient_color, t),
        fog_color: lerp_color(a.fog_color, b.fog_color, t),
    }
}

/// Find the two keyframes bracketing `m` and the interpolation factor.
fn find_bracket(rows: &[LightDataRow], m: f32) -> (&LightDataRow, &LightDataRow, f32) {
    for i in 0..rows.len() {
        let next = (i + 1) % rows.len();
        let t0 = rows[i].time;
        let t1 = if next == 0 {
            rows[next].time + 2880.0
        } else {
            rows[next].time
        };
        let m_adj = if next == 0 && m < t0 { m + 2880.0 } else { m };
        if m_adj >= t0 && m_adj <= t1 {
            let span = t1 - t0;
            let t = if span > 0.0 { (m_adj - t0) / span } else { 0.0 };
            return (&rows[i], &rows[next], t);
        }
    }
    // Fallback: wrap around from last to first
    let last = &rows[rows.len() - 1];
    let first = &rows[0];
    let span = (first.time + 2880.0) - last.time;
    let t = if span > 0.0 {
        (m + 2880.0 - last.time) / span
    } else {
        0.0
    };
    (last, first, t)
}

/// Interpolate between LightData keyframes at the given time (0–2880).
fn interpolate_colors(rows: &[LightDataRow], minutes: f32) -> SkyColorSet {
    match rows.len() {
        0 => default_sky_colors(),
        1 => lerp_rows(&rows[0], &rows[0], 0.0),
        _ => {
            let m = minutes.rem_euclid(2880.0);
            let (a, b, t) = find_bracket(rows, m);
            lerp_rows(a, b, t)
        }
    }
}

// ---------------------------------------------------------------------------
// Sky dome mesh + spawning
// ---------------------------------------------------------------------------

/// Marker component for the sky dome entity.
#[derive(Component)]
pub struct SkyDome;

/// Resource holding parsed LightData keyframes.
#[derive(Resource)]
struct LightKeyframes(Vec<LightDataRow>);

/// Compute vertices for one latitude ring of the sky dome.
fn push_ring(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    radius: f32,
    lon_segments: u32,
    v: f32,
    theta: f32,
) {
    let y = radius * theta.cos();
    let ring_r = radius * theta.sin();
    for lon in 0..=lon_segments {
        let u = lon as f32 / lon_segments as f32;
        let phi = 2.0 * std::f32::consts::PI * u;
        let x = ring_r * phi.cos();
        let z = ring_r * phi.sin();
        positions.push([x, y, z]);
        normals.push([-x / radius, -y / radius, -z / radius]);
        uvs.push([u, v]);
    }
}

/// Generate triangle indices for the dome grid (reversed winding for inside-out).
fn build_dome_indices(lon_segments: u32, lat_segments: u32) -> Vec<u32> {
    let mut indices = Vec::new();
    for lat in 0..lat_segments {
        for lon in 0..lon_segments {
            let a = lat * (lon_segments + 1) + lon;
            let b = a + lon_segments + 1;
            indices.extend_from_slice(&[a, a + 1, b, b, a + 1, b + 1]);
        }
    }
    indices
}

/// Build an inverted UV sphere (viewed from inside) covering upper hemisphere.
fn build_sky_dome_mesh(radius: f32, lon_segments: u32, lat_segments: u32) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    for lat in 0..=lat_segments {
        let v = lat as f32 / lat_segments as f32;
        // Map v=0 to slightly below horizon (-10°), v=1 to zenith
        let theta = std::f32::consts::PI * (0.55 - v * 0.55);
        push_ring(
            &mut positions,
            &mut normals,
            &mut uvs,
            radius,
            lon_segments,
            v,
            theta,
        );
    }
    let indices = build_dome_indices(lon_segments, lat_segments);
    let mut mesh = Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));
    mesh
}

/// Spawn the sky dome as a child of the camera entity and set up fog + IBL.
pub fn spawn_sky_dome(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    sky_materials: &mut Assets<SkyMaterial>,
    images: &mut Assets<Image>,
    camera_entity: Entity,
) {
    let mesh = build_sky_dome_mesh(900.0, 32, 16);
    let material = sky_materials.add(SkyMaterial {
        uniforms: SkyUniforms::default(),
    });
    let dome = commands
        .spawn((
            Name::new("sky_dome"),
            SkyDome,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material),
            Transform::IDENTITY,
            Visibility::default(),
        ))
        .id();
    commands.entity(dome).set_parent_in_place(camera_entity);

    // Distance fog matching sky_smog color
    let default_colors = default_sky_colors();
    commands.entity(camera_entity).insert(DistanceFog {
        color: default_colors.sky_smog,
        directional_light_color: default_colors.sky_band2,
        directional_light_exponent: 8.0,
        falloff: FogFalloff::ExponentialSquared { density: 0.0008 },
    });

    // Procedural sky cubemap for IBL
    let cubemap = build_sky_cubemap(&default_colors);
    let cubemap_handle = images.add(cubemap);
    commands.insert_resource(SkyEnvMapHandle(cubemap_handle.clone()));
    commands
        .entity(camera_entity)
        .insert(GeneratedEnvironmentMapLight {
            environment_map: cubemap_handle,
            intensity: 300.0,
            rotation: Quat::IDENTITY,
            affects_lightmapped_mesh_diffuse: true,
        });
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn advance_game_time(time: Res<Time>, mut game_time: ResMut<GameTime>) {
    if game_time.speed > 0.0 {
        game_time.minutes += time.delta_secs() * game_time.speed;
        game_time.minutes = game_time.minutes.rem_euclid(2880.0);
    }
}

fn color_to_vec4(c: Color) -> Vec4 {
    let lin = c.to_linear();
    Vec4::new(lin.red, lin.green, lin.blue, 1.0)
}

fn update_sky_colors(
    game_time: Res<GameTime>,
    keyframes: Res<LightKeyframes>,
    sky_dome_q: Query<&MeshMaterial3d<SkyMaterial>, With<SkyDome>>,
    mut sky_materials: ResMut<Assets<SkyMaterial>>,
    mut dir_lights: Query<&mut DirectionalLight>,
    mut ambient_q: Query<&mut AmbientLight>,
    mut water_materials: ResMut<Assets<crate::water_material::WaterMaterial>>,
) {
    let colors = interpolate_colors(&keyframes.0, game_time.minutes);
    update_sky_dome_material(&sky_dome_q, &mut sky_materials, &colors);
    sync_lights(&mut dir_lights, &mut ambient_q, &colors);
    sync_water_sky_color(&mut water_materials, &colors);
}

fn update_sky_dome_material(
    sky_dome_q: &Query<&MeshMaterial3d<SkyMaterial>, With<SkyDome>>,
    sky_materials: &mut Assets<SkyMaterial>,
    colors: &SkyColorSet,
) {
    for mat_handle in sky_dome_q.iter() {
        if let Some(mat) = sky_materials.get_mut(mat_handle) {
            mat.uniforms.sky_top = color_to_vec4(colors.sky_top);
            mat.uniforms.sky_middle = color_to_vec4(colors.sky_middle);
            mat.uniforms.sky_band1 = color_to_vec4(colors.sky_band1);
            mat.uniforms.sky_band2 = color_to_vec4(colors.sky_band2);
            mat.uniforms.sky_smog = color_to_vec4(colors.sky_smog);
        }
    }
}

fn sync_lights(
    dir_lights: &mut Query<&mut DirectionalLight>,
    ambient_q: &mut Query<&mut AmbientLight>,
    colors: &SkyColorSet,
) {
    for mut light in dir_lights.iter_mut() {
        light.color = colors.direct_color;
    }
    for mut amb in ambient_q.iter_mut() {
        amb.color = colors.ambient_color;
    }
}

fn sync_water_sky_color(
    water_materials: &mut Assets<crate::water_material::WaterMaterial>,
    colors: &SkyColorSet,
) {
    let sky_vec4 = color_to_vec4(colors.sky_band2);
    for (_id, mat) in water_materials.iter_mut() {
        mat.settings.sky_color = sky_vec4;
    }
}

// ---------------------------------------------------------------------------
// Sun direction (arc across sky based on time of day)
// ---------------------------------------------------------------------------

/// Compute the sun's elevation (-1..1) from game time.
/// -1 = directly below (midnight), 0 = horizon (dawn/dusk), 1 = zenith (noon).
fn sun_elevation(minutes: f32) -> f32 {
    (minutes / 2880.0 * TAU - FRAC_PI_2).sin()
}

/// Compute the sun's pitch rotation for the DirectionalLight.
/// Dawn(720)=horizon, Noon(1440)=overhead, Dusk(2160)=opposite horizon.
fn sun_rotation(minutes: f32) -> Quat {
    // Map time to angle: noon → straight down, dawn/dusk → horizontal
    let pitch = FRAC_PI_2 - (minutes / 2880.0) * TAU;
    // Add slight Y rotation so shadows aren't perfectly axis-aligned
    Quat::from_rotation_y(0.3) * Quat::from_rotation_x(pitch)
}

fn update_sun_direction(
    game_time: Res<GameTime>,
    mut dir_lights: Query<(&mut Transform, &mut DirectionalLight)>,
) {
    let elev = sun_elevation(game_time.minutes);
    let rotation = sun_rotation(game_time.minutes);
    // Dim light when sun is below horizon
    let intensity = if elev > 0.0 {
        light_consts::lux::OVERCAST_DAY * elev.sqrt()
    } else {
        // Moonlight: very dim
        light_consts::lux::OVERCAST_DAY * 0.02
    };
    for (mut transform, mut light) in dir_lights.iter_mut() {
        transform.rotation = rotation;
        light.illuminance = intensity;
    }
}

// ---------------------------------------------------------------------------
// Distance fog
// ---------------------------------------------------------------------------

fn update_fog(
    game_time: Res<GameTime>,
    keyframes: Res<LightKeyframes>,
    mut fog_q: Query<&mut DistanceFog>,
) {
    let colors = interpolate_colors(&keyframes.0, game_time.minutes);
    for mut fog in fog_q.iter_mut() {
        fog.color = colors.sky_smog;
        fog.directional_light_color = colors.sky_band2;
    }
}

// ---------------------------------------------------------------------------
// Environment map (IBL) from sky gradient
// ---------------------------------------------------------------------------

/// Cubemap face size for the procedural sky environment map.
const ENV_MAP_SIZE: u32 = 32;

/// Marker for the sky environment cubemap image asset.
#[derive(Resource)]
struct SkyEnvMapHandle(Handle<Image>);

/// Build a cubemap Image (6 faces) from the given sky color set.
fn build_sky_cubemap(colors: &SkyColorSet) -> Image {
    let face_pixels = (ENV_MAP_SIZE * ENV_MAP_SIZE) as usize;
    let total_bytes = face_pixels * 6 * 8; // 6 faces, Rgba16Float = 8 bytes/pixel
    let mut data = vec![0u8; total_bytes];

    for face in 0..6u32 {
        let offset = (face as usize) * face_pixels * 8;
        fill_cubemap_face(&mut data[offset..offset + face_pixels * 8], face, colors);
    }

    let mut image = Image::new(
        Extent3d {
            width: ENV_MAP_SIZE,
            height: ENV_MAP_SIZE,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba16Float,
        RenderAssetUsages::default(),
    );
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..Default::default()
    });
    image
}

/// Fill one cubemap face with sky gradient colors based on elevation.
fn fill_cubemap_face(data: &mut [u8], face: u32, colors: &SkyColorSet) {
    for y in 0..ENV_MAP_SIZE {
        for x in 0..ENV_MAP_SIZE {
            let dir = cubemap_direction(face, x, y);
            let elev = dir.y.asin() / FRAC_PI_2; // -1..1
            let color = sample_sky_gradient(colors, elev);
            let pixel_offset = ((y * ENV_MAP_SIZE + x) as usize) * 8;
            write_rgba16f(&mut data[pixel_offset..pixel_offset + 8], color);
        }
    }
}

/// Get the 3D direction for a cubemap face pixel.
/// Faces: 0=+X, 1=-X, 2=+Y, 3=-Y, 4=+Z, 5=-Z
fn cubemap_direction(face: u32, x: u32, y: u32) -> Vec3 {
    let u = (x as f32 + 0.5) / ENV_MAP_SIZE as f32 * 2.0 - 1.0;
    let v = (y as f32 + 0.5) / ENV_MAP_SIZE as f32 * 2.0 - 1.0;
    let dir = match face {
        0 => Vec3::new(1.0, -v, -u),  // +X
        1 => Vec3::new(-1.0, -v, u),  // -X
        2 => Vec3::new(u, 1.0, v),    // +Y
        3 => Vec3::new(u, -1.0, -v),  // -Y
        4 => Vec3::new(u, -v, 1.0),   // +Z
        _ => Vec3::new(-u, -v, -1.0), // -Z
    };
    dir.normalize()
}

/// Sample the sky gradient at a given elevation (-1..1).
fn sample_sky_gradient(colors: &SkyColorSet, elev: f32) -> LinearRgba {
    let elev = elev.clamp(-0.1, 1.0);
    let normalized = ((elev + 0.1) / 1.1).clamp(0.0, 1.0); // 0..1
    let (a, b, t) = if normalized < 0.1 {
        (colors.sky_smog, colors.sky_smog, 0.0)
    } else if normalized < 0.3 {
        let t = (normalized - 0.1) / 0.2;
        (colors.sky_smog, colors.sky_band2, t)
    } else if normalized < 0.5 {
        let t = (normalized - 0.3) / 0.2;
        (colors.sky_band2, colors.sky_band1, t)
    } else if normalized < 0.7 {
        let t = (normalized - 0.5) / 0.2;
        (colors.sky_band1, colors.sky_middle, t)
    } else {
        let t = (normalized - 0.7) / 0.3;
        (colors.sky_middle, colors.sky_top, t)
    };
    let a = a.to_linear();
    let b = b.to_linear();
    LinearRgba::new(
        a.red + (b.red - a.red) * t,
        a.green + (b.green - a.green) * t,
        a.blue + (b.blue - a.blue) * t,
        1.0,
    )
}

/// Write a LinearRgba color as 4×f16 bytes.
fn write_rgba16f(dst: &mut [u8], c: LinearRgba) {
    let vals = [c.red, c.green, c.blue, c.alpha];
    for (i, &v) in vals.iter().enumerate() {
        let h = half::f16::from_f32(v);
        let bytes = h.to_le_bytes();
        dst[i * 2] = bytes[0];
        dst[i * 2 + 1] = bytes[1];
    }
}

/// Tracks the last game-time minute the env map was rebuilt at.
#[derive(Resource, Default)]
struct EnvMapLastMinute(f32);

fn update_sky_env_map(
    game_time: Res<GameTime>,
    keyframes: Res<LightKeyframes>,
    env_handle: Option<Res<SkyEnvMapHandle>>,
    mut images: ResMut<Assets<Image>>,
    mut last: Local<f32>,
) {
    let Some(handle) = env_handle else { return };
    // Only rebuild cubemap when time changes by >1 minute (avoid per-frame GPU re-upload)
    if (game_time.minutes - *last).abs() < 1.0 {
        return;
    }
    *last = game_time.minutes;
    let colors = interpolate_colors(&keyframes.0, game_time.minutes);
    if let Some(image) = images.get_mut(&handle.0) {
        *image = build_sky_cubemap(&colors);
    }
}

// ---------------------------------------------------------------------------
// Time display systems
// ---------------------------------------------------------------------------

fn spawn_time_display(mut commands: Commands) {
    commands.spawn((
        TimeDisplay,
        Visibility::Hidden,
        Text::new("12:00"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(220.0),
            ..default()
        },
    ));
}

fn show_time_display(mut query: Query<&mut Visibility, With<TimeDisplay>>) {
    for mut vis in &mut query {
        *vis = Visibility::Visible;
    }
}

fn hide_time_display(mut query: Query<&mut Visibility, With<TimeDisplay>>) {
    for mut vis in &mut query {
        *vis = Visibility::Hidden;
    }
}

/// Convert GameTime minutes (0–2880) to HH:MM clock string.
fn format_game_clock(total: f32) -> String {
    let m = total.rem_euclid(2880.0);
    let hours = (m / 120.0) as u32 % 24;
    let mins = ((m % 120.0) / 2.0) as u32;
    format!("{hours:02}:{mins:02}")
}

fn update_time_display(game_time: Res<GameTime>, mut query: Query<&mut Text, With<TimeDisplay>>) {
    let clock = format_game_clock(game_time.minutes);
    for mut text in &mut query {
        **text = clock.clone();
    }
}

fn time_speed_controls(keys: Res<ButtonInput<KeyCode>>, mut game_time: ResMut<GameTime>) {
    if keys.just_pressed(KeyCode::BracketRight) {
        game_time.speed = match game_time.speed as u32 {
            0 => 1.0,
            1 => 10.0,
            10 => 60.0,
            _ => 0.0,
        };
    }
    if keys.just_pressed(KeyCode::BracketLeft) {
        game_time.speed = match game_time.speed as u32 {
            0 => 60.0,
            60 => 10.0,
            10 => 1.0,
            _ => 0.0,
        };
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        let keyframes = load_light_data("data/LightData.ron", 12);
        info!(
            "Loaded {} sky keyframes for LightParamID 12",
            keyframes.len()
        );
        app.add_plugins(MaterialPlugin::<SkyMaterial>::default())
            .insert_resource(GameTime::default())
            .insert_resource(LightKeyframes(keyframes))
            .add_systems(Startup, spawn_time_display)
            .add_systems(
                Update,
                advance_game_time.run_if(in_state(GameState::InWorld)),
            )
            .add_systems(
                Update,
                update_sky_colors
                    .after(advance_game_time)
                    .run_if(in_state(GameState::InWorld)),
            )
            .add_systems(
                Update,
                update_sun_direction
                    .after(advance_game_time)
                    .run_if(in_state(GameState::InWorld)),
            )
            .add_systems(
                Update,
                update_fog
                    .after(advance_game_time)
                    .run_if(in_state(GameState::InWorld)),
            )
            .add_systems(
                Update,
                update_sky_env_map
                    .after(advance_game_time)
                    .run_if(in_state(GameState::InWorld)),
            )
            .add_systems(
                Update,
                update_time_display
                    .after(advance_game_time)
                    .run_if(in_state(GameState::InWorld)),
            )
            .add_systems(
                Update,
                time_speed_controls.run_if(in_state(GameState::InWorld)),
            )
            .add_systems(OnEnter(GameState::InWorld), show_time_display)
            .add_systems(OnExit(GameState::InWorld), hide_time_display);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_bgr32_red() {
        let c = decode_bgr32(0x000000FF);
        let lin = c.to_linear();
        assert!((lin.red - 1.0).abs() < 0.01);
        assert!(lin.green < 0.01);
        assert!(lin.blue < 0.01);
    }

    #[test]
    fn decode_bgr32_blue() {
        let c = decode_bgr32(0x00FF0000);
        let lin = c.to_linear();
        assert!(lin.red < 0.01);
        assert!(lin.green < 0.01);
        assert!((lin.blue - 1.0).abs() < 0.01);
    }

    #[test]
    fn decode_bgr32_white() {
        let c = decode_bgr32(0x00FFFFFF);
        let lin = c.to_linear();
        assert!((lin.red - 1.0).abs() < 0.01);
        assert!((lin.green - 1.0).abs() < 0.01);
        assert!((lin.blue - 1.0).abs() < 0.01);
    }

    #[test]
    fn interpolation_midpoint() {
        let rows = vec![
            LightDataRow {
                time: 0.0,
                direct_color: Color::WHITE,
                ambient_color: Color::WHITE,
                sky_top: Color::linear_rgb(0.0, 0.0, 0.0),
                sky_middle: Color::BLACK,
                sky_band1: Color::BLACK,
                sky_band2: Color::BLACK,
                sky_smog: Color::BLACK,
                fog_color: Color::BLACK,
            },
            LightDataRow {
                time: 1440.0,
                direct_color: Color::WHITE,
                ambient_color: Color::WHITE,
                sky_top: Color::linear_rgb(1.0, 1.0, 1.0),
                sky_middle: Color::WHITE,
                sky_band1: Color::WHITE,
                sky_band2: Color::WHITE,
                sky_smog: Color::WHITE,
                fog_color: Color::WHITE,
            },
        ];
        let result = interpolate_colors(&rows, 720.0);
        let top = result.sky_top.to_linear();
        assert!((top.red - 0.5).abs() < 0.05);
    }

    #[test]
    fn game_time_to_clock() {
        // noon = 1440 minutes → 12:00
        assert_eq!(format_game_clock(1440.0), "12:00");
        // 6am = 720 minutes → 06:00
        assert_eq!(format_game_clock(720.0), "06:00");
        // midnight = 0 → 00:00
        assert_eq!(format_game_clock(0.0), "00:00");
        // midnight wrap = 2880 → 00:00
        assert_eq!(format_game_clock(2880.0), "00:00");
        // dusk = 2160 → 18:00
        assert_eq!(format_game_clock(2160.0), "18:00");
        // 6:30am = 720 + 60 = 780 → 06:30
        assert_eq!(format_game_clock(780.0), "06:30");
    }

    #[test]
    fn load_light_data_real() {
        let rows = load_light_data("data/LightData.ron", 12);
        assert!(!rows.is_empty(), "Should find rows for LightParamID 12");
        for w in rows.windows(2) {
            assert!(w[0].time <= w[1].time, "Rows should be sorted by time");
        }
    }
}
