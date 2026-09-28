use game_engine_core::camera_control_data::{
    CameraState, DEFAULT_CAMERA_FOV_DEGREES, MAX_CAMERA_FOV_DEGREES, MIN_CAMERA_FOV_DEGREES,
};
use game_engine_core::camera_follow_data::{follow_camera, keep_in_sight};
use glam::Vec3;

fn camera_at(distance: f32) -> CameraState {
    CameraState {
        pitch: 0.0,
        distance,
        target_distance: distance,
        ..Default::default()
    }
}

fn step(
    camera: &mut CameraState,
    current: Vec3,
    target: Vec3,
    dt: f32,
    terrain: Option<&mut dyn FnMut(f32, f32) -> Option<f32>>,
    hit: Option<f32>,
    ground: Option<f32>,
) -> Vec3 {
    follow_camera(camera, current, target, dt, terrain, |_, _| hit, |_| ground).position
}

#[test]
fn original_defaults_and_angle_validation_are_atomic() {
    assert_eq!(
        (
            MIN_CAMERA_FOV_DEGREES,
            MAX_CAMERA_FOV_DEGREES,
            DEFAULT_CAMERA_FOV_DEGREES
        ),
        (90.0, 120.0, 90.0)
    );
    let mut camera = CameraState::default();
    assert_eq!(
        (
            camera.pitch,
            camera.yaw,
            camera.distance,
            camera.target_distance
        ),
        (-0.3, 0.0, 15.0, 15.0)
    );
    assert_eq!(
        (
            camera.min_distance,
            camera.max_distance,
            camera.follow_speed,
            camera.zoom_speed,
            camera.collision_distance
        ),
        (2.0, 40.0, 10.0, 8.0, None)
    );
    assert_eq!(
        camera.set_direction_degrees(None, None),
        Err("provide yaw or pitch in degrees".into())
    );
    assert_eq!(
        camera.set_direction_degrees(Some(f32::NAN), None),
        Err("camera angles must be finite".into())
    );
    assert_eq!(
        camera.set_direction_degrees(None, Some(89.0)),
        Err("camera pitch must be between -88 and 88 degrees".into())
    );
    assert_eq!((camera.yaw, camera.pitch), (0.0, -0.3));
    camera
        .set_direction_degrees(Some(90.0), Some(-88.0))
        .unwrap();
    assert!((camera.yaw - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    assert!((camera.pitch - (-88.0_f32).to_radians()).abs() < 1e-6);
    camera.set_direction_degrees(None, Some(0.0)).unwrap();
    assert!((camera.yaw - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
}

#[test]
fn zoom_follow_and_yxz_orbit_produce_original_pose() {
    let mut camera = camera_at(10.0);
    camera.distance = 5.0;
    camera.yaw = std::f32::consts::FRAC_PI_2;
    let pose = follow_camera(
        &mut camera,
        Vec3::ZERO,
        Vec3::new(8.0, 100.0, 0.0),
        0.025,
        None,
        |_, _| None,
        |_| None,
    );
    assert_eq!(camera.distance, 6.0); // 8 * 0.025 of the five-yard zoom gap
    let eye = Vec3::new(8.0, 101.8, 0.0);
    assert!(pose.eye_target.abs_diff_eq(eye, 1e-5));
    assert!(
        pose.position
            .abs_diff_eq((eye - Vec3::NEG_X * 6.0) * 0.25, 1e-5)
    );
}

#[test]
fn clear_terrain_preserves_distance_and_samples_full_segment() {
    let mut camera = camera_at(12.0);
    let mut count = 0;
    let mut terrain = |_: f32, _: f32| {
        count += 1;
        Some(0.0)
    };
    let position = step(
        &mut camera,
        Vec3::new(0.0, 2.0, 12.0),
        Vec3::new(0.0, 0.2, 0.0),
        0.1,
        Some(&mut terrain),
        None,
        Some(0.0),
    );
    assert_eq!(count, 24);
    assert!(position.abs_diff_eq(Vec3::new(0.0, 2.0, 12.0), 1e-5));
    assert_eq!(camera.collision_distance, None);
}

#[test]
fn hill_pulls_forward_with_clearance_and_first_sample_rule() {
    let mut camera = camera_at(12.0);
    let mut terrain = |_: f32, z: f32| if z >= 5.0 { Some(1.81) } else { None };
    let position = step(
        &mut camera,
        Vec3::new(0.0, 2.0, 12.0),
        Vec3::new(0.0, 0.2, 0.0),
        0.1,
        Some(&mut terrain),
        None,
        None,
    );
    // First blocking sample z=5.0; 5.0 - 0.3 collision offset.
    assert!(position.abs_diff_eq(Vec3::new(0.0, 2.0, 4.7), 1e-5));
    assert!(camera.collision_distance.is_some());
}

#[test]
fn mesh_near_far_and_tiny_hits_keep_original_distance_policy() {
    for (hit, expected, collided) in [
        (Some(8.0), 7.7, true),
        (Some(20.0), 15.0, false),
        (Some(0.2), 0.5, true),
    ] {
        let mut camera = camera_at(15.0);
        let position = step(
            &mut camera,
            Vec3::new(0.0, 1.8, 15.0),
            Vec3::ZERO,
            0.1,
            None,
            hit,
            None,
        );
        assert!(
            (position.z - expected).abs() < 1e-5,
            "hit {hit:?}: {position:?}"
        );
        assert_eq!(camera.collision_distance.is_some(), collided);
    }
}

#[test]
fn collision_recovery_resumes_from_pulled_in_distance_and_clears_near_target() {
    let mut camera = camera_at(10.0);
    step(
        &mut camera,
        Vec3::new(0.0, 1.8, 10.0),
        Vec3::ZERO,
        0.1,
        None,
        Some(4.5),
        None,
    );
    assert!(camera.collision_distance.is_some());
    let recovered = step(
        &mut camera,
        Vec3::new(0.0, 1.8, 4.2),
        Vec3::ZERO,
        0.1,
        None,
        None,
        None,
    );
    assert!((recovered.z - 7.1).abs() < 1e-5);
    assert!(camera.collision_distance.is_some());
    let finished = step(&mut camera, recovered, Vec3::ZERO, 0.2, None, None, None);
    assert!((finished.z - 10.0).abs() < 1e-5);
    assert_eq!(camera.collision_distance, None);
}

#[test]
fn collision_recovery_keeps_following_a_player_running_away() {
    const DT: f32 = 1.0 / 60.0;
    const RUN_SPEED: f32 = 7.0;
    let mut camera = CameraState {
        pitch: 0.0,
        distance: 10.0,
        target_distance: 10.0,
        follow_speed: 10.0,
        ..Default::default()
    };
    let mut target = Vec3::ZERO;
    let mut current = Vec3::new(0.0, 1.8, 10.0);
    let mut run = |camera: &mut CameraState, current: &mut Vec3, hit: Option<f32>| {
        target.z -= RUN_SPEED * DT;
        *current = step(camera, *current, target, DT, None, hit, None);
        current.distance(target + Vec3::Y * 1.8)
    };
    for _ in 0..60 {
        run(&mut camera, &mut current, None);
    }
    // One frame of obstruction just inside the steady running distance.
    run(&mut camera, &mut current, Some(9.5));
    assert_eq!(camera.collision_distance, Some(9.2));
    let mut worst = 0.0_f32;
    for _ in 0..60 {
        worst = worst.max(run(&mut camera, &mut current, None));
    }
    // Follow smoothing alone trails by about run speed / follow speed.
    assert!(
        worst < 10.0 + RUN_SPEED / 10.0 + 0.1,
        "camera fell {worst} m behind"
    );
    assert_eq!(camera.collision_distance, None);
}

#[test]
fn ground_floor_and_wmo_only_no_floor_are_caller_selected() {
    let mut camera = camera_at(10.0);
    let current = Vec3::new(0.0, -17.5, 10.0);
    let target = Vec3::new(0.0, -19.3, 0.0);
    let no_floor = step(&mut camera, current, target, 0.1, None, None, None);
    assert!((no_floor.y - (-19.3 + 1.8)).abs() < 1e-5);
    let floor = step(&mut camera, current, target, 0.1, None, None, Some(0.0));
    assert!((floor.y - 0.5).abs() < 1e-5);
}

#[test]
fn smoothed_camera_is_pulled_in_front_of_a_blocker_between_it_and_the_eye() {
    let eye = Vec3::new(0.0, 1.8, 0.0);
    let camera = Vec3::new(0.0, 1.8, 10.0);
    let mut rays = Vec::new();
    let pulled = keep_in_sight(eye, camera, |origin, direction| {
        rays.push((origin, direction));
        Some(4.0)
    });
    assert_eq!(rays, vec![(eye, Vec3::Z)]);
    assert!(
        (pulled - Vec3::new(0.0, 1.8, 3.7)).length() < 1e-5,
        "{pulled}"
    );
    assert_eq!(keep_in_sight(eye, camera, |_, _| Some(12.0)), camera);
    assert_eq!(keep_in_sight(eye, camera, |_, _| None), camera);
    assert_eq!(keep_in_sight(eye, eye, |_, _| Some(0.1)), eye);
}
