//! Cage of Carnage's normal-play creature_text sounds. TDB BroadcastText female
//! SoundKitID2 overrides creature_text.Sound (TC CreatureTextMgr::SendChat).
//! Exact local 12.1.0.69933 SoundKit/Entry rows; all entries have frequency/volume1
//! and PlayerConditionID0. Files are local CASC Ogg copies at sounds/{fdid}.ogg.
use crate::ui_sound_kits::UiSoundKit;

const KITS: &[UiSoundKit] = &[
    kit(269088, &[6177849]),
    kit(269089, &[6177850]),
    kit(269090, &[6177851]),
    kit(269091, &[6177852]),
    kit(269092, &[6177853]),
    kit(269096, &[6177857]),
    kit(269097, &[6177858]),
    kit(269098, &[6177859]),
    kit(269101, &[6177862]),
];

const fn kit(id: u32, files: &'static [u32]) -> UiSoundKit {
    UiSoundKit {
        id,
        volume: 0.799999952,
        files,
    }
}

pub fn creature_text_sound_kit(id: u32) -> Option<&'static UiSoundKit> {
    KITS.iter().find(|kit| kit.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_cage_voice_uses_female_broadcast_sounds_not_unrelated_text_kits() {
        let prepare = creature_text_sound_kit(269092).unwrap();
        assert_eq!(prepare.file(0), 6177853);
        assert_eq!(prepare.volume, 0.799999952);
        assert_eq!(creature_text_sound_kit(269096).unwrap().file(0), 6177857);
        assert_eq!(creature_text_sound_kit(269097).unwrap().file(0), 6177858);
        assert_eq!(creature_text_sound_kit(269098).unwrap().file(0), 6177859);
        assert!(creature_text_sound_kit(276108).is_none());
        assert!(creature_text_sound_kit(276094).is_none());
    }
}
