#[path = "../../rendering/character/npc_appearance.rs"]
mod npc_appearance;

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{ModelDisplay, Npc, Position as NetPosition, Rotation as NetRotation};

use crate::creature_display::CreatureDisplayMap;
use crate::game::inworld_scene_stage::{
    InWorldSceneStage, configured_inworld_scene_stage, npc_visuals_are_enabled,
};
use crate::m2_effect_material::M2EffectMaterial;
use crate::networking::{InterpolationTarget, LocalAliveState, RemoteEntity, RotationTarget};
use crate::rendering::sky::GameTime;

const DAWN_MINUTES: f32 = 720.0;
const DUSK_MINUTES: f32 = 2160.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NpcSchedule {
    DayOnly,
    NightOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NpcVisibilityPolicy {
    Always,
    Hidden,
    DeadOnly,
    Scheduled(NpcSchedule),
}

pub(crate) fn npc_visibility_policy(template_id: u32) -> NpcVisibilityPolicy {
    match template_id {
        6491 => NpcVisibilityPolicy::DeadOnly, // Spirit Healer
        918 => NpcVisibilityPolicy::Scheduled(NpcSchedule::NightOnly), // Osborne the Night Man
        12783 => NpcVisibilityPolicy::Scheduled(NpcSchedule::DayOnly), // Lieutenant Karter
        32820 => NpcVisibilityPolicy::Hidden,  // Wild Turkey clutter near spawn
        26724 | 26738 | 26739 | 26740..=26745 | 26747..=26759 | 26765 | 33252 => {
            NpcVisibilityPolicy::Hidden // [DND] TAR pedestals and other debug vendors
        }
        _ => NpcVisibilityPolicy::Always,
    }
}

fn schedule_is_active(schedule: NpcSchedule, minutes: f32) -> bool {
    match schedule {
        NpcSchedule::DayOnly => (DAWN_MINUTES..DUSK_MINUTES).contains(&minutes),
        NpcSchedule::NightOnly => !(DAWN_MINUTES..DUSK_MINUTES).contains(&minutes),
    }
}

fn npc_should_be_visible(
    policy: NpcVisibilityPolicy,
    local_alive: bool,
    game_minutes: f32,
) -> bool {
    match policy {
        NpcVisibilityPolicy::Always => true,
        NpcVisibilityPolicy::Hidden => false,
        NpcVisibilityPolicy::DeadOnly => !local_alive,
        NpcVisibilityPolicy::Scheduled(schedule) => schedule_is_active(schedule, game_minutes),
    }
}

fn apply_visibility_policy(
    npc: &Npc,
    visibility: &mut Visibility,
    local_alive: bool,
    game_minutes: f32,
) {
    let should_show = npc_should_be_visible(
        npc_visibility_policy(npc.template_id),
        local_alive,
        game_minutes,
    );
    let desired = if should_show {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    if *visibility != desired {
        *visibility = desired;
    }
}

fn apply_changed_npc_visibility_policy(
    local_alive: Res<LocalAliveState>,
    game_time: Res<GameTime>,
    mut npcs: Query<(&Npc, &mut Visibility), (With<Remote>, Changed<Npc>)>,
) {
    for (npc, mut visibility) in &mut npcs {
        apply_visibility_policy(npc, &mut visibility, local_alive.0, game_time.minutes);
    }
}

fn refresh_dead_only_npc_visibility(
    local_alive: Res<LocalAliveState>,
    game_time: Res<GameTime>,
    mut npcs: Query<(&Npc, &mut Visibility), With<Remote>>,
) {
    if !local_alive.is_changed() {
        return;
    }

    for (npc, mut visibility) in &mut npcs {
        if npc_visibility_policy(npc.template_id) == NpcVisibilityPolicy::DeadOnly {
            apply_visibility_policy(npc, &mut visibility, local_alive.0, game_time.minutes);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NpcVisibilityDayPhase {
    Day,
    Night,
}

fn npc_visibility_day_phase(minutes: f32) -> NpcVisibilityDayPhase {
    if schedule_is_active(NpcSchedule::DayOnly, minutes) {
        NpcVisibilityDayPhase::Day
    } else {
        NpcVisibilityDayPhase::Night
    }
}

fn refresh_scheduled_npc_visibility(
    local_alive: Res<LocalAliveState>,
    game_time: Res<GameTime>,
    mut last_phase: Local<Option<NpcVisibilityDayPhase>>,
    mut npcs: Query<(&Npc, &mut Visibility), With<Remote>>,
) {
    let current_phase = npc_visibility_day_phase(game_time.minutes);
    let changed_from_phase = last_phase.replace(current_phase);
    let Some(changed_from_phase) = changed_from_phase else {
        return;
    };
    if changed_from_phase == current_phase {
        return;
    }

    for (npc, mut visibility) in &mut npcs {
        if matches!(
            npc_visibility_policy(npc.template_id),
            NpcVisibilityPolicy::Scheduled(_)
        ) {
            apply_visibility_policy(npc, &mut visibility, local_alive.0, game_time.minutes);
        }
    }
}

fn refresh_npc_visibility_on_state_or_stage_change(
    state: Res<State<crate::game_state::GameState>>,
    stage: Option<Res<InWorldSceneStage>>,
    local_alive: Res<LocalAliveState>,
    game_time: Res<GameTime>,
    mut npcs: Query<(&Npc, &mut Visibility), With<Remote>>,
) {
    let stage_changed = stage.as_ref().is_some_and(|stage| stage.is_changed());
    if !state.is_changed() && !stage_changed {
        return;
    }

    for (npc, mut visibility) in &mut npcs {
        apply_visibility_policy(npc, &mut visibility, local_alive.0, game_time.minutes);
    }
}

fn npc_visibility_policy_is_active(
    state: Res<State<crate::game_state::GameState>>,
    stage: Option<Res<InWorldSceneStage>>,
) -> bool {
    match state.get() {
        crate::game_state::GameState::Loading => true,
        crate::game_state::GameState::InWorld => {
            crate::game::inworld_scene_stage::inworld_scene_stage_allows_npcs(stage)
        }
        _ => false,
    }
}

pub(crate) fn register_npc_visibility_policy_systems(app: &mut App) {
    npc_appearance::register_npc_appearance_systems(app);
    app.add_systems(
        Update,
        (
            apply_changed_npc_visibility_policy,
            refresh_dead_only_npc_visibility,
            refresh_scheduled_npc_visibility,
            refresh_npc_visibility_on_state_or_stage_change,
        )
            .chain()
            .after(crate::networking_player::sync_local_alive_state)
            .run_if(npc_visibility_policy_is_active),
    );
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use lightyear::prelude::Remote;
    use shared::components::{Health as NetHealth, Npc};

    use crate::game::inworld_scene_stage::InWorldSceneStage;
    use crate::game_state::GameState;
    use crate::networking::{LocalAliveState, LocalPlayer};
    use crate::networking_player::sync_local_alive_state;
    use crate::rendering::sky::GameTime;

    use super::{
        DAWN_MINUTES, DUSK_MINUTES, NpcSchedule, NpcVisibilityPolicy, npc_should_be_visible,
        npc_visibility_policy, register_npc_visibility_policy_systems, schedule_is_active,
    };

    #[derive(Resource, Default)]
    struct VisibilityChangeCount(usize);

    fn count_visibility_changes(
        changed: Query<(), Changed<Visibility>>,
        mut count: ResMut<VisibilityChangeCount>,
    ) {
        count.0 += changed.iter().count();
    }

    fn visibility_policy_test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.insert_state(GameState::InWorld);
        app.insert_resource(InWorldSceneStage::Npcs);
        app.init_resource::<LocalAliveState>();
        app.init_resource::<GameTime>();
        app.init_resource::<VisibilityChangeCount>();
        register_npc_visibility_policy_systems(&mut app);
        app.add_systems(Last, count_visibility_changes);
        app
    }

    fn staged_visibility_policy_test_app(stage: InWorldSceneStage, sync_alive_state: bool) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.insert_state(GameState::InWorld);
        app.insert_resource(stage);
        app.init_resource::<LocalAliveState>();
        app.init_resource::<GameTime>();

        if sync_alive_state {
            app.add_systems(Update, sync_local_alive_state);
        }
        register_npc_visibility_policy_systems(&mut app);
        app
    }

    fn set_visibility(app: &mut App, entity: Entity, visibility: Visibility) {
        *app.world_mut()
            .entity_mut(entity)
            .get_mut::<Visibility>()
            .expect("NPC visibility") = visibility;
    }

    fn spawn_visibility_test_npc(
        app: &mut App,
        template_id: u32,
        visibility: Visibility,
    ) -> Entity {
        app.world_mut()
            .spawn((Npc { template_id }, Remote, visibility))
            .id()
    }

    #[test]
    fn stable_npc_visibility_policy_does_not_mark_visibility_changed() {
        let mut app = visibility_policy_test_app();
        let npc = spawn_visibility_test_npc(&mut app, 32820, Visibility::Hidden);

        app.update();
        app.world_mut().resource_mut::<VisibilityChangeCount>().0 = 0;
        app.update();

        assert_eq!(app.world().resource::<VisibilityChangeCount>().0, 0);
        assert_eq!(
            *app.world().get::<Visibility>(npc).unwrap(),
            Visibility::Hidden
        );
    }

    #[test]
    fn npc_visibility_policy_applies_real_state_change_once() {
        let mut app = visibility_policy_test_app();
        let npc = spawn_visibility_test_npc(&mut app, 6491, Visibility::Hidden);

        app.update();
        app.world_mut().resource_mut::<VisibilityChangeCount>().0 = 0;
        app.world_mut().resource_mut::<LocalAliveState>().0 = false;
        app.update();

        assert_eq!(app.world().resource::<VisibilityChangeCount>().0, 1);
        assert_eq!(
            *app.world().get::<Visibility>(npc).unwrap(),
            Visibility::Visible
        );

        app.world_mut().resource_mut::<VisibilityChangeCount>().0 = 0;
        app.update();
        assert_eq!(app.world().resource::<VisibilityChangeCount>().0, 0);
    }

    #[test]
    fn event_driven_npc_visibility_updates_added_npc_without_refreshing_unchanged_npc() {
        let mut app = staged_visibility_policy_test_app(InWorldSceneStage::Npcs, false);
        let unchanged = spawn_visibility_test_npc(&mut app, 0, Visibility::Visible);
        app.update();
        set_visibility(&mut app, unchanged, Visibility::Hidden);

        let added = spawn_visibility_test_npc(&mut app, 32820, Visibility::Visible);
        app.update();

        assert_eq!(
            *app.world().get::<Visibility>(added).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            *app.world().get::<Visibility>(unchanged).unwrap(),
            Visibility::Hidden,
            "an unrelated NPC must not be refreshed"
        );
    }

    #[test]
    fn event_driven_npc_visibility_updates_changed_npc_without_refreshing_unchanged_npc() {
        let mut app = staged_visibility_policy_test_app(InWorldSceneStage::Npcs, false);
        let unchanged = spawn_visibility_test_npc(&mut app, 0, Visibility::Visible);
        let changed = spawn_visibility_test_npc(&mut app, 0, Visibility::Visible);
        app.update();
        set_visibility(&mut app, unchanged, Visibility::Hidden);
        app.world_mut()
            .entity_mut(changed)
            .get_mut::<Npc>()
            .expect("changed NPC")
            .template_id = 32820;

        app.update();

        assert_eq!(
            *app.world().get::<Visibility>(changed).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            *app.world().get::<Visibility>(unchanged).unwrap(),
            Visibility::Hidden,
            "an unchanged NPC must not be refreshed"
        );
    }

    #[test]
    fn event_driven_npc_visibility_updates_dead_only_npcs_on_alive_transition() {
        let mut app = staged_visibility_policy_test_app(InWorldSceneStage::Npcs, true);
        let player = app
            .world_mut()
            .spawn((
                LocalPlayer,
                NetHealth {
                    current: 100.0,
                    max: 100.0,
                },
            ))
            .id();
        let dead_only = spawn_visibility_test_npc(&mut app, 6491, Visibility::Hidden);
        let scheduled = spawn_visibility_test_npc(&mut app, 12783, Visibility::Visible);
        app.update();
        set_visibility(&mut app, scheduled, Visibility::Hidden);
        app.world_mut()
            .entity_mut(player)
            .get_mut::<NetHealth>()
            .expect("local health")
            .current = 0.0;

        app.update();

        assert_eq!(
            *app.world().get::<Visibility>(dead_only).unwrap(),
            Visibility::Visible
        );
        assert_eq!(
            *app.world().get::<Visibility>(scheduled).unwrap(),
            Visibility::Hidden,
            "alive transitions must not refresh scheduled NPCs"
        );
    }

    #[test]
    fn event_driven_npc_visibility_updates_scheduled_npcs_at_dawn_and_dusk() {
        let mut app = staged_visibility_policy_test_app(InWorldSceneStage::Npcs, false);
        app.world_mut().resource_mut::<GameTime>().minutes = DAWN_MINUTES - 1.0;
        let day_only = spawn_visibility_test_npc(&mut app, 12783, Visibility::Hidden);
        let night_only = spawn_visibility_test_npc(&mut app, 918, Visibility::Visible);
        let always = spawn_visibility_test_npc(&mut app, 0, Visibility::Visible);
        app.update();
        set_visibility(&mut app, always, Visibility::Hidden);

        app.world_mut().resource_mut::<GameTime>().minutes = DAWN_MINUTES;
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(day_only).unwrap(),
            Visibility::Visible
        );
        assert_eq!(
            *app.world().get::<Visibility>(night_only).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            *app.world().get::<Visibility>(always).unwrap(),
            Visibility::Hidden,
            "dawn must not refresh unrelated NPCs"
        );

        app.world_mut().resource_mut::<GameTime>().minutes = DUSK_MINUTES;
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(day_only).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            *app.world().get::<Visibility>(night_only).unwrap(),
            Visibility::Visible
        );
        assert_eq!(
            *app.world().get::<Visibility>(always).unwrap(),
            Visibility::Hidden,
            "dusk must not refresh unrelated NPCs"
        );
    }

    #[test]
    fn event_driven_npc_visibility_handles_time_jump_across_midnight_and_dawn() {
        let mut app = staged_visibility_policy_test_app(InWorldSceneStage::Npcs, false);
        app.world_mut().resource_mut::<GameTime>().minutes = 2879.0;
        let day_only = spawn_visibility_test_npc(&mut app, 12783, Visibility::Hidden);
        let night_only = spawn_visibility_test_npc(&mut app, 918, Visibility::Visible);
        let always = spawn_visibility_test_npc(&mut app, 0, Visibility::Visible);
        app.update();
        set_visibility(&mut app, always, Visibility::Hidden);

        app.world_mut().resource_mut::<GameTime>().minutes = 721.0;
        app.update();

        assert_eq!(
            *app.world().get::<Visibility>(day_only).unwrap(),
            Visibility::Visible
        );
        assert_eq!(
            *app.world().get::<Visibility>(night_only).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            *app.world().get::<Visibility>(always).unwrap(),
            Visibility::Hidden,
            "a wrapped time jump must not refresh unrelated NPCs"
        );
    }

    #[test]
    fn event_driven_npc_visibility_refreshes_existing_npc_when_npcs_stage_activates() {
        let mut app = staged_visibility_policy_test_app(InWorldSceneStage::Empty, false);
        let npc = spawn_visibility_test_npc(&mut app, 6491, Visibility::Hidden);
        app.world_mut().resource_mut::<LocalAliveState>().0 = false;
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(npc).unwrap(),
            Visibility::Hidden
        );

        *app.world_mut().resource_mut::<InWorldSceneStage>() = InWorldSceneStage::Npcs;
        app.update();

        assert_eq!(
            *app.world().get::<Visibility>(npc).unwrap(),
            Visibility::Visible
        );
    }

    #[test]
    fn day_schedule_is_active_between_dawn_and_dusk() {
        assert!(!schedule_is_active(
            NpcSchedule::DayOnly,
            DAWN_MINUTES - 0.1
        ));
        assert!(schedule_is_active(NpcSchedule::DayOnly, DAWN_MINUTES));
        assert!(schedule_is_active(
            NpcSchedule::DayOnly,
            (DAWN_MINUTES + DUSK_MINUTES) * 0.5,
        ));
        assert!(!schedule_is_active(NpcSchedule::DayOnly, DUSK_MINUTES));
    }

    #[test]
    fn night_schedule_wraps_across_midnight() {
        assert!(schedule_is_active(NpcSchedule::NightOnly, 0.0));
        assert!(schedule_is_active(
            NpcSchedule::NightOnly,
            DAWN_MINUTES - 0.1
        ));
        assert!(!schedule_is_active(NpcSchedule::NightOnly, DAWN_MINUTES));
        assert!(schedule_is_active(NpcSchedule::NightOnly, DUSK_MINUTES));
        assert!(schedule_is_active(NpcSchedule::NightOnly, 2879.9));
    }

    #[test]
    fn dead_only_visibility_still_depends_on_local_alive() {
        assert!(!npc_should_be_visible(
            NpcVisibilityPolicy::DeadOnly,
            true,
            1440.0
        ));
        assert!(npc_should_be_visible(
            NpcVisibilityPolicy::DeadOnly,
            false,
            1440.0,
        ));
    }

    #[test]
    fn osborne_the_night_man_is_night_only() {
        assert_eq!(
            npc_visibility_policy(918),
            NpcVisibilityPolicy::Scheduled(NpcSchedule::NightOnly)
        );
        assert!(npc_should_be_visible(
            npc_visibility_policy(918),
            true,
            DUSK_MINUTES,
        ));
        assert!(!npc_should_be_visible(
            npc_visibility_policy(918),
            true,
            1440.0,
        ));
    }

    #[test]
    fn lieutenant_karter_is_day_only() {
        assert_eq!(
            npc_visibility_policy(12783),
            NpcVisibilityPolicy::Scheduled(NpcSchedule::DayOnly)
        );
        assert!(npc_should_be_visible(
            npc_visibility_policy(12783),
            true,
            1440.0,
        ));
        assert!(!npc_should_be_visible(
            npc_visibility_policy(12783),
            true,
            DUSK_MINUTES,
        ));
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
    With<Remote>,
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
    scene_stage: Option<Res<InWorldSceneStage>>,
) {
    let entity = trigger.entity;
    let Ok((pos, npc, rotation, model_display)) = query.get(entity) else {
        return;
    };
    insert_npc_transform(&mut commands, entity, pos, rotation);
    let scene_stage = configured_inworld_scene_stage(scene_stage);
    if !npc_visuals_are_enabled(scene_stage) {
        return;
    }

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
    if m2_loaded {
        let display_id = model_display.map_or(0, |display| display.display_id);
        npc_appearance::queue_npc_appearance(&mut commands, visual_root, display_id);
    }
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

/// Parent of a replicated NPC's M2 model; selects the model for animation LOD.
#[derive(Component)]
pub(crate) struct NpcVisualRoot;

fn spawn_npc_visual_root(commands: &mut Commands, entity: Entity, scale: f32) -> Entity {
    let visual_root = commands
        .spawn((
            Name::new("NpcVisualRoot"),
            NpcVisualRoot,
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
