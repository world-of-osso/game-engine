use std::path::PathBuf;

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{
    EquipmentAppearance as NetEquipmentAppearance, Health as NetHealth, Player as NetPlayer,
    Position as NetPosition, Rotation as NetRotation,
};

use crate::camera::{CharacterFacing, MoveDirection, MovementState, Player};
use crate::character_customization::CharacterCustomizationSelection;
use crate::character_models::{ensure_named_model_bundle, race_model_wow_path};
use crate::creature_display::CreatureDisplayMap;
use crate::equipment::EquipmentItem;
use crate::equipment_appearance;
use crate::m2_effect_material::M2EffectMaterial;
use crate::networking::{
    InterpolationTarget, LocalAliveState, LocalPlayer, RemoteEntity, ReplicatedVisualEntity,
    ResolvedModelAssetInfo, RotationTarget, SelectedCharacterId,
};
use game_engine::asset::char_texture::CharTextureData;
use game_engine::customization_data::CustomizationDb;
use game_engine::outfit_data::OutfitData;

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub(crate) struct AppliedPlayerAppearance {
    pub(crate) selection: CharacterCustomizationSelection,
    pub(crate) equipment: NetEquipmentAppearance,
}

/// Convert local MovementState + CharacterFacing into a world-space direction vector.
pub(crate) fn movement_to_direction(
    movement: &MovementState,
    facing: &CharacterFacing,
) -> [f32; 3] {
    let forward = [facing.yaw.sin(), 0.0, facing.yaw.cos()];
    let right = [-forward[2], 0.0, forward[0]];
    let mut dir = [0.0f32; 3];
    match movement.direction {
        MoveDirection::Forward => {
            dir[0] += forward[0];
            dir[2] += forward[2];
        }
        MoveDirection::Backward => {
            dir[0] -= forward[0];
            dir[2] -= forward[2];
        }
        MoveDirection::Left => {
            dir[0] -= right[0];
            dir[2] -= right[2];
        }
        MoveDirection::Right => {
            dir[0] += right[0];
            dir[2] += right[2];
        }
        MoveDirection::None => {}
    }
    dir
}

/// When the server replicates a new player, spawn a visible capsule mesh.
pub(crate) fn spawn_replicated_player(
    trigger: On<Add, NetPlayer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    creature_display_map: Res<CreatureDisplayMap>,
    query: Query<(&NetPosition, &NetPlayer, Option<&NetRotation>), With<Replicated>>,
    selected: Option<Res<SelectedCharacterId>>,
) {
    let entity = trigger.entity;
    let Ok((pos, player, rotation)) = query.get(entity) else {
        return;
    };
    let is_local = is_local_player_entity(&player.name, selected.as_deref());
    info!(
        "Spawning replicated player '{}' (local={is_local}) at ({:.1}, {:.1}, {:.1})",
        player.name, pos.x, pos.y, pos.z
    );
    let position = crate::networking::net_position_to_bevy(pos);
    let yaw = rotation.map_or(std::f32::consts::PI, |r| r.y);
    commands.entity(entity).insert((
        Transform::from_translation(position).with_rotation(Quat::from_rotation_y(yaw)),
        Visibility::default(),
        ReplicatedVisualEntity,
        RemoteEntity,
        InterpolationTarget { target: position },
        RotationTarget { yaw },
    ));
    attach_player_model(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut effect_materials,
        &mut images,
        &mut inv_bp,
        &creature_display_map,
        entity,
        player,
        is_local,
    );
}

#[allow(clippy::too_many_arguments)]
fn attach_player_model(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    creature_display_map: &CreatureDisplayMap,
    entity: Entity,
    player: &NetPlayer,
    is_local: bool,
) {
    let model_spawned = try_spawn_player_m2(
        commands,
        meshes,
        materials,
        effect_materials,
        images,
        inv_bp,
        creature_display_map,
        entity,
        player,
    );
    if !model_spawned {
        let (capsule, material) = build_player_capsule(meshes, materials, is_local);
        commands
            .entity(entity)
            .insert((Mesh3d(capsule), MeshMaterial3d(material)));
    }
}

fn try_spawn_player_m2(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    creature_display_map: &CreatureDisplayMap,
    entity: Entity,
    player: &NetPlayer,
) -> bool {
    let Some(model_path) = resolve_player_model_path(player) else {
        return false;
    };
    let mut ctx = crate::m2_scene::M2SceneSpawnContext {
        commands,
        assets: crate::m2_spawn::SpawnAssets {
            meshes,
            materials,
            effect_materials,
            skybox_materials: None,
            images,
            inverse_bindposes: inv_bp,
        },
        creature_display_map,
    };
    let spawned = crate::m2_scene::spawn_full_m2_on_entity(&mut ctx, &model_path, entity);
    if spawned {
        commands.entity(entity).insert(ResolvedModelAssetInfo {
            model_path: model_path.display().to_string(),
            skin_path: crate::asset::m2::ensure_primary_skin_path(&model_path)
                .map(|p| p.display().to_string()),
            display_scale: None,
        });
    }
    spawned
}

fn build_player_capsule(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    is_local: bool,
) -> (Handle<Mesh>, Handle<StandardMaterial>) {
    let capsule = meshes.add(Capsule3d::new(0.4, 1.6));
    let color = if is_local {
        Color::srgb(0.2, 1.0, 0.3)
    } else {
        Color::srgb(0.2, 0.6, 1.0)
    };
    let material = materials.add(StandardMaterial {
        base_color: color,
        ..default()
    });
    (capsule, material)
}

pub(crate) fn net_player_customization_selection(
    player: &NetPlayer,
) -> CharacterCustomizationSelection {
    CharacterCustomizationSelection {
        race: player.race,
        class: player.class,
        sex: player.appearance.sex,
        appearance: player.appearance,
    }
}

pub(crate) fn resolve_player_model_path(player: &NetPlayer) -> Option<PathBuf> {
    race_model_wow_path(player.race, player.appearance.sex).and_then(ensure_named_model_bundle)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn sync_replicated_player_customization(
    mut commands: Commands,
    customization_db: Res<CustomizationDb>,
    char_tex: Res<CharTextureData>,
    outfit_data: Res<OutfitData>,
    player_query: Query<
        (
            Entity,
            &NetPlayer,
            Option<&NetEquipmentAppearance>,
            Option<&AppliedPlayerAppearance>,
            Option<&Children>,
        ),
        With<ReplicatedVisualEntity>,
    >,
    parent_query: Query<&ChildOf>,
    geoset_query: Query<(Entity, &crate::m2_spawn::GeosetMesh, &ChildOf)>,
    mut visibility_query: Query<&mut Visibility>,
    equipment_item_query: Query<(), With<EquipmentItem>>,
    material_query: Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&crate::m2_spawn::GeosetMesh>,
        Option<&crate::m2_spawn::BatchTextureType>,
        &ChildOf,
    )>,
    mut equipment_query: Query<&mut crate::equipment::Equipment>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, player, equipment_appearance, applied, children) in &player_query {
        let selection = net_player_customization_selection(player);
        let equipment_snapshot = equipment_appearance.cloned().unwrap_or_default();
        if applied.is_some_and(|a| a.selection == selection && a.equipment == equipment_snapshot) {
            continue;
        }
        if children.is_none_or(|c| c.is_empty()) {
            continue;
        }
        apply_player_customization_for_entity(
            &mut commands,
            entity,
            selection,
            equipment_snapshot,
            &customization_db,
            &char_tex,
            &outfit_data,
            &parent_query,
            &geoset_query,
            &mut visibility_query,
            &equipment_item_query,
            &material_query,
            &mut equipment_query,
            &mut images,
            &mut materials,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_player_customization_for_entity(
    commands: &mut Commands,
    entity: Entity,
    selection: CharacterCustomizationSelection,
    equipment_snapshot: NetEquipmentAppearance,
    customization_db: &CustomizationDb,
    char_tex: &CharTextureData,
    outfit_data: &OutfitData,
    parent_query: &Query<&ChildOf>,
    geoset_query: &Query<(Entity, &crate::m2_spawn::GeosetMesh, &ChildOf)>,
    visibility_query: &mut Query<&mut Visibility>,
    equipment_item_query: &Query<(), With<EquipmentItem>>,
    material_query: &Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&crate::m2_spawn::GeosetMesh>,
        Option<&crate::m2_spawn::BatchTextureType>,
        &ChildOf,
    )>,
    equipment_query: &mut Query<&mut crate::equipment::Equipment>,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
) {
    let resolved_equipment = equipment_appearance::resolve_equipment_appearance(
        &equipment_snapshot,
        outfit_data,
        selection.race,
        selection.sex,
    );
    crate::character_customization::apply_character_customization(
        selection,
        customization_db,
        char_tex,
        Some(&resolved_equipment),
        entity,
        images,
        materials,
        parent_query,
        geoset_query,
        visibility_query,
        equipment_item_query,
        material_query,
    );
    if let Ok(mut equipment) = equipment_query.get_mut(entity) {
        equipment_appearance::apply_runtime_equipment(&mut equipment, &resolved_equipment);
    }
    commands.entity(entity).insert(AppliedPlayerAppearance {
        selection,
        equipment: equipment_snapshot,
    });
}

pub(crate) fn is_local_player_entity(
    player_name: &str,
    selected: Option<&SelectedCharacterId>,
) -> bool {
    let Some(sel) = selected else { return false };
    let Some(ref name) = sel.character_name else {
        return false;
    };
    name == player_name
}

pub(crate) fn choose_local_player_entity<'a>(
    selected_name: &str,
    players: impl Iterator<Item = (Entity, &'a NetPlayer)>,
) -> (Option<Entity>, usize) {
    let mut matches = Vec::new();
    for (entity, player) in players {
        if player.name == selected_name {
            matches.push(entity);
        }
    }
    matches.sort_by_key(|entity| entity.to_bits());
    (matches.last().copied(), matches.len())
}

/// Retroactively tag the local player when SelectedCharacterId arrives after replication.
#[allow(clippy::type_complexity)]
pub(crate) fn tag_local_player(
    mut commands: Commands,
    selected: Option<Res<SelectedCharacterId>>,
    players: Query<(Entity, &NetPlayer, Has<LocalPlayer>), With<Replicated>>,
) {
    let Some(sel) = selected else { return };
    let Some(ref name) = sel.character_name else {
        return;
    };
    if players.iter().any(|(_, p, local)| local && p.name == *name) {
        return;
    }
    let (chosen, match_count) =
        choose_local_player_entity(name, players.iter().map(|(e, p, _)| (e, p)));
    if match_count > 1 {
        warn!(
            "Found {match_count} replicated players named '{}'; choosing newest entity as local",
            name
        );
    }
    for (entity, player, is_local) in players.iter() {
        apply_local_player_tag(&mut commands, entity, player, is_local, chosen, name);
    }
}

fn apply_local_player_tag(
    commands: &mut Commands,
    entity: Entity,
    player: &NetPlayer,
    is_local: bool,
    chosen: Option<Entity>,
    name: &str,
) {
    let should_be_local = Some(entity) == chosen && player.name == name;
    if should_be_local && !is_local {
        info!("Tagging local player '{}' on entity {:?}", name, entity);
        commands.entity(entity).insert((
            LocalPlayer,
            Player,
            MovementState::default(),
            CharacterFacing::default(),
            crate::collision::CharacterPhysics::default(),
        ));
    } else if !should_be_local && is_local {
        commands.entity(entity).remove::<(
            LocalPlayer,
            Player,
            MovementState,
            CharacterFacing,
            crate::collision::CharacterPhysics,
        )>();
    }
}

pub(crate) fn sync_local_alive_state(
    mut local_alive: ResMut<LocalAliveState>,
    local_player_query: Query<&NetHealth, With<LocalPlayer>>,
) {
    local_alive.0 = local_player_query
        .iter()
        .next()
        .map(|health| health.current > 0.0)
        .unwrap_or(true);
}
