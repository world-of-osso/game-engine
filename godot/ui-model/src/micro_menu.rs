//! Retail micro menu (`Blizzard_MicroMenu/Mainline/MainMenuBarMicroButtons.xml`/`.lua`,
//! `MicroMenuContainer.xml`, `MicroMenuContainerOverrides.lua`): `MainMenuBarMicroButton`
//! 32×40 buttons laid out with `childXPadding=-5` at the active preset's anchor
//! (`crate::hud_layout`). Each button draws its
//! `LoadMicroButtonTextures` atlas for its state over `Background`/`PushedBackground`.
//! Buttons whose window exists natively toggle it; the rest take the disabled branch of
//! their Retail `UpdateMicroButton` and say why in their tooltip.

use game_engine_core::input_bindings_data::InputAction;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::game_tooltip::GameTooltip;
use crate::hud_layout::hud_layout;
use crate::inworld_unit_frames_component::PortraitSlot;
use crate::quest_art::DynName;
use crate::tooltip_presentation::{TOOLTIP_WHITE, TooltipLineState, TooltipPresentation};

pub const MICRO_MENU: &str = "MicroMenuContainer";
pub const ACTION_PREFIX: &str = "micro:";
pub const ACTION_CHARACTER: &str = "micro:CharacterMicroButton";
pub const ACTION_PLAYER_SPELLS: &str = "micro:PlayerSpellsMicroButton";
pub const ACTION_QUEST_LOG: &str = "micro:QuestLogMicroButton";
pub const ACTION_MAIN_MENU: &str = "micro:MainMenuMicroButton";

const BUTTON_W: f32 = 32.0;
const BUTTON_H: f32 = 40.0;
const GAP: f32 = -5.0;
/// `MicroMenuContainer` width: the buttons overlapping by `GAP`.
pub const MICRO_MENU_W: f32 = MICRO_BUTTONS.len() as f32 * (BUTTON_W + GAP) - GAP;
/// The 32×41 atlas art (`useAtlasSize`) is centred on the 32×40 button.
const ART_H: f32 = 41.0;
/// `OnDisable`: `SetAlpha(0.5)`.
const DISABLED_ALPHA: f32 = 0.5;

const BACKGROUND_UP: &str = "UI-HUD-MicroMenu-ButtonBG-Up";
const BACKGROUND_DOWN: &str = "UI-HUD-MicroMenu-ButtonBG-Down";
/// The CharacterMicroButton's `Shadow` and `PushedShadow`.
const PORTRAIT_SHADOW: &str = "UI-HUD-MicroMenu-Portrait-Shadow";
const PORTRAIT_DOWN: &str = "UI-HUD-MicroMenu-Portrait-Down";

/// The CharacterMicroButton's `Portrait` (`SetPortraitTexture(self.Portrait, "player")`,
/// Mainline/MainMenuBarMicroButtons.lua:553-563): TOPLEFT (7, -7) to BOTTOMRIGHT (-7, 7),
/// TexCoords (0.2, 0.8, 0.0666, 0.9), under the 35×65 `PortraitMask`
/// (`UI-HUD-MicroMenu-Portrait-Mask`, atlas 2519 FileDataID 5228950) at the button's
/// CENTER (MainMenuBarMicroButtons.xml:67-84; `SetNormal`, .lua:602-615).
pub const CHARACTER_PORTRAIT: PortraitSlot = PortraitSlot {
    frame: "CharacterMicroButtonPortrait",
    rect: (7.0, 7.0, BUTTON_W - 14.0, BUTTON_H - 14.0),
    mask_fdid: 5_228_950,
    mask_rect: (
        (BUTTON_W - PORTRAIT_MASK.0) / 2.0,
        (BUTTON_H - PORTRAIT_MASK.1) / 2.0,
        PORTRAIT_MASK.0,
        PORTRAIT_MASK.1,
    ),
    tex_coords: [0.2, 0.8, 0.0666, 0.9],
};
/// Pushed (`SetPushed`, .lua:589-600): the portrait's BOTTOMRIGHT at (-6, 5) and the mask
/// at CENTER (2, -2).
pub const CHARACTER_PORTRAIT_PUSHED: PortraitSlot = PortraitSlot {
    rect: (7.0, 7.0, BUTTON_W - 13.0, BUTTON_H - 12.0),
    mask_rect: (
        (BUTTON_W - PORTRAIT_MASK.0) / 2.0 + 2.0,
        (BUTTON_H - PORTRAIT_MASK.1) / 2.0 + 2.0,
        PORTRAIT_MASK.0,
        PORTRAIT_MASK.1,
    ),
    ..CHARACTER_PORTRAIT
};
const PORTRAIT_MASK: (f32, f32) = (35.0, 65.0);

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
    /// `LoadMicroButtonTextures` name of the `UI-HUD-MicroMenu-<name>-Up/-Down/-Disabled/
    /// -Mouseover` atlases (MainMenuBarMicroButtons.lua:32-38); `None` for the
    /// CharacterMicroButton, which draws the portrait instead.
    art: Option<&'static str>,
    /// `None` when the button's window is native.
    pub unavailable: Option<Unavailable>,
}

impl MicroButton {
    /// Share the existing normal micro-menu icon with the launcher; Character uses its portrait.
    pub fn icon_atlas(&self) -> Option<String> {
        self.art.map(|name| format!("UI-HUD-MicroMenu-{name}-Up"))
    }
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
        art: Some("Professions"),
        unavailable: Some(SYSTEM_DISABLED),
    },
    // `PlayerSpellsUtil.TogglePlayerSpellsFrame`: the native spellbook.
    MicroButton {
        name: "PlayerSpellsMicroButton",
        title: "Talents & Spellbook",
        binding: Some(InputAction::ToggleTalents),
        art: Some("SpecTalents"),
        unavailable: None,
    },
    // No achievement earned and not `CanShowAchievementUI`: disabled with `minLevel`
    // `MIN_RES_SICKNESS_LEVEL`.
    MicroButton {
        name: "AchievementMicroButton",
        title: "Achievements",
        binding: Some(InputAction::ToggleAchievements),
        art: Some("Achievements"),
        unavailable: Some(Unavailable::MinLevel(10)),
    },
    // `ToggleQuestLog`: the native QuestLogFrame.
    MicroButton {
        name: "QuestLogMicroButton",
        title: "Quest Log",
        binding: Some(InputAction::ToggleQuestLog),
        art: Some("Questlog"),
        unavailable: None,
    },
    // Not `C_Housing.IsHousingServiceEnabled()`.
    MicroButton {
        name: "HousingMicroButton",
        title: "Housing Dashboard",
        binding: None,
        art: Some("Housing"),
        unavailable: Some(Unavailable::Reason(
            "This action is not available right now",
        )),
    },
    // `C_Club.IsEnabled() and not BNConnected()`.
    MicroButton {
        name: "GuildMicroButton",
        title: "Guild & Communities",
        binding: None,
        art: Some("GuildCommunities"),
        unavailable: Some(Unavailable::Reason(
            "Unavailable\n\nBlizzard services are currently unavailable.",
        )),
    },
    // Not `C_LFGInfo.CanPlayerUseGroupFinder()`: no group finder service.
    MicroButton {
        name: "LFDMicroButton",
        title: "Group Finder",
        binding: None,
        art: Some("Groupfinder"),
        unavailable: Some(SYSTEM_DISABLED),
    },
    // No CollectionsJournal natively; the Lua's only disabled branch is the kiosk one.
    MicroButton {
        name: "CollectionsMicroButton",
        title: "Warband Collections",
        binding: None,
        art: Some("Collections"),
        unavailable: Some(SYSTEM_DISABLED),
    },
    // Not `AdventureGuideUtil.IsAvailable()`: `FEATURE_NOT_YET_AVAILABLE`.
    MicroButton {
        name: "EJMicroButton",
        title: "Adventure Guide",
        binding: Some(InputAction::ToggleEncounterJournal),
        art: Some("AdventureGuide"),
        unavailable: Some(Unavailable::Reason("This feature is not yet available.")),
    },
    // Not `C_StorePublic.IsEnabled()` outside CN: `BLIZZARD_STORE_ERROR_UNAVAILABLE`.
    MicroButton {
        name: "StoreMicroButton",
        title: "Shop",
        binding: None,
        art: Some("Shop"),
        unavailable: Some(Unavailable::Reason("The shop is currently unavailable.")),
    },
    MicroButton {
        name: "MainMenuMicroButton",
        title: "Game Menu",
        binding: None,
        art: Some("GameMenu"),
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

    /// The CharacterMicroButton portrait's slot in its current state.
    pub fn character_portrait(&self) -> PortraitSlot {
        if self.shows_pushed(CHARACTER) {
            CHARACTER_PORTRAIT_PUSHED
        } else {
            CHARACTER_PORTRAIT
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
        MicroButtonState::Disabled => return None,
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

/// The message a click on a button without a native window shows in the error frame:
/// its Retail unavailable line. Buttons stay lit (user decision 2026-10-02); the window
/// they open is not converted yet.
pub fn unavailable_message(action: &str) -> Option<String> {
    let name = action.strip_prefix(ACTION_PREFIX)?;
    let button = MICRO_BUTTONS.iter().find(|button| button.name == name)?;
    Some(match button.unavailable? {
        Unavailable::Reason(reason) => reason.to_owned(),
        Unavailable::MinLevel(level) => {
            format!("This feature becomes available at level {level}.")
        }
    })
}

pub fn micro_menu_screen(ctx: &SharedContext) -> Element {
    let view = ctx.get::<MicroMenuView>().cloned().unwrap_or_default();
    let layout = hud_layout(ctx);
    let at = layout.micro_menu.place((MICRO_MENU_W, BUTTON_H));
    let buttons: Element = (0..MICRO_BUTTONS.len())
        .flat_map(|index| micro_button(&view, index))
        .collect();
    rsx! {
        r#frame {
            name: {DynName(MICRO_MENU.into())},
            width: MICRO_MENU_W,
            height: BUTTON_H,
            pos_type: "absolute",
            left: {at.left.as_str()},
            right: {at.right.as_str()},
            top: {at.top.as_str()},
            bottom: {at.bottom.as_str()},
            margin_left: {at.margin_left},
            margin_top: {at.margin_top},
            {buttons}
        }
    }
}

fn micro_button(view: &MicroMenuView, index: usize) -> Element {
    let button = &MICRO_BUTTONS[index];
    // `Portrait` (ARTWORK) over `Background` and `Shadow`, under `PushedShadow`.
    let mut portrait = button.art.is_none().then(|| view.character_portrait());
    let mut layers = Element::new();
    for (layer, (atlas, offset)) in button_layers(view, index).into_iter().enumerate() {
        if layer == 2
            && let Some(slot) = portrait.take()
        {
            layers.extend(portrait_frame(&slot));
        }
        layers.extend(art_layer(button.name, layer, &atlas, offset));
    }
    if let Some(slot) = portrait {
        layers.extend(portrait_frame(&slot));
    }
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
fn button_layers(view: &MicroMenuView, index: usize) -> Vec<(String, [f32; 2])> {
    let pushed = view.shows_pushed(index);
    let background = if pushed {
        BACKGROUND_DOWN
    } else {
        BACKGROUND_UP
    };
    let mut layers = vec![(background.to_owned(), [0.0, 0.0])];
    match MICRO_BUTTONS[index].art {
        // `Shadow` (BORDER, CENTER) and, pushed, `PushedShadow` (CENTER 1,-4).
        None => {
            layers.push((PORTRAIT_SHADOW.to_owned(), [0.0, 0.0]));
            if pushed {
                layers.push((PORTRAIT_DOWN.to_owned(), [1.0, 4.0]));
            }
        }
        // `OnEnter` hides the normal texture under the `-Mouseover` highlight.
        Some(name) => {
            let state = match view.state(index) {
                MicroButtonState::Disabled => "Disabled",
                _ if pushed => "Down",
                _ if view.hovered == Some(index) => "Mouseover",
                _ => "Up",
            };
            layers.push((format!("UI-HUD-MicroMenu-{name}-{state}"), [0.0, 0.0]));
        }
    }
    layers
}

/// The empty slot the client fills with the player's rendered portrait.
fn portrait_frame(slot: &PortraitSlot) -> Element {
    let (x, y, width, height) = slot.rect;
    rsx! {
        r#frame {
            name: {DynName(slot.frame.into())},
            width,
            height,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

fn art_layer(button: &str, layer: usize, atlas: &str, [dx, dy]: [f32; 2]) -> Element {
    rsx! {
        texture {
            name: {DynName(format!("{button}Art{layer}"))},
            width: BUTTON_W,
            height: ART_H,
            texture_atlas: atlas,
            pos_type: "absolute",
            left: dx,
            top: {(BUTTON_H - ART_H) / 2.0 + dy},
        }
    }
}
