use std::path::PathBuf;

use crate::animation::M2AnimData;
use bevy::ecs::system::{RunSystemOnce, SystemParam};
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use bevy_replicon::client::confirm_history::EntityReplicated;
use lightyear::prelude::*;
use shared::components::{
    EquipmentAppearance as NetEquipmentAppearance, Health as NetHealth, Mounted,
    Player as NetPlayer, Position as NetPosition, Rotation as NetRotation,
};

use crate::camera::{CharacterFacing, MoveDirection, MovementState, Player};
use crate::character_customization::CharacterCustomizationSelection;
use crate::character_models::{ensure_named_model_bundle, race_model_wow_path};
use crate::creature_display::CreatureDisplayMap;
use crate::equipment::EquipmentItem;
use crate::equipment_appearance::{self, ResolvedEquipmentAppearance};
use crate::game::inworld_scene_stage::{
    InWorldSceneStage, configured_inworld_scene_stage, player_visual_is_enabled,
};
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
    pub(crate) mount_display_id: Option<u32>,
}

#[derive(Component)]
pub(crate) struct MountedVisualRoot;

struct PlayerModelSpawnContext<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<StandardMaterial>,
    effect_materials: &'a mut Assets<M2EffectMaterial>,
    images: &'a mut Assets<Image>,
    inv_bp: &'a mut Assets<SkinnedMeshInverseBindposes>,
    creature_display_map: &'a CreatureDisplayMap,
}

type LocalPlayerTagQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static NetPlayer, Has<LocalPlayer>), With<Remote>>;

#[derive(SystemParam)]
pub(crate) struct ReplicatedPlayerCustomizationParams<'w, 's> {
    commands: Commands<'w, 's>,
    customization_db: Res<'w, CustomizationDb>,
    char_tex: Res<'w, CharTextureData>,
    outfit_data: Res<'w, OutfitData>,
    player_query: Query<
        'w,
        's,
        (
            Entity,
            &'static NetPlayer,
            Option<&'static Mounted>,
            Option<&'static NetEquipmentAppearance>,
            Option<&'static AppliedPlayerAppearance>,
            Option<&'static Children>,
        ),
        With<ReplicatedVisualEntity>,
    >,
    parent_query: Query<'w, 's, &'static ChildOf>,
    geoset_query: Query<
        'w,
        's,
        (
            Entity,
            &'static crate::m2_spawn::GeosetMesh,
            &'static ChildOf,
        ),
    >,
    visibility_query: Query<'w, 's, &'static mut Visibility>,
    equipment_item_query: Query<'w, 's, (), With<EquipmentItem>>,
    nameplate_query: Query<'w, 's, (), With<crate::rendering::nameplate::Nameplate>>,
    material_query: Query<
        'w,
        's,
        (
            Entity,
            &'static MeshMaterial3d<StandardMaterial>,
            Option<&'static crate::m2_spawn::GeosetMesh>,
            Option<&'static crate::m2_spawn::BatchTextureType>,
            &'static ChildOf,
        ),
    >,
    children_query: Query<'w, 's, &'static Children>,
    equipment_query: Query<'w, 's, &'static mut crate::equipment::Equipment>,
    meshes: ResMut<'w, Assets<Mesh>>,
    effect_materials: ResMut<'w, Assets<M2EffectMaterial>>,
    images: ResMut<'w, Assets<Image>>,
    inv_bp: ResMut<'w, Assets<SkinnedMeshInverseBindposes>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum DesiredPlayerVisual {
    Character,
    Mount { mount_display_id: u32 },
}

#[derive(Clone, Debug, PartialEq)]
struct MountVisualAsset {
    model_path: PathBuf,
    skin_fdids: [u32; 3],
    display_scale: f32,
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
    scene_stage: Option<Res<InWorldSceneStage>>,
    query: Query<
        (
            &NetPosition,
            &NetPlayer,
            Option<&NetRotation>,
            Option<&Mounted>,
        ),
        With<Remote>,
    >,
    selected: Option<Res<SelectedCharacterId>>,
) {
    let entity = trigger.entity;
    let Ok((pos, player, rotation, mounted)) = query.get(entity) else {
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
        RemoteEntity,
        InterpolationTarget { target: position },
        RotationTarget { yaw },
    ));

    let scene_stage = configured_inworld_scene_stage(scene_stage);
    if !player_visual_is_enabled(scene_stage, is_local) {
        return;
    }

    commands.entity(entity).insert(ReplicatedVisualEntity);
    let mut ctx = PlayerModelSpawnContext {
        commands: &mut commands,
        meshes: &mut meshes,
        materials: &mut materials,
        effect_materials: &mut effect_materials,
        images: &mut images,
        inv_bp: &mut inv_bp,
        creature_display_map: &creature_display_map,
    };
    attach_player_model(&mut ctx, entity, player, mounted, is_local);
}

fn attach_player_model(
    ctx: &mut PlayerModelSpawnContext<'_, '_, '_>,
    entity: Entity,
    player: &NetPlayer,
    mounted: Option<&Mounted>,
    is_local: bool,
) {
    let model_spawned = match desired_player_visual(mounted) {
        DesiredPlayerVisual::Character => try_spawn_player_m2(ctx, entity, player),
        DesiredPlayerVisual::Mount { mount_display_id } => {
            try_spawn_mounted_player_model(ctx, entity, mount_display_id, is_local)
        }
    };
    if !model_spawned {
        let (capsule, material) = build_player_capsule(ctx.meshes, ctx.materials, is_local);
        ctx.commands
            .entity(entity)
            .insert((Mesh3d(capsule), MeshMaterial3d(material)));
    }
}

fn try_spawn_player_m2(
    ctx: &mut PlayerModelSpawnContext<'_, '_, '_>,
    entity: Entity,
    player: &NetPlayer,
) -> bool {
    let Some(model_path) = resolve_player_character_model_path(player) else {
        return false;
    };
    let mut m2_ctx = crate::m2_scene::M2SceneSpawnContext {
        commands: ctx.commands,
        assets: crate::m2_spawn::SpawnAssets {
            meshes: ctx.meshes,
            materials: ctx.materials,
            effect_materials: ctx.effect_materials,
            skybox_materials: None,
            images: ctx.images,
            inverse_bindposes: ctx.inv_bp,
        },
        creature_display_map: ctx.creature_display_map,
    };
    let spawned = crate::m2_scene::spawn_full_m2_on_entity(&mut m2_ctx, &model_path, entity);
    if spawned {
        ctx.commands.entity(entity).insert(ResolvedModelAssetInfo {
            model_path: model_path.display().to_string(),
            skin_path: crate::asset::m2::ensure_primary_skin_path(&model_path)
                .map(|p| p.display().to_string()),
            display_scale: None,
        });
    }
    spawned
}

fn try_spawn_mounted_player_model(
    ctx: &mut PlayerModelSpawnContext<'_, '_, '_>,
    entity: Entity,
    mount_display_id: u32,
    is_local: bool,
) -> bool {
    let Some(asset) = resolve_mount_visual_asset(ctx.creature_display_map, mount_display_id) else {
        return false;
    };
    let visual_root =
        spawn_mounted_visual_root(ctx.commands, entity, asset.display_scale, is_local);
    let mut assets = crate::m2_spawn::SpawnAssets {
        meshes: ctx.meshes,
        materials: ctx.materials,
        effect_materials: ctx.effect_materials,
        skybox_materials: None,
        images: ctx.images,
        inverse_bindposes: ctx.inv_bp,
    };
    ctx.commands.entity(entity).insert(ResolvedModelAssetInfo {
        model_path: asset.model_path.display().to_string(),
        skin_path: crate::asset::m2::ensure_primary_skin_path(&asset.model_path)
            .map(|path| path.display().to_string()),
        display_scale: Some(asset.display_scale),
    });
    crate::m2_spawn::spawn_m2_on_entity(
        ctx.commands,
        &mut assets,
        &asset.model_path,
        visual_root,
        &asset.skin_fdids,
    )
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

pub(crate) fn resolve_player_character_model_path(player: &NetPlayer) -> Option<PathBuf> {
    race_model_wow_path(player.race, player.appearance.sex).and_then(ensure_named_model_bundle)
}

pub(crate) fn resolve_player_model_path(player: &NetPlayer) -> Option<PathBuf> {
    resolve_player_character_model_path(player)
}

fn desired_player_visual(mounted: Option<&Mounted>) -> DesiredPlayerVisual {
    mounted
        .map(|mounted| DesiredPlayerVisual::Mount {
            mount_display_id: mounted.mount_display_id,
        })
        .unwrap_or(DesiredPlayerVisual::Character)
}

fn resolve_mount_visual_asset(
    display_map: &CreatureDisplayMap,
    mount_display_id: u32,
) -> Option<MountVisualAsset> {
    let model_fdid = display_map.get_fdid(mount_display_id)?;
    let model_path = crate::asset::asset_cache::model(model_fdid)?;
    Some(MountVisualAsset {
        model_path,
        skin_fdids: display_map
            .get_skin_fdids(mount_display_id)
            .unwrap_or([0, 0, 0]),
        display_scale: display_map
            .get_scale(mount_display_id)
            .filter(|scale| *scale > 0.0)
            .unwrap_or(1.0),
    })
}

fn spawn_mounted_visual_root(
    commands: &mut Commands,
    entity: Entity,
    display_scale: f32,
    is_local: bool,
) -> Entity {
    let mut root = commands.spawn((
        MountedVisualRoot,
        Name::new("MountedVisualRoot"),
        Transform::from_scale(Vec3::splat(display_scale.max(0.01))),
        Visibility::default(),
    ));
    if is_local {
        root.insert(MovementState::default());
    }
    let root = root.id();
    commands.entity(entity).add_child(root);
    root
}

fn clear_player_visual_children(
    commands: &mut Commands,
    entity: Entity,
    children_query: &Query<&Children>,
    nameplate_query: &Query<(), With<crate::rendering::nameplate::Nameplate>>,
) {
    let Ok(children) = children_query.get(entity) else {
        return;
    };
    for child in children.iter() {
        if nameplate_query.get(child).is_ok() {
            continue;
        }
        commands.entity(child).despawn();
    }
}

fn clear_player_visual_components(commands: &mut Commands, entity: Entity) {
    commands.entity(entity).remove::<(
        crate::animation::M2AnimData,
        crate::animation::M2AnimPlayer,
        crate::equipment::AttachmentPoints,
        Mesh3d,
        MeshMaterial3d<StandardMaterial>,
        ResolvedModelAssetInfo,
    )>();
}

fn reset_player_visual(params: &mut ReplicatedPlayerCustomizationParams, entity: Entity) {
    clear_player_visual_children(
        &mut params.commands,
        entity,
        &params.children_query,
        &params.nameplate_query,
    );
    clear_player_visual_components(&mut params.commands, entity);
}

fn apply_mount_fallback_visual(params: &mut ReplicatedPlayerCustomizationParams, entity: Entity) {
    let (capsule, material) =
        build_player_capsule(&mut params.meshes, &mut params.materials, false);
    params
        .commands
        .entity(entity)
        .insert((Mesh3d(capsule), MeshMaterial3d(material)));
}

fn resolved_mount_model_info(asset: &MountVisualAsset) -> ResolvedModelAssetInfo {
    ResolvedModelAssetInfo {
        model_path: asset.model_path.display().to_string(),
        skin_path: crate::asset::m2::ensure_primary_skin_path(&asset.model_path)
            .map(|path| path.display().to_string()),
        display_scale: Some(asset.display_scale),
    }
}

fn apply_mounted_visual(
    params: &mut ReplicatedPlayerCustomizationParams,
    entity: Entity,
    mount_display_id: u32,
) {
    let Some(asset) = resolve_mount_visual_asset(&CreatureDisplayMap, mount_display_id) else {
        apply_mount_fallback_visual(params, entity);
        return;
    };
    let visual_root =
        spawn_mounted_visual_root(&mut params.commands, entity, asset.display_scale, false);
    let mut assets = crate::m2_spawn::SpawnAssets {
        meshes: &mut params.meshes,
        materials: &mut params.materials,
        effect_materials: &mut params.effect_materials,
        skybox_materials: None,
        images: &mut params.images,
        inverse_bindposes: &mut params.inv_bp,
    };
    let _ = crate::m2_spawn::spawn_m2_on_entity(
        &mut params.commands,
        &mut assets,
        &asset.model_path,
        visual_root,
        &asset.skin_fdids,
    );
    params
        .commands
        .entity(entity)
        .insert(resolved_mount_model_info(&asset));
}

fn apply_character_visual(
    params: &mut ReplicatedPlayerCustomizationParams,
    entity: Entity,
    selection: CharacterCustomizationSelection,
    resolved_equipment: &crate::equipment_appearance::ResolvedEquipmentAppearance,
) {
    crate::character_customization::apply_character_customization(
        selection,
        &params.customization_db,
        &params.char_tex,
        Some(resolved_equipment),
        entity,
        &mut params.images,
        &mut params.materials,
        &params.parent_query,
        &params.geoset_query,
        &mut params.visibility_query,
        &params.equipment_item_query,
        &params.material_query,
    );
}

fn apply_runtime_equipment_snapshot(
    params: &mut ReplicatedPlayerCustomizationParams,
    entity: Entity,
    resolved_equipment: &crate::equipment_appearance::ResolvedEquipmentAppearance,
) {
    if let Ok(mut equipment) = params.equipment_query.get_mut(entity) {
        let previous = equipment.clone();
        equipment_appearance::apply_runtime_equipment(&mut equipment, resolved_equipment);
        if *equipment != previous {
            params
                .commands
                .trigger(crate::equipment::EquipmentChanged { entity });
        }
        return;
    }
    let mut equipment = crate::equipment::Equipment::default();
    equipment_appearance::apply_runtime_equipment(&mut equipment, resolved_equipment);
    params.commands.entity(entity).insert(equipment);
}

#[derive(EntityEvent)]
struct PlayerAppearanceChanged {
    entity: Entity,
}

type AppearanceReadQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static NetPlayer,
        Option<&'static Mounted>,
        Option<&'static NetEquipmentAppearance>,
        Option<&'static AppliedPlayerAppearance>,
        Option<&'static Children>,
    ),
    (With<ReplicatedVisualEntity>, With<ResolvedModelAssetInfo>),
>;

pub(crate) fn register_player_appearance_events(app: &mut App) {
    app.add_message::<EntityReplicated>()
        .add_systems(
            game_engine::network_tick::NetworkTick,
            forward_player_appearance_updates
                .in_set(game_engine::network_tick::NetworkTickSystems::Apply),
        )
        .add_observer(player_appearance_ready::<ReplicatedVisualEntity>)
        .add_observer(player_appearance_ready::<ResolvedModelAssetInfo>)
        .add_observer(apply_player_appearance_event);
}

fn forward_player_appearance_updates(
    mut messages: MessageReader<EntityReplicated>,
    mut commands: Commands,
    players: AppearanceReadQuery,
    stage: Option<Res<InWorldSceneStage>>,
) {
    if !configured_inworld_scene_stage(stage).includes(InWorldSceneStage::Character) {
        messages.clear();
        return;
    }
    let mut seen = std::collections::HashSet::new();
    for message in messages.read() {
        if seen.insert(message.entity) {
            queue_player_appearance(message.entity, &players, &mut commands);
        }
    }
}

fn player_appearance_ready<C: Component>(
    event: On<Insert, C>,
    mut commands: Commands,
    players: AppearanceReadQuery,
    stage: Option<Res<InWorldSceneStage>>,
) {
    if configured_inworld_scene_stage(stage).includes(InWorldSceneStage::Character) {
        queue_player_appearance(event.entity, &players, &mut commands);
    }
}

fn queue_player_appearance(entity: Entity, players: &AppearanceReadQuery, commands: &mut Commands) {
    let Ok((player, mounted, equipment, applied, children)) = players.get(entity) else {
        return;
    };
    if children.is_none_or(|children| children.is_empty()) {
        return;
    }
    let selection = net_player_customization_selection(player);
    let equipment = equipment.cloned().unwrap_or_default();
    let mount_display_id = mounted.map(|mounted| mounted.mount_display_id);
    if applied.is_some_and(|applied| {
        applied.selection == selection
            && applied.equipment == equipment
            && applied.mount_display_id == mount_display_id
    }) {
        return;
    }
    commands.trigger(PlayerAppearanceChanged { entity });
}

fn apply_player_appearance_event(
    event: On<PlayerAppearanceChanged>,
    mut params: ReplicatedPlayerCustomizationParams,
) {
    let Ok((entity, player, mounted, equipment, applied, children)) =
        params.player_query.get(event.entity)
    else {
        return;
    };
    if children.is_none_or(|children| children.is_empty()) {
        return;
    }
    let selection = net_player_customization_selection(player);
    let equipment = equipment.cloned().unwrap_or_default();
    let mount_display_id = mounted.map(|mounted| mounted.mount_display_id);
    if applied.is_some_and(|applied| {
        applied.selection == selection
            && applied.equipment == equipment
            && applied.mount_display_id == mount_display_id
    }) {
        return;
    }
    apply_player_customization_for_entity(
        &mut params,
        entity,
        selection,
        equipment,
        mount_display_id,
    );
}

fn apply_player_customization_for_entity(
    params: &mut ReplicatedPlayerCustomizationParams,
    entity: Entity,
    selection: CharacterCustomizationSelection,
    equipment_snapshot: NetEquipmentAppearance,
    mount_display_id: Option<u32>,
) {
    let mut resolved_equipment = resolve_player_equipment(params, &equipment_snapshot, selection);
    let replacement_player = {
        let (_, player, _, _, applied, _) = params
            .player_query
            .get(entity)
            .expect("appearance target must remain available during its observer");
        if let Some(previous) = applied {
            // Slots omitted by the new full snapshot must clear prior replicated models.
            resolved_equipment
                .explicit_slots
                .extend(previous.equipment.entries.iter().map(|entry| entry.slot));
        }
        player_model_changed(applied, selection, mount_display_id).then(|| player.clone())
    };
    // Publish deduplication state before model insertion observers run.
    insert_applied_player_appearance(
        params,
        entity,
        selection,
        equipment_snapshot,
        mount_display_id,
    );
    if let Some(player) = replacement_player {
        reset_player_visual(params, entity);
        if let Some(mount_display_id) = mount_display_id {
            apply_mounted_visual(params, entity, mount_display_id);
        } else {
            restore_character_visual(params, entity, player, selection, resolved_equipment);
            return;
        }
    } else if mount_display_id.is_none() {
        apply_character_visual(params, entity, selection, &resolved_equipment);
    }
    apply_runtime_equipment_snapshot(params, entity, &resolved_equipment);
}

fn player_model_changed(
    applied: Option<&AppliedPlayerAppearance>,
    selection: CharacterCustomizationSelection,
    mount_display_id: Option<u32>,
) -> bool {
    applied.is_some_and(|previous| {
        previous.mount_display_id != mount_display_id
            || (mount_display_id.is_none()
                && (previous.selection.race != selection.race
                    || previous.selection.sex != selection.sex))
    })
}

fn resolve_player_equipment(
    params: &ReplicatedPlayerCustomizationParams,
    equipment_snapshot: &NetEquipmentAppearance,
    selection: CharacterCustomizationSelection,
) -> ResolvedEquipmentAppearance {
    equipment_appearance::resolve_equipment_appearance(
        equipment_snapshot,
        &params.outfit_data,
        selection.race,
        selection.sex,
    )
}

fn restore_character_visual(
    params: &mut ReplicatedPlayerCustomizationParams,
    entity: Entity,
    player: NetPlayer,
    selection: CharacterCustomizationSelection,
    resolved_equipment: ResolvedEquipmentAppearance,
) {
    let mut context = PlayerModelSpawnContext {
        commands: &mut params.commands,
        meshes: &mut params.meshes,
        materials: &mut params.materials,
        effect_materials: &mut params.effect_materials,
        images: &mut params.images,
        inv_bp: &mut params.inv_bp,
        creature_display_map: &CreatureDisplayMap,
    };
    if !try_spawn_player_m2(&mut context, entity, &player) {
        error!(
            "Failed to restore character model for '{}' on {entity:?}",
            player.name
        );
        return;
    }
    // New mesh/joint commands must be applied before customization queries read them.
    params.commands.queue(move |world: &mut World| {
        world
            .run_system_once(move |mut params: ReplicatedPlayerCustomizationParams| {
                apply_character_visual(&mut params, entity, selection, &resolved_equipment);
                apply_runtime_equipment_snapshot(&mut params, entity, &resolved_equipment);
            })
            .expect("restored character customization must have its registered resources");
    });
}

fn insert_applied_player_appearance(
    params: &mut ReplicatedPlayerCustomizationParams,
    entity: Entity,
    selection: CharacterCustomizationSelection,
    equipment_snapshot: NetEquipmentAppearance,
    mount_display_id: Option<u32>,
) {
    params
        .commands
        .entity(entity)
        .insert(AppliedPlayerAppearance {
            selection,
            equipment: equipment_snapshot,
            mount_display_id,
        });
}

type ChangedMovementParents<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static Children),
    (With<Player>, Or<(Changed<MovementState>, Added<Player>)>),
>;
type AddedMountRoots<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static ChildOf),
    (
        With<MountedVisualRoot>,
        Or<(
            Added<MountedVisualRoot>,
            Added<MovementState>,
            Changed<ChildOf>,
        )>,
    ),
>;

pub(crate) fn sync_local_mount_visual_movement(
    mut queries: ParamSet<(
        (
            ChangedMovementParents,
            AddedMountRoots,
            Query<&MovementState, With<Player>>,
        ),
        Query<&mut MovementState, With<MountedVisualRoot>>,
    )>,
) {
    let (changed_parents, added_roots, parents) = queries.p0();
    let dirty_roots = changed_parents
        .iter()
        .flat_map(|(parent, children)| children.iter().map(move |child| (child, parent)))
        .chain(
            added_roots
                .iter()
                .map(|(root, parent)| (root, parent.parent())),
        );
    let updates: std::collections::HashMap<_, _> = dirty_roots
        .filter_map(|(root, parent)| {
            parents
                .get(parent)
                .ok()
                .map(|movement| (root, copy_movement_state(movement)))
        })
        .collect();
    let mut roots = queries.p1();
    for (entity, state) in updates {
        if let Ok(mut movement) = roots.get_mut(entity) {
            *movement = state;
        }
    }
}

fn copy_movement_state(movement: &MovementState) -> MovementState {
    MovementState {
        direction: movement.direction,
        running: movement.running,
        jumping: movement.jumping,
        autorun: movement.autorun,
        swimming: movement.swimming,
    }
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
pub(crate) fn tag_local_player(
    mut commands: Commands,
    selected: Option<Res<SelectedCharacterId>>,
    players: LocalPlayerTagQuery<'_, '_>,
    changed_players: Query<
        (),
        (
            With<Remote>,
            With<NetPlayer>,
            Or<(Changed<NetPlayer>, Added<Remote>)>,
        ),
    >,
) {
    let Some(sel) = selected else { return };
    if !sel.is_changed() && changed_players.is_empty() {
        return;
    }
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
    changed_health: Query<
        (),
        (
            With<LocalPlayer>,
            Or<(Changed<NetHealth>, Added<LocalPlayer>)>,
        ),
    >,
    local_players: Query<(), With<LocalPlayer>>,
    mut removed_health: RemovedComponents<NetHealth>,
    mut removed_local: RemovedComponents<LocalPlayer>,
    mut initialized: Local<bool>,
) {
    let health_removed = removed_health.read().fold(false, |removed, entity| {
        local_players.contains(entity) || removed
    });
    let local_removed = removed_local.read().count() != 0;
    if *initialized && changed_health.is_empty() && !health_removed && !local_removed {
        return;
    }
    *initialized = true;
    let is_alive = local_player_query
        .iter()
        .next()
        .map(|health| health.current > 0.0)
        .unwrap_or(true);
    if local_alive.0 != is_alive {
        local_alive.0 = is_alive;
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/player_appearance_event_tests.rs"]
mod appearance_event_tests;

#[cfg(test)]
#[path = "../../../tests/unit/player_appearance_spawn_tests.rs"]
mod appearance_spawn_tests;

#[cfg(test)]
#[path = "player_tests.rs"]
mod tests;
