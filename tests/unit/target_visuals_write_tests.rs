use super::*;

#[derive(Resource, Default)]
struct TargetCircleChanges(usize);

fn observe_target_circle_changes(
    circles: Query<(), (With<TargetMarker>, Changed<Transform>)>,
    mut changes: ResMut<TargetCircleChanges>,
) {
    changes.0 = circles.iter().count();
}

#[test]
fn stationary_target_circle_does_not_change_transform() {
    let mut app = App::new();
    app.init_resource::<TargetCircleChanges>();
    app.init_resource::<game_engine::targeting::CurrentTarget>();
    app.add_systems(Update, update_target_circle);
    app.add_systems(PostUpdate, observe_target_circle_changes);
    let target = app
        .world_mut()
        .spawn(Transform::from_xyz(3.0, 4.0, 5.0))
        .id();
    app.world_mut()
        .resource_mut::<game_engine::targeting::CurrentTarget>()
        .0 = Some(target);
    let circle = app
        .world_mut()
        .spawn((
            TargetMarker,
            TargetMarkerScaleFactor(1.0),
            Transform::default(),
        ))
        .id();
    app.update();
    assert_eq!(app.world().resource::<TargetCircleChanges>().0, 1);
    app.update();
    assert_eq!(app.world().resource::<TargetCircleChanges>().0, 0);
    app.world_mut()
        .get_mut::<Transform>(target)
        .unwrap()
        .translation
        .x = 9.0;
    app.update();
    assert_eq!(app.world().resource::<TargetCircleChanges>().0, 1);
    assert_eq!(
        app.world().get::<Transform>(circle).unwrap().translation,
        Vec3::new(9.0, 4.05, 5.0)
    );
}
