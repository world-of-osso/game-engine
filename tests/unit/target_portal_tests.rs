use super::*;

#[test]
fn zone_transition_contact_triggers_only_when_entering_new_portal() {
    let portal_a = Entity::from_bits(11);
    let portal_b = Entity::from_bits(22);
    let mut active_contact = None;

    assert!(!update_zone_transition_contact(&mut active_contact, None));
    assert_eq!(active_contact, None);

    assert!(update_zone_transition_contact(
        &mut active_contact,
        Some(portal_a)
    ));
    assert_eq!(active_contact, Some(portal_a));

    assert!(!update_zone_transition_contact(
        &mut active_contact,
        Some(portal_a)
    ));
    assert_eq!(active_contact, Some(portal_a));

    assert!(update_zone_transition_contact(
        &mut active_contact,
        Some(portal_b)
    ));
    assert_eq!(active_contact, Some(portal_b));

    assert!(!update_zone_transition_contact(&mut active_contact, None));
    assert_eq!(active_contact, None);
}

#[test]
fn player_inside_zone_transition_accepts_doodad_collider_bounds() {
    let player_position = Vec3::new(1.5, 2.0, 3.5);
    let collider = game_engine::culling::DoodadCollider {
        world_min: Vec3::new(1.0, 1.0, 3.0),
        world_max: Vec3::new(2.0, 3.0, 4.0),
    };

    assert!(player_inside_zone_transition(
        player_position,
        &GlobalTransform::IDENTITY,
        Some(&collider),
        None,
    ));
    assert!(!player_inside_zone_transition(
        Vec3::new(2.5, 2.0, 3.5),
        &GlobalTransform::IDENTITY,
        Some(&collider),
        None,
    ));
}

#[test]
fn player_inside_zone_transition_accepts_wmo_group_bounds() {
    let portal_transform =
        GlobalTransform::from(Transform::from_translation(Vec3::new(10.0, 0.0, 20.0)));
    let group = game_engine::culling::WmoGroup {
        group_index: 7,
        bbox_min: Vec3::new(-1.0, 0.0, -2.0),
        bbox_max: Vec3::new(1.0, 4.0, 2.0),
        is_antiportal: false,
    };

    assert!(player_inside_zone_transition(
        Vec3::new(10.5, 1.0, 19.0),
        &portal_transform,
        None,
        Some(&group),
    ));
    assert!(!player_inside_zone_transition(
        Vec3::new(12.5, 1.0, 19.0),
        &portal_transform,
        None,
        Some(&group),
    ));
}

#[test]
fn zone_transition_collision_sets_loading_state_once_per_contact() {
    let mut app = game_engine::test_harness::headless_app();
    app.init_resource::<NextState<GameState>>();
    app.init_resource::<ZoneTransitionContactState>();
    app.add_systems(Update, trigger_zone_transition_on_collision);

    app.world_mut().spawn((
        Player,
        GlobalTransform::from_translation(Vec3::new(1.5, 2.0, 3.5)),
    ));
    app.world_mut().spawn((
        WorldObjectInteraction {
            kind: WorldObjectInteractionKind::ZoneTransition,
        },
        GlobalTransform::IDENTITY,
        game_engine::culling::DoodadCollider {
            world_min: Vec3::new(1.0, 1.0, 3.0),
            world_max: Vec3::new(2.0, 3.0, 4.0),
        },
    ));

    app.update();
    assert!(matches!(
        app.world().resource::<NextState<GameState>>(),
        NextState::Pending(GameState::Loading)
    ));
    assert!(
        app.world()
            .resource::<ZoneTransitionContactState>()
            .active_portal
            .is_some()
    );

    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .reset();
    app.update();
    assert!(matches!(
        app.world().resource::<NextState<GameState>>(),
        NextState::Unchanged
    ));
}
