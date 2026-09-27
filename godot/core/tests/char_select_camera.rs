use game_engine_core::char_select_camera_data::solo_camera_params;
use game_engine_core::customization_data::ModelPresentation;
use glam::Vec3;

#[test]
fn scene_one_solo_camera_preserves_authored_lift_and_narrows_fov() {
    // WarbandScene.csv scene 1 and first character slot, mapped from WoW [x, y, z] to [x, z, -y].
    let scene_eye = Vec3::new(-2982.99, 455.523, -468.057);
    let scene_focus = Vec3::new(-2985.54, 454.399, -456.018);
    let character = Vec3::new(-2981.82, 452.826, -457.35);

    let (eye, focus, fov) = solo_camera_params(
        scene_eye,
        scene_focus,
        65.0,
        character,
        ModelPresentation::default(),
    );

    assert!((focus - Vec3::new(-2981.82, 453.826, -457.35)).length() < 0.001);
    assert!(((eye.y - focus.y) - 1.1240234).abs() < 0.0001);
    assert!((eye - Vec3::new(-2980.4934, 454.95, -463.61313)).length() < 0.002);
    assert!(((eye - focus).length() - 6.5).abs() < 0.001);
    assert_eq!(fov, 55.0);
}

#[test]
fn presentation_controls_focus_and_clamps_distance_to_authored_range() {
    let scene_eye = Vec3::new(0.0, 2.0, 10.0);
    let scene_focus = Vec3::ZERO;
    let character = Vec3::new(5.0, 3.0, 6.0);
    let presentation = ModelPresentation {
        customize_scale: 1.1,
        camera_distance_offset: -20.0,
    };

    let (eye, focus, fov) =
        solo_camera_params(scene_eye, scene_focus, 40.0, character, presentation);
    assert!((focus - Vec3::new(5.0, 4.1, 6.0)).length() < 0.0001);
    assert!(((eye - focus).length() - 3.5).abs() < 0.0001);
    assert_eq!(eye.y - focus.y, 2.0);
    assert_eq!(fov, 40.0);

    let (eye, focus, _) = solo_camera_params(
        scene_eye,
        scene_focus,
        65.0,
        character,
        ModelPresentation {
            customize_scale: 0.0,
            camera_distance_offset: 100.0,
        },
    );
    assert!((focus.y - 3.01).abs() < 0.0001);
    assert!(((eye - focus).length() - scene_eye.length()).abs() < 0.0001);
}
