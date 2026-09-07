use super::*;
use crate::camera::Player;
use game_engine::network_tick::{NetworkTick, NetworkTickPlugin, NetworkTickSystems};

fn scheduling_app(state: crate::game_state::GameState) -> App {
    let mut app = App::new();
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.insert_state(state);
    app.add_plugins(NetworkTickPlugin);
    app.init_resource::<Time>();
    app.init_resource::<crate::rendering::sky::GameTime>();
    app.init_resource::<LocalAliveState>();
    app.init_resource::<CharacterList>();
    app.insert_resource(SelectedCharacterId {
        character_name: Some("Alice".into()),
        ..default()
    });
    register_inworld_replication_systems(&mut app);
    register_entity_tag_systems(&mut app);
    app
}

fn alice() -> NetPlayer {
    NetPlayer {
        name: "Alice".into(),
        race: 1,
        class: 2,
        appearance: default(),
    }
}

fn render_only(app: &mut App) {
    // Real time is unchanged: the production Update driver has no network ticks due.
    for _ in 0..3 {
        app.world_mut().run_schedule(Update);
    }
}

#[test]
fn player_scheduling_tags_received_player_on_network_tick_not_render_frames() {
    for state in [
        crate::game_state::GameState::Loading,
        crate::game_state::GameState::InWorld,
    ] {
        let mut app = scheduling_app(state);
        app.add_systems(
            NetworkTick,
            (|mut commands: Commands, mut sent: Local<bool>| {
                if !*sent {
                    commands.spawn((Remote, alice()));
                    *sent = true;
                }
            })
            .in_set(NetworkTickSystems::Receive),
        );
        app.world_mut().run_schedule(NetworkTick);
        let entity = app
            .world_mut()
            .query_filtered::<Entity, With<NetPlayer>>()
            .single(app.world())
            .unwrap();
        assert!(app.world().get::<LocalPlayer>(entity).is_some());

        app.world_mut()
            .resource_mut::<SelectedCharacterId>()
            .character_name = Some("Bob".into());
        let bob = app
            .world_mut()
            .spawn((
                Remote,
                NetPlayer {
                    name: "Bob".into(),
                    ..alice()
                },
            ))
            .id();
        render_only(&mut app);
        assert!(app.world().get::<LocalPlayer>(bob).is_none());
        assert!(app.world().get::<LocalPlayer>(entity).is_some());
        app.world_mut().run_schedule(NetworkTick);
        assert!(app.world().get::<LocalPlayer>(bob).is_some());
        assert!(app.world().get::<LocalPlayer>(entity).is_none());
    }
}

#[test]
fn player_scheduling_mount_changes_wait_for_network_tick() {
    let mut app = scheduling_app(crate::game_state::GameState::InWorld);
    let parent = app
        .world_mut()
        .spawn((Player, MovementState::default()))
        .id();
    let child = app
        .world_mut()
        .spawn((
            crate::networking_player::MountedVisualRoot,
            MovementState::default(),
            ChildOf(parent),
        ))
        .id();
    app.world_mut().run_schedule(NetworkTick);
    app.world_mut()
        .get_mut::<MovementState>(parent)
        .unwrap()
        .direction = MoveDirection::Forward;
    render_only(&mut app);
    assert_eq!(
        app.world().get::<MovementState>(child).unwrap().direction,
        MoveDirection::None
    );
    app.world_mut().run_schedule(NetworkTick);
    assert_eq!(
        app.world().get::<MovementState>(child).unwrap().direction,
        MoveDirection::Forward
    );
    let changed = app
        .world()
        .entity(child)
        .get_ref::<MovementState>()
        .unwrap()
        .last_changed();
    render_only(&mut app);
    app.world_mut().run_schedule(NetworkTick);
    assert_eq!(
        app.world()
            .entity(child)
            .get_ref::<MovementState>()
            .unwrap()
            .last_changed(),
        changed
    );
}

#[test]
fn player_scheduling_alive_changes_wait_for_network_tick() {
    for state in [
        crate::game_state::GameState::Loading,
        crate::game_state::GameState::InWorld,
    ] {
        let mut app = scheduling_app(state);
        let entity = app
            .world_mut()
            .spawn((
                LocalPlayer,
                NetHealth {
                    current: 10.0,
                    max: 10.0,
                },
            ))
            .id();
        app.world_mut().run_schedule(NetworkTick);
        app.world_mut()
            .get_mut::<NetHealth>(entity)
            .unwrap()
            .current = 0.0;
        render_only(&mut app);
        assert!(app.world().resource::<LocalAliveState>().0);
        app.world_mut().run_schedule(NetworkTick);
        assert!(!app.world().resource::<LocalAliveState>().0);
        app.world_mut().entity_mut(entity).remove::<NetHealth>();
        render_only(&mut app);
        assert!(!app.world().resource::<LocalAliveState>().0);
        app.world_mut().run_schedule(NetworkTick);
        assert!(app.world().resource::<LocalAliveState>().0);
    }
}
