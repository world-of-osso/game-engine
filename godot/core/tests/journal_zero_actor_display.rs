use game_engine_core::{adventure_guide::read_model_scene, model_scene_actor::read_scene_actors};
use glam::Vec3;
use std::path::PathBuf;

fn data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

#[test]
fn journal_zero_display_actors_use_retail_defaults_without_losing_authored_transforms() {
    for (scene, actor_id, flags, normalized, z, camera_id, distance) in [
        (609, 1696, 9, 1., 5., 595, 0.5),
        (978, 2418, 9, 0.14, 0.5, 960, 10.),
        (1380, 4131, 0, 0., 0., 1361, 8.),
    ] {
        let (actor, camera) = read_model_scene(&data_root(), scene).unwrap();
        assert_eq!(actor.id, actor_id);
        assert_eq!((actor.scale, actor.alpha, actor.animation), (1., 1., 0));
        assert_eq!(actor.flags, flags);
        assert_eq!(actor.normalized_scale, normalized);
        assert_eq!(actor.position, [0., 0., z]);
        assert_eq!(camera.id, camera_id);
        let (eye, target, _) = camera.vectors(0., camera.zoom_percent());
        assert!((eye.distance(target) - distance).abs() < 0.00001);
        let size = Vec3::new(0.9984, 4.236, 1.121);
        let scale = actor.actor_scale(size);
        assert!((scale - 1. / (1. + normalized)).abs() < 0.00001);
        let expected_y = if flags & 8 != 0 { z - 2. * scale } else { z };
        assert!(
            actor
                .actor_origin(Vec3::new(0., 2., 0.), scale)
                .abs_diff_eq(Vec3::new(0., expected_y, 0.), 0.00001)
        );
    }
}

#[test]
fn journal_defaults_do_not_change_the_shared_nonjournal_reader() {
    for scene in [609, 978, 1380] {
        assert_eq!(
            read_scene_actors(&data_root(), scene).unwrap_err(),
            "UiModelSceneActorDisplay 0 missing"
        );
    }
    let (actor, _) = read_model_scene(&data_root(), 9).unwrap();
    assert_eq!(
        (actor.id, actor.scale, actor.alpha, actor.animation),
        (10, 3., 1., 0)
    );
}
