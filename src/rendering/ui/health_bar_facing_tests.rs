use super::*;
use bevy::transform::TransformPlugin;

fn scene() -> (App, Entity, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::state::app::StatesPlugin,
        TransformPlugin,
    ));
    app.init_state::<GameState>();
    app.insert_resource(State::new(GameState::InWorld));
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<HudVisibilityToggles>();
    app.add_plugins(HealthBarPlugin);
    let actor = app
        .world_mut()
        .spawn((
            Transform::from_xyz(100.0, 7.0, -30.0)
                .with_rotation(Quat::from_euler(EulerRot::YXZ, 1.3, 0.2, -0.1))
                .with_scale(Vec3::new(0.7, 1.2, 0.9)),
            Visibility::Visible,
            Health {
                current: 75.0,
                max: 100.0,
            },
        ))
        .id();
    let bar = app.world().get::<Children>(actor).unwrap()[0];
    let camera = app
        .world_mut()
        .spawn((Camera3d::default(), Transform::from_xyz(108.0, 12.0, -18.0)))
        .id();
    (app, actor, bar, camera)
}

fn assert_front_faces_camera(app: &App, bar: Entity, camera: Entity) {
    let pose = app.world().get::<GlobalTransform>(bar).unwrap();
    let camera = app
        .world()
        .get::<GlobalTransform>(camera)
        .unwrap()
        .translation();
    let affine = pose.affine();
    let x = affine.transform_vector3(Vec3::X);
    let y = affine.transform_vector3(Vec3::Y);
    let front = x.cross(y).normalize();
    assert!(
        front.dot((camera - pose.translation()).normalize()) > 0.99999,
        "actual world mesh +Z front {front:?} must face camera"
    );
    for child in app.world().get::<Children>(bar).unwrap().iter() {
        let local = app.world().get::<Transform>(child).unwrap();
        let actual = app.world().get::<GlobalTransform>(child).unwrap();
        assert!(
            actual
                .affine()
                .abs_diff_eq((*pose * *local).affine(), 0.0001)
        );
    }
}

#[test]
fn parented_bar_faces_camera_after_actor_and_camera_motion_in_same_frame() {
    let (mut app, actor, bar, camera) = scene();
    app.update();
    assert_front_faces_camera(&app, bar, camera);
    app.world_mut()
        .get_mut::<Transform>(actor)
        .unwrap()
        .rotation = Quat::from_rotation_y(-0.8);
    app.world_mut()
        .get_mut::<Transform>(camera)
        .unwrap()
        .translation = Vec3::new(92.0, 16.0, -35.0);
    app.update();
    assert_front_faces_camera(&app, bar, camera);
    let expected_center = app
        .world()
        .get::<GlobalTransform>(actor)
        .unwrap()
        .transform_point(Vec3::new(0.0, 2.5, 0.0));
    assert!(
        app.world()
            .get::<GlobalTransform>(bar)
            .unwrap()
            .translation()
            .distance(expected_center)
            < 0.0001
    );
}

#[test]
fn parented_billboard_preserves_fill_visibility_and_settled_local_transform_ticks() {
    let (mut app, actor, bar, camera) = scene();
    app.update();
    app.update();
    let tick = app
        .world()
        .entity(bar)
        .get_ref::<Transform>()
        .unwrap()
        .last_changed();
    app.update();
    assert_eq!(
        app.world()
            .entity(bar)
            .get_ref::<Transform>()
            .unwrap()
            .last_changed(),
        tick
    );
    assert_front_faces_camera(&app, bar, camera);
    app.world_mut().get_mut::<Health>(actor).unwrap().current = 25.0;
    app.update();
    let foreground = app
        .world()
        .get::<Children>(bar)
        .unwrap()
        .iter()
        .find(|&e| app.world().get::<HealthBarForeground>(e).is_some())
        .unwrap();
    assert_eq!(
        app.world().get::<Transform>(foreground),
        Some(&foreground_transform(0.25))
    );
    let material = &app
        .world()
        .get::<MeshMaterial3d<StandardMaterial>>(foreground)
        .unwrap()
        .0;
    assert_eq!(
        app.world()
            .resource::<Assets<StandardMaterial>>()
            .get(material)
            .unwrap()
            .base_color,
        health_bar_color(25.0, 100.0)
    );
    app.world_mut()
        .resource_mut::<HudVisibilityToggles>()
        .show_health_bars = false;
    app.update();
    assert_eq!(
        app.world().get::<Visibility>(bar),
        Some(&Visibility::Hidden)
    );
    app.world_mut()
        .resource_mut::<HudVisibilityToggles>()
        .show_health_bars = true;
    app.update();
    assert_eq!(
        app.world().get::<Visibility>(bar),
        Some(&Visibility::Inherited)
    );
}
