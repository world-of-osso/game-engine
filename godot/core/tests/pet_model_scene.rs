use game_engine_core::pet_model_scene::PetModelScene;
use glam::Vec3;

fn squirrel_card() -> PetModelScene {
    serde_json::from_str(
        r#"{
        "id":6,"target":[0,0,0],"yaw":2.6179938316345215,"pitch":0,"roll":0,
        "zoom_distance":14,"actor_position":[0,0,0],"actor_orientation":[0,0,0],
        "origin_flags":9,"normalized_scale":1,"scale":7
    }"#,
    )
    .unwrap()
}

#[test]
fn pet_model_scene_uses_authored_orbit_camera_not_mesh_fit_distance() {
    let scene = squirrel_card();
    let (eye, target, up) = scene.camera_vectors();
    assert!(eye.abs_diff_eq(Vec3::new(12.124355, 0.0, 7.0), 0.00001));
    assert_eq!(target, Vec3::ZERO);
    assert_eq!(up, Vec3::Y);
    assert!((eye.distance(target) - 14.0).abs() < 0.00001);
}

#[test]
fn pet_model_scene_normalizes_like_blizzard_actor_mixin() {
    let scene = squirrel_card();
    let size = Vec3::new(0.6, 1.0, 0.7);
    assert!((scene.actor_scale(size) - 11.21).abs() < 0.00001);
    let mut loadout = scene;
    loadout.scale = 3.0;
    loadout.normalized_scale = 0.85;
    let expected = 3.0 / (1.0 + (0.7 / 1.121 - 1.0) * 0.85);
    assert!((loadout.actor_scale(size) - expected).abs() < 0.00001);
    loadout.normalized_scale = 0.0;
    assert_eq!(loadout.actor_scale(size), 3.0);
}

#[test]
fn pet_model_scene_centers_only_authored_origin_axes() {
    let scene = squirrel_card();
    let center = Vec3::new(0.2, 0.5, -0.1);
    assert_eq!(scene.actor_origin(center, 2.0), Vec3::new(0.0, -1.0, 0.0));
    let mut scene = scene;
    scene.origin_flags = 2 | 4 | 8;
    assert_eq!(scene.actor_origin(center, 2.0), -center * 2.0);
}
