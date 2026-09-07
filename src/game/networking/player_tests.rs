use super::*;
use bevy::ecs::system::RunSystemOnce;

#[test]
fn idle_mount_movement_only_changes_for_parent_updates_or_new_roots() {
    let mut app = App::new();
    app.init_resource::<LocalAliveStateChangeCount>();
    app.add_systems(
        Update,
        (
            sync_local_mount_visual_movement,
            |changed: Query<(), (With<MountedVisualRoot>, Changed<MovementState>)>,
             mut count: ResMut<LocalAliveStateChangeCount>| {
                count.0 += changed.iter().count();
            },
        )
            .chain(),
    );
    let parent = app
        .world_mut()
        .spawn((Player, MovementState::default()))
        .id();
    let child = app
        .world_mut()
        .spawn((MountedVisualRoot, MovementState::default(), ChildOf(parent)))
        .id();
    app.update();
    assert_eq!(app.world().resource::<LocalAliveStateChangeCount>().0, 1);
    app.update();
    assert_eq!(app.world().resource::<LocalAliveStateChangeCount>().0, 1);
    app.world_mut()
        .get_mut::<MovementState>(parent)
        .unwrap()
        .direction = MoveDirection::Forward;
    app.update();
    assert_eq!(
        app.world().get::<MovementState>(child).unwrap().direction,
        MoveDirection::Forward
    );
    let late = app
        .world_mut()
        .spawn((MountedVisualRoot, MovementState::default(), ChildOf(parent)))
        .id();
    app.update();
    assert_eq!(
        app.world().get::<MovementState>(late).unwrap().direction,
        MoveDirection::Forward
    );
}

#[test]
fn idle_player_tagging_waits_for_replica_or_selection_changes() {
    let mut app = App::new();
    app.add_systems(Update, tag_local_player);
    app.insert_resource(SelectedCharacterId {
        character_name: Some("Alice".into()),
        ..default()
    });
    let mut player = sample_player();
    player.name = "Bob".into();
    let entity = app.world_mut().spawn((Remote, player)).id();
    app.update();
    app.world_mut()
        .get_mut::<NetPlayer>(entity)
        .unwrap()
        .bypass_change_detection()
        .name = "Alice".into();
    app.update();
    assert!(app.world().get::<LocalPlayer>(entity).is_none());
    app.world_mut()
        .get_mut::<NetPlayer>(entity)
        .unwrap()
        .set_changed();
    app.update();
    assert!(app.world().get::<LocalPlayer>(entity).is_some());
    let bob = app
        .world_mut()
        .spawn((
            Remote,
            NetPlayer {
                name: "Bob".into(),
                ..sample_player()
            },
        ))
        .id();
    app.update();
    app.world_mut()
        .resource_mut::<SelectedCharacterId>()
        .character_name = Some("Bob".into());
    app.update();
    assert!(app.world().get::<LocalPlayer>(entity).is_none());
    assert!(app.world().get::<LocalPlayer>(bob).is_some());
}

#[test]
fn idle_alive_state_waits_for_health_changes_and_handles_removal() {
    let mut app = App::new();
    app.init_resource::<LocalAliveState>();
    app.add_systems(Update, sync_local_alive_state);
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
    app.update();
    app.world_mut()
        .get_mut::<NetHealth>(entity)
        .unwrap()
        .bypass_change_detection()
        .current = 0.0;
    app.update();
    assert!(app.world().resource::<LocalAliveState>().0);
    app.world_mut()
        .get_mut::<NetHealth>(entity)
        .unwrap()
        .set_changed();
    app.update();
    assert!(!app.world().resource::<LocalAliveState>().0);
    app.world_mut().entity_mut(entity).remove::<NetHealth>();
    app.update();
    assert!(app.world().resource::<LocalAliveState>().0);
    app.world_mut().entity_mut(entity).insert(NetHealth {
        current: 0.0,
        max: 10.0,
    });
    app.update();
    assert!(!app.world().resource::<LocalAliveState>().0);
    app.world_mut().entity_mut(entity).remove::<LocalPlayer>();
    app.update();
    assert!(app.world().resource::<LocalAliveState>().0);
}

#[test]
fn player_tagging_preserves_existing_duplicate_and_selects_newest_initial_match() {
    let mut app = App::new();
    app.add_systems(Update, tag_local_player);
    app.insert_resource(SelectedCharacterId {
        character_name: Some("Alice".into()),
        ..default()
    });
    let first = app.world_mut().spawn((Remote, sample_player())).id();
    let second = app.world_mut().spawn((Remote, sample_player())).id();
    app.update();
    let chosen = [first, second]
        .into_iter()
        .max_by_key(|entity| entity.to_bits())
        .unwrap();
    assert!(app.world().get::<LocalPlayer>(chosen).is_some());
    let late = app.world_mut().spawn((Remote, sample_player())).id();
    app.update();
    assert!(app.world().get::<LocalPlayer>(chosen).is_some());
    assert!(app.world().get::<LocalPlayer>(late).is_none());
}

#[test]
fn alive_state_tracks_late_local_marker_and_despawn() {
    let mut app = App::new();
    app.init_resource::<LocalAliveState>();
    app.add_systems(Update, sync_local_alive_state);
    let entity = app
        .world_mut()
        .spawn(NetHealth {
            current: 0.0,
            max: 10.0,
        })
        .id();
    app.update();
    assert!(app.world().resource::<LocalAliveState>().0);
    app.world_mut().entity_mut(entity).insert(LocalPlayer);
    app.update();
    assert!(!app.world().resource::<LocalAliveState>().0);
    app.world_mut().despawn(entity);
    app.update();
    assert!(app.world().resource::<LocalAliveState>().0);
}

pub(super) fn sample_player() -> NetPlayer {
    NetPlayer {
        name: "Alice".into(),
        race: 1,
        class: 2,
        appearance: Default::default(),
    }
}

#[test]
fn desired_player_visual_prefers_mount_when_present() {
    let mounted = Mounted {
        mount_display_id: 101,
    };
    assert_eq!(
        desired_player_visual(Some(&mounted)),
        DesiredPlayerVisual::Mount {
            mount_display_id: 101
        }
    );
    assert_eq!(desired_player_visual(None), DesiredPlayerVisual::Character);
}

#[test]
fn applied_visual_state_includes_mount_display_id() {
    let selection = net_player_customization_selection(&sample_player());
    let applied = AppliedPlayerAppearance {
        selection,
        equipment: NetEquipmentAppearance::default(),
        mount_display_id: Some(202),
    };

    assert_eq!(applied.mount_display_id, Some(202));
}

#[test]
fn local_mount_visual_root_copies_parent_movement_state() {
    let mut app = App::new();
    let parent = app
        .world_mut()
        .spawn((
            Player,
            MovementState {
                direction: MoveDirection::Forward,
                running: false,
                jumping: true,
                autorun: true,
                swimming: false,
            },
        ))
        .id();
    let child = app
        .world_mut()
        .spawn((MountedVisualRoot, MovementState::default(), ChildOf(parent)))
        .id();

    app.world_mut()
        .run_system_once(sync_local_mount_visual_movement)
        .expect("sync mount movement");

    let movement = app
        .world()
        .entity(child)
        .get::<MovementState>()
        .expect("mounted visual movement");
    assert_eq!(movement.direction, MoveDirection::Forward);
    assert!(!movement.running);
    assert!(movement.jumping);
    assert!(movement.autorun);
}

#[derive(Resource, Default)]
struct LocalAliveStateChangeCount(usize);

fn count_local_alive_state_changes(
    local_alive: Res<LocalAliveState>,
    mut count: ResMut<LocalAliveStateChangeCount>,
) {
    if local_alive.is_changed() {
        count.0 += 1;
    }
}

#[test]
fn event_driven_npc_visibility_sync_ignores_unchanged_health() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<LocalAliveState>();
    app.init_resource::<LocalAliveStateChangeCount>();
    app.add_systems(
        Update,
        (sync_local_alive_state, count_local_alive_state_changes).chain(),
    );
    app.world_mut().spawn((
        LocalPlayer,
        NetHealth {
            current: 100.0,
            max: 100.0,
        },
    ));

    app.update();
    app.world_mut()
        .resource_mut::<LocalAliveStateChangeCount>()
        .0 = 0;
    app.update();

    assert_eq!(
        app.world().resource::<LocalAliveStateChangeCount>().0,
        0,
        "unchanged health must not emit a LocalAliveState change"
    );
    assert!(app.world().resource::<LocalAliveState>().0);
}
