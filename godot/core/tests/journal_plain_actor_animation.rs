use game_engine_core::adventure_guide::{
    read_model_scene, read_plain_animation, resolve_plain_animation_clip,
};
use game_engine_core::spell_visual::read_animation_fallbacks;
use std::collections::{HashMap, HashSet};
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
fn journal_plain_clip_walks_real_animationdata_rows_including_zero() {
    let fallbacks = read_animation_fallbacks(&data_root().join("db2/12.1.5.70448")).unwrap();
    for (source, next) in [(27, 25), (26, 25), (213, 0), (0, 147)] {
        assert_eq!(fallbacks.get(&source), Some(&next));
    }
    for (requested, available, expected) in [
        (27, vec![0, 25], 25),
        (27, vec![25, 27], 27),
        (26, vec![0, 25], 25),
        (213, vec![0, 147], 0),
        (213, vec![147], 147),
        (0, vec![147], 147),
        (213, vec![213, 0], 213),
        (25, vec![25], 25),
    ] {
        let available: HashSet<u16> = available.into_iter().collect();
        assert_eq!(
            resolve_plain_animation_clip(requested, &available, &fallbacks).unwrap(),
            expected
        );
    }
    let absent = HashSet::from([999]);
    assert!(
        resolve_plain_animation_clip(27, &absent, &fallbacks)
            .unwrap_err()
            .contains("no base variation")
    );
    let missing = resolve_plain_animation_clip(65535, &absent, &fallbacks).unwrap_err();
    assert!(missing.contains("row 65535 missing"), "{missing}");
    let cycle = HashMap::from([(27, 25), (25, 27)]);
    let error = resolve_plain_animation_clip(27, &absent, &cycle).unwrap_err();
    assert!(error.contains("cycle at 27"), "{error}");
}

#[test]
fn journal_plain_animation_missing_actor_is_explicit() {
    let error = read_plain_animation(&data_root(), u32::MAX).unwrap_err();
    assert!(error.contains("4294967295"), "{error}");
}
