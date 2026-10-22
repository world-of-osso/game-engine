//! Raw1751 rows are source data, not an inferred native camera/FOV.
use game_engine_core::talking_head_camera::read_camera;
use std::path::Path;

#[test]
fn talking_head_camera_retains_source_model_transform_and_fixed_camera_without_fitting() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/talking_head_camera");
    let camera = read_camera(&root, 1751).unwrap();
    assert_eq!(camera.id, 1751);
    assert_eq!(
        camera.model_position,
        [2.82894468, -0.0669228658, -2.48507595]
    );
    assert_eq!(camera.yaw_pitch_roll, [0.220450103, 0.0, 0.0]);
    assert_eq!(camera.camera_position, [4.0, 0.0, 0.0]);
    assert_eq!(camera.camera_target, [0.0, 0.0, 0.0]);
    assert_eq!(camera.flags, 0);
    assert!(read_camera(&root, 0).unwrap_err().contains("UiCamera0"));
}
