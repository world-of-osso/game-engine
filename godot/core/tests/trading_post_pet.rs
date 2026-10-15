use game_engine_core::trading_post_pet::PetComparison;
use std::path::PathBuf;

fn data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

#[test]
fn trading_post_pet_keeps_category_actor_and_separate_dressed_player_scene() {
    let comparison = PetComparison::read(&data_root(), 419, 555, 430, 1, 0).unwrap();
    assert_eq!(comparison.pet.id, 1216);
    assert_eq!(comparison.pet.flags, 2);
    assert_eq!(comparison.pet.position, [-2.2, -3.4, 0.]);
    assert_eq!(comparison.pet.normalized_scale, 0.);
    // Requested scale is category-authored only; native pet desiredScale remains a gap.
    assert_eq!(comparison.pet.scale, 1.);
    assert_eq!(comparison.player.id, 1702);
    assert_eq!(comparison.player.position, [-2., 2.2, 0.]);
    assert!((comparison.player.orientation[0] - (-55_f32).to_radians()).abs() < 0.00001);
    assert_eq!(comparison.player_camera.id, 434);
    assert_eq!(comparison.player_camera.scene_id, 430);
    // Nil player displayData bypasses Products.TranslateDisplayInfo rounding.
    assert!((comparison.player_camera.target[2] - 1.01).abs() < 0.00001);
    assert!((comparison.player_camera.yaw - 150_f32.to_radians()).abs() < 0.00001);
    assert_eq!(comparison.player_camera.zoom_distance, 6.5);
}

#[test]
fn trading_post_pet_player_uses_sex_specific_actor_before_race_actor() {
    let comparison = PetComparison::read(&data_root(), 419, 419, 430, 1, 1).unwrap();
    assert_eq!(comparison.player.id, 1703);
    assert!((comparison.player.position[0] + 2.05).abs() < 0.00001);
    assert_eq!(comparison.player.position[1], 2.2);
    assert_eq!(comparison.pet.position, [0., -3., 0.]);
    let error = PetComparison::read(&data_root(), 419, 419, 430, u8::MAX, 0).unwrap_err();
    assert!(error.contains("255"), "{error}");
}
