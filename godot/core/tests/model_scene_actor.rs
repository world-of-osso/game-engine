use game_engine_core::model_scene_actor::read_scene_actors;
use glam::Vec3;
use std::path::Path;

#[test]
fn trading_post_mount_scene_preserves_installed_actor_transforms() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let actors = read_scene_actors(&root, 418).unwrap();
    let mount = &actors["mount"];
    let rider = &actors["player-rider"];
    assert_eq!(mount.id, 1212);
    assert_eq!(mount.flags, 9);
    assert_eq!(mount.position, [0.; 3]);
    assert_eq!(mount.orientation, [0.; 3]);
    assert_eq!(mount.normalized_scale, 0.25);
    assert_eq!(mount.scale, 3.5);
    assert_eq!(mount.alpha, 1.);
    assert_eq!(rider.id, 1214);
    assert_eq!(rider.flags, 0);
    assert_eq!(rider.normalized_scale, 0.);
    assert_eq!(rider.scale, 1.);
    let size = Vec3::new(1.9968, 4.236, 2.242);
    let scale = mount.actor_scale(size);
    assert!((scale - 2.8).abs() < 1e-5);
    assert_eq!(
        mount.actor_origin(Vec3::new(0.5, 1., 0.7), scale),
        Vec3::new(0., -2.8, 0.)
    );
    assert_eq!(rider.actor_scale(size), 1.);
    assert_eq!(rider.actor_origin(Vec3::ONE, 1.), Vec3::ZERO);
}

#[test]
fn missing_mount_scene_fails_instead_of_using_generic_actor() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    assert!(
        read_scene_actors(&root, u32::MAX)
            .unwrap_err()
            .contains("4294967295")
    );
}
