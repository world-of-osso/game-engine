use game_engine_core::model_scene_camera::read_perks_camera;
use glam::Vec3;
use std::path::PathBuf;

fn data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

#[test]
fn trading_post_camera_reads_product_override_even_without_primary_tag() {
    let camera = read_perks_camera(&data_root(), 633).unwrap();
    assert_eq!(camera.id, 619);
    assert_eq!(camera.scene_id, 633);
    assert_eq!(camera.target, [0.; 3]);
    assert!((camera.yaw - 137.5_f32.to_radians()).abs() < 0.00001);
    assert_eq!(camera.pitch, 0.);
    assert_eq!(camera.roll, 0.);
    assert_eq!(camera.zoom_distance, 23.);
    assert_eq!(camera.min_zoom_distance, 0.);
    assert_eq!(camera.max_zoom_distance, 99.);
    let (eye, target, up) = camera.vectors(0., camera.zoom_percent());
    assert!(eye.abs_diff_eq(Vec3::new(16.957382, 0., 15.538568), 0.002));
    assert_eq!(target, Vec3::ZERO);
    assert_eq!(up, Vec3::Y);
    assert!((eye.distance(target) - 23.).abs() < 0.00001);
}

#[test]
fn trading_post_camera_category_defaults_and_decimal_rounding_match_display_translation() {
    let camera = read_perks_camera(&data_root(), 418).unwrap();
    assert_eq!(camera.id, 422);
    assert_eq!(camera.target, [0., 0., 1.2]);
    assert!((camera.yaw - 150_f32.to_radians()).abs() < 0.00001);
    assert_eq!(camera.zoom_distance, 20.);
    let (eye, target, _) = camera.vectors(0., camera.zoom_percent());
    assert_eq!(target, Vec3::new(0., 1.2, 0.));
    assert!(eye.abs_diff_eq(Vec3::new(17.320509, 1.2, 10.), 0.00002));
    let fixed = read_perks_camera(&data_root(), 420).unwrap();
    assert_eq!(fixed.min_zoom_distance, fixed.max_zoom_distance);
    assert_eq!(fixed.zoom_percent(), 0.);
    let (eye, target, _) = fixed.vectors(0., 1.);
    assert!((eye.distance(target) - fixed.zoom_distance).abs() < 0.00001);
}

#[test]
fn trading_post_camera_orbit_and_zoom_move_eye_without_changing_authored_target() {
    let camera = read_perks_camera(&data_root(), 633).unwrap();
    let percent = camera.zoom_percent();
    let (initial, target, _) = camera.vectors(0., percent);
    let (rotated, rotated_target, _) = camera.vectors(0.4, percent);
    assert!(initial.distance(rotated) > 8.);
    assert_eq!(rotated_target, target);
    assert!((rotated.distance(target) - 23.).abs() < 0.00001);
    let (zoomed, zoomed_target, _) = camera.vectors(0.4, percent - 0.1);
    assert_eq!(zoomed_target, target);
    assert!((zoomed.distance(target) - 13.1).abs() < 0.00002);
    let (clamped, _, _) = camera.vectors(0.4, 2.);
    assert!((clamped.distance(target) - 99.).abs() < 0.00002);
}

#[test]
fn trading_post_camera_missing_scene_is_an_error_not_mesh_fit() {
    let error = read_perks_camera(&data_root(), u32::MAX).unwrap_err();
    assert!(error.contains("4294967295"), "{error}");
}
