use std::path::{Path, PathBuf};

use bevy::ecs::query::QueryFilter;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use game_engine::culling::{Wmo, WmoGroup};

use crate::creature_display;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene;
use crate::networking::{CurrentZone, LocalPlayer};
use crate::skybox_m2_material::SkyboxM2Material;
use crate::terrain::AdtManager;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum InWorldSkyboxPhase {
    Steady,
    FadingIn,
    FadingOut,
}

#[derive(Component)]
pub(super) struct InWorldSkybox {
    pub path: PathBuf,
    pub phase: InWorldSkyboxPhase,
    pub elapsed: f32,
}

const INWORLD_SKYBOX_CROSSFADE_SECONDS: f32 = 0.6;

pub(super) fn bevy_to_wow_position(pos: Vec3) -> [f32; 3] {
    [pos.x, -pos.z, pos.y]
}

fn ensure_skybox_wow_path(wow_path: &str) -> Option<PathBuf> {
    if !wow_path.ends_with(".m2") {
        return None;
    }
    let filename = Path::new(wow_path).file_name()?;
    let local = PathBuf::from("data/models/skyboxes").join(filename);
    let fdid = game_engine::listfile::lookup_path(wow_path)?;
    crate::asset::asset_cache::file_at_path(fdid, &local)
}

fn point_inside_wmo_group(local_point: Vec3, group: &WmoGroup) -> bool {
    local_point.x >= group.bbox_min.x
        && local_point.y >= group.bbox_min.y
        && local_point.z >= group.bbox_min.z
        && local_point.x <= group.bbox_max.x
        && local_point.y <= group.bbox_max.y
        && local_point.z <= group.bbox_max.z
}

pub(super) fn active_wmo_local_skybox_wow_path(
    camera_translation: Vec3,
    wmo_q: &Query<
        (
            Entity,
            &GlobalTransform,
            &crate::terrain_objects::WmoLocalSkybox,
        ),
        With<Wmo>,
    >,
    wmo_group_q: &Query<(&WmoGroup, &ChildOf)>,
) -> Option<String> {
    let mut best: Option<(f32, String)> = None;

    for (wmo_entity, wmo_transform, skybox) in wmo_q.iter() {
        let local_camera = wmo_transform
            .affine()
            .inverse()
            .transform_point3(camera_translation);
        let contains_camera = wmo_group_q.iter().any(|(group, child_of)| {
            child_of.parent() == wmo_entity && point_inside_wmo_group(local_camera, group)
        });
        if !contains_camera {
            continue;
        }

        let distance_sq = wmo_transform
            .translation()
            .distance_squared(camera_translation);
        match &best {
            Some((best_distance_sq, _)) if distance_sq >= *best_distance_sq => {}
            _ => best = Some((distance_sq, skybox.wow_path.clone())),
        }
    }

    best.map(|(_, wow_path)| wow_path)
}

fn resolve_inworld_skybox_path(map_id: u32, bevy_position: Vec3) -> Option<PathBuf> {
    let wow_position = bevy_to_wow_position(bevy_position);
    let light_params_id =
        crate::light_lookup::resolve_skybox_light_params_id(map_id, wow_position)?;
    let light_skybox_id = crate::light_lookup::resolve_light_skybox_id(light_params_id)?;
    let wow_path = crate::light_lookup::resolve_light_skybox_wow_path(light_skybox_id)?;
    ensure_skybox_wow_path(wow_path)
}

pub(super) fn resolve_inworld_map_id(adt_manager: &AdtManager, current_zone: &CurrentZone) -> u32 {
    if adt_manager.map_name.is_empty() {
        current_zone.zone_id
    } else {
        crate::light_lookup::map_name_to_id(&adt_manager.map_name).unwrap_or(current_zone.zone_id)
    }
}

fn resolve_inworld_camera_anchor(
    player_q: &Query<&Transform, (With<crate::camera::Player>, With<LocalPlayer>)>,
    camera_translation: Vec3,
) -> Vec3 {
    match player_q.single() {
        Ok(transform) => transform.translation,
        Err(_) => camera_translation,
    }
}

fn active_camera_translation<F: QueryFilter>(
    camera_q: &Query<(&Camera, &Transform), F>,
) -> Option<Vec3> {
    camera_q
        .iter()
        .find(|(camera, _)| camera.is_active)
        .map(|(_, camera_transform)| camera_transform.translation)
}

fn has_active_inworld_skybox_path(skybox_q: &Query<(Entity, &InWorldSkybox)>, path: &Path) -> bool {
    skybox_q.iter().any(|(_, skybox)| {
        skybox.phase != InWorldSkyboxPhase::FadingOut && skybox.path.as_path() == path
    })
}

fn mark_inworld_skyboxes_fading_out(
    commands: &mut Commands,
    skybox_q: &Query<(Entity, &InWorldSkybox)>,
    keep_path: Option<&Path>,
) {
    for (entity, skybox) in skybox_q.iter() {
        if keep_path == Some(skybox.path.as_path()) {
            continue;
        }
        if skybox.phase == InWorldSkyboxPhase::FadingOut {
            continue;
        }
        commands.entity(entity).insert(InWorldSkybox {
            path: skybox.path.clone(),
            phase: InWorldSkyboxPhase::FadingOut,
            elapsed: 0.0,
        });
    }
}

pub(super) fn should_replace_skybox(current: Option<&Path>, desired: Option<&Path>) -> bool {
    current != desired
}

fn spawn_inworld_skybox(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    skybox_materials: &mut Assets<SkyboxM2Material>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    creature_display_map: &creature_display::CreatureDisplayMap,
    path: &Path,
    position: Vec3,
    initial_alpha: f32,
) -> Option<Entity> {
    let spawned = m2_scene::spawn_animated_static_skybox_m2_parts(
        commands,
        meshes,
        materials,
        effect_materials,
        skybox_materials,
        images,
        inverse_bp,
        path,
        Transform::from_translation(position),
        creature_display_map,
        Some(Color::srgba(1.0, 1.0, 1.0, initial_alpha)),
    )?;

    Some(spawned.root)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn sync_inworld_authored_skybox(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
    mut skybox_materials: ResMut<Assets<SkyboxM2Material>>,
    mut images: ResMut<Assets<Image>>,
    mut inverse_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    creature_display_map: Res<creature_display::CreatureDisplayMap>,
    adt_manager: Res<AdtManager>,
    player_q: Query<&Transform, (With<crate::camera::Player>, With<LocalPlayer>)>,
    camera_q: Query<(&Camera, &Transform), With<Camera3d>>,
    skybox_q: Query<(Entity, &InWorldSkybox)>,
    wmo_q: Query<
        (
            Entity,
            &GlobalTransform,
            &crate::terrain_objects::WmoLocalSkybox,
        ),
        With<Wmo>,
    >,
    wmo_group_q: Query<(&WmoGroup, &ChildOf)>,
    current_zone: Res<CurrentZone>,
) {
    let Some(camera_translation) = active_camera_translation(&camera_q) else {
        return;
    };
    let anchor_pos = resolve_inworld_camera_anchor(&player_q, camera_translation);
    let map_id = resolve_inworld_map_id(&adt_manager, &current_zone);
    let desired_path = active_wmo_local_skybox_wow_path(camera_translation, &wmo_q, &wmo_group_q)
        .as_deref()
        .and_then(ensure_skybox_wow_path)
        .or_else(|| resolve_inworld_skybox_path(map_id, anchor_pos));
    let has_existing_skybox = skybox_q.iter().next().is_some();

    if let Some(path) = desired_path.as_deref()
        && has_active_inworld_skybox_path(&skybox_q, path)
    {
        mark_inworld_skyboxes_fading_out(&mut commands, &skybox_q, Some(path));
        return;
    }

    mark_inworld_skyboxes_fading_out(&mut commands, &skybox_q, desired_path.as_deref());

    let Some(path) = desired_path else {
        return;
    };
    let phase = if has_existing_skybox {
        InWorldSkyboxPhase::FadingIn
    } else {
        InWorldSkyboxPhase::Steady
    };
    let initial_alpha = if phase == InWorldSkyboxPhase::FadingIn {
        0.0
    } else {
        1.0
    };
    let Some(spawned_root) = spawn_inworld_skybox(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut effect_materials,
        &mut skybox_materials,
        &mut images,
        &mut inverse_bp,
        &creature_display_map,
        &path,
        camera_translation,
        initial_alpha,
    ) else {
        return;
    };
    commands.entity(spawned_root).insert(InWorldSkybox {
        path,
        phase,
        elapsed: 0.0,
    });
}

pub(super) fn sync_inworld_skybox_to_camera(
    camera_q: Query<(&Camera, &Transform), (With<Camera3d>, Without<InWorldSkybox>)>,
    mut skybox_q: Query<&mut Transform, With<InWorldSkybox>>,
) {
    let Some(camera_translation) = active_camera_translation(&camera_q) else {
        return;
    };
    for mut transform in &mut skybox_q {
        transform.translation = camera_translation;
    }
}

fn set_inworld_skybox_alpha(
    entity: Entity,
    alpha: f32,
    children_q: &Query<&Children>,
    material_q: &Query<&MeshMaterial3d<SkyboxM2Material>>,
    skybox_materials: &mut Assets<SkyboxM2Material>,
) {
    if let Ok(material) = material_q.get(entity)
        && let Some(material) = skybox_materials.get_mut(material)
    {
        material.settings.color.w = alpha;
    }
    if let Ok(children) = children_q.get(entity) {
        for child in children.iter() {
            set_inworld_skybox_alpha(child, alpha, children_q, material_q, skybox_materials);
        }
    }
}

pub(super) fn update_inworld_skybox_transition(
    time: Res<Time>,
    mut commands: Commands,
    children_q: Query<&Children>,
    material_q: Query<&MeshMaterial3d<SkyboxM2Material>>,
    mut skybox_materials: ResMut<Assets<SkyboxM2Material>>,
    mut skybox_q: Query<(Entity, &mut InWorldSkybox)>,
) {
    for (entity, mut skybox) in &mut skybox_q {
        let alpha = match skybox.phase {
            InWorldSkyboxPhase::Steady => 1.0,
            InWorldSkyboxPhase::FadingIn => {
                skybox.elapsed += time.delta_secs();
                let alpha = (skybox.elapsed / INWORLD_SKYBOX_CROSSFADE_SECONDS).clamp(0.0, 1.0);
                if alpha >= 1.0 {
                    skybox.phase = InWorldSkyboxPhase::Steady;
                    skybox.elapsed = 0.0;
                }
                alpha
            }
            InWorldSkyboxPhase::FadingOut => {
                skybox.elapsed += time.delta_secs();
                let alpha =
                    1.0 - (skybox.elapsed / INWORLD_SKYBOX_CROSSFADE_SECONDS).clamp(0.0, 1.0);
                if alpha <= 0.0 {
                    commands.entity(entity).despawn();
                    continue;
                }
                alpha
            }
        };
        set_inworld_skybox_alpha(
            entity,
            alpha,
            &children_q,
            &material_q,
            &mut skybox_materials,
        );
    }
}

pub(super) fn teardown_inworld_skybox(
    mut commands: Commands,
    skybox_q: Query<Entity, With<InWorldSkybox>>,
) {
    for entity in &skybox_q {
        commands.entity(entity).despawn();
    }
}
