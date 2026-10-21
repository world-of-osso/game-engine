//! Exact selected70448 SoundKit/Entry source rows. No guessed voice duration.
use game_engine_core::spell_visual::{SoundFile, SpellVisualCatalog};
use std::path::Path;

#[test]
fn talking_head_voice_resolves_actual_orator_kits_without_spell_references() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/talking_head_sounds");
    for (kit, fdid) in [(250774, 5834729), (250775, 5834730), (250776, 5834731)] {
        let sound = SpellVisualCatalog::read_sound_kit(&dir, kit).unwrap();
        assert_eq!(sound.sound_kit_id, kit);
        assert_eq!(sound.volume, 0.799999952);
        assert!(!sound.looping);
        assert_eq!(
            sound.files,
            [SoundFile {
                fdid,
                frequency: 1,
                volume: 1.0
            }]
        );
    }
    assert!(
        SpellVisualCatalog::read_sound_kit(&dir, 0)
            .unwrap_err()
            .contains("SoundKit0")
    );
}
