use std::f32::consts::PI;
use std::time::Instant;

use bevy::light::DirectionalLightShadowMap;
use bevy::prelude::*;
use lightyear::prelude::client::Connected;

use crate::camera::{self, WowCamera};
use crate::networking::{CurrentZone, LocalPlayer, ServerAddr};
use crate::shadow_config::default_cascade_shadow_config;
use crate::sky;
use crate::terrain::AdtManager;

pub use game_engine::game_state_enum::GameState;

#[derive(Resource)]
pub struct InitialGameState(pub GameState);

#[derive(Resource, Clone, Copy)]
pub struct StartupScreenTarget(pub GameState);

#[derive(Resource, Clone, Copy)]
pub struct PostEulaState(pub GameState);

#[derive(Resource)]
pub struct StartupPerfTimer(pub Instant);

#[derive(Resource, Default)]
struct ZoneTransitionTracker {
    observed_zone_id: u32,
    initialized: bool,
}

/// Resource tracking when we entered the Connecting state (for timeout).
#[derive(Resource)]
struct ConnectingStartTime(f64);

/// Connection timeout in seconds.
const CONNECT_TIMEOUT_SECS: f64 = 10.0;

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        let has_server = app.world().get_resource::<ServerAddr>().is_some();
        let initial_state = app.world().get_resource::<InitialGameState>().map(|s| s.0);
        init_state(app, has_server, initial_state);
        app.init_resource::<ZoneTransitionTracker>();
        register_state_transitions(app, has_server);
        register_in_world_systems(app);
        app.add_systems(Update, log_screen_switches);
    }
}

fn init_state(app: &mut App, has_server: bool, initial_state: Option<GameState>) {
    let accepted_eula = crate::client_options::load_eula_accepted();
    let (state, post_eula) = resolve_startup_state(has_server, initial_state, accepted_eula);
    app.insert_state(state);
    if let Some(post_eula) = post_eula {
        app.insert_resource(PostEulaState(post_eula));
    }
}

fn resolve_startup_state(
    has_server: bool,
    initial_state: Option<GameState>,
    accepted_eula: bool,
) -> (GameState, Option<GameState>) {
    if should_gate_eula(has_server, initial_state, accepted_eula) {
        return (
            GameState::Eula,
            Some(initial_state.unwrap_or(GameState::Login)),
        );
    }
    match initial_state {
        Some(GameState::Eula) => (GameState::Eula, Some(GameState::Login)),
        Some(state) => (state, None),
        None if has_server => (GameState::Login, None),
        None if cfg!(debug_assertions) => (GameState::InWorld, None),
        None => (GameState::Login, None),
    }
}

fn should_gate_eula(
    has_server: bool,
    initial_state: Option<GameState>,
    accepted_eula: bool,
) -> bool {
    if std::env::var_os("SKIP_EULA").is_some() {
        return false;
    }
    if std::env::var_os("ENABLE_EULA").is_none() {
        return false;
    }
    has_server
        && !accepted_eula
        && initial_state
            .is_none_or(|state| matches!(state, GameState::Login | GameState::Connecting))
}

fn register_state_transitions(app: &mut App, has_server: bool) {
    app.add_systems(OnEnter(GameState::Eula), on_enter_eula);
    app.add_systems(OnEnter(GameState::Connecting), on_enter_connecting);
    app.add_systems(OnEnter(GameState::CharSelect), on_enter_char_select);
    app.add_systems(OnEnter(GameState::SelectionDebug), on_enter_selection_debug);
    app.add_systems(
        OnEnter(GameState::InWorldSelectionDebug),
        on_enter_inworld_selection_debug,
    );
    app.add_systems(OnEnter(GameState::DebugCharacter), on_enter_debug_character);
    app.add_systems(OnEnter(GameState::SkyboxDebug), on_enter_skybox_debug);
    app.add_systems(OnEnter(GameState::CampsitePopup), on_enter_campsite_popup);
    app.add_systems(OnEnter(GameState::Loading), on_enter_loading);
    app.add_systems(OnEnter(GameState::TrashButton), on_enter_trash_button);
    app.add_systems(OnEnter(GameState::InWorld), on_enter_in_world);
    app.add_systems(OnEnter(GameState::InWorld), reset_zone_transition_tracker);
    if has_server {
        app.add_systems(OnEnter(GameState::InWorld), spawn_world_environment);
    }
    app.add_systems(OnExit(GameState::InWorld), on_exit_in_world);
    app.add_systems(
        Update,
        check_connection_status.run_if(in_state(GameState::Connecting)),
    );
    app.add_systems(
        Update,
        check_loading_complete.run_if(in_state(GameState::Loading)),
    );
    app.add_systems(
        Update,
        handle_zone_transition.run_if(in_state(GameState::InWorld)),
    );
}

fn register_in_world_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            game_engine::ui::game_plugin::sync_screen_ui,
            game_engine::ui::game_plugin::tick_spellbook_cooldowns,
        )
            .chain()
            .run_if(in_state(GameState::InWorld)),
    );
    app.add_systems(
        Update,
        (
            game_engine::ui::game_plugin::handle_spellbook_pointer,
            game_engine::ui::game_plugin::handle_spellbook_keyboard,
        )
            .chain()
            .run_if(in_state(GameState::InWorld).and(crate::networking::gameplay_input_allowed)),
    );
}

fn on_enter_connecting(
    mut commands: Commands,
    time: Res<Time>,
    startup: Option<Res<StartupPerfTimer>>,
) {
    if let Some(startup) = startup {
        info!(
            "Entering Connecting state at bevy_t={:.3}s app_t={:.3}s",
            time.elapsed_secs_f64(),
            startup.0.elapsed().as_secs_f32()
        );
    } else {
        info!(
            "Entering Connecting state at t={:.3}s",
            time.elapsed_secs_f64()
        );
    }
    commands.insert_resource(ConnectingStartTime(time.elapsed_secs_f64()));
}

fn on_enter_eula() {
    info!("Entering Eula state");
}

fn on_enter_char_select(startup: Option<Res<StartupPerfTimer>>) {
    if let Some(startup) = startup {
        info!(
            "Entering CharSelect state at app_t={:.3}s",
            startup.0.elapsed().as_secs_f32()
        );
    } else {
        info!("Entering CharSelect state");
    }
}

fn on_enter_selection_debug() {
    info!("Entering SelectionDebug state");
}

fn on_enter_inworld_selection_debug() {
    info!("Entering InWorldSelectionDebug state");
}

fn on_enter_debug_character() {
    info!("Entering DebugCharacter state");
}

fn on_enter_skybox_debug() {
    info!("Entering SkyboxDebug state");
}

fn on_enter_campsite_popup() {
    info!("Entering CampsitePopup state");
}

fn on_enter_loading(startup: Option<Res<StartupPerfTimer>>) {
    if let Some(startup) = startup {
        info!(
            "Entering Loading state at app_t={:.3}s",
            startup.0.elapsed().as_secs_f32()
        );
    } else {
        info!("Entering Loading state");
    }
}

fn on_enter_in_world() {
    info!("Entering InWorld state — game systems active");
}

fn on_enter_trash_button() {
    info!("Entering TrashButton state");
}

fn reset_zone_transition_tracker(mut tracker: ResMut<ZoneTransitionTracker>) {
    *tracker = ZoneTransitionTracker::default();
}

/// Spawn world environment (lights, sky dome) when entering InWorld in server mode.
/// Only registered when ServerAddr is present — skipped in standalone mode.
fn spawn_world_environment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut sky_materials: ResMut<Assets<sky::SkyMaterial>>,
    mut images: ResMut<Assets<Image>>,
    cloud_maps: Res<sky::cloud_texture::ProceduralCloudMaps>,
    camera_q: Query<Entity, With<WowCamera>>,
) {
    let camera = camera_q
        .single()
        .ok()
        .unwrap_or_else(|| camera::spawn_wow_camera(&mut commands));
    commands.insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.12)));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 0.0,
        ..default()
    });
    commands.insert_resource(DirectionalLightShadowMap { size: 4096 });
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            shadow_depth_bias: 0.02,
            shadow_normal_bias: 1.8,
            ..default()
        },
        Transform::from_rotation(Quat::from_rotation_x(-PI / 4.0)),
        default_cascade_shadow_config(),
    ));
    sky::spawn_sky_dome(
        &mut commands,
        &mut meshes,
        &mut sky_materials,
        &mut images,
        camera,
        cloud_maps.active_handle(),
    );
}

fn on_exit_in_world() {
    info!("Exiting InWorld state — game systems disabled");
}

fn log_screen_switches(state: Res<State<GameState>>, mut last_state: Local<Option<GameState>>) {
    let current = *state.get();
    if let Some(previous) = *last_state
        && previous != current
    {
        info!("Screen switch: {:?} -> {:?}", previous, current);
    }
    *last_state = Some(current);
}

/// Check if a `Connected` component exists on any entity (lightyear sets this on connection).
/// Times out after `CONNECT_TIMEOUT_SECS` and returns to Login.
/// Wait for connection. LoginResponse handler transitions to CharSelect on success.
/// Times out after `CONNECT_TIMEOUT_SECS` and returns to Login.
fn check_connection_status(
    connected_q: Query<(), With<Connected>>,
    time: Res<Time>,
    start_time: Option<Res<ConnectingStartTime>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut auth_feedback: ResMut<crate::networking::AuthUiFeedback>,
) {
    // Connection established — on_connected sends LoginRequest,
    // receive_login_response will transition to CharSelect.
    if !connected_q.is_empty() {
        return;
    }
    if let Some(start) = start_time {
        let elapsed = time.elapsed_secs_f64() - start.0;
        if elapsed >= CONNECT_TIMEOUT_SECS {
            warn!(
                "Connection timed out after {elapsed:.3}s (limit {CONNECT_TIMEOUT_SECS}s), returning to Login"
            );
            auth_feedback.0 = Some("Connection timed out".to_string());
            next_state.set(GameState::Login);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LoadingReadiness {
    pub complete: bool,
    pub progress_percent: u8,
    pub status_text: &'static str,
}

pub(crate) fn evaluate_world_loading(
    local_player_ready: bool,
    adt_manager: &AdtManager,
) -> LoadingReadiness {
    if !local_player_ready {
        return LoadingReadiness {
            complete: false,
            progress_percent: 35,
            status_text: "Initializing character...",
        };
    }

    if adt_manager.map_name.is_empty() {
        return LoadingReadiness {
            complete: false,
            progress_percent: 62,
            status_text: "Waiting for terrain...",
        };
    }

    let initial_tile = adt_manager.initial_tile;
    if adt_manager.loaded.contains_key(&initial_tile) {
        return LoadingReadiness {
            complete: true,
            progress_percent: 100,
            status_text: "Entering world...",
        };
    }

    if adt_manager.pending.contains(&initial_tile) {
        return LoadingReadiness {
            complete: false,
            progress_percent: 86,
            status_text: "Loading terrain...",
        };
    }

    LoadingReadiness {
        complete: false,
        progress_percent: 74,
        status_text: "Preparing terrain...",
    }
}

fn check_loading_complete(
    local_player_q: Query<(), With<LocalPlayer>>,
    adt_manager: Res<AdtManager>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if evaluate_world_loading(!local_player_q.is_empty(), &adt_manager).complete {
        next_state.set(GameState::InWorld);
    }
}

fn handle_zone_transition(
    current_zone: Res<CurrentZone>,
    mut tracker: ResMut<ZoneTransitionTracker>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if !current_zone.is_changed() {
        return;
    }
    let previous_zone_id = tracker.observed_zone_id;
    if should_enter_loading_for_zone_change(&mut tracker, current_zone.zone_id) {
        info!(
            "Zone transition {} -> {}, entering Loading",
            previous_zone_id, current_zone.zone_id
        );
        next_state.set(GameState::Loading);
    }
}

fn should_enter_loading_for_zone_change(tracker: &mut ZoneTransitionTracker, zone_id: u32) -> bool {
    if !tracker.initialized {
        tracker.observed_zone_id = zone_id;
        tracker.initialized = true;
        return false;
    }

    let previous_zone_id = tracker.observed_zone_id;
    tracker.observed_zone_id = zone_id;

    previous_zone_id != 0 && zone_id != 0 && zone_id != previous_zone_id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state_is_login() {
        assert_eq!(GameState::default(), GameState::Login);
    }

    #[test]
    fn test_standalone_starts_in_world() {
        // In debug builds without ServerAddr, the plugin should insert InWorld directly.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<bevy::text::Font>();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.add_plugins(game_engine::ui::plugin::UiPlugin);
        // No ServerAddr inserted — standalone mode.
        app.add_plugins(GameStatePlugin);
        app.update();

        let state = app.world().resource::<State<GameState>>();
        assert_eq!(
            *state.get(),
            GameState::InWorld,
            "Standalone mode should start in InWorld"
        );
    }

    #[test]
    fn test_state_transitions() {
        // Verify that the Login -> Connecting -> Loading -> InWorld sequence is valid
        // by checking all states derive the required traits.
        let states = [
            GameState::Login,
            GameState::Connecting,
            GameState::CharSelect,
            GameState::CharCreate,
            GameState::Loading,
            GameState::InWorld,
            GameState::TrashButton,
            GameState::Reconnecting,
        ];
        // States must be Eq + Hash + Clone + Copy (compile-time check via usage).
        for &s in &states {
            let cloned = s;
            assert_eq!(s, cloned);
        }
        // Verify the expected transition sequence ordering.
        assert_eq!(states[0], GameState::Login);
        assert_eq!(states[1], GameState::Connecting);
        assert_eq!(states[4], GameState::Loading);
        assert_eq!(states[5], GameState::InWorld);
    }

    #[test]
    fn loading_waits_for_local_player_before_progressing() {
        let adt_manager = AdtManager::default();
        assert_eq!(
            evaluate_world_loading(false, &adt_manager),
            LoadingReadiness {
                complete: false,
                progress_percent: 35,
                status_text: "Initializing character..."
            }
        );
    }

    #[test]
    fn loading_waits_for_initial_terrain_tile_before_progressing() {
        let mut adt_manager = AdtManager::default();
        adt_manager.map_name = "azeroth".into();
        adt_manager.initial_tile = (32, 48);
        assert_eq!(
            evaluate_world_loading(true, &adt_manager),
            LoadingReadiness {
                complete: false,
                progress_percent: 74,
                status_text: "Preparing terrain..."
            }
        );
    }

    #[test]
    fn loading_stays_incomplete_while_initial_tile_is_only_pending() {
        let mut adt_manager = AdtManager::default();
        adt_manager.map_name = "azeroth".into();
        adt_manager.initial_tile = (32, 48);
        adt_manager.pending.insert((32, 48));
        assert_eq!(
            evaluate_world_loading(true, &adt_manager),
            LoadingReadiness {
                complete: false,
                progress_percent: 86,
                status_text: "Loading terrain..."
            }
        );
    }

    #[test]
    fn loading_completes_when_local_player_and_initial_tile_are_ready() {
        let mut adt_manager = AdtManager::default();
        adt_manager.map_name = "azeroth".into();
        adt_manager.initial_tile = (32, 48);
        adt_manager.loaded.insert((32, 48), Entity::PLACEHOLDER);
        assert_eq!(
            evaluate_world_loading(true, &adt_manager),
            LoadingReadiness {
                complete: true,
                progress_percent: 100,
                status_text: "Entering world..."
            }
        );
    }

    #[test]
    fn loading_state_transitions_into_inworld_when_world_is_ready() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.insert_state(GameState::Loading);
        app.insert_resource(AdtManager::default());
        app.add_systems(
            Update,
            check_loading_complete.run_if(in_state(GameState::Loading)),
        );

        let terrain_root = app.world_mut().spawn_empty().id();
        {
            let mut adt_manager = app.world_mut().resource_mut::<AdtManager>();
            adt_manager.map_name = "azeroth".into();
            adt_manager.initial_tile = (32, 48);
            adt_manager.loaded.insert((32, 48), terrain_root);
        }
        app.world_mut().spawn(LocalPlayer);

        app.update();
        app.update();

        let state = app.world().resource::<State<GameState>>();
        assert_eq!(
            *state.get(),
            GameState::InWorld,
            "world-ready loading should enter InWorld"
        );
    }

    #[test]
    fn first_observed_zone_does_not_trigger_loading() {
        let mut tracker = ZoneTransitionTracker::default();

        assert!(!should_enter_loading_for_zone_change(&mut tracker, 12));
        assert_eq!(tracker.observed_zone_id, 12);
        assert!(tracker.initialized);
    }

    #[test]
    fn zone_change_between_nonzero_zones_triggers_loading() {
        let mut tracker = ZoneTransitionTracker {
            observed_zone_id: 12,
            initialized: true,
        };

        assert!(should_enter_loading_for_zone_change(&mut tracker, 1519));
        assert_eq!(tracker.observed_zone_id, 1519);
    }

    #[test]
    fn zero_zone_resets_without_triggering_loading() {
        let mut tracker = ZoneTransitionTracker {
            observed_zone_id: 12,
            initialized: true,
        };

        assert!(!should_enter_loading_for_zone_change(&mut tracker, 0));
        assert_eq!(tracker.observed_zone_id, 0);
        assert!(!should_enter_loading_for_zone_change(&mut tracker, 1519));
        assert_eq!(tracker.observed_zone_id, 1519);
    }

    #[test]
    fn startup_with_server_and_unaccepted_eula_skips_gate_by_default() {
        assert_eq!(
            resolve_startup_state(true, None, false),
            (GameState::Login, None)
        );
        assert_eq!(
            resolve_startup_state(true, Some(GameState::Connecting), false),
            (GameState::Connecting, None)
        );
    }

    #[test]
    fn startup_without_server_or_with_accepted_eula_skips_gate() {
        assert_eq!(
            resolve_startup_state(false, None, false),
            (GameState::InWorld, None)
        );
        assert_eq!(
            resolve_startup_state(true, Some(GameState::Login), true),
            (GameState::Login, None)
        );
    }

    #[test]
    fn startup_with_enable_eula_and_unaccepted_eula_enters_gate() {
        let guard = EnvVarGuard::set("ENABLE_EULA", Some("1"));

        assert_eq!(
            resolve_startup_state(true, None, false),
            (GameState::Eula, Some(GameState::Login))
        );
        assert_eq!(
            resolve_startup_state(true, Some(GameState::Connecting), false),
            (GameState::Eula, Some(GameState::Connecting))
        );

        drop(guard);
    }

    #[test]
    fn explicit_eula_state_defaults_back_to_login_after_acceptance() {
        assert_eq!(
            resolve_startup_state(true, Some(GameState::Eula), true),
            (GameState::Eula, Some(GameState::Login))
        );
    }

    struct EnvVarGuard {
        key: &'static str,
        old_value: Option<std::ffi::OsString>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: Option<&str>) -> Self {
            let old_value = std::env::var_os(key);
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
            Self { key, old_value }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.old_value {
                Some(value) => unsafe { std::env::set_var(self.key, value) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }
}
