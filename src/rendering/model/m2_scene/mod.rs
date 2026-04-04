use std::path::Path;

use bevy::prelude::*;

use crate::animation::{
    BonePivot, M2_BONE_SPHERICAL_BILLBOARD, M2AnimData, M2AnimPlayer, SphericalBillboard,
};
use crate::asset;
use crate::asset::{
    m2_anim::{BoneAnimTracks, M2AnimSequence, M2Bone},
    m2_attach,
    m2_light::M2Light,
    m2_particle::M2ParticleEmitter,
};
use crate::camera::{CharacterFacing, MovementState, Player};
use crate::creature_display;
use crate::equipment;
use crate::m2_spawn;
use crate::particle;

mod static_spawn;

pub struct M2SceneSpawnContext<'a, 'w, 's> {
    pub commands: &'a mut Commands<'w, 's>,
    pub assets: m2_spawn::SpawnAssets<'a>,
    pub creature_display_map: &'a creature_display::CreatureDisplayMap,
}

struct M2SceneAttachOptions {
    default_main_hand_torch: bool,
    force_skybox_material: bool,
    skybox_color: Option<Color>,
}

struct M2SceneAnimPayload {
    bones: Vec<M2Bone>,
    sequences: Vec<M2AnimSequence>,
    bone_tracks: Vec<BoneAnimTracks>,
    particle_emitters: Vec<M2ParticleEmitter>,
    attachments: Vec<m2_attach::M2Attachment>,
    attachment_lookup: Vec<i16>,
}

struct M2SceneVisuals {
    batches: Vec<asset::m2::M2RenderBatch>,
    lights: Vec<M2Light>,
}

struct M2SceneAttachedVisuals {
    visual_root: Entity,
    skinning: m2_spawn::SkinningResult,
}

#[allow(unused_imports)]
pub use self::static_spawn::{
    SpawnedAnimatedStaticM2, spawn_animated_static_m2, spawn_animated_static_m2_parts,
    spawn_animated_static_m2_parts_from_model, spawn_animated_static_m2_parts_with_skin_fdids,
    spawn_animated_static_skybox_m2_parts,
};

/// Attach equipment (attachment points + default main-hand torch) to a model entity.
pub fn attach_equipment_to_model(
    commands: &mut Commands,
    model_entity: Entity,
    attachments: &[m2_attach::M2Attachment],
    attachment_lookup: &[i16],
    default_main_hand_torch: bool,
) {
    if attachments.is_empty() {
        return;
    }
    let attach_pts = equipment::build_attachment_points(attachments, attachment_lookup);
    let mut equip = equipment::Equipment::default();
    if default_main_hand_torch {
        let torch = Path::new("data/models/club_1h_torch_a_01.m2");
        if torch.exists() {
            equip
                .slots
                .insert(equipment::EquipmentSlot::MainHand, torch.to_path_buf());
        }
    }
    commands.entity(model_entity).insert((attach_pts, equip));
}

fn spawn_anim_and_particles(
    ctx: &mut M2SceneSpawnContext<'_, '_, '_>,
    payload: M2SceneAnimPayload,
    skinning: &m2_spawn::SkinningResult,
    model_entity: Entity,
    visual_root: Entity,
    default_main_hand_torch: bool,
) {
    let M2SceneAnimPayload {
        bones,
        sequences,
        bone_tracks,
        particle_emitters,
        attachments,
        attachment_lookup,
    } = payload;
    let joint_entities =
        attach_bone_pivots_and_player(ctx.commands, &bones, &sequences, skinning, model_entity);
    spawn_particle_emitters(
        ctx.commands,
        ctx.assets.images,
        &particle_emitters,
        &bones,
        skinning,
        visual_root,
    );
    insert_anim_data_if_present(
        ctx.commands,
        model_entity,
        joint_entities,
        bones,
        sequences,
        bone_tracks,
    );
    attach_anim_equipment(
        ctx.commands,
        model_entity,
        &attachments,
        &attachment_lookup,
        default_main_hand_torch,
    );
}

fn insert_anim_data_if_present(
    commands: &mut Commands,
    model_entity: Entity,
    joint_entities: Option<Vec<Entity>>,
    bones: Vec<M2Bone>,
    sequences: Vec<M2AnimSequence>,
    bone_tracks: Vec<BoneAnimTracks>,
) {
    if let Some(joints) = joint_entities {
        commands.entity(model_entity).insert(M2AnimData {
            spherical_billboards: crate::animation::propagate_spherical_billboards(&bones),
            bones,
            sequences,
            bone_tracks,
            joint_entities: joints,
        });
    }
}

fn attach_anim_equipment(
    commands: &mut Commands,
    model_entity: Entity,
    attachments: &[m2_attach::M2Attachment],
    attachment_lookup: &[i16],
    default_main_hand_torch: bool,
) {
    attach_equipment_to_model(
        commands,
        model_entity,
        attachments,
        attachment_lookup,
        default_main_hand_torch,
    );
}

fn spawn_particle_emitters(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    particle_emitters: &[M2ParticleEmitter],
    bones: &[M2Bone],
    skinning: &m2_spawn::SkinningResult,
    model_entity: Entity,
) {
    if particle_emitters.is_empty() {
        return;
    }
    let bone_slice = skinning.as_ref().map(|(_, joints)| joints.as_slice());
    particle::spawn_emitters(
        commands,
        images,
        particle_emitters,
        bones,
        bone_slice,
        model_entity,
    );
}

fn load_m2_model(
    m2_path: &Path,
    creature_display_map: &creature_display::CreatureDisplayMap,
) -> Option<asset::m2::M2Model> {
    let skin_fdids = creature_display_map
        .resolve_skin_fdids_for_model_path(m2_path)
        .unwrap_or([0, 0, 0]);
    load_m2_model_with_skin_fdids(m2_path, &skin_fdids)
}

fn load_m2_model_with_skin_fdids(
    m2_path: &Path,
    skin_fdids: &[u32; 3],
) -> Option<asset::m2::M2Model> {
    asset::m2::load_m2_uncached(m2_path, skin_fdids)
        .map_err(|e| {
            eprintln!("Failed to load M2 {}: {e}", m2_path.display());
        })
        .ok()
}

pub fn spawn_m2_model(ctx: &mut M2SceneSpawnContext<'_, '_, '_>, m2_path: &Path) {
    let Some(model) = load_m2_model(m2_path, ctx.creature_display_map) else {
        return;
    };
    let model_entity = spawn_player_root(ctx.commands, m2_path);
    attach_m2_model_parts(
        ctx,
        model,
        model_entity,
        M2SceneAttachOptions {
            default_main_hand_torch: true,
            force_skybox_material: false,
            skybox_color: None,
        },
    );
}

pub fn spawn_full_m2_on_entity(
    ctx: &mut M2SceneSpawnContext<'_, '_, '_>,
    m2_path: &Path,
    entity: Entity,
) -> bool {
    let Some(model) = load_m2_model(m2_path, ctx.creature_display_map) else {
        return false;
    };
    attach_m2_model_parts(
        ctx,
        model,
        entity,
        M2SceneAttachOptions {
            default_main_hand_torch: false,
            force_skybox_material: false,
            skybox_color: None,
        },
    );
    true
}

fn attach_m2_model_parts(
    ctx: &mut M2SceneSpawnContext<'_, '_, '_>,
    model: asset::m2::M2Model,
    model_entity: Entity,
    options: M2SceneAttachOptions,
) {
    let (visuals, payload) = split_m2_scene_model(model);
    let attached =
        attach_m2_scene_visuals(ctx, model_entity, visuals.batches, &payload.bones, &options);
    spawn_m2_scene_runtime(
        ctx,
        payload,
        attached,
        model_entity,
        &visuals.lights,
        options.default_main_hand_torch,
    );
}

fn split_m2_scene_model(model: asset::m2::M2Model) -> (M2SceneVisuals, M2SceneAnimPayload) {
    let asset::m2::M2Model {
        batches,
        bones,
        sequences,
        bone_tracks,
        particle_emitters,
        attachments,
        attachment_lookup,
        lights,
        ..
    } = model;
    (
        M2SceneVisuals { batches, lights },
        M2SceneAnimPayload {
            bones,
            sequences,
            bone_tracks,
            particle_emitters,
            attachments,
            attachment_lookup,
        },
    )
}

fn attach_m2_scene_visuals(
    ctx: &mut M2SceneSpawnContext<'_, '_, '_>,
    model_entity: Entity,
    batches: Vec<asset::m2::M2RenderBatch>,
    bones: &[M2Bone],
    options: &M2SceneAttachOptions,
) -> M2SceneAttachedVisuals {
    let visual_root = visual_root_entity(
        ctx.commands,
        model_entity,
        m2_spawn::ground_offset_y(&batches),
        options.force_skybox_material,
    );
    let skinning = m2_spawn::attach_m2_batches(
        ctx.commands,
        &mut ctx.assets,
        batches,
        bones,
        visual_root,
        options.force_skybox_material,
        options.skybox_color,
    );
    M2SceneAttachedVisuals {
        visual_root,
        skinning,
    }
}

fn spawn_m2_scene_runtime(
    ctx: &mut M2SceneSpawnContext<'_, '_, '_>,
    payload: M2SceneAnimPayload,
    attached: M2SceneAttachedVisuals,
    model_entity: Entity,
    lights: &[M2Light],
    default_main_hand_torch: bool,
) {
    spawn_anim_and_particles(
        ctx,
        payload,
        &attached.skinning,
        model_entity,
        attached.visual_root,
        default_main_hand_torch,
    );
    m2_spawn::spawn_model_point_lights(
        ctx.commands,
        lights,
        &attached.skinning,
        attached.visual_root,
        model_entity,
    );
}

fn visual_root_entity(
    commands: &mut Commands,
    model_entity: Entity,
    ground_offset_y: f32,
    force_skybox_material: bool,
) -> Entity {
    if force_skybox_material {
        model_entity
    } else {
        m2_spawn::ensure_grounded_model_root(commands, model_entity, ground_offset_y)
    }
}

pub fn spawn_player_root(commands: &mut Commands, m2_path: &Path) -> Entity {
    let name = m2_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("m2_model");
    commands
        .spawn((
            Name::new(name.to_owned()),
            Player,
            MovementState::default(),
            CharacterFacing::default(),
            crate::collision::CharacterPhysics::default(),
            Transform::from_xyz(0.0, 0.0, 0.0)
                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
            Visibility::default(),
        ))
        .id()
}

/// Spawn a static (non-player) M2 model as a scene prop.
pub fn spawn_static_m2(
    ctx: &mut M2SceneSpawnContext<'_, '_, '_>,
    m2_path: &Path,
    transform: Transform,
) -> Option<Entity> {
    let (root, model_root) = spawn_static_model_root(ctx.commands, m2_path, transform);
    if spawn_static_model_on_entity(ctx, m2_path, model_root) {
        Some(root)
    } else {
        ctx.commands.entity(root).despawn();
        None
    }
}

fn spawn_static_model_root(
    commands: &mut Commands,
    m2_path: &Path,
    transform: Transform,
) -> (Entity, Entity) {
    let name = m2_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("prop");
    let root = commands
        .spawn((Name::new(name.to_owned()), transform, Visibility::default()))
        .id();
    // Keep the visual skeleton/meshes under an identity child so Bevy skinning
    // computes world_from_local from the actor root only once.
    let model_root = commands
        .spawn((
            Name::new(format!("{name}ModelRoot")),
            Transform::IDENTITY,
            Visibility::default(),
        ))
        .id();
    commands.entity(model_root).insert(ChildOf(root));
    (root, model_root)
}

fn spawn_static_model_on_entity(
    ctx: &mut M2SceneSpawnContext<'_, '_, '_>,
    m2_path: &Path,
    model_root: Entity,
) -> bool {
    let skin_fdids = ctx
        .creature_display_map
        .resolve_skin_fdids_for_model_path(m2_path)
        .unwrap_or([0, 0, 0]);
    m2_spawn::spawn_m2_on_entity(
        ctx.commands,
        &mut ctx.assets,
        m2_path,
        model_root,
        &skin_fdids,
    )
}

/// Attach BonePivot components to joint entities and insert M2AnimPlayer on the model.
/// Returns the joint entity list if animation is active, otherwise None.
pub fn attach_bone_pivots_and_player(
    commands: &mut Commands,
    bones: &[M2Bone],
    sequences: &[M2AnimSequence],
    skinning: &m2_spawn::SkinningResult,
    model_entity: Entity,
) -> Option<Vec<Entity>> {
    let (_, joints) = skinning.as_ref()?;
    for (i, bone) in bones.iter().enumerate() {
        let p = bone.pivot;
        let mut entity = commands.entity(joints[i]);
        entity.insert(BonePivot(Vec3::new(p[0], p[2], -p[1])));
        if bone.flags & M2_BONE_SPHERICAL_BILLBOARD != 0 {
            entity.insert(SphericalBillboard {
                pivot: Vec3::new(p[0], p[2], -p[1]),
            });
        }
    }
    if sequences.is_empty() {
        return None;
    }
    let stand_idx = sequences.iter().position(|s| s.id == 0).unwrap_or(0);
    commands.entity(model_entity).insert(M2AnimPlayer {
        current_seq_idx: stand_idx,
        time_ms: 0.0,
        looping: true,
        transition: None,
    });
    Some(joints.clone())
}

#[cfg(test)]
mod tests {
    use super::{load_m2_model_with_skin_fdids, visual_root_entity};
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn skybox_visual_root_skips_grounding_even_with_nonzero_offset() {
        let mut world = World::default();
        let model_entity = world.spawn_empty().id();

        let visual_root = world
            .run_system_once(move |mut commands: Commands| {
                visual_root_entity(&mut commands, model_entity, 12.5, true)
            })
            .expect("skybox visual root");
        assert_eq!(visual_root, model_entity);

        let rooted_children = world
            .query::<&Name>()
            .iter(&world)
            .filter(|name| name.as_str() == "GroundedModelRoot")
            .count();
        assert_eq!(rooted_children, 0);
    }

    #[test]
    fn non_skybox_visual_root_still_grounds_nonzero_offset_models() {
        let mut world = World::default();
        let model_entity = world.spawn_empty().id();

        let grounded_root = world
            .run_system_once(move |mut commands: Commands| {
                visual_root_entity(&mut commands, model_entity, 12.5, false)
            })
            .expect("grounded visual root");
        assert_ne!(grounded_root, model_entity);

        let grounded_name = world.get::<Name>(grounded_root).map(Name::as_str);
        assert_eq!(grounded_name, Some("GroundedModelRoot"));
    }

    #[test]
    fn static_m2_scene_load_uses_model_cache() {
        let Some((model_path, _skin_path)) = copy_torch_model_to_temp() else {
            return;
        };
        let cache_entries_before = crate::asset::m2::model_cache_stats().entries;

        let first = load_m2_model_with_skin_fdids(&model_path, &[0, 0, 0]);
        let cache_entries_after_first = crate::asset::m2::model_cache_stats().entries;

        let second = load_m2_model_with_skin_fdids(&model_path, &[0, 0, 0]);
        let cache_entries_after_second = crate::asset::m2::model_cache_stats().entries;

        assert!(first.is_some());
        assert!(second.is_some());
        assert_eq!(cache_entries_after_first, cache_entries_before + 1);
        assert_eq!(cache_entries_after_second, cache_entries_after_first);
    }

    fn copy_torch_model_to_temp() -> Option<(PathBuf, PathBuf)> {
        let source_model = Path::new("data/models/club_1h_torch_a_01.m2");
        let source_skin = Path::new("data/models/club_1h_torch_a_0100.skin");
        if !source_model.exists() || !source_skin.exists() {
            return None;
        }
        let unique = format!(
            "m2_scene_cache_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        );
        let temp_dir = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&temp_dir).expect("create temp dir");
        let model_path = temp_dir.join("club_1h_torch_a_01.m2");
        let skin_path = temp_dir.join("club_1h_torch_a_0100.skin");
        std::fs::copy(source_model, &model_path).expect("copy temp torch model");
        std::fs::copy(source_skin, &skin_path).expect("copy temp torch skin");
        Some((model_path, skin_path))
    }
}
