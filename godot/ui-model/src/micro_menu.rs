//! Retail micro menu (`Blizzard_MicroMenu/Mainline/MainMenuBarMicroButtons.xml`,
//! `MicroMenuContainer.xml`): `MainMenuBarMicroButton` 32×40 buttons laid out with
//! `childXPadding=-5` at the bottom right of `MicroButtonAndBagsBar` (232×80,
//! BOTTOMRIGHT -6,6, `EditModePresetLayouts.lua`), the order and atlas crops of the
//! root HUD's `inworld_hud_micro.rs`. Every button clicks `micro:<button name>`; the
//! host acts on those whose window exists natively and reports the rest as unconverted.

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::quest_art::DynName;

pub const MICRO_MENU: &str = "MicroMenuContainer";
pub const ACTION_PREFIX: &str = "micro:";
pub const ACTION_CHARACTER: &str = "micro:CharacterMicroButton";
pub const ACTION_SPELLBOOK: &str = "micro:SpellbookMicroButton";
pub const ACTION_MAIN_MENU: &str = "micro:MainMenuMicroButton";

const BUTTON_W: f32 = 32.0;
const BUTTON_H: f32 = 40.0;
const GAP: f32 = -5.0;
const INSET: f32 = 6.0;
/// The 32×41 atlas art is centred on the 32×40 button.
const ART_H: f32 = 41.0;
/// UiTextureAtlas 2136 `4708813`, 1024×512; members are 64×82 (2x).
const SHEET: u32 = 4_708_813;

pub const MICRO_BUTTONS: [&str; 11] = [
    "CharacterMicroButton",
    "SpellbookMicroButton",
    "TalentMicroButton",
    "AchievementMicroButton",
    "QuestLogMicroButton",
    "GuildMicroButton",
    "LFDMicroButton",
    "CollectionsMicroButton",
    "EJMicroButton",
    "StoreMicroButton",
    "MainMenuMicroButton",
];

/// `ui-hud-micromenu-buttonbg-up-2x` (23030).
const BUTTON_BG: (f32, f32) = (67.0, 253.0);

/// `ui-hud-micromenu-<name>-up-2x`. The character button draws the player portrait in
/// Retail; here it keeps only the button background.
fn icon(button: &str) -> Option<(f32, f32)> {
    Some(match button {
        "SpellbookMicroButton" => (595.0, 169.0), // spellbookabilities 17208
        "TalentMicroButton" => (529.0, 337.0),    // spectalents 17204
        "AchievementMicroButton" => (1.0, 253.0), // achievements 17166
        "QuestLogMicroButton" => (463.0, 169.0),  // questlog 17196
        "GuildMicroButton" => (265.0, 421.0),     // guildcommunities 17191
        "LFDMicroButton" => (199.0, 253.0),       // groupfinder 17187
        "CollectionsMicroButton" => (133.0, 85.0), // collections 17178
        "EJMicroButton" => (67.0, 85.0),          // adventureguide 17170
        "StoreMicroButton" => (529.0, 1.0),       // shop 17200
        "MainMenuMicroButton" => (133.0, 421.0),  // gamemenu 17183
        _ => return None,
    })
}


/// `()` state: the menu has no data of its own.
pub fn micro_menu_screen(_ctx: &SharedContext) -> Element {
    let width = MICRO_BUTTONS.len() as f32 * (BUTTON_W + GAP) - GAP;
    let buttons: Element = MICRO_BUTTONS
        .iter()
        .enumerate()
        .flat_map(|(index, name)| micro_button(index, name))
        .collect();
    rsx! {
        r#frame {
            name: {DynName(MICRO_MENU.into())},
            width,
            height: BUTTON_H,
            pos_type: "absolute",
            right: INSET,
            bottom: INSET,
            {buttons}
        }
    }
}

fn micro_button(index: usize, name: &str) -> Element {
    let layers: Element = [Some(BUTTON_BG), icon(name)]
        .into_iter()
        .flatten()
        .enumerate()
        .flat_map(|(layer, crop)| art_layer(name, layer, crop))
        .collect();
    let action = format!("{ACTION_PREFIX}{name}");
    rsx! {
        button {
            name: {DynName(name.into())},
            width: BUTTON_W,
            height: BUTTON_H,
            text: "",
            onclick: {action.as_str()},
            pos_type: "absolute",
            left: {index as f32 * (BUTTON_W + GAP)},
            top: 0.0,
            {layers}
        }
    }
}

fn art_layer(button: &str, layer: usize, (left, top): (f32, f32)) -> Element {
    let coords = format!(
        "{},{},{},{}",
        left / 1024.0,
        (left + 64.0) / 1024.0,
        top / 512.0,
        (top + 82.0) / 512.0
    );
    rsx! {
        texture {
            name: {DynName(format!("{button}Art{layer}"))},
            width: BUTTON_W,
            height: ART_H,
            texture_fdid: SHEET,
            tex_coords: {coords.as_str()},
            pos_type: "absolute",
            left: 0.0,
            top: {(BUTTON_H - ART_H) / 2.0},
        }
    }
}
