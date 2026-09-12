use std::f32::consts::PI;
use std::marker::PhantomData;

use bevy::camera::ClearColorConfig;
use bevy::ecs::system::SystemParam;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use game_engine::scene_tree::{NodeProps, SceneNode, SceneTree};

use crate::camera::additive_particle_glow_tonemapping;
use crate::creature_display;
use crate::game_state::GameState;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene;
use crate::orbit_camera::OrbitCamera;
use crate::scenes::char_select::warband::{SelectedWarbandScene, WarbandScenes};
use crate::scenes::teardown::teardown_tagged_scene;
use crate::skybox_m2_material::SkyboxM2Material;
mod logging;
mod resolution;
use resolution::{ResolvedDebugSkybox, resolve_debug_skybox};

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkyboxDebugOverride {
    LightSkyboxId(u32),
    SkyboxFileDataId(u32),
}

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SkyboxDebugViewMode {
    #[default]
    Default,
    AuthoredOnlyVerification,
}

impl SkyboxDebugViewMode {
    fn shows_reference_objects(self) -> bool {
        matches!(self, Self::Default)
    }
}

#[derive(Component)]
struct SkyboxDebugScene;

#[derive(Component)]
struct SkyboxDebugSkybox;

struct SkyboxDebugSetup {
    scene: Option<crate::scenes::char_select::warband::WarbandSceneEntry>,
    focus: Vec3,
    eye: Vec3,
}

struct SpawnedSkyboxDebug {
    root: Entity,
    path: std::path::PathBuf,
    source: String,
    light_params_id: Option<u32>,
    light_params_flags: Option<crate::light_lookup::LightParamsFlags>,
    light_skybox_id: Option<u32>,
    light_skybox_flags: Option<crate::light_lookup::LightSkyboxFlags>,
}

const SKYBOX_DEBUG_CLEAR_COLOR: Color = Color::BLACK;
const SKYBOX_DEBUG_BASELINE_CLEAR_COLOR: Color = Color::srgb(0.05, 0.06, 0.08);
const SKYBOX_DEBUG_FOG_COLOR: Color = Color::srgb(0.18, 0.2, 0.23);

#[derive(Clone, Copy, Debug, PartialEq)]
struct SkyboxDebugComposition {
    clear_color: Color,
    shows_procedural_visible_baseline: bool,
    shows_procedural_fog: bool,
}

pub struct SkyboxDebugScenePlugin;

impl Plugin for SkyboxDebugScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            setup_scene_once
                .run_if(in_state(GameState::SkyboxDebug))
                .run_if(no_debug_scene_root),
        );
        app.add_systems(
            Update,
            sync_skybox_to_camera.run_if(in_state(GameState::SkyboxDebug)),
        );
        app.add_systems(
            Update,
            sync_skyboxdebug_camera_fov.run_if(in_state(GameState::SkyboxDebug)),
        );
        app.add_systems(OnExit(GameState::SkyboxDebug), teardown_scene);
    }
}

#[derive(SystemParam)]
struct SkyboxDebugSceneParams<'w, 's> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    effect_materials: ResMut<'w, Assets<M2EffectMaterial>>,
    sky_materials: ResMut<'w, Assets<crate::sky_material::SkyMaterial>>,
    skybox_materials: ResMut<'w, Assets<SkyboxM2Material>>,
    images: ResMut<'w, Assets<Image>>,
    cloud_maps: Option<Res<'w, crate::sky::cloud_texture::ProceduralCloudMaps>>,
    inv_bp: ResMut<'w, Assets<SkinnedMeshInverseBindposes>>,
    creature_display_map: Res<'w, creature_display::CreatureDisplayMap>,
    camera_options: Res<'w, crate::client_options::CameraOptions>,
    warband: Res<'w, WarbandScenes>,
    selected_scene: Option<Res<'w, SelectedWarbandScene>>,
    override_spec: Option<Res<'w, SkyboxDebugOverride>>,
    view_mode: Option<Res<'w, SkyboxDebugViewMode>>,
    marker: PhantomData<&'s ()>,
}

fn setup_scene(mut commands: Commands, mut params: SkyboxDebugSceneParams) {
    let view_mode = skybox_debug_view_mode(&params);
    let setup = build_skybox_debug_setup(&params);
    let resolved = resolve_debug_skybox(
        setup.scene.as_ref(),
        params.override_spec.as_deref().copied(),
    );
    let composition = skybox_debug_composition(view_mode, resolved.as_ref());
    initialize_skybox_debug_scene(&mut commands, &mut params, &setup, composition);
    spawn_skybox_debug_reference_objects(
        &mut commands,
        &mut params.meshes,
        &mut params.materials,
        &mut params.images,
        view_mode,
    );
    let Some(resolved) = resolved else {
        warn_missing_debug_skybox(setup.scene.as_ref());
        return;
    };
    let Some(spawned) = spawn_resolved_debug_skybox(&mut commands, &mut params, &setup, &resolved)
    else {
        return;
    };
    tag_debug_skybox_scene_entities(&mut commands, &resolved, &spawned);
    let spawned = build_spawned_debug_skybox(resolved, spawned);
    logging::log_debug_skybox_spawn(&setup, &spawned);
    insert_skybox_debug_scene_tree(&mut commands, spawned, params.camera_options.fov_degrees);
}

fn no_debug_scene_root(query: Query<Entity, With<SkyboxDebugScene>>) -> bool {
    query.is_empty()
}

fn setup_scene_once(commands: Commands, params: SkyboxDebugSceneParams) {
    setup_scene(commands, params);
}

fn skybox_debug_view_mode(params: &SkyboxDebugSceneParams<'_, '_>) -> SkyboxDebugViewMode {
    params.view_mode.as_deref().copied().unwrap_or_default()
}

fn build_skybox_debug_setup(params: &SkyboxDebugSceneParams<'_, '_>) -> SkyboxDebugSetup {
    let scene = params
        .selected_scene
        .as_ref()
        .and_then(|selected| {
            params
                .warband
                .scenes
                .iter()
                .find(|scene| scene.id == selected.scene_id)
        })
        .or_else(|| params.warband.scenes.first())
        .cloned();
    let focus = Vec3::new(0.0, 1.0, 0.0);
    let orbit = OrbitCamera::new(focus, 7.5);
    SkyboxDebugSetup {
        scene,
        focus,
        eye: orbit.eye_position(),
    }
}

fn initialize_skybox_debug_scene(
    commands: &mut Commands,
    params: &mut SkyboxDebugSceneParams<'_, '_>,
    setup: &SkyboxDebugSetup,
    composition: SkyboxDebugComposition,
) {
    let cloud_texture =
        ensure_debug_cloud_texture(commands, &mut params.images, params.cloud_maps.as_deref());
    spawn_debug_scene_environment(
        commands,
        &mut params.meshes,
        &mut params.sky_materials,
        &mut params.images,
        cloud_texture,
        setup,
        params.camera_options.fov_degrees,
        composition,
    );
    spawn_skybox_debug_light(commands);
}

fn skybox_debug_composition(
    view_mode: SkyboxDebugViewMode,
    resolved: Option<&ResolvedDebugSkybox>,
) -> SkyboxDebugComposition {
    match view_mode {
        SkyboxDebugViewMode::AuthoredOnlyVerification => authored_only_debug_composition(),
        SkyboxDebugViewMode::Default => default_debug_composition(resolved),
    }
}

fn authored_only_debug_composition() -> SkyboxDebugComposition {
    SkyboxDebugComposition {
        clear_color: SKYBOX_DEBUG_CLEAR_COLOR,
        shows_procedural_visible_baseline: false,
        shows_procedural_fog: false,
    }
}

fn default_debug_composition(resolved: Option<&ResolvedDebugSkybox>) -> SkyboxDebugComposition {
    let light_skybox_flags = resolved.and_then(|resolved| resolved.light_skybox_flags);
    let light_params_flags = resolved.and_then(|resolved| resolved.light_params_flags);
    let shows_procedural_visible_baseline =
        procedural_baseline_enabled(light_skybox_flags, light_params_flags);
    let shows_procedural_fog = procedural_fog_enabled(light_skybox_flags, light_params_flags);
    SkyboxDebugComposition {
        clear_color: baseline_clear_color(shows_procedural_visible_baseline),
        shows_procedural_visible_baseline,
        shows_procedural_fog,
    }
}

fn procedural_baseline_enabled(
    light_skybox_flags: Option<crate::light_lookup::LightSkyboxFlags>,
    light_params_flags: Option<crate::light_lookup::LightParamsFlags>,
) -> bool {
    let skybox_blend_enabled = light_skybox_flags
        .map(|flags| {
            flags.contains(crate::light_lookup::LightSkyboxFlags::COMBINE_PROCEDURAL_AND_SKYBOX)
        })
        .unwrap_or(true);
    let hides_celestial_baseline = light_params_flags
        .map(light_params_suppresses_celestial_visibility)
        .unwrap_or(false);
    skybox_blend_enabled && !hides_celestial_baseline
}

fn procedural_fog_enabled(
    light_skybox_flags: Option<crate::light_lookup::LightSkyboxFlags>,
    light_params_flags: Option<crate::light_lookup::LightParamsFlags>,
) -> bool {
    let skybox_fog_blend_enabled = light_skybox_flags
        .map(|flags| {
            flags.contains(crate::light_lookup::LightSkyboxFlags::PROCEDURAL_FOG_COLOR_BLEND)
        })
        .unwrap_or(true);
    let height_fog_enabled = light_params_flags
        .map(|flags| flags.contains(crate::light_lookup::LightParamsFlags::HEIGHT_FOG_ABOVE_PLANE))
        .unwrap_or(false);
    skybox_fog_blend_enabled || height_fog_enabled
}

fn baseline_clear_color(shows_procedural_visible_baseline: bool) -> Color {
    if shows_procedural_visible_baseline {
        SKYBOX_DEBUG_BASELINE_CLEAR_COLOR
    } else {
        SKYBOX_DEBUG_CLEAR_COLOR
    }
}

fn light_params_suppresses_celestial_visibility(
    flags: crate::light_lookup::LightParamsFlags,
) -> bool {
    flags.contains(crate::light_lookup::LightParamsFlags::DONT_INHERIT_SKYBOX)
        || flags.contains(crate::light_lookup::LightParamsFlags::HIDE_SUN)
        || flags.contains(crate::light_lookup::LightParamsFlags::HIDE_MOON)
        || flags.contains(crate::light_lookup::LightParamsFlags::HIDE_STARS)
        || flags.contains(crate::light_lookup::LightParamsFlags::OVERRIDE_CELESTIAL_SPHERE)
        || flags.contains(crate::light_lookup::LightParamsFlags::HIDE_CELESTIAL_OBJECT)
}

fn spawn_skybox_debug_light(commands: &mut Commands) {
    commands.spawn((
        Name::new("SkyboxDebugLight"),
        crate::sky::SkySun,
        SkyboxDebugScene,
        DirectionalLight {
            illuminance: 2500.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 5.0, PI / 6.0, 0.0)),
    ));
}

fn ensure_debug_cloud_texture(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    cloud_maps: Option<&crate::sky::cloud_texture::ProceduralCloudMaps>,
) -> Handle<Image> {
    if let Some(cloud_maps) = cloud_maps {
        return cloud_maps.active_handle();
    }
    let cloud_maps = crate::sky::cloud_texture::create_procedural_cloud_maps(images);
    let active = cloud_maps.active_handle();
    commands.insert_resource(cloud_maps);
    active
}

fn spawn_debug_scene_environment(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    sky_materials: &mut Assets<crate::sky_material::SkyMaterial>,
    images: &mut Assets<Image>,
    cloud_texture: Handle<Image>,
    setup: &SkyboxDebugSetup,
    fov_degrees: f32,
    composition: SkyboxDebugComposition,
) -> Entity {
    insert_debug_scene_environment_resources(commands, composition);
    let camera = spawn_debug_scene_camera(commands, setup, fov_degrees, composition);
    if composition.shows_procedural_visible_baseline {
        let dome = crate::sky::spawn_sky_dome_entity(
            commands,
            meshes,
            sky_materials,
            camera,
            cloud_texture,
        );
        commands.entity(dome).insert(SkyboxDebugScene);
    }
    insert_debug_scene_env_map(commands, images);
    camera
}

fn insert_debug_scene_environment_resources(
    commands: &mut Commands,
    composition: SkyboxDebugComposition,
) {
    commands.insert_resource(ClearColor(composition.clear_color));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 60.0,
        ..default()
    });
}

fn spawn_debug_scene_camera(
    commands: &mut Commands,
    setup: &SkyboxDebugSetup,
    fov_degrees: f32,
    composition: SkyboxDebugComposition,
) -> Entity {
    let mut camera = commands.spawn(debug_scene_camera_bundle(setup, fov_degrees, composition));
    if composition.shows_procedural_fog {
        camera.insert(debug_scene_fog());
    }
    camera.id()
}

fn debug_scene_camera_bundle(
    setup: &SkyboxDebugSetup,
    fov_degrees: f32,
    composition: SkyboxDebugComposition,
) -> impl Bundle {
    let orbit = OrbitCamera::new(setup.focus, 7.5);
    (
        Name::new("SkyboxDebugCamera"),
        SkyboxDebugScene,
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(composition.clear_color),
            ..default()
        },
        additive_particle_glow_tonemapping(),
        Projection::Perspective(PerspectiveProjection {
            fov: fov_degrees.to_radians(),
            ..default()
        }),
        Transform::from_translation(setup.eye).looking_at(setup.focus, Vec3::Y),
        orbit,
    )
}

fn debug_scene_fog() -> DistanceFog {
    DistanceFog {
        color: SKYBOX_DEBUG_FOG_COLOR,
        falloff: FogFalloff::Linear {
            start: 15.0,
            end: 45.0,
        },
        ..default()
    }
}

fn insert_debug_scene_env_map(commands: &mut Commands, images: &mut Assets<Image>) {
    let colors = crate::sky_lightdata::default_sky_colors();
    let cubemap_handle = images.add(crate::sky::build_sky_cubemap(&colors));
    commands.insert_resource(crate::sky::SkyEnvMapHandle(cubemap_handle));
}

fn spawn_skybox_debug_reference_objects(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    view_mode: SkyboxDebugViewMode,
) {
    if !should_spawn_skybox_debug_reference_objects(view_mode) {
        return;
    }
    spawn_default_skybox_debug_reference_objects(commands, meshes, materials, images);
}

fn should_spawn_skybox_debug_reference_objects(view_mode: SkyboxDebugViewMode) -> bool {
    view_mode.shows_reference_objects()
}

fn spawn_default_skybox_debug_reference_objects(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) {
    spawn_debug_reference_plane(commands, meshes, materials, images);
}

fn spawn_debug_reference_plane(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) {
    let ground = crate::ground::spawn_ground_plane_entity(commands, meshes, materials, images);
    commands.entity(ground).insert((
        Name::new("SkyboxDebugGroundPlane"),
        SkyboxDebugScene,
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
}

fn spawn_resolved_debug_skybox(
    commands: &mut Commands,
    params: &mut SkyboxDebugSceneParams<'_, '_>,
    setup: &SkyboxDebugSetup,
    resolved: &ResolvedDebugSkybox,
) -> Option<m2_scene::SpawnedAnimatedStaticM2> {
    let mut ctx = m2_scene::M2SceneSpawnContext {
        commands,
        assets: crate::m2_spawn::SpawnAssets {
            meshes: &mut params.meshes,
            materials: &mut params.materials,
            effect_materials: &mut params.effect_materials,
            skybox_materials: Some(&mut params.skybox_materials),
            images: &mut params.images,
            inverse_bindposes: &mut params.inv_bp,
        },
        creature_display_map: &params.creature_display_map,
    };
    let spawned = m2_scene::spawn_animated_static_skybox_m2_parts(
        &mut ctx,
        &resolved.path,
        Transform::from_translation(setup.focus),
        None,
    );
    let Some(spawned) = spawned else {
        warn!(
            "skybox_debug_scene: failed to spawn skybox model at {}",
            resolved.path.display()
        );
        return None;
    };
    Some(spawned)
}

fn tag_debug_skybox_scene_entities(
    commands: &mut Commands,
    resolved: &ResolvedDebugSkybox,
    spawned: &m2_scene::SpawnedAnimatedStaticM2,
) {
    commands.entity(spawned.root).insert((
        bevy::camera::visibility::NoFrustumCulling,
        SkyboxDebugScene,
        SkyboxDebugSkybox,
        Name::new(format!("SkyboxDebug:{}", resolved.path.display())),
    ));
    commands
        .entity(spawned.model_root)
        .insert((bevy::camera::visibility::NoFrustumCulling, SkyboxDebugScene));
}

fn build_spawned_debug_skybox(
    resolved: ResolvedDebugSkybox,
    spawned: m2_scene::SpawnedAnimatedStaticM2,
) -> SpawnedSkyboxDebug {
    SpawnedSkyboxDebug {
        root: spawned.root,
        path: resolved.path,
        source: resolved.source,
        light_params_id: resolved.light_params_id,
        light_params_flags: resolved.light_params_flags,
        light_skybox_id: resolved.light_skybox_id,
        light_skybox_flags: resolved.light_skybox_flags,
    }
}

fn warn_missing_debug_skybox(
    scene: Option<&crate::scenes::char_select::warband::WarbandSceneEntry>,
) {
    match scene {
        Some(scene) => warn!(
            "skybox_debug_scene: failed to resolve skybox model for scene {} ({})",
            scene.id, scene.name
        ),
        None => warn!("skybox_debug_scene: no warband scene available for skybox selection"),
    }
}

fn insert_skybox_debug_scene_tree(
    commands: &mut Commands,
    spawned: SpawnedSkyboxDebug,
    fov_degrees: f32,
) {
    commands.insert_resource(SceneTree {
        root: build_skybox_debug_scene_root(&spawned, fov_degrees),
    });
}

fn build_skybox_debug_scene_root(spawned: &SpawnedSkyboxDebug, fov_degrees: f32) -> SceneNode {
    SceneNode {
        label: "SkyboxDebugScene".into(),
        entity: None,
        props: NodeProps::Scene,
        children: vec![camera_scene_node(fov_degrees), skybox_scene_node(spawned)],
    }
}

fn camera_scene_node(fov_degrees: f32) -> SceneNode {
    SceneNode {
        label: "Camera".into(),
        entity: None,
        props: NodeProps::Camera { fov: fov_degrees },
        children: vec![],
    }
}

fn skybox_scene_node(spawned: &SpawnedSkyboxDebug) -> SceneNode {
    SceneNode {
        label: "Skybox".into(),
        entity: Some(spawned.root),
        props: NodeProps::Object {
            kind: "Skybox".into(),
            model: spawned.path.display().to_string(),
        },
        children: vec![],
    }
}

fn sync_skybox_to_camera(
    camera_query: Query<&OrbitCamera, (With<Camera3d>, With<SkyboxDebugScene>)>,
    mut skybox_query: Query<&mut Transform, (With<SkyboxDebugSkybox>, Without<OrbitCamera>)>,
) {
    let Ok(orbit) = camera_query.single() else {
        return;
    };
    for mut transform in &mut skybox_query {
        transform.translation = orbit.focus;
    }
}

fn sync_skyboxdebug_camera_fov(
    options: Res<crate::client_options::CameraOptions>,
    mut camera_query: Query<
        &mut Projection,
        (With<Camera3d>, With<OrbitCamera>, With<SkyboxDebugScene>),
    >,
) {
    if !options.is_changed() {
        return;
    }
    for mut projection in &mut camera_query {
        if let Projection::Perspective(ref mut perspective) = *projection {
            perspective.fov = options.fov_degrees.to_radians();
        }
    }
}

fn teardown_scene(commands: Commands, query: Query<Entity, With<SkyboxDebugScene>>) {
    teardown_tagged_scene::<SkyboxDebugScene>(commands, query);
}

#[cfg(test)]
mod tests;
