use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

use super::*;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        50,
    )));
    app.add_systems(Update, follow_server_movement);
    app
}

fn local_player(app: &mut App, at: Vec3, server: Vec3) -> Entity {
    app.world_mut()
        .spawn((
            LocalPlayer,
            MovementControl::default(),
            NetPosition {
                x: server.x,
                y: server.y,
                z: server.z,
            },
            Transform::from_translation(at),
            CharacterFacing::default(),
        ))
        .id()
}

fn translation(app: &App, entity: Entity) -> Vec3 {
    app.world().get::<Transform>(entity).unwrap().translation
}

#[test]
fn the_local_player_keeps_its_own_position_until_the_server_teleports_it() {
    let mut app = app();
    let player = local_player(&mut app, Vec3::new(1.0, 0.0, 1.0), Vec3::ZERO);
    app.update();
    app.update();
    assert_eq!(translation(&app, player), Vec3::new(1.0, 0.0, 1.0));

    // Spirit release: the server moved the ghost to its graveyard.
    let graveyard = Vec3::new(-9000.0, 80.0, 400.0);
    *app.world_mut().get_mut::<NetPosition>(player).unwrap() = NetPosition {
        x: graveyard.x,
        y: graveyard.y,
        z: graveyard.z,
    };
    app.world_mut()
        .get_mut::<MovementControl>(player)
        .unwrap()
        .epoch = 1;
    app.update();

    assert_eq!(translation(&app, player), graveyard);
    assert_eq!(
        app.world().get::<AdoptedMovementEpoch>(player),
        Some(&AdoptedMovementEpoch(1))
    );
}

#[test]
fn a_controlled_player_follows_the_server_position_and_facing() {
    let mut app = app();
    let player = local_player(&mut app, Vec3::ZERO, Vec3::ZERO);
    app.update();
    app.world_mut().entity_mut(player).insert(NetRotation {
        x: 0.0,
        y: 1.5,
        z: 0.0,
    });
    app.world_mut()
        .get_mut::<MovementControl>(player)
        .unwrap()
        .controlled = true;
    *app.world_mut().get_mut::<NetPosition>(player).unwrap() = NetPosition {
        x: 10.0,
        y: 5.0,
        z: 0.0,
    };

    for _ in 0..30 {
        app.update();
    }

    assert!(translation(&app, player).distance(Vec3::new(10.0, 5.0, 0.0)) < 0.01);
    assert_eq!(app.world().get::<CharacterFacing>(player).unwrap().yaw, 1.5);
}
