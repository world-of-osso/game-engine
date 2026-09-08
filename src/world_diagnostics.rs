//! Temporary read-only InWorld probe. Opt in at startup with WOO_WORLD_DIAGNOSTICS.
//! Remove after the dark-scene and movement causes are established and fixed.

use std::time::{Duration, Instant};

use bevy::{
    camera::Exposure,
    ecs::{query::QueryData, system::SystemParam},
    light::GeneratedEnvironmentMapLight,
    prelude::*,
};
use game_engine::{movement_control::ScriptedMovement, ui::plugin::UiState};
use shared::components::Position as NetworkPosition;

use crate::{
    InWorldSceneStage,
    camera::{CharacterFacing, MovementState, Player, WowCamera},
    collision::CharacterPhysics,
    game::inworld_scene_stage::effective_inworld_scene_stage,
    game_state::GameState,
    networking::{LocalPlayer, ReconnectState},
    scenes::game_menu::UiModalOpen,
    sky::{GameTime, SkyEnvMapHandle},
    taxi::TaxiState,
};

pub(crate) fn register_if_enabled(app: &mut App) {
    if std::env::var_os("WOO_WORLD_DIAGNOSTICS").is_some() {
        app.add_systems(
            Last,
            log_world_diagnostics
                .run_if(in_state(GameState::InWorld))
                .run_if(wall_clock_sample_due),
        );
    }
}

fn wall_clock_sample_due(mut last_sample: Local<Option<Instant>>) -> bool {
    let now = Instant::now();
    if last_sample.is_some_and(|last| now.duration_since(last) < Duration::from_secs(1)) {
        return false;
    }
    *last_sample = Some(now);
    true
}

#[derive(QueryData)]
struct PlayerDiagnostic {
    entity: Entity,
    transform: Option<&'static Transform>,
    network_position: Option<&'static NetworkPosition>,
    movement: Option<&'static MovementState>,
    facing: Option<&'static CharacterFacing>,
    physics: Option<&'static CharacterPhysics>,
    local: Has<LocalPlayer>,
}

#[derive(SystemParam)]
struct WorldDiagnostics<'w, 's> {
    time: Res<'w, Time>,
    real_time: Res<'w, Time<Real>>,
    virtual_time: Res<'w, Time<Virtual>>,
    stage: Option<Res<'w, InWorldSceneStage>>,
    reconnect: Option<Res<'w, ReconnectState>>,
    modal: Option<Res<'w, UiModalOpen>>,
    taxi: Option<Res<'w, TaxiState>>,
    scripted: Option<Res<'w, ScriptedMovement>>,
    ui: Option<Res<'w, UiState>>,
    players: Query<'w, 's, PlayerDiagnostic, With<Player>>,
    local_players: Query<'w, 's, Entity, With<LocalPlayer>>,
    global_ambient: Option<Res<'w, GlobalAmbientLight>>,
    ambient: Query<'w, 's, (Entity, &'static AmbientLight)>,
    directional: Query<
        'w,
        's,
        (
            Entity,
            &'static DirectionalLight,
            Option<&'static Transform>,
            Option<&'static GlobalTransform>,
        ),
    >,
    cameras: Query<
        'w,
        's,
        (
            Entity,
            &'static Camera,
            Has<WowCamera>,
            Option<&'static AmbientLight>,
            Option<&'static GeneratedEnvironmentMapLight>,
            Option<&'static Exposure>,
        ),
        With<Camera3d>,
    >,
    game_time: Option<Res<'w, GameTime>>,
    sky_environment: Option<Res<'w, SkyEnvMapHandle>>,
}

fn log_world_diagnostics(snapshot: WorldDiagnostics) {
    log_runtime(&snapshot);
    log_players(&snapshot);
    log_lighting(&snapshot);
    log_cameras(&snapshot);
}

fn log_runtime(snapshot: &WorldDiagnostics) {
    let focused = snapshot
        .ui
        .as_deref()
        .and_then(|ui| ui.registry.focused_frame.and_then(|id| ui.registry.get(id)));
    info!(
        time_delta_secs = snapshot.time.delta_secs(),
        real_delta_secs = snapshot.real_time.delta_secs(),
        virtual_delta_secs = snapshot.virtual_time.delta_secs(),
        virtual_paused = snapshot.virtual_time.is_paused(),
        stage_configured = ?snapshot.stage.as_deref(),
        stage_effective = ?effective_inworld_scene_stage(snapshot.stage.as_deref().copied()),
        reconnect_phase = ?snapshot.reconnect.as_deref().map(|state| state.phase),
        reconnect_active = ?snapshot.reconnect.as_deref().map(ReconnectState::is_active),
        modal_open = snapshot.modal.is_some(),
        taxi_active = ?snapshot.taxi.as_deref().map(TaxiState::is_active),
        scripted_movement = ?snapshot.scripted.as_deref(),
        focused_frame_name = ?focused.and_then(|frame| frame.name.as_deref()),
        focused_frame_is_editbox = ?focused.map(|frame| frame.is_editbox()),
        "world_diagnostics runtime"
    );
}

fn has_movement_components(player: &PlayerDiagnosticItem<'_, '_>) -> bool {
    [
        player.transform.is_some(),
        player.movement.is_some(),
        player.facing.is_some(),
        player.physics.is_some(),
    ]
    .into_iter()
    .all(|present| present)
}

fn log_players(snapshot: &WorldDiagnostics) {
    info!(
        player_count = snapshot.players.iter().count(),
        local_player_count = snapshot.local_players.iter().count(),
        movement_query_matches = snapshot
            .players
            .iter()
            .filter(has_movement_components)
            .count(),
        "world_diagnostics player_counts"
    );
    for player in &snapshot.players {
        log_player(player);
    }
}

fn log_player(player: PlayerDiagnosticItem<'_, '_>) {
    info!(
        entity = ?player.entity,
        local_player = player.local,
        position = ?player.transform.map(|transform| transform.translation),
        network_position_xyz = ?player.network_position,
        has_movement_state = player.movement.is_some(),
        has_character_facing = player.facing.is_some(),
        has_character_physics = player.physics.is_some(),
        direction = ?player.movement.map(|movement| movement.direction),
        running = ?player.movement.map(|movement| movement.running),
        autorun = ?player.movement.map(|movement| movement.autorun),
        jumping = ?player.movement.map(|movement| movement.jumping),
        swimming = ?player.movement.map(|movement| movement.swimming),
        facing_yaw = ?player.facing.map(|facing| facing.yaw),
        grounded = ?player.physics.map(|physics| physics.grounded),
        vertical_velocity = ?player.physics.map(|physics| physics.vertical_velocity),
        "world_diagnostics player"
    );
}

fn log_lighting(snapshot: &WorldDiagnostics) {
    info!(
        global_ambient_present = snapshot.global_ambient.is_some(),
        global_ambient_color = ?snapshot.global_ambient.as_deref().map(|light| light.color),
        global_ambient_brightness = ?snapshot.global_ambient.as_deref().map(|light| light.brightness),
        ambient_component_count = snapshot.ambient.iter().count(),
        directional_light_count = snapshot.directional.iter().count(),
        game_time_minutes = ?snapshot.game_time.as_deref().map(|time| time.minutes),
        sky_environment_present = snapshot.sky_environment.is_some(),
        sky_environment_id = ?snapshot.sky_environment.as_deref().map(|handle| handle.0.id()),
        "world_diagnostics lighting"
    );
    for (entity, light) in &snapshot.ambient {
        info!(?entity, color = ?light.color, brightness = light.brightness,
            "world_diagnostics ambient_component");
    }
    for (entity, light, transform, global_transform) in &snapshot.directional {
        info!(?entity, color = ?light.color, illuminance = light.illuminance,
            local_forward = ?transform.map(Transform::forward),
            global_forward = ?global_transform.map(GlobalTransform::forward),
            shadow_maps_enabled = light.shadow_maps_enabled,
            "world_diagnostics directional");
    }
}

fn log_cameras(snapshot: &WorldDiagnostics) {
    for (entity, camera, wow_camera, ambient, environment, exposure) in &snapshot.cameras {
        info!(
            ?entity,
            camera_active = camera.is_active,
            wow_camera,
            ambient_override_present = ambient.is_some(),
            ambient_override_color = ?ambient.map(|light| light.color),
            ambient_override_brightness = ?ambient.map(|light| light.brightness),
            environment_present = environment.is_some(),
            environment_id = ?environment.map(|light| light.environment_map.id()),
            environment_intensity = ?environment.map(|light| light.intensity),
            environment_rotation = ?environment.map(|light| light.rotation),
            environment_lightmapped_diffuse = ?environment.map(|light| light.affects_lightmapped_mesh_diffuse),
            exposure_present = exposure.is_some(),
            exposure_ev100 = ?exposure.map(|exposure| exposure.ev100),
            "world_diagnostics camera"
        );
    }
}
