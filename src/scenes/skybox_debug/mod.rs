use std::f32::consts::PI;
use std::marker::PhantomData;

use bevy::camera::ClearColorConfig;
use bevy::ecs::system::SystemParam;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use game_engine::scene_tree::{NodeProps, SceneNode, SceneTree};

use crate::asset::m2_anim::{AnimTrack, evaluate_i16_track};
use crate::camera::additive_particle_glow_tonemapping;
use crate::creature_display;
use crate::game_state::GameState;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene;
use crate::orbit_camera::OrbitCamera;
use crate::scenes::char_select::warband::{SelectedWarbandScene, WarbandScenes};
use crate::scenes::teardown::teardown_tagged_scene;
use crate::skybox_m2_material::SkyboxM2Material;
use game_engine::asset::read_bytes::fixed16_to_f32;

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
}

const SKYBOX_DEBUG_CLEAR_COLOR: Color = Color::BLACK;

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
    initialize_skybox_debug_scene(&mut commands, &mut params, &setup);
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
    log_debug_skybox_spawn(&setup, &spawned);
    insert_skybox_debug_scene_tree(&mut commands, spawned);
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
) {
    let _ = ensure_debug_cloud_texture(commands, &mut params.images, params.cloud_maps.as_deref());
    spawn_debug_scene_environment(commands, &mut params.images, setup);
    spawn_skybox_debug_light(commands);
}

fn spawn_skybox_debug_light(commands: &mut Commands) {
    commands.spawn((
        Name::new("SkyboxDebugLight"),
        SkyboxDebugScene,
        DirectionalLight {
            illuminance: 2500.0,
            shadows_enabled: false,
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
    images: &mut Assets<Image>,
    setup: &SkyboxDebugSetup,
) -> Entity {
    insert_debug_scene_environment_resources(commands);
    let camera = spawn_debug_scene_camera(commands, setup);
    insert_debug_scene_env_map(commands, images);
    camera
}

fn insert_debug_scene_environment_resources(commands: &mut Commands) {
    commands.insert_resource(ClearColor(SKYBOX_DEBUG_CLEAR_COLOR));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 60.0,
        ..default()
    });
}

fn spawn_debug_scene_camera(commands: &mut Commands, setup: &SkyboxDebugSetup) -> Entity {
    commands.spawn(debug_scene_camera_bundle(setup)).id()
}

fn debug_scene_camera_bundle(setup: &SkyboxDebugSetup) -> impl Bundle {
    let orbit = OrbitCamera::new(setup.focus, 7.5);
    (
        Name::new("SkyboxDebugCamera"),
        SkyboxDebugScene,
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(SKYBOX_DEBUG_CLEAR_COLOR),
            ..default()
        },
        additive_particle_glow_tonemapping(),
        Projection::Perspective(PerspectiveProjection {
            fov: 60.0_f32.to_radians(),
            far: 100_000.0,
            ..default()
        }),
        Transform::from_translation(setup.eye).looking_at(setup.focus, Vec3::Y),
        orbit,
    )
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
    if !view_mode.shows_reference_objects() {
        return;
    }
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

fn log_debug_skybox_spawn(setup: &SkyboxDebugSetup, spawned: &SpawnedSkyboxDebug) {
    let authored_light_params = setup
        .scene
        .as_ref()
        .and_then(|scene| scene.authored_light_params_id());
    let authored_light_skybox = setup
        .scene
        .as_ref()
        .and_then(|scene| scene.authored_light_skybox_id());
    info!(
        "skybox_debug_scene: resolved skybox {} via {} (scene={:?}, LightParamsID={:?}, LightSkyboxID={:?})",
        spawned.path.display(),
        spawned.source,
        setup
            .scene
            .as_ref()
            .map(|scene| (scene.id, scene.name.as_str())),
        authored_light_params,
        authored_light_skybox
    );
    log_debug_skybox_alpha_tracks(&spawned.path);
}

fn log_debug_skybox_alpha_tracks(path: &std::path::Path) {
    let Ok(model) = crate::asset::m2::load_skybox_m2_uncached(path, &[0, 0, 0]) else {
        warn!(
            "skybox_debug_scene: failed to load {} for alpha-track dump",
            path.display()
        );
        return;
    };
    let default_sequence_index = model
        .sequences
        .iter()
        .position(|sequence| sequence.id == 0)
        .unwrap_or(0);
    info!(
        "skybox_debug_scene: alpha dump {} transparency_tracks={} color_tracks={} batches={}",
        path.display(),
        model.transparency_tracks.len(),
        model.color_tracks.len(),
        model.batches.len()
    );
    for (track_index, track) in model.transparency_tracks.iter().enumerate() {
        info!(
            "skybox_debug_scene: transparency_track[{track_index}] {}",
            format_opacity_track_samples(track, default_sequence_index, &model.global_sequences)
        );
    }
    log_debug_skybox_additive_batches(&model);
    for (batch_index, batch) in model.batches.iter().enumerate() {
        let transparency_track = batch
            .transparency_track_index
            .map_or_else(|| "-".into(), |idx| idx.to_string());
        let color_track = batch
            .color_opacity_track_index
            .map_or_else(|| "-".into(), |idx| idx.to_string());
        let transparency_samples = batch.transparency_anim.as_ref().map_or_else(
            || "none".into(),
            |track| {
                format_opacity_track_samples(track, default_sequence_index, &model.global_sequences)
            },
        );
        let color_samples = batch.color_opacity_anim.as_ref().map_or_else(
            || "none".into(),
            |track| {
                format_opacity_track_samples(track, default_sequence_index, &model.global_sequences)
            },
        );
        info!(
            "skybox_debug_scene: batch[{batch_index}] mesh_part_id={} blend={} flags=0x{:x} priority_plane={} layer={} transparency_track={} color_opacity_track={} transparency_samples={} color_samples={}",
            batch.mesh_part_id,
            batch.blend_mode,
            batch.render_flags,
            batch.priority_plane,
            batch.material_layer,
            transparency_track,
            color_track,
            transparency_samples,
            color_samples
        );
    }
}

fn log_debug_skybox_additive_batches(model: &crate::asset::m2::M2Model) {
    let additive_batches: Vec<_> = model
        .batches
        .iter()
        .enumerate()
        .filter(|(_, batch)| batch.blend_mode == 4)
        .collect();
    info!(
        "skybox_debug_scene: additive_batches={}",
        additive_batches.len()
    );
    for (batch_index, batch) in additive_batches {
        let duplicate_count = model
            .batches
            .iter()
            .filter(|other| {
                other.blend_mode == batch.blend_mode
                    && other.priority_plane == batch.priority_plane
                    && other.material_layer == batch.material_layer
            })
            .count();
        info!(
            "skybox_debug_scene: additive_batch[{batch_index}] mesh_part_id={} flags=0x{:x} priority_plane={} layer={} shader_id=0x{:x} texture_count={} use_uv_2_1={} use_uv_2_2={} use_env_map_2={} texture_anim={} texture_anim_2={} texture_fdid={:?} texture_2_fdid={:?} extra_textures={:?} duplicates_same_priority_layer={duplicate_count}",
            batch.mesh_part_id,
            batch.render_flags,
            batch.priority_plane,
            batch.material_layer,
            batch.shader_id,
            batch.texture_count,
            batch.use_uv_2_1,
            batch.use_uv_2_2,
            batch.use_env_map_2,
            batch.texture_anim.is_some(),
            batch.texture_anim_2.is_some(),
            batch.texture_fdid,
            batch.texture_2_fdid,
            batch.extra_texture_fdids
        );
        log_additive_batch_geometry(model, batch_index, batch);
    }
}

fn log_additive_batch_geometry(
    model: &crate::asset::m2::M2Model,
    batch_index: usize,
    batch: &crate::asset::m2::M2RenderBatch,
) {
    let triangle_count = match batch.mesh.indices() {
        Some(bevy::mesh::Indices::U16(indices)) => indices.len() / 3,
        Some(bevy::mesh::Indices::U32(indices)) => indices.len() / 3,
        None => 0,
    };
    let vertex_count = batch.mesh.count_vertices();
    let mut joint_indices = std::collections::BTreeSet::new();
    if let Some(bevy::mesh::VertexAttributeValues::Uint16x4(joints)) =
        batch.mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX)
    {
        for joint_set in joints {
            for joint in joint_set {
                joint_indices.insert(*joint as usize);
            }
        }
    }
    let joint_summary = if joint_indices.is_empty() {
        "none".to_string()
    } else {
        joint_indices
            .iter()
            .map(|joint_index| {
                let billboard = model
                    .bones
                    .get(*joint_index)
                    .map(|bone| bone.flags & crate::animation::M2_BONE_SPHERICAL_BILLBOARD != 0)
                    .unwrap_or(false);
                format!("{joint_index}:billboard={billboard}")
            })
            .collect::<Vec<_>>()
            .join(",")
    };
    info!(
        "skybox_debug_scene: additive_batch[{batch_index}] geometry verts={} tris={} joints=[{}]",
        vertex_count, triangle_count, joint_summary
    );
}

fn format_opacity_track_samples(
    track: &AnimTrack<i16>,
    default_sequence_index: usize,
    global_sequences: &[u32],
) -> String {
    let seq_idx = if track.sequences.is_empty() {
        0
    } else {
        default_sequence_index.min(track.sequences.len() - 1)
    };
    let Some((timestamps, _)) = track.sequences.get(seq_idx) else {
        return format!("global_seq={} seq={} empty", track.global_sequence, seq_idx);
    };
    let duration = if track.global_sequence >= 0 {
        global_sequences
            .get(track.global_sequence as usize)
            .copied()
            .unwrap_or_else(|| timestamps.last().copied().unwrap_or(0).saturating_add(1))
    } else {
        timestamps.last().copied().unwrap_or(0).saturating_add(1)
    };
    let samples = [
        0_u32,
        duration / 4,
        duration / 2,
        duration.saturating_sub(1),
    ]
    .into_iter()
    .map(|time_ms| {
        let value = evaluate_i16_track(track, seq_idx, time_ms)
            .map(|raw| format!("{:.3}", fixed16_to_f32(raw).clamp(0.0, 1.0)))
            .unwrap_or_else(|| "none".into());
        format!("{time_ms}ms={value}")
    })
    .collect::<Vec<_>>()
    .join(",");
    format!(
        "global_seq={} seq={} keyframes={} duration={} [{}]",
        track.global_sequence,
        seq_idx,
        timestamps.len(),
        duration,
        samples
    )
}

fn insert_skybox_debug_scene_tree(commands: &mut Commands, spawned: SpawnedSkyboxDebug) {
    commands.insert_resource(SceneTree {
        root: build_skybox_debug_scene_root(&spawned),
    });
}

struct ResolvedDebugSkybox {
    path: std::path::PathBuf,
    source: String,
    light_skybox_id: Option<u32>,
    light_skybox_flags: Option<crate::light_lookup::LightSkyboxFlags>,
}

fn build_skybox_debug_scene_root(spawned: &SpawnedSkyboxDebug) -> SceneNode {
    SceneNode {
        label: "SkyboxDebugScene".into(),
        entity: None,
        props: NodeProps::Scene,
        children: vec![camera_scene_node(), skybox_scene_node(spawned)],
    }
}

fn camera_scene_node() -> SceneNode {
    SceneNode {
        label: "Camera".into(),
        entity: None,
        props: NodeProps::Camera { fov: 60.0 },
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

fn resolve_debug_skybox(
    scene: Option<&crate::scenes::char_select::warband::WarbandSceneEntry>,
    override_spec: Option<SkyboxDebugOverride>,
) -> Option<ResolvedDebugSkybox> {
    match override_spec {
        Some(SkyboxDebugOverride::LightSkyboxId(light_skybox_id)) => {
            let path = ensure_skybox_fdid(crate::light_lookup::resolve_light_skybox_fdid(
                light_skybox_id,
            )?)?;
            Some(ResolvedDebugSkybox {
                path,
                source: format!("forced LightSkyboxID={light_skybox_id}"),
                light_skybox_id: Some(light_skybox_id),
                light_skybox_flags: crate::light_lookup::resolve_light_skybox_flags(
                    light_skybox_id,
                ),
            })
        }
        Some(SkyboxDebugOverride::SkyboxFileDataId(fdid)) => {
            let path = ensure_skybox_fdid(fdid)?;
            Some(ResolvedDebugSkybox {
                path,
                source: format!("forced SkyboxFileDataID={fdid}"),
                light_skybox_id: None,
                light_skybox_flags: None,
            })
        }
        None => {
            let scene = scene?;
            let light_skybox_id = scene.authored_light_skybox_id();
            Some(ResolvedDebugSkybox {
                path: crate::scenes::char_select::warband::ensure_warband_skybox(scene)?,
                source: format!("warband scene {} ({})", scene.id, scene.name),
                light_skybox_id,
                light_skybox_flags: light_skybox_id
                    .and_then(crate::light_lookup::resolve_light_skybox_flags),
            })
        }
    }
}

fn ensure_skybox_fdid(fdid: u32) -> Option<std::path::PathBuf> {
    let wow_path = game_engine::listfile::lookup_fdid(fdid)?;
    if !wow_path.ends_with(".m2") {
        return None;
    }
    let filename = std::path::Path::new(wow_path).file_name()?;
    let local = std::path::PathBuf::from("data/models/skyboxes").join(filename);
    crate::asset::asset_cache::file_at_path(fdid, &local)
}

fn sync_skybox_to_camera(
    camera_query: Query<&Transform, (With<Camera3d>, With<OrbitCamera>, With<SkyboxDebugScene>)>,
    mut skybox_query: Query<&mut Transform, (With<SkyboxDebugSkybox>, Without<OrbitCamera>)>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };
    for mut transform in &mut skybox_query {
        transform.translation = camera_transform.translation;
    }
}

fn teardown_scene(commands: Commands, query: Query<Entity, With<SkyboxDebugScene>>) {
    teardown_tagged_scene::<SkyboxDebugScene>(commands, query);
}

#[cfg(test)]
mod tests {
    use super::{
        SkyboxDebugOverride, SkyboxDebugScene, SkyboxDebugSetup, SkyboxDebugSkybox,
        SkyboxDebugViewMode, resolve_debug_skybox, spawn_debug_scene_environment,
        spawn_skybox_debug_reference_objects, sync_skybox_to_camera,
    };
    use crate::orbit_camera::OrbitCamera;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;

    #[test]
    fn debug_override_resolves_light_skybox_id() {
        let resolved = resolve_debug_skybox(None, Some(SkyboxDebugOverride::LightSkyboxId(653)))
            .expect("resolved light skybox override");
        assert!(
            resolved
                .path
                .ends_with("data/models/skyboxes/11xp_cloudsky01.m2"),
            "unexpected resolved path: {}",
            resolved.path.display()
        );
        assert_eq!(resolved.source, "forced LightSkyboxID=653");
        assert_eq!(
            resolved.light_skybox_flags,
            Some(
                crate::light_lookup::LightSkyboxFlags::FULL_DAY_SKYBOX
                    | crate::light_lookup::LightSkyboxFlags::COMBINE_PROCEDURAL_AND_SKYBOX
                    | crate::light_lookup::LightSkyboxFlags::PROCEDURAL_FOG_COLOR_BLEND
                    | crate::light_lookup::LightSkyboxFlags::FORCE_SUNSHAFTS
            )
        );
    }

    #[test]
    fn debug_override_resolves_skybox_fdid() {
        let resolved =
            resolve_debug_skybox(None, Some(SkyboxDebugOverride::SkyboxFileDataId(5_412_968)))
                .expect("resolved skybox fdid override");
        assert!(
            resolved
                .path
                .ends_with("data/models/skyboxes/11xp_cloudsky01.m2"),
            "unexpected resolved path: {}",
            resolved.path.display()
        );
        assert_eq!(resolved.source, "forced SkyboxFileDataID=5412968");
    }

    #[test]
    fn default_debug_scene_uses_shared_campsite_fallback_when_first_scene_has_no_local_authored_skybox()
     {
        let scene = crate::scenes::char_select::warband::WarbandScenes::load()
            .scenes
            .into_iter()
            .find(|scene| scene.id == 1)
            .expect("known scene");
        let resolved = resolve_debug_skybox(Some(&scene), None).expect("resolved default skybox");

        assert!(
            resolved
                .path
                .ends_with("data/models/skyboxes/costalislandskybox.m2"),
            "unexpected resolved path: {}",
            resolved.path.display()
        );
        assert_eq!(resolved.source, "warband scene 1 (Adventurer's Rest)");
    }

    #[test]
    fn debug_skybox_sync_uses_camera_translation() {
        let mut app = App::new();
        app.add_systems(Update, sync_skybox_to_camera);

        app.world_mut().spawn((
            SkyboxDebugScene,
            Camera3d::default(),
            OrbitCamera::new(Vec3::new(3.0, 4.0, 5.0), 7.5),
            Transform::from_translation(Vec3::new(30.0, 40.0, 50.0)),
        ));
        let skybox = app
            .world_mut()
            .spawn((
                SkyboxDebugScene,
                SkyboxDebugSkybox,
                Transform::from_translation(Vec3::ZERO),
            ))
            .id();

        app.update();

        let transform = app
            .world()
            .get::<Transform>(skybox)
            .expect("skybox transform");
        assert_eq!(transform.translation, Vec3::new(30.0, 40.0, 50.0));
    }

    #[test]
    fn debug_scene_initialization_uses_black_background_without_procedural_sky() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.init_resource::<Assets<crate::sky_material::SkyMaterial>>();
        app.init_resource::<Assets<Image>>();

        let _ = app.world_mut().run_system_once(
            |mut commands: Commands, mut images: ResMut<Assets<Image>>| {
                let setup = SkyboxDebugSetup {
                    scene: None,
                    focus: Vec3::new(0.0, 1.0, 0.0),
                    eye: Vec3::new(0.0, 1.0, 7.5),
                };
                spawn_debug_scene_environment(&mut commands, &mut images, &setup);
            },
        );

        let dome_count = {
            let world = app.world_mut();
            let mut query = world
                .query_filtered::<Entity, (With<crate::sky::SkyDome>, With<SkyboxDebugScene>)>();
            query.iter(world).count()
        };
        let fog_count = {
            let world = app.world_mut();
            let mut query =
                world.query_filtered::<Entity, (With<Camera3d>, With<DistanceFog>, With<SkyboxDebugScene>)>();
            query.iter(world).count()
        };

        assert_eq!(dome_count, 0);
        assert_eq!(fog_count, 0);
    }

    #[test]
    fn verification_mode_skips_debug_reference_objects() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.init_resource::<Assets<Image>>();

        let _ = app.world_mut().run_system_once(
            |mut commands: Commands,
             mut meshes: ResMut<Assets<Mesh>>,
             mut materials: ResMut<Assets<StandardMaterial>>,
             mut images: ResMut<Assets<Image>>| {
                spawn_skybox_debug_reference_objects(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut images,
                    SkyboxDebugViewMode::AuthoredOnlyVerification,
                );
            },
        );
        let ground_plane_count = {
            let world = app.world_mut();
            let mut query = world.query::<&Name>();
            query
                .iter(world)
                .filter(|name| name.as_str() == "SkyboxDebugGroundPlane")
                .count()
        };

        assert_eq!(ground_plane_count, 0);
    }

    #[test]
    fn default_mode_spawns_grass_ground_plane() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.init_resource::<Assets<Image>>();

        let _ = app.world_mut().run_system_once(
            |mut commands: Commands,
             mut meshes: ResMut<Assets<Mesh>>,
             mut materials: ResMut<Assets<StandardMaterial>>,
             mut images: ResMut<Assets<Image>>| {
                spawn_skybox_debug_reference_objects(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut images,
                    SkyboxDebugViewMode::Default,
                );
            },
        );
        let ground_plane_count = {
            let world = app.world_mut();
            let mut query = world.query::<(&Name, Entity)>();
            query
                .iter(world)
                .filter(|(name, _)| name.as_str() == "SkyboxDebugGroundPlane")
                .count()
        };

        assert_eq!(ground_plane_count, 1);
    }

    #[test]
    fn verification_mode_spawns_black_background_without_procedural_sky() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.init_resource::<Assets<crate::sky_material::SkyMaterial>>();
        app.init_resource::<Assets<Image>>();

        let _ = app.world_mut().run_system_once(
            |mut commands: Commands, mut images: ResMut<Assets<Image>>| {
                let setup = SkyboxDebugSetup {
                    scene: None,
                    focus: Vec3::new(0.0, 1.0, 0.0),
                    eye: Vec3::new(0.0, 1.0, 7.5),
                };
                spawn_debug_scene_environment(&mut commands, &mut images, &setup);
            },
        );

        let dome_count = {
            let world = app.world_mut();
            let mut query = world
                .query_filtered::<Entity, (With<crate::sky::SkyDome>, With<SkyboxDebugScene>)>();
            query.iter(world).count()
        };
        let fog_count = {
            let world = app.world_mut();
            let mut query =
                world.query_filtered::<Entity, (With<Camera3d>, With<DistanceFog>, With<SkyboxDebugScene>)>();
            query.iter(world).count()
        };
        let clear_color = app.world().resource::<ClearColor>().0;

        assert_eq!(dome_count, 0);
        assert_eq!(fog_count, 0);
        assert_eq!(clear_color, Color::BLACK);
    }
}
