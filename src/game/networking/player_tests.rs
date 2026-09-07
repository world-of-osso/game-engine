use super::*;
use bevy::ecs::system::RunSystemOnce;

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
