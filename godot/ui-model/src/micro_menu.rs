//! Retail micro menu (`Blizzard_MicroMenu/Mainline/MainMenuBarMicroButtons.xml`/`.lua`,
//! `MicroMenuContainer.xml`, `MicroMenuContainerOverrides.lua`): `MainMenuBarMicroButton`
//! 32×40 buttons laid out with `childXPadding=-5` at the bottom right of
//! `MicroButtonAndBagsBar` (232×80, BOTTOMRIGHT -6,6). Each button draws its
//! `LoadMicroButtonTextures` atlas for its state over `Background`/`PushedBackground`.
//! Buttons whose window exists natively toggle it; the rest take the disabled branch of
//! their Retail `UpdateMicroButton` and say why in their tooltip.

use game_engine_core::input_bindings_data::InputAction;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::game_tooltip::GameTooltip;
use crate::item_tooltip::RED_FONT_COLOR;
use crate::quest_art::DynName;
use crate::tooltip_presentation::{
    TOOLTIP_WHITE, TooltipLineState, TooltipPresentation, description_lines,
};

pub const MICRO_MENU: &str = "MicroMenuContainer";
pub const ACTION_PREFIX: &str = "micro:";
pub const ACTION_CHARACTER: &str = "micro:CharacterMicroButton";
pub const ACTION_PLAYER_SPELLS: &str = "micro:PlayerSpellsMicroButton";
pub const ACTION_QUEST_LOG: &str = "micro:QuestLogMicroButton";
pub const ACTION_MAIN_MENU: &str = "micro:MainMenuMicroButton";

const BUTTON_W: f32 = 32.0;
const BUTTON_H: f32 = 40.0;
const GAP: f32 = -5.0;
const INSET: f32 = 6.0;
/// The 32×41 atlas art (`useAtlasSize`) is centred on the 32×40 button.
const ART_H: f32 = 41.0;
/// UiTextureAtlas 2136 `4708813`, 1024×512; members are 64×82 (2x).
const SHEET: u32 = 4_708_813;
/// `OnDisable`: `SetAlpha(0.5)`.
const DISABLED_ALPHA: f32 = 0.5;

/// Committed (left, top) of a 64×82 member of the sheet.
type Crop = (f32, f32);

/// `UI-HUD-MicroMenu-<name>-Up/-Down/-Disabled/-Mouseover`.
#[derive(Clone, Copy)]
struct ButtonArt {
    up: Crop,
    down: Crop,
    disabled: Crop,
    mouseover: Crop,
}

/// `UI-HUD-MicroMenu-ButtonBG-Up` / `-Down`.
const BACKGROUND_UP: Crop = (67.0, 253.0);
const BACKGROUND_DOWN: Crop = (67.0, 169.0);
/// `UI-HUD-MicroMenu-Portrait-Shadow` / `-Down` of the CharacterMicroButton.
const PORTRAIT_SHADOW: Crop = (397.0, 1.0);
const PORTRAIT_DOWN: Crop = (331.0, 421.0);

const fn art(up: Crop, down: Crop, disabled: Crop, mouseover: Crop) -> Option<ButtonArt> {
    Some(ButtonArt {
        up,
        down,
        disabled,
        mouseover,
    })
}

/// Why a button without a native window is disabled: its Retail tooltip line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unavailable {
    /// `disabledTooltip`.
    Reason(&'static str),
    /// `minLevel`: `FEATURE_BECOMES_AVAILABLE_AT_LEVEL`.
    MinLevel(u32),
}

/// `ERR_SYSTEM_DISABLED`, the `SetKioskTooltip` line of the buttons whose Retail
/// `UpdateMicroButton` has no other disabled branch.
const SYSTEM_DISABLED: Unavailable = Unavailable::Reason("This system is currently disabled.");

pub struct MicroButton {
    pub name: &'static str,
    /// `tooltipText` before `MicroButtonTooltipText` adds the binding.
    pub title: &'static str,
    /// The binding `MicroButtonTooltipText` shows, where the client has that action.
    pub binding: Option<InputAction>,
    /// `None` for the CharacterMicroButton, which draws the portrait instead.
    art: Option<ButtonArt>,
    /// `None` when the button's window is native.
    pub unavailable: Option<Unavailable>,
}

/// `MicroMenuMixin:GenerateButtonInfos` order; `HelpMicroButton` stays hidden outside CN.
pub const MICRO_BUTTONS: [MicroButton; 12] = [
    MicroButton {
        name: "CharacterMicroButton",
        title: "Character Info",
        binding: Some(InputAction::ToggleCharacter),
        art: None,
        unavailable: None,
    },
    // No ProfessionsBookFrame natively; the Lua's only disabled branch is the kiosk one.
    MicroButton {
        name: "ProfessionMicroButton",
        title: "Professions",
        binding: Some(InputAction::ToggleProfessions),
        art: art(
            (397.0, 337.0),
            (397.0, 169.0),
            (397.0, 85.0),
            (397.0, 253.0),
        ),
        unavailable: Some(SYSTEM_DISABLED),
    },
    // `PlayerSpellsUtil.TogglePlayerSpellsFrame`: the native spellbook.
    MicroButton {
        name: "PlayerSpellsMicroButton",
        title: "Talents & Spellbook",
        binding: Some(InputAction::ToggleTalents),
        art: art(
            (529.0, 337.0),
            (529.0, 169.0),
            (529.0, 85.0),
            (529.0, 253.0),
        ),
        unavailable: None,
    },
    // No achievement earned and not `CanShowAchievementUI`: disabled with `minLevel`
    // `MIN_RES_SICKNESS_LEVEL`.
    MicroButton {
        name: "AchievementMicroButton",
        title: "Achievements",
        binding: Some(InputAction::ToggleAchievements),
        art: art((1.0, 253.0), (1.0, 85.0), (1.0, 1.0), (1.0, 169.0)),
        unavailable: Some(Unavailable::MinLevel(10)),
    },
    // `ToggleQuestLog`: the native QuestLogFrame.
    MicroButton {
        name: "QuestLogMicroButton",
        title: "Quest Log",
        binding: Some(InputAction::ToggleQuestLog),
        art: art((463.0, 169.0), (463.0, 1.0), (397.0, 421.0), (463.0, 85.0)),
        unavailable: None,
    },
    // Not `C_Housing.IsHousingServiceEnabled()`.
    MicroButton {
        name: "HousingMicroButton",
        title: "Housing Dashboard",
        binding: None,
        art: art(
            (331.0, 337.0),
            (331.0, 169.0),
            (331.0, 85.0),
            (331.0, 253.0),
        ),
        unavailable: Some(Unavailable::Reason(
            "This action is not available right now",
        )),
    },
    // `C_Club.IsEnabled() and not BNConnected()`.
    MicroButton {
        name: "GuildMicroButton",
        title: "Guild & Communities",
        binding: None,
        art: art(
            (265.0, 421.0),
            (199.0, 421.0),
            (199.0, 337.0),
            (265.0, 337.0),
        ),
        unavailable: Some(Unavailable::Reason(
            "Unavailable\n\nBlizzard services are currently unavailable.",
        )),
    },
    // Not `C_LFGInfo.CanPlayerUseGroupFinder()`: no group finder service.
    MicroButton {
        name: "LFDMicroButton",
        title: "Group Finder",
        binding: None,
        art: art((199.0, 253.0), (199.0, 85.0), (199.0, 1.0), (199.0, 169.0)),
        unavailable: Some(SYSTEM_DISABLED),
    },
    // No CollectionsJournal natively; the Lua's only disabled branch is the kiosk one.
    MicroButton {
        name: "CollectionsMicroButton",
        title: "Warband Collections",
        binding: None,
        art: art((133.0, 85.0), (67.0, 421.0), (67.0, 337.0), (133.0, 1.0)),
        unavailable: Some(SYSTEM_DISABLED),
    },
    // Not `AdventureGuideUtil.IsAvailable()`: `FEATURE_NOT_YET_AVAILABLE`.
    MicroButton {
        name: "EJMicroButton",
        title: "Adventure Guide",
        binding: Some(InputAction::ToggleEncounterJournal),
        art: art((67.0, 85.0), (1.0, 421.0), (1.0, 337.0), (67.0, 1.0)),
        unavailable: Some(Unavailable::Reason("This feature is not yet available.")),
    },
    // Not `C_StorePublic.IsEnabled()` outside CN: `BLIZZARD_STORE_ERROR_UNAVAILABLE`.
    MicroButton {
        name: "StoreMicroButton",
        title: "Shop",
        binding: None,
        art: art((529.0, 1.0), (463.0, 421.0), (463.0, 253.0), (463.0, 337.0)),
        unavailable: Some(Unavailable::Reason("The shop is currently unavailable.")),
    },
    MicroButton {
        name: "MainMenuMicroButton",
        title: "Game Menu",
        binding: None,
        art: art(
            (133.0, 421.0),
            (133.0, 253.0),
            (133.0, 169.0),
            (133.0, 337.0),
        ),
        unavailable: None,
    },
];

const CHARACTER: usize = 0;
const PLAYER_SPELLS: usize = 2;
const QUEST_LOG: usize = 4;
const MAIN_MENU: usize = 11;

/// The native windows the micro buttons show as pushed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OpenWindows {
    pub character: bool,
    pub player_spells: bool,
    pub quest_log: bool,
    pub game_menu: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MicroButtonState {
    Normal,
    Pushed,
    Disabled,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MicroMenuView {
    pub open: OpenWindows,
    /// `MICRO_BUTTONS` index under the pointer.
    pub hovered: Option<usize>,
    /// `MICRO_BUTTONS` index held down (`OnMouseDown` → `SetPushed`).
    pub pressed: Option<usize>,
}

impl MicroMenuView {
    /// `UpdateMicroButtons`: an open GameMenuFrame pushes the MainMenuMicroButton and
    /// `DisableMicroButtons(false)` disables the rest; otherwise each button is disabled
    /// without a native window and pushed while its window shows.
    pub fn state(&self, index: usize) -> MicroButtonState {
        if self.open.game_menu {
            return if index == MAIN_MENU {
                MicroButtonState::Pushed
            } else {
                MicroButtonState::Disabled
            };
        }
        if MICRO_BUTTONS[index].unavailable.is_some() {
            return MicroButtonState::Disabled;
        }
        let open = match index {
            CHARACTER => self.open.character,
            PLAYER_SPELLS => self.open.player_spells,
            QUEST_LOG => self.open.quest_log,
            _ => false,
        };
        if open {
            MicroButtonState::Pushed
        } else {
            MicroButtonState::Normal
        }
    }

    fn shows_pushed(&self, index: usize) -> bool {
        match self.state(index) {
            MicroButtonState::Pushed => true,
            MicroButtonState::Normal => self.pressed == Some(index),
            MicroButtonState::Disabled => false,
        }
    }
}

/// `MICRO_BUTTONS` index of a button frame name.
pub fn micro_button_index(name: &str) -> Option<usize> {
    MICRO_BUTTONS.iter().position(|button| button.name == name)
}

/// `MainMenuBarMicroButtonMixin:ShouldShowTooltip` + `EvaluateTooltipVisibility`: the
/// white `tooltipText` with its binding and, on a disabled button, its red reason.
/// While the game menu disables the bar (`MICRO_BUTTONS_DISABLED`) disabled buttons show
/// none.
pub fn micro_button_tooltip(
    view: &MicroMenuView,
    index: usize,
    key: Option<&str>,
) -> Option<GameTooltip> {
    let button = MICRO_BUTTONS.get(index)?;
    let title = match key {
        Some(key) => format!("{} ({key})", button.title),
        None => button.title.to_owned(),
    };
    let lines = match view.state(index) {
        MicroButtonState::Disabled if view.open.game_menu => return None,
        MicroButtonState::Disabled => match button.unavailable? {
            Unavailable::Reason(reason) => description_lines(reason, RED_FONT_COLOR),
            Unavailable::MinLevel(level) => description_lines(
                &format!("This feature becomes available at level {level}."),
                RED_FONT_COLOR,
            ),
        },
        _ => Vec::<TooltipLineState>::new(),
    };
    Some(GameTooltip::new(
        TooltipPresentation {
            title,
            title_color: TOOLTIP_WHITE,
            lines,
            ..TooltipPresentation::hidden()
        },
        None,
    ))
}

pub fn micro_menu_screen(ctx: &SharedContext) -> Element {
    let view = ctx.get::<MicroMenuView>().cloned().unwrap_or_default();
    let width = MICRO_BUTTONS.len() as f32 * (BUTTON_W + GAP) - GAP;
    let buttons: Element = (0..MICRO_BUTTONS.len())
        .flat_map(|index| micro_button(&view, index))
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

fn micro_button(view: &MicroMenuView, index: usize) -> Element {
    let button = &MICRO_BUTTONS[index];
    let layers: Element = button_layers(view, index)
        .into_iter()
        .enumerate()
        .flat_map(|(layer, (crop, offset))| art_layer(button.name, layer, crop, offset))
        .collect();
    let state = view.state(index);
    let disabled = state == MicroButtonState::Disabled;
    let alpha = if disabled { DISABLED_ALPHA } else { 1.0 };
    let action = format!("{ACTION_PREFIX}{}", button.name);
    rsx! {
        button {
            name: {DynName(button.name.into())},
            width: BUTTON_W,
            height: BUTTON_H,
            text: "",
            button_default_skin: false,
            disabled,
            alpha,
            onclick: {action.as_str()},
            pos_type: "absolute",
            left: {index as f32 * (BUTTON_W + GAP)},
            top: 0.0,
            {layers}
        }
    }
}

/// `Background` or `PushedBackground`, then the portrait shadows or the state atlas.
fn button_layers(view: &MicroMenuView, index: usize) -> Vec<(Crop, [f32; 2])> {
    let pushed = view.shows_pushed(index);
    let background = if pushed {
        BACKGROUND_DOWN
    } else {
        BACKGROUND_UP
    };
    let mut layers = vec![(background, [0.0, 0.0])];
    match MICRO_BUTTONS[index].art {
        // `Shadow` (BORDER, CENTER) and, pushed, `PushedShadow` (CENTER 1,-4).
        None => {
            layers.push((PORTRAIT_SHADOW, [0.0, 0.0]));
            if pushed {
                layers.push((PORTRAIT_DOWN, [1.0, 4.0]));
            }
        }
        // `OnEnter` hides the normal texture under the `-Mouseover` highlight.
        Some(art) => {
            let crop = match view.state(index) {
                MicroButtonState::Disabled => art.disabled,
                _ if pushed => art.down,
                _ if view.hovered == Some(index) => art.mouseover,
                _ => art.up,
            };
            layers.push((crop, [0.0, 0.0]));
        }
    }
    layers
}

fn art_layer(button: &str, layer: usize, (left, top): Crop, [dx, dy]: [f32; 2]) -> Element {
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
            left: dx,
            top: {(BUTTON_H - ART_H) / 2.0 + dy},
        }
    }
}
