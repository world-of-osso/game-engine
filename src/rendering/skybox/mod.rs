use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, TAU};

use bevy::asset::RenderAssetUsages;
use bevy::ecs::system::SystemParam;
use bevy::pbr::{DistanceFog, FogFalloff, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
};

use super::weather::{ActiveWeather, weather_adjusted_fog};
use crate::game_state::GameState;
use crate::light_lookup::WeightedLightParams;
use crate::retail_light::RetailSceneLight;
use crate::scenes::char_select::scene::CharSelectScene;
use crate::sky_lightdata::{
    LightDataRow, SkyColorSet, default_sky_colors, load_light_data, sample_light_blend,
};
use game_engine::ui::frame::WidgetData;
use game_engine::ui::plugin::UiState;
use game_engine::ui::screens::inworld_hud_component::MINIMAP_CLOCK;

#[cfg(test)]
#[path = "tests/cloud_sampling_gpu.rs"]
mod cloud_sampling_gpu_tests;
#[path = "cloud_texture.rs"]
pub mod cloud_texture;
mod inworld_skybox;
#[path = "sky_cubemap_data.rs"]
mod sky_cubemap_data;
mod sky_gradient;

use self::inworld_skybox::{
    sync_inworld_authored_skybox, sync_inworld_skybox_to_camera, teardown_inworld_skybox,
    update_inworld_light_blend, update_inworld_skybox_transition,
};
use cloud_texture::create_procedural_cloud_maps;
use sky_gradient::{SKY_BAND_MAX, SkyDomePoint, sky_dome_profile};

pub use crate::sky_material::{SkyMaterial, SkyUniforms};

/// Environmental directional light owned by sky color and time-of-day updates.
#[derive(Component)]
pub struct SkySun;

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

const LIGHT_DATA_PATH: &str = "data/LightData.ron";

/// Azeroth's global Light row 1, clear slot: the sky light for screens without
/// a world position (character select, skybox debug) and before InWorld's first
/// position lookup.
const PRE_WORLD_LIGHT_PARAMS_ID: u32 = 12;

/// LightData keyframes per LightParams and the weighted LightParams the sky and
/// fog currently blend.
#[derive(Resource)]
struct LightKeyframes {
    rows_by_param: HashMap<u32, Vec<LightDataRow>>,
    blend: Vec<WeightedLightParams>,
}

impl LightKeyframes {
    fn for_params(light_params_id: u32, rows: Vec<LightDataRow>) -> Self {
        Self {
            rows_by_param: HashMap::from([(light_params_id, rows)]),
            blend: vec![WeightedLightParams {
                light_params_id,
                weight: 1.0,
            }],
        }
    }

    fn sample(&self, minutes: f32) -> SkyColorSet {
        sample_light_blend(&self.rows_by_param, &self.blend, minutes)
    }

    /// Switch to `blend`, loading keyframes for LightParams seen for the first time.
    fn set_blend(&mut self, blend: Vec<WeightedLightParams>) {
        for light in &blend {
            self.rows_by_param
                .entry(light.light_params_id)
                .or_insert_with(|| {
                    let rows = load_light_data(LIGHT_DATA_PATH, light.light_params_id);
                    if rows.is_empty() {
                        warn!(
                            "No LightData keyframes for LightParams {}",
                            light.light_params_id
                        );
                    }
                    rows
                });
        }
        self.blend = blend;
    }
}

/// Dome radius in yards. The lowered client dome reaches 1.71 radii below the eye,
/// inside the 900-yard extent of the previous dome and the default far plane.
pub(crate) const SKY_DOME_RADIUS: f32 = 500.0;

/// Compute vertices for one ring of the sky dome. UV.y carries the ring's band
/// coordinate (normalized), which the sky shader maps to LightData colour stops.
fn push_ring(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    radius: f32,
    lon_segments: u32,
    point: SkyDomePoint,
    band: f32,
) {
    for lon in 0..=lon_segments {
        let u = lon as f32 / lon_segments as f32;
        let phi = TAU * u;
        let position = Vec3::new(
            point.horizontal * phi.cos(),
            point.height,
            point.horizontal * phi.sin(),
        ) * radius;
        positions.push(position.to_array());
        normals.push((-position.normalize()).to_array());
        uvs.push([u, band / SKY_BAND_MAX]);
    }
}

/// Generate triangle indices for the dome grid (reversed winding for inside-out).
fn build_dome_indices(lon_segments: u32, lat_segments: u32) -> Vec<u32> {
    let mut indices = Vec::new();
    for lat in 0..lat_segments {
        for lon in 0..lon_segments {
            let a = lat * (lon_segments + 1) + lon;
            let b = a + lon_segments + 1;
            indices.extend_from_slice(&[a, b, a + 1, b, b + 1, a + 1]);
        }
    }
    indices
}

/// Build the inverted client sky dome (two poles, five rings concentrated at the
/// horizon), viewed from inside.
fn build_sky_dome_mesh(radius: f32, lon_segments: u32) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let profile = sky_dome_profile();
    let lat_segments = (profile.len() - 1) as u32;
    for (band, point) in profile.into_iter().enumerate() {
        push_ring(
            &mut positions,
            &mut normals,
            &mut uvs,
            radius,
            lon_segments,
            point,
            band as f32,
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

fn fog_falloff_from_colors(colors: &SkyColorSet) -> FogFalloff {
    let end = colors.fog_end.max(1.0);
    let start = colors.fog_start.clamp(0.0, end - 0.001);
    FogFalloff::Linear { start, end }
}

/// Spawn the sky dome as a child of the camera entity and set up fog and the
/// sky cubemap.
pub fn spawn_sky_dome(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    sky_materials: &mut Assets<SkyMaterial>,
    images: &mut Assets<Image>,
    camera_entity: Entity,
    cloud_texture: Handle<Image>,
) -> Entity {
    let dome = spawn_sky_dome_entity(
        commands,
        meshes,
        sky_materials,
        camera_entity,
        cloud_texture,
    );
    let default_colors = default_sky_colors();
    insert_default_sky_fog(commands, camera_entity, &default_colors);
    insert_sky_env_map(commands, images, &default_colors);
    dome
}

pub(crate) fn spawn_sky_dome_entity(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    sky_materials: &mut Assets<SkyMaterial>,
    camera_entity: Entity,
    cloud_texture: Handle<Image>,
) -> Entity {
    let mesh = build_sky_dome_mesh(SKY_DOME_RADIUS, 32);
    let material = sky_materials.add(SkyMaterial {
        uniforms: SkyUniforms::default(),
        cloud_texture,
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
    commands.entity(camera_entity).add_child(dome);
    dome
}

fn insert_default_sky_fog(commands: &mut Commands, camera_entity: Entity, colors: &SkyColorSet) {
    // SkyFogColor, the same colour as the dome's horizon ring; LightData
    // authors no sun glow for the world fog.
    commands.entity(camera_entity).insert(DistanceFog {
        color: colors.fog_color,
        directional_light_color: Color::NONE,
        directional_light_exponent: 8.0,
        falloff: fog_falloff_from_colors(colors),
    });
}

/// Sky gradient cubemap sampled by terrain layers with cube-map reflection.
pub(crate) fn insert_sky_env_map(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    colors: &SkyColorSet,
) {
    let cubemap_handle = images.add(build_sky_cubemap(colors));
    commands.insert_resource(SkyEnvMapHandle(cubemap_handle));
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Create the sky cubemap once the current scene's camera is active: the world camera in
/// world, the character-select scene camera at character select.
fn initialize_sky_env_map(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    game_time: Res<GameTime>,
    keyframes: Res<LightKeyframes>,
    env_map: Option<Res<SkyEnvMapHandle>>,
    state: Res<State<GameState>>,
    world_cameras: Query<&Camera, With<crate::camera::WowCamera>>,
    char_select_cameras: Query<&Camera, With<CharSelectScene>>,
) {
    let active = |camera: &Camera| camera.is_active;
    let scene_camera_active = match state.get() {
        GameState::InWorld => world_cameras.iter().any(active),
        GameState::CharSelect => char_select_cameras.iter().any(active),
        _ => false,
    };
    if env_map.is_some() || !scene_camera_active {
        return;
    }
    let colors = keyframes.sample(game_time.minutes);
    insert_sky_env_map(&mut commands, &mut images, &colors);
}

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

#[derive(SystemParam)]
struct SkyVisualParams<'w, 's> {
    sky_dome_q: Query<'w, 's, Ref<'static, MeshMaterial3d<SkyMaterial>>, With<SkyDome>>,
    sky_materials: ResMut<'w, Assets<SkyMaterial>>,
    water_materials: ResMut<'w, Assets<crate::water_material::WaterMaterial>>,
}

fn update_sky_colors(
    game_time: Res<GameTime>,
    keyframes: Res<LightKeyframes>,
    mut visuals: SkyVisualParams,
    mut last_minutes: Local<f32>,
) {
    let has_new_dome = visuals
        .sky_dome_q
        .iter()
        .any(|material| material.is_added());
    let time_changed = (game_time.minutes - *last_minutes).abs() >= 0.01;
    let needs_update = time_changed || keyframes.is_changed() || has_new_dome;
    if !needs_update {
        return;
    }
    *last_minutes = game_time.minutes;
    let colors = keyframes.sample(game_time.minutes);
    update_sky_dome_material(
        &visuals.sky_dome_q,
        &mut visuals.sky_materials,
        &colors,
        game_time.minutes,
    );
    sync_water_sky_color(&mut visuals.water_materials, &colors);
}

fn update_sky_dome_material(
    sky_dome_q: &Query<Ref<MeshMaterial3d<SkyMaterial>>, With<SkyDome>>,
    sky_materials: &mut Assets<SkyMaterial>,
    colors: &SkyColorSet,
    minutes: f32,
) {
    let sun_direction = (sun_rotation(minutes) * Vec3::NEG_Z).normalize_or_zero();
    let cloud_scroll = Vec2::new(minutes * 0.00012, minutes * 0.00004);
    for mat_handle in sky_dome_q.iter() {
        if let Some(mut mat) = sky_materials.get_mut(&mat_handle.0) {
            write_sky_gradient_uniforms(&mut mat.uniforms, colors);
            mat.uniforms.sun_color = color_to_vec4(colors.sun_color);
            mat.uniforms.sun_halo_color = color_to_vec4(colors.sun_halo_color);
            mat.uniforms.cloud_emissive_color = color_to_vec4(colors.cloud_emissive_color);
            mat.uniforms.cloud_layer1_ambient_color =
                color_to_vec4(colors.cloud_layer1_ambient_color);
            mat.uniforms.cloud_layer2_ambient_color =
                color_to_vec4(colors.cloud_layer2_ambient_color);
            mat.uniforms.sun_direction =
                Vec4::new(sun_direction.x, sun_direction.y, sun_direction.z, 0.0);
            mat.uniforms.cloud_params =
                Vec4::new(colors.cloud_density, cloud_scroll.x, cloud_scroll.y, 0.0);
        }
    }
}

fn write_sky_gradient_uniforms(uniforms: &mut SkyUniforms, colors: &SkyColorSet) {
    uniforms.sky_top = color_to_vec4(colors.sky_top);
    uniforms.sky_middle = color_to_vec4(colors.sky_middle);
    uniforms.sky_band1 = color_to_vec4(colors.sky_band1);
    uniforms.sky_band2 = color_to_vec4(colors.sky_band2);
    uniforms.sky_smog = color_to_vec4(colors.sky_smog);
    uniforms.sky_fog = color_to_vec4(colors.fog_color);
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

fn init_procedural_cloud_maps(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(create_procedural_cloud_maps(&mut images));
}

// ---------------------------------------------------------------------------
// Sun direction
// ---------------------------------------------------------------------------

fn sun_rotation(minutes: f32) -> Quat {
    let pitch = FRAC_PI_2 - (minutes / 2880.0) * TAU;
    Quat::from_rotation_y(0.3) * Quat::from_rotation_x(pitch)
}

/// Point the SkySun shadow caster along the Retail sun direction. Its colour and
/// illuminance shade nothing: world materials read `RetailSceneLight`.
fn update_sun_direction(
    scene_light: Res<RetailSceneLight>,
    mut suns: Query<&mut Transform, With<SkySun>>,
    new_suns: Query<Entity, Added<SkySun>>,
) {
    if !scene_light.is_changed() && new_suns.is_empty() {
        return;
    }
    let rotation = Quat::from_rotation_arc(Vec3::NEG_Z, scene_light.sun_direction.normalize());
    for mut transform in &mut suns {
        transform.rotation = rotation;
    }
}

// ---------------------------------------------------------------------------
// Distance fog
// ---------------------------------------------------------------------------

/// Sample the light blend into the Retail scene light and the world cameras'
/// `DistanceFog` (the fog source the Retail shaders read per camera).
fn update_scene_light(
    game_time: Res<GameTime>,
    keyframes: Res<LightKeyframes>,
    weather: Option<Res<ActiveWeather>>,
    mut scene_light: ResMut<RetailSceneLight>,
    mut fog_q: Query<&mut DistanceFog, Without<CharSelectScene>>,
    mut last_minutes: Local<f32>,
) {
    let weather_changed = weather.as_ref().is_some_and(|weather| weather.is_changed());
    if (game_time.minutes - *last_minutes).abs() < 0.01
        && !weather_changed
        && !keyframes.is_changed()
    {
        return;
    }
    *last_minutes = game_time.minutes;
    let colors = keyframes.sample(game_time.minutes);
    let (fog_color, directional_color, falloff) = weather_adjusted_fog(&colors, weather.as_deref());
    let mut light = RetailSceneLight::from_sky_colors(&colors, game_time.minutes);
    let srgba = fog_color.to_srgba();
    light.fog_color = Vec3::new(srgba.red, srgba.green, srgba.blue);
    if let FogFalloff::Linear { start, end } = falloff {
        light.fog_start = start;
        light.fog_end = end;
    }
    scene_light.set_if_neq(light);
    for mut fog in fog_q.iter_mut() {
        fog.color = fog_color;
        fog.directional_light_color = directional_color;
        fog.falloff = falloff.clone();
    }
}

// ---------------------------------------------------------------------------
// Environment map (IBL) from sky gradient
// ---------------------------------------------------------------------------

const ENV_MAP_SIZE: u32 = sky_cubemap_data::ENV_MAP_SIZE;

#[derive(Resource, Clone)]
pub(crate) struct SkyEnvMapHandle(pub Handle<Image>);

pub(crate) fn build_sky_cubemap(colors: &SkyColorSet) -> Image {
    let data = sky_cubemap_data::build_sky_cubemap(&sky_gradient::linear_sky_stops(colors));
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

#[cfg(test)]
fn cubemap_direction(face: u32, x: u32, y: u32) -> Vec3 {
    Vec3::from_array(sky_cubemap_data::cubemap_direction(face, x, y))
}

fn update_sky_env_map(
    game_time: Res<GameTime>,
    keyframes: Res<LightKeyframes>,
    env_handle: Option<Res<SkyEnvMapHandle>>,
    mut images: ResMut<Assets<Image>>,
    mut last: Local<f32>,
) {
    let Some(handle) = env_handle else { return };
    if (game_time.minutes - *last).abs() < 1.0 && !keyframes.is_changed() {
        return;
    }
    *last = game_time.minutes;
    let colors = keyframes.sample(game_time.minutes);
    if let Some(mut image) = images.get_mut(&handle.0) {
        *image = build_sky_cubemap(&colors);
    }
}

// ---------------------------------------------------------------------------
// Time display systems
// ---------------------------------------------------------------------------

/// Convert GameTime minutes (0–2880) to HH:MM clock string.
fn format_game_clock(total: f32) -> String {
    let m = total.rem_euclid(2880.0);
    let hours = (m / 120.0) as u32 % 24;
    let mins = ((m % 120.0) / 2.0) as u32;
    format!("{hours:02}:{mins:02}")
}

/// Game clock text in the minimap cluster (`MinimapClock`).
fn update_time_display(
    game_time: Res<GameTime>,
    mut ui: ResMut<UiState>,
    mut shown: Local<Option<String>>,
) {
    let clock = format_game_clock(game_time.minutes);
    if shown.as_deref() == Some(clock.as_str()) {
        return;
    }
    let Some(id) = ui.registry.get_by_name(MINIMAP_CLOCK.0) else {
        return;
    };
    if let Some(frame) = ui.registry.get_mut(id)
        && let Some(WidgetData::FontString(text)) = &mut frame.widget_data
    {
        text.text = clock.clone();
        *shown = Some(clock);
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

#[derive(Resource)]
pub(crate) struct SkyboxVisualsDisabled;

fn skybox_visuals_enabled(disabled: Option<Res<SkyboxVisualsDisabled>>) -> bool {
    disabled.is_none()
}

fn remove_disabled_sky_domes(
    mut commands: Commands,
    disabled: Option<Res<SkyboxVisualsDisabled>>,
    domes: Query<Entity, With<SkyDome>>,
) {
    if disabled.is_none() {
        return;
    }
    for dome in &domes {
        commands.entity(dome).despawn();
    }
}

fn sky_scene_active(state: Res<State<GameState>>) -> bool {
    matches!(
        state.get(),
        GameState::InWorld | GameState::CharSelect | GameState::SkyboxDebug
    )
}

fn register_inworld_systems(app: &mut App) {
    let iw = in_state(GameState::InWorld);
    app.add_systems(
        Update,
        (advance_game_time, time_speed_controls)
            .run_if(iw.clone())
            .run_if(crate::game::inworld_scene_stage::inworld_scene_stage_allows_skybox),
    );
    register_sky_visual_systems(app);
    app.add_systems(
        Update,
        update_inworld_light_blend
            .before(update_sky_colors)
            .before(update_scene_light)
            .before(update_sky_env_map)
            .before(initialize_sky_env_map)
            .run_if(in_state(GameState::InWorld))
            .run_if(crate::game::inworld_scene_stage::inworld_scene_stage_allows_lighting),
    );
    app.add_systems(
        Update,
        (
            sync_inworld_authored_skybox,
            update_inworld_skybox_transition.after(sync_inworld_authored_skybox),
            sync_inworld_skybox_to_camera.after(update_inworld_skybox_transition),
        )
            .run_if(iw)
            .run_if(crate::game::inworld_scene_stage::inworld_scene_stage_allows_skybox)
            .run_if(skybox_visuals_enabled),
    );
    app.add_systems(OnExit(GameState::InWorld), teardown_inworld_skybox);
}

fn register_sky_visual_systems(app: &mut App) {
    register_shared_sky_visual_systems(app);
    register_inworld_time_display_system(app);
}

fn register_shared_sky_visual_systems(app: &mut App) {
    let sky_active = sky_scene_active;
    app.add_systems(
        Update,
        initialize_sky_env_map
            .after(advance_game_time)
            .run_if(in_state(GameState::InWorld).or_else(in_state(GameState::CharSelect)))
            .run_if(crate::game::inworld_scene_stage::inworld_scene_stage_allows_lighting),
    )
    .add_systems(
        Update,
        update_sky_colors
            .after(sync_inworld_authored_skybox)
            .after(advance_game_time)
            .run_if(sky_active)
            .run_if(crate::game::inworld_scene_stage::inworld_scene_stage_allows_lighting),
    )
    .add_systems(
        Update,
        update_sun_direction
            .after(update_scene_light)
            .run_if(sky_scene_active)
            .run_if(crate::game::inworld_scene_stage::inworld_scene_stage_allows_lighting),
    )
    .add_systems(
        Update,
        update_scene_light
            .after(advance_game_time)
            .run_if(sky_scene_active)
            .run_if(crate::game::inworld_scene_stage::inworld_scene_stage_allows_lighting),
    )
    .add_systems(
        Update,
        update_sky_env_map
            .after(advance_game_time)
            .after(initialize_sky_env_map)
            .run_if(sky_scene_active)
            .run_if(crate::game::inworld_scene_stage::inworld_scene_stage_allows_lighting),
    );
}

fn register_inworld_time_display_system(app: &mut App) {
    let iw = in_state(GameState::InWorld);
    app.add_systems(
        Update,
        update_time_display
            .after(advance_game_time)
            .run_if(iw)
            .run_if(crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui),
    );
}

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        let keyframes = load_light_data(LIGHT_DATA_PATH, PRE_WORLD_LIGHT_PARAMS_ID);
        info!(
            "Loaded {} sky keyframes for LightParamID {PRE_WORLD_LIGHT_PARAMS_ID}",
            keyframes.len()
        );
        let keyframes_for_pre_world =
            LightKeyframes::for_params(PRE_WORLD_LIGHT_PARAMS_ID, keyframes);
        let empty = crate::game::inworld_scene_stage::configured_inworld_scene_stage_for_app(app)
            == crate::game::inworld_scene_stage::InWorldSceneStage::Empty;
        if empty {
            app.init_asset::<SkyMaterial>();
        } else {
            app.add_plugins(MaterialPlugin::<SkyMaterial>::default());
        }
        app.add_systems(PostUpdate, remove_disabled_sky_domes)
            .insert_resource(GameTime::default())
            .insert_resource(RetailSceneLight::from_sky_colors(
                &keyframes_for_pre_world.sample(GameTime::default().minutes),
                GameTime::default().minutes,
            ))
            .insert_resource(keyframes_for_pre_world)
            .add_systems(Startup, init_procedural_cloud_maps);
        register_inworld_systems(app);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
