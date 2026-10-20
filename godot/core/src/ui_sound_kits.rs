//! Interface sound kits that native frames play (`PlaySound(SOUNDKIT.*)`), with their
//! `SoundKit.VolumeFloat` and `SoundKitEntry` files cached from Retail
//! (`SoundKitConstants.lua` names). Files are local Ogg copies at `sounds/ui/{fdid}.ogg`.

/// `SOUNDKIT.IG_CHARACTER_INFO_OPEN`.
pub const IG_CHARACTER_INFO_OPEN: u32 = 839;
/// `SOUNDKIT.IG_CHARACTER_INFO_CLOSE`.
pub const IG_CHARACTER_INFO_CLOSE: u32 = 840;
/// `SOUNDKIT.IG_MAINMENU_OPTION_CHECKBOX_ON`.
pub const IG_MAINMENU_OPTION_CHECKBOX_ON: u32 = 856;
/// `SOUNDKIT.IG_MAINMENU_OPTION_CHECKBOX_OFF`.
pub const IG_MAINMENU_OPTION_CHECKBOX_OFF: u32 = 857;
/// `SOUNDKIT.ITEM_REPAIR`.
pub const ITEM_REPAIR: u32 = 7994;
/// Cached Mainline SoundKitConstants.lua RAID_BOSS_EMOTE_WARNING.
pub const RAID_BOSS_EMOTE_WARNING: u32 = 12197;
/// Retail ReadyCheckListenerFrame OnShow / SoundKitConstants.lua:84.
pub const READY_CHECK: u32 = 8960;
/// Retail SoundKitConstants.lua PlayerCountdown kits.
pub const UI_COUNTDOWN_TIMER: u32 = 158565;
pub const UI_COUNTDOWN_FINISHED: u32 = 158566;
pub const UI_COUNTDOWN_BAR_STATE_STARTS: u32 = 158958;
pub const UI_COUNTDOWN_BAR_STATE_FINISHED: u32 = 158959;
pub const UI_COUNTDOWN_MEDIUM_NUMBER_FINISHED: u32 = 158960;
/// Retail Expert submode pointer and assigned-key activation sounds.
pub const HOUSING_EXPERTMODE_SUB_MENU_BUTTON_TOGGLE: u32 = 286137;
pub const HOUSING_EXPERTMODE_SUB_MENU_BUTTON_TOGGLE_SHORTCUT: u32 = 304887;
/// Retail PerksProgram.OnShow/OnHide, active70391 SoundKitConstants.lua.
pub const TRADING_POST_UI_MENU_OPEN: u32 = 218932;
pub const TRADING_POST_UI_MENU_CLOSE: u32 = 218933;
/// Retail product OnEnter, including the inherited frozen product template.
pub const TRADING_POST_UI_ITEM_HOVER: u32 = 218934;
/// Retail Products.OnProductSelected, except silenced provider reselection.
pub const TRADING_POST_UI_ITEM_SELECT: u32 = 218230;
/// Retail FreezeDraggedItem after accepting the actual dragged product.
pub const TRADING_POST_UI_ITEM_LOCKING: u32 = 218229;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiSoundKit {
    pub id: u32,
    pub volume: f32,
    /// `SoundKitEntry.FileDataID`s, all `Frequency` 1 and `Volume` 1.
    pub files: &'static [u32],
}

const KITS: &[UiSoundKit] = &[
    UiSoundKit {
        id: HOUSING_EXPERTMODE_SUB_MENU_BUTTON_TOGGLE,
        volume: 0.125_892_53,
        files: &[6_838_456],
    },
    UiSoundKit {
        id: HOUSING_EXPERTMODE_SUB_MENU_BUTTON_TOGGLE_SHORTCUT,
        volume: 0.125_892_53,
        files: &[6_838_456],
    },
    UiSoundKit {
        id: TRADING_POST_UI_MENU_OPEN,
        volume: 0.562_341_33,
        files: &[4956048, 4956050, 4956052, 4956054, 4956056],
    },
    // Lifecycle/files/gain only. Kit218933 pitch adjustment/variation remains a
    // named audio-engine gap; this existing player deliberately has no pitch path.
    UiSoundKit {
        id: TRADING_POST_UI_MENU_CLOSE,
        volume: 0.01,
        files: &[
            4555731, 4555733, 4555735, 4555737, 4555739, 4555741, 4555743, 4555745, 4555747,
            4555749,
        ],
    },
    UiSoundKit {
        id: RAID_BOSS_EMOTE_WARNING,
        volume: 0.398_107_17,
        files: &[567_394],
    },
    UiSoundKit {
        id: UI_COUNTDOWN_TIMER,
        volume: 0.9,
        files: &[567474],
    },
    UiSoundKit {
        id: UI_COUNTDOWN_FINISHED,
        volume: 0.8,
        files: &[567438],
    },
    UiSoundKit {
        id: UI_COUNTDOWN_BAR_STATE_STARTS,
        volume: 0.9,
        files: &[566236],
    },
    UiSoundKit {
        id: UI_COUNTDOWN_BAR_STATE_FINISHED,
        volume: 0.9,
        files: &[566236],
    },
    UiSoundKit {
        id: UI_COUNTDOWN_MEDIUM_NUMBER_FINISHED,
        volume: 0.9,
        files: &[566236],
    },
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
    // Active70391 SoundKit857/Entry926: same file as ON, with its authored OFF gain.
    UiSoundKit {
        id: IG_MAINMENU_OPTION_CHECKBOX_OFF,
        volume: 0.5,
        files: &[567_407],
    },
    UiSoundKit {
        id: ITEM_REPAIR,
        volume: 0.69,
        // sound/spells/tradeskills/mininghit{a..}.ogg
        files: &[569_801, 569_811, 569_792, 569_794, 569_821],
    },
    UiSoundKit {
        id: TRADING_POST_UI_ITEM_HOVER,
        volume: 0.891_250_9,
        files: &[4_906_953],
    },
    UiSoundKit {
        id: TRADING_POST_UI_ITEM_LOCKING,
        volume: 0.707_945_76,
        files: &[4_908_453, 4_908_455, 4_908_457, 4_908_459],
    },
    UiSoundKit {
        id: READY_CHECK,
        volume: 1.0,
        files: &[567_478],
    },
    UiSoundKit {
        id: TRADING_POST_UI_ITEM_SELECT,
        volume: 0.79,
        files: &[4_906_955],
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
    fn timeline_native_sound_uses_cached_retail_raid_boss_warning_rows() {
        let warning = ui_sound_kit(12197).expect("Retail RaidBossEmote warning kit");
        assert_eq!((warning.volume, warning.file(0)), (0.398_107_17, 567_394));
    }

    #[test]
    fn journeys_watch_sounds_use_the_authored_on_off_gain_and_existing_clip() {
        let on = ui_sound_kit(856).unwrap();
        let off = ui_sound_kit(857).expect("Retail checkbox OFF kit");
        assert_eq!((on.volume, on.file(0)), (0.251_188_64, 567_407));
        assert_eq!((off.volume, off.file(0)), (0.5, 567_407));
    }

    #[test]
    fn housing_expert_submode_sounds_resolve_the_same_authored_clip_and_gain() {
        for id in [286137, 304887] {
            let kit = ui_sound_kit(id).expect("Retail Expert submode sound kit");
            assert_eq!((kit.volume, kit.file(0)), (0.125_892_53, 6_838_456));
            assert_eq!(kit.file(1), 6_838_456);
        }
    }

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
