use super::*;

#[test]
fn test_tab_cycles_targets() {
    let mut app = game_engine::test_harness::headless_app();
    app.init_resource::<CurrentTarget>();

    let _player = app
        .world_mut()
        .spawn((Transform::from_xyz(0.0, 0.0, 0.0), Player))
        .id();

    let e1 = app
        .world_mut()
        .spawn((Transform::from_xyz(5.0, 0.0, 0.0), RemoteEntity))
        .id();
    let e2 = app
        .world_mut()
        .spawn((Transform::from_xyz(10.0, 0.0, 0.0), RemoteEntity))
        .id();
    let e3 = app
        .world_mut()
        .spawn((Transform::from_xyz(15.0, 0.0, 0.0), RemoteEntity))
        .id();

    let sorted = vec![e1, e2, e3];
    let t1 = pick_next_target(&sorted, None);
    assert_eq!(t1, Some(e1));
    let t2 = pick_next_target(&sorted, t1);
    assert_eq!(t2, Some(e2));
    let t3 = pick_next_target(&sorted, t2);
    assert_eq!(t3, Some(e3));
    let t4 = pick_next_target(&sorted, t3);
    assert_eq!(t4, Some(e1));
}

#[test]
fn test_tab_target_ignores_non_npc_remote_entities() {
    let mut app = game_engine::test_harness::headless_app();
    app.init_resource::<CurrentTarget>();
    app.insert_resource(InputBindings::default());
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.add_systems(Update, tab_target);

    let _player = app
        .world_mut()
        .spawn((Transform::from_xyz(0.0, 0.0, 0.0), Player))
        .id();

    let ignored_player = app
        .world_mut()
        .spawn((
            Transform::from_xyz(2.0, 0.0, 0.0),
            RemoteEntity,
            shared::components::Player {
                name: "Friendly".into(),
                race: 1,
                class: 2,
                appearance: Default::default(),
            },
        ))
        .id();

    let npc = app
        .world_mut()
        .spawn((
            Transform::from_xyz(4.0, 0.0, 0.0),
            RemoteEntity,
            Npc { template_id: 42 },
        ))
        .id();

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Tab);
    game_engine::test_harness::run_updates(&mut app, 1);

    let current = app.world().resource::<CurrentTarget>();
    assert_eq!(current.0, Some(npc));
    assert_ne!(current.0, Some(ignored_player));
}

#[test]
fn test_tab_target_skips_hidden_npcs() {
    let mut app = game_engine::test_harness::headless_app();
    app.init_resource::<CurrentTarget>();
    app.insert_resource(InputBindings::default());
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.add_systems(Update, tab_target);

    let _player = app
        .world_mut()
        .spawn((Transform::from_xyz(0.0, 0.0, 0.0), Player))
        .id();

    let hidden = app
        .world_mut()
        .spawn((
            Transform::from_xyz(2.0, 0.0, 0.0),
            RemoteEntity,
            Npc { template_id: 1 },
            Visibility::Hidden,
        ))
        .id();
    let visible = app
        .world_mut()
        .spawn((
            Transform::from_xyz(4.0, 0.0, 0.0),
            RemoteEntity,
            Npc { template_id: 2 },
            Visibility::Visible,
        ))
        .id();

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Tab);
    game_engine::test_harness::run_updates(&mut app, 1);

    let current = app.world().resource::<CurrentTarget>();
    assert_eq!(current.0, Some(visible));
    assert_ne!(current.0, Some(hidden));
}

#[test]
fn test_escape_clears_target() {
    let mut app = game_engine::test_harness::headless_app();
    app.init_resource::<CurrentTarget>();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.add_systems(Update, clear_target);

    let entity = app.world_mut().spawn_empty().id();
    app.world_mut().resource_mut::<CurrentTarget>().0 = Some(entity);

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    game_engine::test_harness::run_updates(&mut app, 1);

    let target = app.world().resource::<CurrentTarget>();
    assert_eq!(target.0, None);
}
