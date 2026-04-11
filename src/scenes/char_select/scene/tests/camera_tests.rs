use super::*;

#[test]
fn camera_params_center_focused_placement_horizontally() {
    let warband = crate::scenes::char_select::warband::WarbandScenes::load();
    let scene = warband
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .expect("Adventurer's Rest");
    let placement = selected_scene_placement(&warband, scene).expect("expected placement");
    let (eye, focus, _) =
        camera_params(Some(scene), Some(&placement), ModelPresentation::default());
    let forward = (focus - eye).normalize();
    let right = forward.cross(Vec3::Y).normalize();
    let rel = placement.bevy_position() - eye;

    assert!(
        rel.dot(right).abs() < 0.001,
        "focused placement should sit on the camera centerline"
    );
}

#[test]
fn camera_params_use_tighter_single_character_framing() {
    let warband = crate::scenes::char_select::warband::WarbandScenes::load();
    let scene = warband
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .expect("Adventurer's Rest");
    let placement = selected_scene_placement(&warband, scene).expect("expected placement");
    let presentation = ModelPresentation::default();
    let (scene_eye, scene_focus, scene_fov) = camera_params(Some(scene), None, presentation);
    let (eye, focus, fov) = camera_params(Some(scene), Some(&placement), presentation);

    assert!(
        eye.distance(focus) < scene_eye.distance(scene_focus),
        "single-character framing should move the camera closer than the raw warband overview"
    );
    assert!(
        fov < scene_fov,
        "single-character framing should narrow the FOV from the warband overview"
    );
}

#[test]
fn char_select_fog_uses_scene_relative_falloff_for_single_character_view() {
    let warband = crate::scenes::char_select::warband::WarbandScenes::load();
    let scene = warband
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .expect("Adventurer's Rest");
    let placement = selected_scene_placement(&warband, scene).expect("expected placement");
    let (eye, focus, _) =
        camera_params(Some(scene), Some(&placement), ModelPresentation::default());
    let camera_distance = eye.distance(focus);
    let fog = char_select_fog(camera_distance);

    let FogFalloff::Linear { start, end } = fog.falloff else {
        panic!("char-select should use linear fog falloff");
    };

    assert!(
        (start - camera_distance * 2.0).abs() < 0.01,
        "fog start should scale from camera distance, got start={start:.2} camera_distance={camera_distance:.2}"
    );
    assert!(
        (end - camera_distance * 5.0).abs() < 0.01,
        "fog end should scale from camera distance, got end={end:.2} camera_distance={camera_distance:.2}"
    );
}

#[test]
fn camera_params_use_model_center_height_for_single_character_focus() {
    let warband = crate::scenes::char_select::warband::WarbandScenes::load();
    let scene = warband
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .expect("Adventurer's Rest");
    let placement = selected_scene_placement(&warband, scene).expect("expected placement");
    let presentation = ModelPresentation {
        customize_scale: 1.1,
        camera_distance_offset: -0.34,
    };

    let (_, focus, _) = camera_params(Some(scene), Some(&placement), presentation);

    assert!(
        (focus.y - (placement.bevy_position().y + presentation.customize_scale)).abs() < 0.001,
        "single-character focus should target model center height, got focus_y={} placement_y={} scale={}",
        focus.y,
        placement.bevy_position().y,
        presentation.customize_scale
    );
}

#[test]
fn camera_params_preserve_authored_vertical_offset_for_single_character_eye() {
    let warband = crate::scenes::char_select::warband::WarbandScenes::load();
    let scene = warband
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .expect("Adventurer's Rest");
    let placement = selected_scene_placement(&warband, scene).expect("expected placement");
    let presentation = ModelPresentation {
        customize_scale: 1.1,
        camera_distance_offset: -0.34,
    };

    let (eye, focus, _) = camera_params(Some(scene), Some(&placement), presentation);
    let authored_vertical = scene.bevy_position().y - scene.bevy_look_at().y;

    assert!(
        ((eye.y - focus.y) - authored_vertical).abs() < 0.001,
        "single-character eye should preserve authored vertical lift, got {} expected {}",
        eye.y - focus.y,
        authored_vertical
    );
}

#[test]
fn character_transform_snaps_character_up_to_warband_terrain() {
    let warband = crate::scenes::char_select::warband::WarbandScenes::load();
    let scene = warband
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .expect("Adventurer's Rest");
    let placement = selected_scene_placement(&warband, scene).expect("expected placement");
    let adt_path = crate::scenes::char_select::warband::ensure_warband_terrain(scene)
        .expect("expected warband terrain");
    let data = std::fs::read(&adt_path).expect("expected ADT data");
    let adt = crate::asset::adt::load_adt(&data).expect("expected ADT parse");
    let mut heightmap = TerrainHeightmap::default();
    let (tile_y, tile_x) = scene.tile_coords();
    heightmap.insert_tile(tile_y, tile_x, &adt);

    let transform = character_transform(
        Some(scene),
        Some(&placement),
        Some(&heightmap),
        ModelPresentation::default(),
    );
    let terrain_y = heightmap
        .height_at(transform.translation.x, transform.translation.z)
        .expect("expected terrain at placement");

    assert!(
        (transform.translation.y - terrain_y).abs() < 0.001,
        "character root should sit on terrain, got root_y={} terrain_y={terrain_y}",
        transform.translation.y
    );
}

#[test]
fn clamp_char_select_eye_keeps_camera_above_terrain() {
    let data = std::fs::read("data/terrain/azeroth_32_48.adt")
        .expect("expected test ADT data/terrain/azeroth_32_48.adt");
    let adt = crate::asset::adt::load_adt(&data).expect("expected ADT to parse");
    let mut heightmap = TerrainHeightmap::default();
    heightmap.insert_tile(32, 48, &adt);

    let [bx, _, bz] = crate::asset::m2::wow_to_bevy(-8949.0, -132.0, 83.0);
    let terrain_y = heightmap
        .height_at(bx, bz)
        .expect("expected terrain at sample position");
    let clamped = clamp_char_select_eye(Vec3::new(bx, terrain_y - 3.0, bz), Some(&heightmap));

    assert!(
        (clamped.y - (terrain_y + CHAR_SELECT_CAMERA_GROUND_CLEARANCE)).abs() < 0.001,
        "camera should stay above terrain, got camera_y={} terrain_y={terrain_y}",
        clamped.y
    );
}

#[test]
fn focused_placement_rotation_faces_camera_reasonably() {
    let warband = crate::scenes::char_select::warband::WarbandScenes::load();
    let scene = warband
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .expect("Adventurer's Rest");
    let placement = selected_scene_placement(&warband, scene).expect("expected placement");
    let rotation = single_character_rotation(scene, &placement, ModelPresentation::default());
    let (eye, _, _) = camera_params(Some(scene), Some(&placement), ModelPresentation::default());
    let to_camera = (eye - placement.bevy_position()).normalize_or_zero();
    let facing = rotation * Vec3::X;
    let angle = facing.angle_between(to_camera).to_degrees();

    assert!(
        angle < 25.0,
        "focused placement should face mostly toward the camera, got {angle:.2} degrees"
    );
}

#[test]
fn camera_params_apply_model_distance_offset() {
    let warband = crate::scenes::char_select::warband::WarbandScenes::load();
    let scene = warband
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .expect("Adventurer's Rest");
    let placement = selected_scene_placement(&warband, scene).expect("expected placement");
    let default_presentation = ModelPresentation::default();
    let human_presentation = ModelPresentation {
        customize_scale: 1.1,
        camera_distance_offset: -0.34,
    };
    let (default_eye, default_focus, _) =
        camera_params(Some(scene), Some(&placement), default_presentation);
    let (eye, focus, _) = camera_params(Some(scene), Some(&placement), human_presentation);

    assert!(eye.distance(focus) < default_eye.distance(default_focus));
}

#[test]
fn orbit_from_eye_focus_preserves_initial_yaw() {
    let eye = Vec3::new(4.0, 3.0, -2.0);
    let focus = Vec3::new(1.5, 1.0, 0.5);

    let orbit = orbit_from_eye_focus(eye, focus);

    assert!(
        orbit_eye(&orbit).distance(eye) < 1e-5,
        "reconstructed orbit eye should match the authored eye position"
    );
}

#[test]
fn char_select_orbit_camera_moves_transform_and_clamps_drag_limits() {
    let mut app = App::new();
    app.insert_resource(crate::client_options::CameraOptions::default());
    app.insert_resource(AccumulatedMouseMotion {
        delta: Vec2::new(500.0, 500.0),
    });
    let mut mouse_buttons = ButtonInput::<MouseButton>::default();
    mouse_buttons.press(MouseButton::Left);
    app.insert_resource(mouse_buttons);

    let orbit = orbit_from_eye_focus(Vec3::new(0.0, 2.0, 6.0), Vec3::new(0.0, 1.0, 0.0));
    let before = Transform::from_translation(orbit_eye(&orbit)).looking_at(orbit.focus, Vec3::Y);
    let entity = app.world_mut().spawn((orbit, before)).id();

    app.world_mut()
        .run_system_once(char_select_orbit_camera)
        .expect("char-select orbit camera should run");

    let orbit = app
        .world()
        .get::<camera::CharSelectOrbit>(entity)
        .expect("char-select orbit");
    let after = app
        .world()
        .get::<Transform>(entity)
        .expect("char-select transform");

    assert!(
        after.translation.distance(before.translation) > 0.1,
        "dragging should move the live camera transform"
    );
    assert!(
        (orbit.yaw + std::f32::consts::FRAC_PI_8).abs() < 0.0001,
        "yaw should clamp at the authored drag limit"
    );
    assert!(
        (orbit.pitch - 0.15).abs() < 0.0001,
        "pitch should clamp at the authored drag limit"
    );
}

#[test]
fn orbit_input_debug_state_reports_live_inputs_and_target_count() {
    let debug = orbit_input_debug_state(true, Vec2::new(12.0, -3.0), 2);

    assert!(debug.left_mouse_pressed);
    assert!(debug.has_mouse_motion);
    assert_eq!(debug.orbit_entity_count, 2);
}

#[test]
fn orbit_input_debug_state_marks_zero_motion_and_missing_targets() {
    let debug = orbit_input_debug_state(false, Vec2::ZERO, 0);

    assert!(!debug.left_mouse_pressed);
    assert!(!debug.has_mouse_motion);
    assert_eq!(debug.orbit_entity_count, 0);
}

#[test]
fn should_log_orbit_input_logs_state_transitions_once() {
    let missing_camera = orbit_input_debug_state(false, Vec2::ZERO, 0);

    assert!(should_log_orbit_input(None, missing_camera));
    assert!(!should_log_orbit_input(
        Some(missing_camera),
        missing_camera
    ));
}

#[test]
fn should_log_orbit_input_always_logs_live_motion() {
    let dragging = orbit_input_debug_state(true, Vec2::new(8.0, 0.0), 1);

    assert!(should_log_orbit_input(Some(dragging), dragging));
}

#[test]
fn debug_orbit_camera_system_ignores_char_select_orbit_entities() {
    let mut app = App::new();
    app.insert_resource(crate::client_options::CameraOptions::default());
    app.insert_resource(AccumulatedMouseMotion {
        delta: Vec2::new(24.0, -6.0),
    });
    app.insert_resource(AccumulatedMouseScroll::default());
    let mut mouse_buttons = ButtonInput::<MouseButton>::default();
    mouse_buttons.press(MouseButton::Left);
    app.insert_resource(mouse_buttons);
    app.add_systems(Update, crate::orbit_camera::orbit_camera_system);

    let orbit = orbit_from_eye_focus(Vec3::new(0.0, 2.0, 6.0), Vec3::new(0.0, 1.0, 0.0));
    let before = Transform::from_translation(orbit_eye(&orbit)).looking_at(orbit.focus, Vec3::Y);
    let entity = app.world_mut().spawn((orbit, before)).id();

    app.update();

    let after = app
        .world()
        .get::<Transform>(entity)
        .expect("char-select transform");
    assert_eq!(
        *after, before,
        "debug orbit camera system should ignore CharSelectOrbit-only entities"
    );
}

#[test]
fn debug_and_char_select_orbit_systems_share_mouse_motion_without_consuming_it() {
    let mut app = App::new();
    app.insert_resource(crate::client_options::CameraOptions::default());
    app.insert_resource(AccumulatedMouseMotion {
        delta: Vec2::new(18.0, -5.0),
    });
    app.insert_resource(AccumulatedMouseScroll::default());
    let mut mouse_buttons = ButtonInput::<MouseButton>::default();
    mouse_buttons.press(MouseButton::Left);
    app.insert_resource(mouse_buttons);
    app.add_systems(
        Update,
        (
            crate::orbit_camera::orbit_camera_system,
            char_select_orbit_camera,
        ),
    );

    let char_select_orbit =
        orbit_from_eye_focus(Vec3::new(0.0, 2.0, 6.0), Vec3::new(0.0, 1.0, 0.0));
    let char_select_transform = Transform::from_translation(orbit_eye(&char_select_orbit))
        .looking_at(char_select_orbit.focus, Vec3::Y);
    let char_select_entity = app
        .world_mut()
        .spawn((char_select_orbit.clone(), char_select_transform))
        .id();

    let debug_orbit = crate::orbit_camera::OrbitCamera::new(Vec3::ZERO, 6.0);
    let debug_transform =
        Transform::from_translation(debug_orbit.eye_position()).looking_at(Vec3::ZERO, Vec3::Y);
    let debug_entity = app.world_mut().spawn((debug_orbit, debug_transform)).id();

    app.update();

    let char_select_after = app
        .world()
        .get::<Transform>(char_select_entity)
        .expect("char-select transform");
    let debug_after = app
        .world()
        .get::<Transform>(debug_entity)
        .expect("debug transform");
    assert_ne!(
        *char_select_after, char_select_transform,
        "char-select orbit should still receive shared mouse motion"
    );
    assert_ne!(
        *debug_after, debug_transform,
        "debug orbit should still receive shared mouse motion"
    );
}

#[test]
fn focused_placement_rotation_faces_camera_tightly() {
    let warband = crate::scenes::char_select::warband::WarbandScenes::load();
    let scene = warband
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .expect("Adventurer's Rest");
    let placement = selected_scene_placement(&warband, scene).expect("expected placement");
    let rotation = single_character_rotation(scene, &placement, ModelPresentation::default());
    let (eye, _, _) = camera_params(Some(scene), Some(&placement), ModelPresentation::default());
    let to_camera = (eye - placement.bevy_position()).normalize_or_zero();
    let to_camera = Vec3::new(to_camera.x, 0.0, to_camera.z).normalize_or_zero();
    let facing = rotation * Vec3::X;
    let facing = Vec3::new(facing.x, 0.0, facing.z).normalize_or_zero();
    let angle = facing.angle_between(to_camera).to_degrees();

    assert!(
        angle < 1.0,
        "single-character rotation should face the camera horizontally, got {angle:.2} degrees"
    );
}

#[test]
fn char_select_ambient_brightness_matches_scene_lighting_budget() {
    assert_eq!(lighting::CHAR_SELECT_AMBIENT_BRIGHTNESS, 150.0);
}
