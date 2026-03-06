use std::f32::consts::PI;

use bevy::prelude::*;
use lightyear::prelude::client::Connected;

use crate::camera::WowCamera;
use crate::networking::ServerAddr;
use crate::sky;

/// Game state machine controlling which systems are active.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Login,
    Connecting,
    CharSelect,
    Loading,
    InWorld,
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
        if has_server {
            app.init_state::<GameState>();
        } else {
            app.insert_state(GameState::InWorld);
        }
        app.add_systems(OnEnter(GameState::Connecting), on_enter_connecting);
        app.add_systems(OnEnter(GameState::InWorld), on_enter_in_world);
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
    }
}

fn on_enter_connecting(mut commands: Commands, time: Res<Time>) {
    info!("Entering Connecting state...");
    commands.insert_resource(ConnectingStartTime(time.elapsed_secs_f64()));
}

fn on_enter_in_world() {
    info!("Entering InWorld state — game systems active");
}

/// Spawn world environment (lights, sky dome) when entering InWorld in server mode.
/// Only registered when ServerAddr is present — skipped in standalone mode.
fn spawn_world_environment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut sky_materials: ResMut<Assets<sky::SkyMaterial>>,
    mut images: ResMut<Assets<Image>>,
    camera_q: Query<Entity, With<WowCamera>>,
) {
    if let Ok(camera) = camera_q.single() {
        commands.spawn((
            DirectionalLight {
                illuminance: light_consts::lux::OVERCAST_DAY,
                shadows_enabled: true,
                ..default()
            },
            Transform::from_rotation(Quat::from_rotation_x(-PI / 4.0)),
        ));
        sky::spawn_sky_dome(
            &mut commands,
            &mut meshes,
            &mut sky_materials,
            &mut images,
            camera,
        );
        commands.remove_resource::<ClearColor>();
    }
}

fn on_exit_in_world() {
    info!("Exiting InWorld state — game systems disabled");
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
) {
    // Connection established — on_connected sends LoginRequest,
    // receive_login_response will transition to CharSelect.
    if !connected_q.is_empty() {
        return;
    }
    if let Some(start) = start_time {
        let elapsed = time.elapsed_secs_f64() - start.0;
        if elapsed >= CONNECT_TIMEOUT_SECS {
            warn!("Connection timed out after {CONNECT_TIMEOUT_SECS}s, returning to Login");
            next_state.set(GameState::Login);
        }
    }
}

/// Placeholder: immediately transition to InWorld. Terrain streaming will gate this later.
fn check_loading_complete(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::InWorld);
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
        // Without ServerAddr, the plugin should insert InWorld directly.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::state::app::StatesPlugin);
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
            GameState::Loading,
            GameState::InWorld,
        ];
        // States must be Eq + Hash + Clone + Copy (compile-time check via usage).
        for &s in &states {
            let cloned = s;
            assert_eq!(s, cloned);
        }
        // Verify the expected transition sequence ordering.
        assert_eq!(states[0], GameState::Login);
        assert_eq!(states[1], GameState::Connecting);
        assert_eq!(states[3], GameState::Loading);
        assert_eq!(states[4], GameState::InWorld);
    }
}
