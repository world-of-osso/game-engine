use game_engine_core::adventure_guide::{Catalog, read_model_scene};
use glam::Vec3;
use std::path::PathBuf;

fn data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

#[test]
fn journal_creatures_retain_distinct_authored_scenes_and_raw_camera_pose() {
    let root = data_root();
    let catalog = Catalog::load(&root.join("cache/adventure_guide.sqlite")).unwrap();
    for (id, scene, actor_id, camera_id, normalized, z) in
        [(382, 9, 10, 6, 0.4, 0.1), (426, 65, 88, 69, 0.8, 0.5)]
    {
        let creature = catalog.creatures.iter().find(|row| row.id == id).unwrap();
        assert_eq!(creature.scene, scene);
        let (actor, camera) = read_model_scene(&root, creature.scene).unwrap();
        assert_eq!(actor.id, actor_id);
        assert_eq!(actor.normalized_scale, normalized);
        assert_eq!(actor.position, [0., 0., z]);
        assert_eq!(actor.scale, 3.);
        assert_eq!(camera.id, camera_id);
        assert_eq!(camera.yaw, 2.79252672);
        assert_eq!(camera.target, [0., 0., 0.25]);
        let (eye, target, up) = camera.vectors(0., camera.zoom_percent());
        assert!(eye.abs_diff_eq(Vec3::new(9.396926, 0.25, 3.420202), 0.00001));
        assert_eq!(target, Vec3::new(0., 0.25, 0.));
        assert_eq!(up, Vec3::Y);
        let size = Vec3::new(0.9984, 4.236, 1.121);
        let scale = actor.actor_scale(size);
        assert!((scale - 3. / (1. + normalized)).abs() < 0.00001);
        assert!(
            actor
                .actor_origin(Vec3::new(0., 2., 0.), scale)
                .abs_diff_eq(Vec3::new(0., z - 2. * scale, 0.), 0.00001)
        );
    }
}

#[test]
fn journal_fixed_zoom_uses_authored_spline_endpoint_not_zero_offsets() {
    // OrbitCameraMixin GetDerived*: 1 - zoomPercent; MathUtil PercentageBetween
    // returns0 for min=max, so these two authored fixed scenes use endpoint1.
    let (_, yaw) = read_model_scene(&data_root(), 268).unwrap();
    assert!((yaw.yaw - (2.79252672 + 0.52359879)).abs() < 0.000001);
    let (_, target) = read_model_scene(&data_root(), 1399).unwrap();
    assert_eq!(target.target, [0., 0., -0.75]);
    let (eye, center, _) = target.vectors(0., target.zoom_percent());
    assert_eq!(center, Vec3::new(0., -0.75, 0.));
    assert!((eye.distance(center) - 15.).abs() < 0.00001);
}

#[test]
fn journal_missing_scene_is_an_error_not_mesh_fit() {
    let error = read_model_scene(&data_root(), u32::MAX).unwrap_err();
    assert!(error.contains("4294967295"), "{error}");
}
