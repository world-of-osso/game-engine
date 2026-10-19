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
fn pet_card_normalization_applies_to_another_requested_scale() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let card = read_scene_actors(&root, 33)
        .unwrap()
        .remove("unwrapped")
        .unwrap();
    assert_eq!(
        (card.id, card.flags, card.normalized_scale, card.scale),
        (38, 9, 0.5, 7.)
    );
    // Width 6.179 dominates: max scale 6.189, factor 1 / (1 + 5.189 * 0.5).
    let large = Vec3::new(6.179, 1.56, 3.678);
    let factor = 1. / (1. + (6.179 / 0.9984 - 1.) * 0.5);
    assert!((card.normalized_scale_of(1., large) - factor).abs() < 1e-5);
    assert!((card.normalized_scale_of(0.7, large) - 0.7 * factor).abs() < 1e-5);
    assert!((card.actor_scale(large) - 7. * factor).abs() < 1e-5);
    // Smaller than the reference human grows: width 0.6 dominates at max scale 0.601.
    let small = Vec3::new(0.6, 0.5, 0.4);
    let grown = 1. / (1. + (0.6 / 0.9984 - 1.) * 0.5);
    assert!(card.normalized_scale_of(1., small) > 1.);
    assert!((card.normalized_scale_of(1., small) - grown).abs() < 1e-5);
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
