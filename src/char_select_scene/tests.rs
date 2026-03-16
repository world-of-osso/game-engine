use super::*;
use crate::networking_auth::CharacterList;
use shared::components::CharacterAppearance;
use shared::protocol::CharacterListEntry;

fn character(character_id: u64, race: u8, sex: u8, name: &str) -> CharacterListEntry {
    CharacterListEntry {
        character_id,
        name: name.to_string(),
        level: 1,
        race,
        class: 1,
        appearance: CharacterAppearance {
            sex,
            ..Default::default()
        },
    }
}

#[test]
fn selected_scene_character_id_uses_selected_index() {
    let char_list = CharacterList(vec![
        character(10, 1, 0, "First"),
        character(20, 2, 0, "Second"),
    ]);

    assert_eq!(selected_scene_character_id(&char_list, Some(1)), Some(20));
}

#[test]
fn selected_scene_character_id_falls_back_to_first_character() {
    let char_list = CharacterList(vec![
        character(10, 1, 0, "First"),
        character(20, 2, 0, "Second"),
    ]);

    assert_eq!(selected_scene_character_id(&char_list, None), Some(10));
    assert_eq!(selected_scene_character_id(&char_list, Some(99)), Some(10));
}

#[test]
fn race_model_wow_path_covers_known_playable_races_and_sex() {
    assert_eq!(
        race_model_wow_path(1, 0),
        Some("character/human/male/humanmale_hd.m2")
    );
    assert_eq!(
        race_model_wow_path(2, 0),
        Some("character/orc/male/orcmale_hd.m2")
    );
    assert_eq!(
        race_model_wow_path(10, 1),
        Some("character/bloodelf/female/bloodelffemale_hd.m2")
    );
    assert_eq!(
        race_model_wow_path(10, 0),
        Some("character/bloodelf/male/bloodelfmale_hd.m2")
    );
    assert_eq!(race_model_wow_path(99, 0), None);
}

#[test]
fn camera_params_center_focused_placement_horizontally() {
    let warband = crate::warband_scene::WarbandScenes::load();
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
    let warband = crate::warband_scene::WarbandScenes::load();
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
fn camera_params_use_model_center_height_for_single_character_focus() {
    let warband = crate::warband_scene::WarbandScenes::load();
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
    let warband = crate::warband_scene::WarbandScenes::load();
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
    let warband = crate::warband_scene::WarbandScenes::load();
    let scene = warband
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .expect("Adventurer's Rest");
    let placement = selected_scene_placement(&warband, scene).expect("expected placement");
    let adt_path =
        crate::warband_scene::ensure_warband_terrain(scene).expect("expected warband terrain");
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
    let warband = crate::warband_scene::WarbandScenes::load();
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
    let warband = crate::warband_scene::WarbandScenes::load();
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
fn focused_placement_rotation_faces_camera_tightly() {
    let warband = crate::warband_scene::WarbandScenes::load();
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
