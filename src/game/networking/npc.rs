use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{ModelDisplay, Npc, Position as NetPosition, Rotation as NetRotation};

use crate::creature_display::CreatureDisplayMap;
use crate::m2_effect_material::M2EffectMaterial;
use crate::networking::{InterpolationTarget, LocalAliveState, RemoteEntity, RotationTarget};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NpcVisibilityPolicy {
    Always,
    Hidden,
    DeadOnly,
}

pub(crate) fn npc_visibility_policy(template_id: u32) -> NpcVisibilityPolicy {
    match template_id {
        6491 => NpcVisibilityPolicy::DeadOnly, // Spirit Healer
        32820 => NpcVisibilityPolicy::Hidden,  // Wild Turkey clutter near spawn
        26724 | 26738 | 26739 | 26740..=26745 | 26747..=26759 | 26765 | 33252 => {
            NpcVisibilityPolicy::Hidden // [DND] TAR pedestals and other debug vendors
        }
        _ => NpcVisibilityPolicy::Always,
    }
}

pub(crate) fn apply_npc_visibility_policy(
    local_alive: Res<LocalAliveState>,
    mut npcs: Query<(&Npc, &mut Visibility), With<Replicated>>,
) {
    for (npc, mut visibility) in &mut npcs {
        let should_show = match npc_visibility_policy(npc.template_id) {
            NpcVisibilityPolicy::Always => true,
            NpcVisibilityPolicy::Hidden => false,
            NpcVisibilityPolicy::DeadOnly => !local_alive.0,
        };
        *visibility = if should_show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

type NpcReplicatedQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static NetPosition,
        &'static Npc,
        Option<&'static NetRotation>,
        Option<&'static ModelDisplay>,
    ),
    With<Replicated>,
>;

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct NpcSpawnAssets<'w> {
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub materials: ResMut<'w, Assets<StandardMaterial>>,
    pub effect_materials: ResMut<'w, Assets<M2EffectMaterial>>,
    pub images: ResMut<'w, Assets<Image>>,
    pub inv_bp: ResMut<'w, Assets<SkinnedMeshInverseBindposes>>,
}

/// When the server replicates a new NPC, try to load its M2 model; fall back to capsule.
pub(crate) fn spawn_replicated_npc(
    trigger: On<Add, Npc>,
    mut commands: Commands,
    mut npc_assets: NpcSpawnAssets,
    query: NpcReplicatedQuery,
    display_map: Option<Res<CreatureDisplayMap>>,
) {
    let entity = trigger.entity;
    let Ok((pos, npc, rotation, model_display)) = query.get(entity) else {
        return;
    };
    insert_npc_transform(&mut commands, entity, pos, rotation);
    let display_scale = npc_display_scale(model_display, display_map.as_deref());
    let visual_root = spawn_npc_visual_root(&mut commands, entity, display_scale);
    let m2_loaded = spawn_npc_model_or_capsule(
        &mut commands,
        &mut npc_assets,
        visual_root,
        entity,
        model_display,
        display_map.as_deref(),
        display_scale,
    );
    debug!(
        "Spawned NPC template_id={} m2={m2_loaded} at ({:.0}, {:.0}, {:.0})",
        npc.template_id, pos.x, pos.y, pos.z
    );
}

fn spawn_npc_model_or_capsule(
    commands: &mut Commands,
    npc_assets: &mut NpcSpawnAssets,
    visual_root: Entity,
    entity: Entity,
    model_display: Option<&ModelDisplay>,
    display_map: Option<&CreatureDisplayMap>,
    display_scale: f32,
) -> bool {
    let mut assets = crate::m2_spawn::SpawnAssets {
        meshes: &mut npc_assets.meshes,
        materials: &mut npc_assets.materials,
        effect_materials: &mut npc_assets.effect_materials,
        skybox_materials: None,
        images: &mut npc_assets.images,
        inverse_bindposes: &mut npc_assets.inv_bp,
    };
    let m2_loaded = try_spawn_npc_model(
        commands,
        &mut assets,
        visual_root,
        entity,
        model_display,
        display_map,
        display_scale,
    );
    if !m2_loaded {
        spawn_npc_capsule(
            commands,
            &mut npc_assets.meshes,
            &mut npc_assets.materials,
            visual_root,
        );
    }
    m2_loaded
}

fn spawn_npc_visual_root(commands: &mut Commands, entity: Entity, scale: f32) -> Entity {
    let visual_root = commands
        .spawn((
            Name::new("NpcVisualRoot"),
            Transform::from_scale(Vec3::splat(scale.max(0.01))),
            Visibility::default(),
        ))
        .id();
    commands.entity(entity).add_child(visual_root);
    visual_root
}

fn insert_npc_transform(
    commands: &mut Commands,
    entity: Entity,
    pos: &NetPosition,
    rotation: Option<&NetRotation>,
) {
    let position = crate::networking::net_position_to_bevy(pos);
    let yaw = rotation.map_or(0.0, |r| r.y);
    let transform = Transform::from_translation(position).with_rotation(Quat::from_rotation_y(yaw));
    commands.entity(entity).insert((
        transform,
        Visibility::default(),
        RemoteEntity,
        InterpolationTarget { target: position },
        RotationTarget { yaw },
    ));
}

fn npc_display_scale(
    model_display: Option<&ModelDisplay>,
    display_map: Option<&CreatureDisplayMap>,
) -> f32 {
    let display_id = model_display.map(|md| md.display_id).unwrap_or(0);
    display_map
        .and_then(|dm| dm.get_scale(display_id))
        .filter(|scale| *scale > 0.0)
        .unwrap_or(1.0)
}

/// Try to resolve display_id → FDID → M2 file and attach meshes. Returns true on success.
fn try_spawn_npc_model(
    commands: &mut Commands,
    assets: &mut crate::m2_spawn::SpawnAssets<'_>,
    visual_root: Entity,
    entity: Entity,
    model_display: Option<&ModelDisplay>,
    display_map: Option<&CreatureDisplayMap>,
    display_scale: f32,
) -> bool {
    let display_id = model_display.map(|md| md.display_id).unwrap_or(0);
    if display_id == 0 {
        return false;
    }
    let fdid = display_map.and_then(|dm| dm.get_fdid(display_id));
    let Some(fdid) = fdid else { return false };
    let skin_fdids = display_map
        .and_then(|dm| dm.get_skin_fdids(display_id))
        .unwrap_or([0, 0, 0]);
    let Some(m2_path) = crate::asset::asset_cache::model(fdid) else {
        return false;
    };
    commands
        .entity(entity)
        .insert(crate::networking::ResolvedModelAssetInfo {
            model_path: m2_path.display().to_string(),
            skin_path: crate::asset::m2::ensure_primary_skin_path(&m2_path)
                .map(|path| path.display().to_string()),
            display_scale: Some(display_scale),
        });
    crate::m2_spawn::spawn_m2_on_entity(commands, assets, &m2_path, visual_root, &skin_fdids)
}

/// Attach a capsule mesh as fallback for NPCs without M2 models.
fn spawn_npc_capsule(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    entity: Entity,
) {
    let capsule = meshes.add(Capsule3d::new(0.3, 1.2));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.3, 0.2),
        ..default()
    });
    commands
        .entity(entity)
        .insert((Mesh3d(capsule), MeshMaterial3d(material)));
}
