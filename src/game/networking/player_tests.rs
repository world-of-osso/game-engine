use super::*;
use bevy::ecs::system::RunSystemOnce;

fn sample_player() -> NetPlayer {
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
