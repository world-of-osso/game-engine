use super::*;
use crate::networking::ResolvedModelAssetInfo;
use bevy::camera::primitives::Aabb;

#[derive(Resource, Default)]
struct TargetResolutionResult(Option<Entity>);

#[derive(Resource, Default)]
struct TargetCircleSizeResult(f32);

#[test]
fn test_target_circle_transform_stays_flat_on_ground() {
    let transform = target_visuals::target_circle_transform(Vec3::new(10.0, 2.0, 5.0));
    assert_eq!(transform.translation, Vec3::new(10.0, 2.08, 5.0));
    assert_eq!(
        transform.rotation,
        Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
    );
}

#[test]
fn test_target_circle_style_default_is_fat_ring() {
    assert_eq!(
        TargetCircleStyle::default(),
        blp_style("Fat Ring", 167207, None, [255, 220, 50])
    );
}

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

#[test]
fn test_target_circle_follows_entity() {
    let mut app = game_engine::test_harness::headless_app();
    app.init_resource::<CurrentTarget>();

    let target = app
        .world_mut()
        .spawn((
            Transform::from_xyz(10.0, 0.0, 5.0),
            GlobalTransform::from_translation(Vec3::new(10.0, 0.0, 5.0)),
            RemoteEntity,
        ))
        .id();

    let circle = app
        .world_mut()
        .spawn((
            Transform::from_xyz(0.0, 0.0, 0.0),
            TargetMarkerScaleFactor(1.0),
            TargetMarker,
        ))
        .id();

    app.world_mut().resource_mut::<CurrentTarget>().0 = Some(target);
    app.add_systems(Update, target_visuals::update_target_circle);
    game_engine::test_harness::run_updates(&mut app, 1);

    let circle_pos = app
        .world()
        .entity(circle)
        .get::<Transform>()
        .unwrap()
        .translation;
    assert!((circle_pos.x - 10.0).abs() < 0.01);
    assert!((circle_pos.z - 5.0).abs() < 0.01);
}

#[test]
fn test_target_circle_rescales_when_target_bounds_appear() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<CurrentTarget>();

    let target = app
        .world_mut()
        .spawn((
            Transform::from_xyz(10.0, 0.0, 5.0),
            GlobalTransform::from_translation(Vec3::new(10.0, 0.0, 5.0)),
            RemoteEntity,
            ResolvedModelAssetInfo {
                model_path: "data/models/test.m2".into(),
                skin_path: None,
                display_scale: Some(1.0),
            },
        ))
        .id();

    let circle = app
        .world_mut()
        .spawn((
            Transform::from_xyz(0.0, 0.0, 0.0),
            TargetMarkerScaleFactor(1.0),
            TargetMarker,
        ))
        .id();

    app.world_mut().resource_mut::<CurrentTarget>().0 = Some(target);
    app.add_systems(Update, target_visuals::update_target_circle);
    app.update();

    let initial_scale = app
        .world()
        .entity(circle)
        .get::<Transform>()
        .unwrap()
        .scale
        .x;
    assert!((initial_scale - 0.7).abs() < 0.001);

    let child = app
        .world_mut()
        .spawn((
            Transform::from_translation(Vec3::new(1.2, 0.0, 0.2)),
            GlobalTransform::from_translation(Vec3::new(11.2, 0.0, 5.2)),
            Aabb {
                center: Vec3::ZERO.into(),
                half_extents: Vec3::new(0.8, 0.6, 0.3).into(),
            },
        ))
        .id();
    app.world_mut().entity_mut(child).insert(ChildOf(target));

    app.update();

    let updated_scale = app
        .world()
        .entity(circle)
        .get::<Transform>()
        .unwrap()
        .scale
        .x;
    assert!((updated_scale - 1.4).abs() < 0.001);
}

#[test]
fn test_resolve_targetable_ancestor_finds_remote_root_from_child_mesh() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<TargetResolutionResult>();

    let root = app
        .world_mut()
        .spawn((Transform::default(), RemoteEntity, Npc { template_id: 99 }))
        .id();
    let child = app.world_mut().spawn(Transform::default()).id();
    app.world_mut().entity_mut(child).insert(ChildOf(root));
    app.add_systems(
        Update,
        move |parent_query: Query<&ChildOf>,
              remote_query: Query<Entity, (With<RemoteEntity>, With<Npc>, Without<Player>)>,
              visibility_query: Query<&Visibility>,
              mut result: ResMut<TargetResolutionResult>| {
            result.0 =
                resolve_targetable_ancestor(child, &parent_query, &remote_query, &visibility_query);
        },
    );
    app.update();

    assert_eq!(
        app.world().resource::<TargetResolutionResult>().0,
        Some(root)
    );
}

#[test]
fn test_resolve_targetable_ancestor_ignores_remote_players() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<TargetResolutionResult>();

    let root = app
        .world_mut()
        .spawn((
            Transform::default(),
            RemoteEntity,
            shared::components::Player {
                name: "Friendly".into(),
                race: 1,
                class: 2,
                appearance: Default::default(),
            },
        ))
        .id();
    let child = app.world_mut().spawn(Transform::default()).id();
    app.world_mut().entity_mut(child).insert(ChildOf(root));
    app.add_systems(
        Update,
        move |parent_query: Query<&ChildOf>,
              remote_query: Query<Entity, (With<RemoteEntity>, With<Npc>, Without<Player>)>,
              visibility_query: Query<&Visibility>,
              mut result: ResMut<TargetResolutionResult>| {
            result.0 =
                resolve_targetable_ancestor(child, &parent_query, &remote_query, &visibility_query);
        },
    );
    app.update();

    assert_eq!(app.world().resource::<TargetResolutionResult>().0, None);
}

#[test]
fn test_resolve_targetable_ancestor_ignores_hidden_npcs() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<TargetResolutionResult>();

    let root = app
        .world_mut()
        .spawn((
            Transform::default(),
            RemoteEntity,
            Npc { template_id: 7 },
            Visibility::Hidden,
        ))
        .id();
    let child = app.world_mut().spawn(Transform::default()).id();
    app.world_mut().entity_mut(child).insert(ChildOf(root));
    app.add_systems(
        Update,
        move |parent_query: Query<&ChildOf>,
              remote_query: Query<Entity, (With<RemoteEntity>, With<Npc>, Without<Player>)>,
              visibility_query: Query<&Visibility>,
              mut result: ResMut<TargetResolutionResult>| {
            result.0 =
                resolve_targetable_ancestor(child, &parent_query, &remote_query, &visibility_query);
        },
    );
    app.update();

    assert_eq!(app.world().resource::<TargetResolutionResult>().0, None);
}

#[test]
fn test_convert_opaque_image_to_alpha_mask_uses_luminance() {
    let mut image = Image::new(
        bevy::render::render_resource::Extent3d {
            width: 2,
            height: 1,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        vec![0, 0, 0, 255, 200, 120, 40, 255],
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    );

    target_visuals::convert_opaque_image_to_alpha_mask(&mut image);

    let data = image.data.expect("image should keep pixel data");
    assert_eq!(&data[0..4], &[0, 0, 0, 0]);
    assert_eq!(&data[4..8], &[200, 200, 200, 200]);
}

#[test]
fn test_target_circle_size_uses_descendant_aabb_footprint() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<TargetCircleSizeResult>();

    let target = app
        .world_mut()
        .spawn((
            Transform::default(),
            GlobalTransform::default(),
            RemoteEntity,
        ))
        .id();
    let child = app
        .world_mut()
        .spawn((
            Transform::from_translation(Vec3::new(1.2, 0.0, 0.2)),
            GlobalTransform::from_translation(Vec3::new(1.2, 0.0, 0.2)),
            Aabb {
                center: Vec3::ZERO.into(),
                half_extents: Vec3::new(0.8, 0.6, 0.3).into(),
            },
        ))
        .id();
    app.world_mut().entity_mut(child).insert(ChildOf(target));
    app.add_systems(
        Update,
        move |parent_query: Query<&ChildOf>,
              target_global_q: Query<&GlobalTransform, Without<TargetMarker>>,
              aabb_query: Query<(Entity, &Aabb, &GlobalTransform)>,
              model_info_query: Query<&ResolvedModelAssetInfo>,
              mut result: ResMut<TargetCircleSizeResult>| {
            result.0 = target_visuals::target_circle_size(
                target,
                &parent_query,
                &target_global_q,
                &aabb_query,
                &model_info_query,
            );
        },
    );
    app.update();
    let size = app.world().resource::<TargetCircleSizeResult>().0;

    assert!((size - 1.4).abs() < 0.001);
}

#[test]
fn test_target_circle_size_falls_back_to_display_scale() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<TargetCircleSizeResult>();

    let target = app
        .world_mut()
        .spawn((
            Transform::default(),
            GlobalTransform::default(),
            RemoteEntity,
            ResolvedModelAssetInfo {
                model_path: "data/models/test.m2".into(),
                skin_path: None,
                display_scale: Some(1.75),
            },
        ))
        .id();
    app.add_systems(
        Update,
        move |parent_query: Query<&ChildOf>,
              target_global_q: Query<&GlobalTransform, Without<TargetMarker>>,
              aabb_query: Query<(Entity, &Aabb, &GlobalTransform)>,
              model_info_query: Query<&ResolvedModelAssetInfo>,
              mut result: ResMut<TargetCircleSizeResult>| {
            result.0 = target_visuals::target_circle_size(
                target,
                &parent_query,
                &target_global_q,
                &aabb_query,
                &model_info_query,
            );
        },
    );
    app.update();
    let size = app.world().resource::<TargetCircleSizeResult>().0;

    assert!((size - 1.225).abs() < 0.001);
}
