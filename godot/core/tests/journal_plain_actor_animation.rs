use game_engine_core::adventure_guide::{read_model_scene, read_plain_animation_25};
use std::path::PathBuf;

fn data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

#[test]
fn journal_plain_animation_25_keeps_source_models_and_excludes_kits_effects_other_clips() {
    for (scene, actor_id) in [(673, 1848), (674, 1849)] {
        let (actor, _) = read_model_scene(&data_root(), scene).unwrap();
        assert_eq!(actor.id, actor_id);
        assert_eq!(actor.animation, 25);
        assert_eq!(
            read_plain_animation_25(&data_root(), actor.id).unwrap(),
            Some(25)
        );
    }
    // Fyrakk actor1959 also declares25, but AnimKit9789 owns its pose.
    // Igira1929 additionally requests spellVisualKit186742; no effect expansion.
    // Atrigan93 and Remornia1061 have other source AnimationIDs, outside this slice.
    for actor in [1959, 1929, 93, 1061, 10, 1696] {
        assert_eq!(read_plain_animation_25(&data_root(), actor).unwrap(), None);
    }
}

#[test]
fn journal_plain_animation_missing_actor_is_explicit() {
    let error = read_plain_animation_25(&data_root(), u32::MAX).unwrap_err();
    assert!(error.contains("4294967295"), "{error}");
}
