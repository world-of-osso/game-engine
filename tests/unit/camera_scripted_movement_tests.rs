use super::*;
use game_engine::movement_control::ScriptedMovement;

fn movement_app() -> (App, Entity) {
    let mut app = App::new();
    app.init_resource::<Time>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<InputBindings>()
        .init_resource::<PathingState>()
        .init_resource::<game_engine::status::MapStatusSnapshot>()
        .init_resource::<ScriptedMovement>()
        .init_resource::<Assets<Mesh>>()
        .add_systems(Update, player_movement);
    let player = app
        .world_mut()
        .spawn((
            Player,
            Transform::default(),
            MovementState::default(),
            CharacterFacing::default(),
            CharacterPhysics::default(),
        ))
        .id();
    (app, player)
}

fn advance_movement(app: &mut App, seconds: f32) {
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs_f32(seconds));
    app.update();
}

fn position(app: &App, player: Entity) -> Vec3 {
    app.world().get::<Transform>(player).unwrap().translation
}

#[test]
fn scripted_forward_moves_for_exact_duration_then_stops() {
    let (mut app, player) = movement_app();
    app.world_mut()
        .resource_mut::<ScriptedMovement>()
        .start(0.25, Some(90.0))
        .unwrap();
    for expected_x in [0.7, 1.4, 1.75] {
        advance_movement(&mut app, 0.1);
        let actual = position(&app, player);
        assert!(
            (actual.x - expected_x).abs() < 0.001,
            "expected x={expected_x}, got {actual:?}"
        );
        assert!(actual.z.abs() < 0.001);
        let movement = app.world().get::<MovementState>(player).unwrap();
        let facing = app.world().get::<CharacterFacing>(player).unwrap();
        let input = crate::networking::movement_to_direction(movement, facing);
        assert!(
            (input[0] - 1.0).abs() < 0.001,
            "script must use normal network movement input"
        );
    }
    advance_movement(&mut app, 0.1);
    assert!((position(&app, player).x - 1.75).abs() < 0.001);
    assert_eq!(
        app.world().get::<MovementState>(player).unwrap().direction,
        MoveDirection::None
    );
}

#[test]
fn scripted_forward_uses_normal_doodad_collision() {
    let (mut app, player) = movement_app();
    let geometry = crate::asset::m2::M2CollisionMesh {
        bounds_min: [0.8, -1.0, -1.0],
        bounds_max: [0.8, 1.0, 2.0],
        vertices: vec![
            [0.8, -1.0, -1.0],
            [0.8, 1.0, -1.0],
            [0.8, 1.0, 2.0],
            [0.8, -1.0, 2.0],
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
    };
    app.world_mut()
        .spawn(game_engine::culling::DoodadCollider::new(
            std::sync::Arc::new(geometry),
            &Transform::default(),
        ));
    app.world_mut()
        .resource_mut::<ScriptedMovement>()
        .start(1.0, Some(90.0))
        .unwrap();
    advance_movement(&mut app, 0.2);
    assert!(
        (position(&app, player).x - 0.75).abs() < 0.001,
        "script must stop at normal collision margin"
    );
}

#[test]
fn manual_movement_cancels_scripted_heading_and_duration() {
    let (mut app, player) = movement_app();
    app.world_mut()
        .resource_mut::<ScriptedMovement>()
        .start(1.0, Some(90.0))
        .unwrap();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyW);
    advance_movement(&mut app, 0.1);
    let actual = position(&app, player);
    assert!(
        actual.x.abs() < 0.001,
        "scripted heading must not override manual movement"
    );
    assert!((actual.z + 0.7).abs() < 0.001);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyW);
    advance_movement(&mut app, 0.1);
    assert_eq!(
        position(&app, player),
        actual,
        "cancelled script must not resume"
    );
}

#[test]
fn opening_modal_cancels_script_without_resuming_after_close() {
    let (mut app, player) = movement_app();
    app.world_mut()
        .resource_mut::<ScriptedMovement>()
        .start(1.0, Some(90.0))
        .unwrap();
    app.world_mut()
        .insert_resource(crate::scenes::game_menu::UiModalOpen);
    advance_movement(&mut app, 0.1);
    assert_eq!(position(&app, player), Vec3::ZERO);
    app.world_mut()
        .remove_resource::<crate::scenes::game_menu::UiModalOpen>();
    advance_movement(&mut app, 0.1);
    assert_eq!(
        position(&app, player),
        Vec3::ZERO,
        "closing modal must not restart cancelled script"
    );
}

#[test]
fn reconnect_cancels_script_without_resuming_after_recovery() {
    let (mut app, player) = movement_app();
    app.world_mut()
        .resource_mut::<ScriptedMovement>()
        .start(1.0, Some(90.0))
        .unwrap();
    app.world_mut()
        .insert_resource(crate::networking::ReconnectState {
            phase: crate::networking::ReconnectPhase::PendingConnect,
            terrain_refresh_seen: false,
        });
    advance_movement(&mut app, 0.1);
    app.world_mut()
        .remove_resource::<crate::networking::ReconnectState>();
    advance_movement(&mut app, 0.1);
    assert_eq!(
        position(&app, player),
        Vec3::ZERO,
        "recovery must not restart cancelled script"
    );
}

#[test]
fn leaving_inworld_cancels_scripted_movement() {
    let mut app = App::new();
    app.add_plugins(bevy::state::app::StatesPlugin)
        .init_state::<GameState>();
    register_scripted_movement(&mut app);
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::InWorld);
    app.update();
    app.world_mut()
        .resource_mut::<ScriptedMovement>()
        .start(1.0, Some(90.0))
        .unwrap();
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Login);
    app.update();
    assert!(
        app.world_mut()
            .resource_mut::<ScriptedMovement>()
            .next_step(0.1)
            .is_none()
    );
}
