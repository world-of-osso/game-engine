use std::f32::consts::{FRAC_PI_2, TAU};

use bevy::asset::RenderAssetUsages;
use bevy::light::GeneratedEnvironmentMapLight;
use bevy::pbr::{DistanceFog, FogFalloff, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
};

use crate::game_state::GameState;
use crate::sky_lightdata::{
    LightDataRow, SkyColorSet, default_sky_colors, interpolate_colors, load_light_data,
};

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
    let default_colors = default_sky_colors();
    commands.entity(camera_entity).insert(DistanceFog {
        color: default_colors.sky_smog,
        directional_light_color: default_colors.sky_band2,
        directional_light_exponent: 8.0,
        falloff: FogFalloff::ExponentialSquared { density: 0.0008 },
    });
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

#[allow(clippy::too_many_arguments)]
fn update_sky_colors(
    game_time: Res<GameTime>,
    keyframes: Res<LightKeyframes>,
    sky_dome_q: Query<&MeshMaterial3d<SkyMaterial>, With<SkyDome>>,
    mut sky_materials: ResMut<Assets<SkyMaterial>>,
    mut dir_lights: Query<&mut DirectionalLight>,
    mut ambient_q: Query<&mut AmbientLight>,
    mut water_materials: ResMut<Assets<crate::water_material::WaterMaterial>>,
    mut last_minutes: Local<f32>,
) {
    if (game_time.minutes - *last_minutes).abs() < 0.01 {
        return;
    }
    *last_minutes = game_time.minutes;
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
// Sun direction
// ---------------------------------------------------------------------------

fn sun_elevation(minutes: f32) -> f32 {
    (minutes / 2880.0 * TAU - FRAC_PI_2).sin()
}

fn sun_rotation(minutes: f32) -> Quat {
    let pitch = FRAC_PI_2 - (minutes / 2880.0) * TAU;
    Quat::from_rotation_y(0.3) * Quat::from_rotation_x(pitch)
}

fn update_sun_direction(
    game_time: Res<GameTime>,
    mut dir_lights: Query<(&mut Transform, &mut DirectionalLight)>,
    mut last_minutes: Local<f32>,
) {
    if (game_time.minutes - *last_minutes).abs() < 0.01 {
        return;
    }
    *last_minutes = game_time.minutes;
    let elev = sun_elevation(game_time.minutes);
    let rotation = sun_rotation(game_time.minutes);
    let intensity = if elev > 0.0 {
        light_consts::lux::OVERCAST_DAY * elev.sqrt()
    } else {
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
    mut last_minutes: Local<f32>,
) {
    if (game_time.minutes - *last_minutes).abs() < 0.01 {
        return;
    }
    *last_minutes = game_time.minutes;
    let colors = interpolate_colors(&keyframes.0, game_time.minutes);
    for mut fog in fog_q.iter_mut() {
        fog.color = colors.sky_smog;
        fog.directional_light_color = colors.sky_band2;
    }
}

// ---------------------------------------------------------------------------
// Environment map (IBL) from sky gradient
// ---------------------------------------------------------------------------

const ENV_MAP_SIZE: u32 = 32;

#[derive(Resource)]
struct SkyEnvMapHandle(Handle<Image>);

fn build_sky_cubemap(colors: &SkyColorSet) -> Image {
    let face_pixels = (ENV_MAP_SIZE * ENV_MAP_SIZE) as usize;
    let total_bytes = face_pixels * 6 * 8;
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

fn fill_cubemap_face(data: &mut [u8], face: u32, colors: &SkyColorSet) {
    for y in 0..ENV_MAP_SIZE {
        for x in 0..ENV_MAP_SIZE {
            let dir = cubemap_direction(face, x, y);
            let elev = dir.y.asin() / FRAC_PI_2;
            let color = sample_sky_gradient(colors, elev);
            let pixel_offset = ((y * ENV_MAP_SIZE + x) as usize) * 8;
            write_rgba16f(&mut data[pixel_offset..pixel_offset + 8], color);
        }
    }
}

fn cubemap_direction(face: u32, x: u32, y: u32) -> Vec3 {
    let u = (x as f32 + 0.5) / ENV_MAP_SIZE as f32 * 2.0 - 1.0;
    let v = (y as f32 + 0.5) / ENV_MAP_SIZE as f32 * 2.0 - 1.0;
    let dir = match face {
        0 => Vec3::new(1.0, -v, -u),
        1 => Vec3::new(-1.0, -v, u),
        2 => Vec3::new(u, 1.0, v),
        3 => Vec3::new(u, -1.0, -v),
        4 => Vec3::new(u, -v, 1.0),
        _ => Vec3::new(-u, -v, -1.0),
    };
    dir.normalize()
}

fn sample_sky_gradient(colors: &SkyColorSet, elev: f32) -> LinearRgba {
    let elev = elev.clamp(-0.1, 1.0);
    let normalized = ((elev + 0.1) / 1.1).clamp(0.0, 1.0);
    let (a, b, t) = if normalized < 0.1 {
        (colors.sky_smog, colors.sky_smog, 0.0)
    } else if normalized < 0.3 {
        (colors.sky_smog, colors.sky_band2, (normalized - 0.1) / 0.2)
    } else if normalized < 0.5 {
        (colors.sky_band2, colors.sky_band1, (normalized - 0.3) / 0.2)
    } else if normalized < 0.7 {
        (
            colors.sky_band1,
            colors.sky_middle,
            (normalized - 0.5) / 0.2,
        )
    } else {
        (colors.sky_middle, colors.sky_top, (normalized - 0.7) / 0.3)
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

fn write_rgba16f(dst: &mut [u8], c: LinearRgba) {
    let vals = [c.red, c.green, c.blue, c.alpha];
    for (i, &v) in vals.iter().enumerate() {
        let h = half::f16::from_f32(v);
        let bytes = h.to_le_bytes();
        dst[i * 2] = bytes[0];
        dst[i * 2 + 1] = bytes[1];
    }
}

fn update_sky_env_map(
    game_time: Res<GameTime>,
    keyframes: Res<LightKeyframes>,
    env_handle: Option<Res<SkyEnvMapHandle>>,
    mut images: ResMut<Assets<Image>>,
    mut last: Local<f32>,
) {
    let Some(handle) = env_handle else { return };
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

fn update_time_display(
    game_time: Res<GameTime>,
    mut query: Query<&mut Text, With<TimeDisplay>>,
    mut last_minutes: Local<f32>,
) {
    if (game_time.minutes - *last_minutes).abs() < 0.5 {
        return;
    }
    *last_minutes = game_time.minutes;
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

fn sky_scene_active(state: Res<State<GameState>>) -> bool {
    matches!(state.get(), GameState::InWorld | GameState::CharSelect)
}

fn register_inworld_systems(app: &mut App) {
    let iw = in_state(GameState::InWorld);
    app.add_systems(Update, advance_game_time.run_if(iw.clone()));
    register_sky_visual_systems(app);
    app.add_systems(Update, time_speed_controls.run_if(iw));
}

fn register_sky_visual_systems(app: &mut App) {
    let sky_active = sky_scene_active;
    let iw = in_state(GameState::InWorld);
    app.add_systems(
        Update,
        update_sky_colors
            .after(advance_game_time)
            .run_if(sky_active),
    )
    .add_systems(
        Update,
        update_sun_direction
            .after(advance_game_time)
            .run_if(sky_scene_active),
    )
    .add_systems(
        Update,
        update_fog.after(advance_game_time).run_if(sky_scene_active),
    )
    .add_systems(
        Update,
        update_sky_env_map
            .after(advance_game_time)
            .run_if(sky_scene_active),
    )
    .add_systems(
        Update,
        update_time_display.after(advance_game_time).run_if(iw),
    );
}

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
            .add_systems(OnEnter(GameState::InWorld), show_time_display)
            .add_systems(OnExit(GameState::InWorld), hide_time_display);
        register_inworld_systems(app);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_time_to_clock() {
        assert_eq!(format_game_clock(1440.0), "12:00");
        assert_eq!(format_game_clock(720.0), "06:00");
        assert_eq!(format_game_clock(0.0), "00:00");
        assert_eq!(format_game_clock(2880.0), "00:00");
        assert_eq!(format_game_clock(2160.0), "18:00");
        assert_eq!(format_game_clock(780.0), "06:30");
    }
}
