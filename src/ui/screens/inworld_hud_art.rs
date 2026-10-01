//! Retail micro menu and bag bar art: `UiTextureAtlasMember.csv` crops on their atlas sheets
//! (`UiTextureAtlas.csv`). Names follow `Blizzard_MicroMenu/Mainline/MainMenuBarMicroButtons.lua`
//! (`UI-HUD-MicroMenu-<Name>-Up`) and `Blizzard_MainMenuBarBagButtons` (`bag-main`,
//! `bag-border-empty`).

pub(super) use super::bags_bar_art::{BACKPACK, BAG_SLOT_EMPTY, SheetCrop};

/// `interface/hud/uiminimap.blp`, 512x512.
const MINIMAP_SHEET: u32 = 4_618_651;

/// `ui-hud-minimap-mail-up` (Minimap.xml:99), 20x15: the new-mail indicator.
pub(super) const MINIMAP_MAIL: SheetCrop = SheetCrop {
    fdid: MINIMAP_SHEET,
    sheet_w: 512.0,
    sheet_h: 512.0,
    left: 463.0,
    right: 483.0,
    top: 140.0,
    bottom: 155.0,
};

/// UiTextureAtlas 2239: `interface/hud/uiguildbanner.blp`, 128x128.
const GUILD_BANNER_SHEET: u32 = 4_764_688;

const fn guild_banner(left: f32, right: f32, top: f32, bottom: f32) -> SheetCrop {
    SheetCrop {
        fdid: GUILD_BANNER_SHEET,
        sheet_w: 128.0,
        sheet_h: 128.0,
        left,
        right,
        top,
        bottom,
    }
}

/// `ui-hud-minimap-guildbanner-background-top` (member 18409), 36x37.
pub(super) const INSTANCE_BANNER_BACKGROUND: SheetCrop = guild_banner(1.0, 37.0, 40.0, 77.0);
/// `ui-hud-minimap-guildbanner-border-top` (member 18411), 36x37.
pub(super) const INSTANCE_BANNER_BORDER: SheetCrop = guild_banner(39.0, 75.0, 1.0, 38.0);
/// `ui-hud-minimap-guildbanner-normal-large` (member 25851), 16x16.
pub(super) const INSTANCE_BANNER_NORMAL: SheetCrop = guild_banner(95.0, 111.0, 19.0, 35.0);
/// `ui-hud-minimap-guildbanner-heroic-large` (member 18455), 16x16.
pub(super) const INSTANCE_BANNER_HEROIC: SheetCrop = guild_banner(77.0, 93.0, 20.0, 36.0);
/// `ui-hud-minimap-guildbanner-mythic-large` (member 18457), 16x16.
pub(super) const INSTANCE_BANNER_MYTHIC: SheetCrop = guild_banner(95.0, 111.0, 1.0, 17.0);

/// UiTextureAtlas 2136, 1024x512.
const MICRO_SHEET: u32 = 4_708_813;

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
