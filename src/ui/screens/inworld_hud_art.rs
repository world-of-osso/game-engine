//! Retail micro menu and bag bar art: `UiTextureAtlasMember.csv` crops on their atlas sheets
//! (`UiTextureAtlas.csv`). Names follow `Blizzard_MicroMenu/Mainline/MainMenuBarMicroButtons.lua`
//! (`UI-HUD-MicroMenu-<Name>-Up`) and `Blizzard_MainMenuBarBagButtons` (`bag-main`,
//! `bag-border-empty`).

/// Pixel crop (left, right, top, bottom) on a sheet.
#[derive(Clone, Copy)]
pub(super) struct SheetCrop {
    pub fdid: u32,
    sheet_w: f32,
    sheet_h: f32,
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

impl SheetCrop {
    /// Normalized `tex_coords` attribute value.
    pub fn tex_coords(self) -> String {
        format!(
            "{},{},{},{}",
            self.left / self.sheet_w,
            self.right / self.sheet_w,
            self.top / self.sheet_h,
            self.bottom / self.sheet_h
        )
    }
}

/// UiTextureAtlas 2136, 1024x512.
const MICRO_SHEET: u32 = 4_708_813;
/// UiTextureAtlas 2098, 512x128.
const BAG_SHEET: u32 = 4_691_255;

const fn micro(left: f32, top: f32) -> SheetCrop {
    SheetCrop {
        fdid: MICRO_SHEET,
        sheet_w: 1024.0,
        sheet_h: 512.0,
        left,
        right: left + 64.0,
        top,
        bottom: top + 82.0,
    }
}

const fn bag(left: f32, right: f32, top: f32, bottom: f32) -> SheetCrop {
    SheetCrop {
        fdid: BAG_SHEET,
        sheet_w: 512.0,
        sheet_h: 128.0,
        left,
        right,
        top,
        bottom,
    }
}

/// `ui-hud-micromenu-buttonbg-up-2x` (member 23030).
pub(super) const MICRO_BUTTON_BG: SheetCrop = micro(67.0, 253.0);

/// `ui-hud-micromenu-<name>-up-2x` per engine micro button. The character button draws the
/// player portrait in Retail, so it keeps only the button background.
pub(super) fn micro_button_icon(button: &str) -> Option<SheetCrop> {
    Some(match button {
        "SpellbookMicroButton" => micro(595.0, 169.0), // spellbookabilities 17208
        "TalentMicroButton" => micro(529.0, 337.0),    // spectalents 17204
        "AchievementMicroButton" => micro(1.0, 253.0), // achievements 17166
        "QuestLogMicroButton" => micro(463.0, 169.0),  // questlog 17196
        "GuildMicroButton" => micro(265.0, 421.0),     // guildcommunities 17191
        "LFDMicroButton" => micro(199.0, 253.0),       // groupfinder 17187
        "CollectionsMicroButton" => micro(133.0, 85.0), // collections 17178
        "EJMicroButton" => micro(67.0, 85.0),          // adventureguide 17170
        "StoreMicroButton" => micro(529.0, 1.0),       // shop 17200
        "MainMenuMicroButton" => micro(133.0, 421.0),  // gamemenu 17183
        _ => return None,
    })
}

/// `bag-main-2x` (member 16752): the backpack button.
pub(super) const BACKPACK: SheetCrop = bag(1.0, 97.0, 1.0, 97.0);
/// `bag-border-empty-2x` (member 16751): an empty bag slot.
pub(super) const BAG_SLOT_EMPTY: SheetCrop = bag(295.0, 356.0, 64.0, 125.0);
