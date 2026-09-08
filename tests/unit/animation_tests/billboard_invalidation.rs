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
            SphericalBillboard {
                pivot: PIVOT,
                pending_pose: None,
            },
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

fn assert_pose_near(actual: Transform, expected: Transform) {
    assert!(actual.translation.distance(expected.translation) < 0.00001);
    assert!(actual.rotation.dot(expected.rotation).abs() > 0.99999);
    assert!(actual.scale.distance(expected.scale) < 0.00001);
}

fn corrected_pose(mut raw: Transform) -> Transform {
    raw.translation = raw.translation + PIVOT - raw.rotation * (raw.scale * PIVOT);
    raw
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
fn billboard_bevy_commit_retains_raw_snapshot_without_overwriting_final_pose() {
    let mut fixture = billboard_fixture();
    let settled = final_pose(&fixture);
    for _ in 0..3 {
        fixture.app.update();
        assert_eq!(final_pose(&fixture), settled);
    }
    let observations = fixture.app.world().resource::<BillboardObservations>();
    assert_eq!(observations.before_billboard.len(), 3);
    assert_eq!(observations.before_billboard, vec![settled; 3]);
    assert_pose_near(
        fixture
            .app
            .world()
            .get::<bevy_curves::RawBonePose>(fixture.bone)
            .unwrap()
            .0,
        raw_pose(),
    );
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
fn billboard_missing_camera_applies_evaluated_raw_pose_and_retains_snapshot() {
    let mut fixture = billboard_fixture();
    fixture.app.world_mut().despawn(fixture.camera);
    fixture.app.update();
    assert_pose_near(final_pose(&fixture), corrected_pose(raw_pose()));
    assert_pose_near(
        fixture
            .app
            .world()
            .get::<bevy_curves::RawBonePose>(fixture.bone)
            .unwrap()
            .0,
        raw_pose(),
    );
    clear_observations(&mut fixture.app);
    fixture.app.update();
    assert!(
        fixture
            .app
            .world()
            .resource::<BillboardObservations>()
            .final_changes
            .is_empty()
    );
}

#[test]
fn billboard_vertical_camera_applies_raw_pose_then_recovers_camera_facing() {
    let mut fixture = billboard_fixture();
    *fixture
        .app
        .world_mut()
        .get_mut::<GlobalTransform>(fixture.camera)
        .unwrap() = GlobalTransform::from(Transform::from_rotation(Quat::from_rotation_x(
        std::f32::consts::FRAC_PI_2,
    )));
    fixture.app.update();
    assert_pose_near(final_pose(&fixture), corrected_pose(raw_pose()));
    clear_observations(&mut fixture.app);
    fixture.app.update();
    assert!(
        fixture
            .app
            .world()
            .resource::<BillboardObservations>()
            .final_changes
            .is_empty()
    );
    *fixture
        .app
        .world_mut()
        .get_mut::<GlobalTransform>(fixture.camera)
        .unwrap() = GlobalTransform::IDENTITY;
    fixture.app.update();
    assert_billboard_pose(final_pose(&fixture), 0.0);
}

#[test]
fn billboard_unsampled_camera_motion_preserves_external_scale() {
    let mut fixture = billboard_fixture();
    fixture
        .app
        .world_mut()
        .get_mut::<AnimationPlayer>(fixture.owner)
        .unwrap()
        .stop_all();
    let external_scale = Vec3::new(0.5, 1.5, 2.5);
    fixture
        .app
        .world_mut()
        .get_mut::<Transform>(fixture.bone)
        .unwrap()
        .scale = external_scale;
    *fixture
        .app
        .world_mut()
        .get_mut::<GlobalTransform>(fixture.camera)
        .unwrap() = GlobalTransform::from(Transform::from_rotation(Quat::from_rotation_y(0.65)));
    fixture.app.update();
    let pose = final_pose(&fixture);
    assert_eq!(pose.scale, external_scale);
    assert_eq!(pose.translation, PIVOT);
    assert!(
        pose.rotation
            .dot(Quat::from_rotation_y(0.65 - std::f32::consts::FRAC_PI_2))
            .abs()
            > 0.99999
    );
    fixture.app.world_mut().despawn(fixture.camera);
    fixture.app.update();
    assert_eq!(final_pose(&fixture), pose);
}

fn play_pose_blend(fixture: &mut BillboardFixture, incoming: Transform) {
    let mut graph = AnimationGraph::new();
    let mut player = AnimationPlayer::default();
    for pose in [raw_pose(), incoming] {
        let clip = fixture
            .app
            .world_mut()
            .resource_mut::<Assets<AnimationClip>>()
            .add(bevy_curves::build_pose_clip([(0, pose)]));
        let node = graph.add_clip(clip, 0.5, graph.root);
        player.play(node).pause().seek_to(0.0);
    }
    let graph = fixture
        .app
        .world_mut()
        .resource_mut::<Assets<AnimationGraph>>()
        .add(graph);
    fixture
        .app
        .world_mut()
        .entity_mut(fixture.owner)
        .insert((player, AnimationGraphHandle(graph)));
    for _ in 0..3 {
        fixture.app.update();
    }
}

#[test]
fn billboard_blended_nonuniform_scale_and_rotation_retain_raw_snapshot() {
    let mut fixture = billboard_fixture();
    let incoming = Transform {
        translation: Vec3::new(-2.0, 8.0, 1.0),
        rotation: Quat::from_rotation_y(0.8),
        scale: Vec3::new(4.0, 1.0, 0.5),
    };
    play_pose_blend(&mut fixture, incoming);
    let raw = raw_pose();
    let blended = Transform {
        translation: raw.translation.lerp(incoming.translation, 0.5),
        rotation: raw.rotation.slerp(incoming.rotation, 0.5),
        scale: raw.scale.lerp(incoming.scale, 0.5),
    };
    assert_pose_near(
        fixture
            .app
            .world()
            .get::<bevy_curves::RawBonePose>(fixture.bone)
            .unwrap()
            .0,
        blended,
    );
    let mut facing = blended;
    facing.translation = PIVOT;
    facing.rotation = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);
    assert_pose_near(final_pose(&fixture), facing);
    fixture.app.world_mut().despawn(fixture.camera);
    fixture.app.update();
    assert_pose_near(final_pose(&fixture), corrected_pose(blended));
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
