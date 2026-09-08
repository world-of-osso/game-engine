use super::*;
use bevy::animation::AnimatedBy;

const PIVOT: Vec3 = Vec3::new(1.0, -2.0, 3.0);
const SCALE: Vec3 = Vec3::new(2.0, 3.0, 4.0);

#[derive(Resource, Default)]
struct BillboardObservations {
    before_billboard: Vec<Transform>,
    final_changes: Vec<Transform>,
}

struct BillboardFixture {
    app: App,
    owner: Entity,
    bone: Entity,
    camera: Entity,
}

fn observe_before_billboard(
    bones: Query<&Transform, With<SphericalBillboard>>,
    mut observations: ResMut<BillboardObservations>,
) {
    observations.before_billboard.extend(bones.iter().copied());
}

fn observe_final_changes(
    bones: Query<&Transform, (With<SphericalBillboard>, Changed<Transform>)>,
    mut observations: ResMut<BillboardObservations>,
) {
    observations.final_changes.extend(bones.iter().copied());
}

fn raw_pose() -> Transform {
    Transform {
        translation: Vec3::new(4.0, 5.0, 6.0),
        rotation: Quat::from_rotation_x(0.3),
        scale: SCALE,
    }
}

fn billboard_fixture() -> BillboardFixture {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::animation::AnimationPlugin,
    ));
    app.init_resource::<BillboardObservations>();
    app.add_systems(
        PostUpdate,
        (
            observe_before_billboard
                .after(bevy::app::AnimationSystems)
                .before(apply_billboard_rotation),
            apply_billboard_rotation
                .after(bevy::app::AnimationSystems)
                .before(bevy::transform::TransformSystems::Propagate),
            observe_final_changes
                .after(apply_billboard_rotation)
                .before(bevy::transform::TransformSystems::Propagate),
        ),
    );
    let clip = app
        .world_mut()
        .resource_mut::<Assets<AnimationClip>>()
        .add(bevy_curves::build_pose_clip([(0, raw_pose())]));
    let mut graph = AnimationGraph::new();
    let node = graph.add_clip(clip, 1.0, graph.root);
    let graph = app
        .world_mut()
        .resource_mut::<Assets<AnimationGraph>>()
        .add(graph);
    let mut player = AnimationPlayer::default();
    player.play(node).pause().seek_to(0.0);
    let owner = app
        .world_mut()
        .spawn((player, AnimationGraphHandle(graph)))
        .id();
    let bone = app
        .world_mut()
        .spawn((
            bevy_curves::bone_target_id(0),
            AnimatedBy(owner),
            BonePivot(PIVOT),
            SphericalBillboard { pivot: PIVOT },
            Transform::default(),
        ))
        .id();
    let camera = app
        .world_mut()
        .spawn((Camera3d::default(), GlobalTransform::IDENTITY))
        .id();
    // Publish graph assets, then allow Bevy to build and evaluate the graph.
    for _ in 0..3 {
        app.update();
    }
    clear_observations(&mut app);
    BillboardFixture {
        app,
        owner,
        bone,
        camera,
    }
}

fn clear_observations(app: &mut App) {
    *app.world_mut().resource_mut::<BillboardObservations>() = BillboardObservations::default();
}

fn final_pose(fixture: &BillboardFixture) -> Transform {
    *fixture.app.world().get::<Transform>(fixture.bone).unwrap()
}

fn assert_billboard_pose(pose: Transform, camera_yaw: f32) {
    assert_eq!(pose.translation, PIVOT);
    assert_eq!(pose.scale, SCALE);
    let expected = Quat::from_rotation_y(camera_yaw - std::f32::consts::FRAC_PI_2);
    assert!(pose.rotation.dot(expected).abs() > 0.99999);
}

#[test]
fn billboard_settled_bevy_pose_does_not_notify_final_transform_changes() {
    let mut fixture = billboard_fixture();
    let settled = final_pose(&fixture);
    assert_billboard_pose(settled, 0.0);
    for _ in 0..3 {
        fixture.app.update();
        assert_eq!(final_pose(&fixture), settled);
    }
    let observations = fixture.app.world().resource::<BillboardObservations>();
    assert_eq!(observations.final_changes.len(), 0);
}

#[test]
fn billboard_bevy_commit_restores_raw_pose_before_each_settled_billboard_pass() {
    let mut fixture = billboard_fixture();
    let settled = final_pose(&fixture);
    for _ in 0..3 {
        fixture.app.update();
        assert_eq!(final_pose(&fixture), settled);
    }
    let observations = fixture.app.world().resource::<BillboardObservations>();
    assert_eq!(observations.before_billboard.len(), 3);
    let raw = raw_pose();
    let corrected_translation = raw.translation + PIVOT - raw.rotation * (raw.scale * PIVOT);
    for pose in &observations.before_billboard {
        assert!(pose.translation.distance(corrected_translation) < 0.00001);
        assert!(pose.rotation.dot(raw.rotation).abs() > 0.99999);
        assert_eq!(pose.scale, SCALE);
        assert!(pose.rotation.dot(settled.rotation).abs() < 0.99);
    }
}

#[test]
fn billboard_camera_motion_changes_final_rotation_and_preserves_animated_scale() {
    let mut fixture = billboard_fixture();
    let settled = final_pose(&fixture);
    *fixture
        .app
        .world_mut()
        .get_mut::<GlobalTransform>(fixture.camera)
        .unwrap() = GlobalTransform::from(Transform::from_rotation(Quat::from_rotation_y(0.65)));
    fixture.app.update();
    let moved = final_pose(&fixture);
    assert_billboard_pose(moved, 0.65);
    assert!(moved.rotation.dot(settled.rotation).abs() < 0.99);
    let observations = fixture.app.world().resource::<BillboardObservations>();
    assert_eq!(observations.final_changes, vec![moved]);
}

#[test]
fn billboard_settled_without_active_clip_does_not_notify_final_transform_changes() {
    let mut fixture = billboard_fixture();
    fixture
        .app
        .world_mut()
        .get_mut::<AnimationPlayer>(fixture.owner)
        .unwrap()
        .stop_all();
    fixture.app.update();
    clear_observations(&mut fixture.app);
    let settled = final_pose(&fixture);
    for _ in 0..3 {
        fixture.app.update();
        assert_eq!(final_pose(&fixture), settled);
    }
    let observations = fixture.app.world().resource::<BillboardObservations>();
    // Without an active clip, the pre-billboard transform already is the final pose.
    assert_eq!(observations.before_billboard, vec![settled; 3]);
    assert_eq!(observations.final_changes.len(), 0);
}
