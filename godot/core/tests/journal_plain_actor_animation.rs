use game_engine_core::adventure_guide::{read_model_scene, read_plain_animation};
use std::path::PathBuf;

fn data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

#[test]
fn journal_plain_animation_25_keeps_source_models_and_excludes_kits_effects() {
    for (scene, actor_id) in [(673, 1848), (674, 1849)] {
        let (actor, _) = read_model_scene(&data_root(), scene).unwrap();
        assert_eq!(actor.id, actor_id);
        assert_eq!(actor.animation, 25);
        assert_eq!(
            read_plain_animation(&data_root(), actor.id).unwrap(),
            Some(25)
        );
    }
    // Fyrakk actor1959 also declares25, but AnimKit9789 owns its pose.
    // Igira1929 additionally requests spellVisualKit186742; no effect expansion.
    for actor in [1959, 1929, 10, 1696] {
        assert_eq!(read_plain_animation(&data_root(), actor).unwrap(), None);
    }
}

#[test]
fn journal_remaining_plain_clips_keep_their_authored_actor_camera_rows() {
    for (scene, actor_id, clip, camera_id) in [
        (70, 93, 27, 74),
        (357, 1061, 213, 363),
        (1071, 2672, 26, 1055),
    ] {
        let (actor, camera) = read_model_scene(&data_root(), scene).unwrap();
        assert_eq!(
            (actor.id, actor.animation, camera.id),
            (actor_id, clip, camera_id)
        );
        assert_eq!(
            read_plain_animation(&data_root(), actor_id).unwrap(),
            Some(clip)
        );
    }
}

#[test]
fn journal_plain_animation_missing_actor_is_explicit() {
    let error = read_plain_animation(&data_root(), u32::MAX).unwrap_err();
    assert!(error.contains("4294967295"), "{error}");
}
