use game_engine_core::asset::m2_format::m2_camera::*;
use std::path::PathBuf;

#[test]
fn human_female_portrait_camera_reads_authored_sparse_fov() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/models/1000764.m2");
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let camera = parse_portrait_camera(&bytes).unwrap();
    assert_eq!(camera.camera_type, 0);
    assert_eq!(camera.position, [0.62774545, -0.40620404, 1.7988782]);
    assert_eq!(camera.target, [-0.03356784, 0.0626493, 1.7492048]);
    assert_eq!(camera.fov, 0.7853966);
    assert_eq!((camera.near_clip, camera.far_clip), (0.22222222, 27.777779));
}

#[test]
fn authored_creation_cameras_match_inspected_values() {
    let expected = [
        (
            623712,
            [-4.364788, 3.848871, 0.5747803],
            [2.290539, -1.0632224, 1.0725104],
        ),
        (
            623714,
            [-4.9478707, 4.3812137, 0.65263426],
            [2.608921, -1.1962228, 1.0975605],
        ),
        (
            623716,
            [-4.9478707, 4.3812137, 0.65263426],
            [2.608921, -1.1962228, 1.2177819],
        ),
    ];
    for (id, position, target) in expected {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("data/models/{id}.m2"));
        let bytes =
            std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let camera = parse_camera_snapshot(&bytes).unwrap();
        assert_eq!(camera.camera_type, -1, "{id}");
        assert_eq!(camera.position, position, "{id}");
        assert_eq!(camera.target, target, "{id}");
        assert_eq!(camera.fov, 0.84570783, "{id}");
        assert_eq!(camera.roll, 0.0, "{id}");
        assert_eq!(camera.near_clip, 0.22222222, "{id}");
        assert_eq!(camera.far_clip, 513.9152, "{id}");
    }
}
