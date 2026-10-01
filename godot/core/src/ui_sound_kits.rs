//! Interface sound kits that native frames play (`PlaySound(SOUNDKIT.*)`), with their
//! `SoundKit.VolumeFloat` and `SoundKitEntry` files from 12.1.0.69933
//! (`SoundKitConstants.lua` names). Files are local Ogg copies at `sounds/ui/{fdid}.ogg`.

/// `SOUNDKIT.IG_CHARACTER_INFO_OPEN`.
pub const IG_CHARACTER_INFO_OPEN: u32 = 839;
/// `SOUNDKIT.IG_CHARACTER_INFO_CLOSE`.
pub const IG_CHARACTER_INFO_CLOSE: u32 = 840;
/// `SOUNDKIT.IG_MAINMENU_OPTION_CHECKBOX_ON`.
pub const IG_MAINMENU_OPTION_CHECKBOX_ON: u32 = 856;
/// `SOUNDKIT.ITEM_REPAIR`.
pub const ITEM_REPAIR: u32 = 7994;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiSoundKit {
    pub id: u32,
    pub volume: f32,
    /// `SoundKitEntry.FileDataID`s, all `Frequency` 1 and `Volume` 1.
    pub files: &'static [u32],
}

const KITS: [UiSoundKit; 4] = [
    UiSoundKit {
        id: IG_CHARACTER_INFO_OPEN,
        volume: 0.398_107_17,
        files: &[567_507], // sound/interface/ucharactersheetopen.ogg
    },
    UiSoundKit {
        id: IG_CHARACTER_INFO_CLOSE,
        volume: 0.501_187_2,
        files: &[567_433], // sound/interface/ucharactersheetclose.ogg
    },
    UiSoundKit {
        id: IG_MAINMENU_OPTION_CHECKBOX_ON,
        volume: 0.251_188_64,
        files: &[567_407], // sound/interface/uchatscrollbutton.ogg
    },
    UiSoundKit {
        id: ITEM_REPAIR,
        volume: 0.69,
        // sound/spells/tradeskills/mininghit{a..}.ogg
        files: &[569_801, 569_811, 569_792, 569_794, 569_821],
    },
];

pub fn ui_sound_kit(id: u32) -> Option<&'static UiSoundKit> {
    KITS.iter().find(|kit| kit.id == id)
}

impl UiSoundKit {
    /// The file a play picks: `SoundKitEntry` rows are equally likely (`Frequency` 1).
    pub fn file(&self, roll: u32) -> u32 {
        self.files[roll as usize % self.files.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merchant_kits_carry_their_soundkit_rows() {
        let open = ui_sound_kit(IG_CHARACTER_INFO_OPEN).unwrap();
        assert_eq!((open.volume, open.file(0)), (0.398_107_17, 567_507));
        let repair = ui_sound_kit(ITEM_REPAIR).unwrap();
        assert_eq!(repair.files.len(), 5);
        assert_eq!(repair.file(6), 569_811);
        assert_eq!(ui_sound_kit(1), None);
    }
}
