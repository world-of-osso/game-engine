//! Retail `PetActionBar` (`Blizzard_ActionBar/Mainline/PetActionBar.xml`,
//! `Shared/PetActionBar.lua`): ten `PetActionButtonTemplate` check buttons
//! (`SmallActionButtonTemplate`, 30×30, `Mainline/ActionButtonTemplate.xml:198-207`) laid out
//! by `ActionBarMixin:UpdateGridLayout` (`Shared/ActionBar.lua`) 2 px apart (Edit Mode
//! `IconPadding` 2, `IconSize` 100%). The XML `Size` 509×43 is replaced by that layout
//! (`ResizeLayoutFrame`): 318×30. In its default position (`EditModePresetLayouts.lua`
//! PetActionBar `BOTTOM` at `MAIN_ACTION_BAR_DEFAULT_OFFSET_Y`, automatic positioning on)
//! `EditModeManagerFrameMixin:UpdateBottomActionBarPositions` stacks it on the shown bottom
//! bars: BOTTOMLEFT at UIParent BOTTOM + (-MainActionBar width / 2, 45 + 45 +
//! `BOTTOM_ACTION_BARS_SPACER_Y` 5); this client shows no MultiBarBottomLeft/Right or
//! StanceBar between them. The autocast overlay is not drawn.

use shared::protocol::{
    ACT_COMMAND, ACT_REACTION, COMMAND_ATTACK, COMMAND_FOLLOW, COMMAND_MOVE_TO, COMMAND_STAY,
    PET_ACTION_BAR_SLOTS, PetSpells, REACT_AGGRESSIVE, REACT_ASSIST, REACT_DEFENSIVE,
    REACT_PASSIVE, pet_action_button_action, pet_action_button_type,
};
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::main_action_bar_component::{BAR_BOTTOM, BAR_W as MAIN_BAR_W, BUTTON_SIZE};
use crate::ui::anchor::FrameName;
use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::GameFont;

pub const PET_ACTION_BAR: FrameName = FrameName("PetActionBar");
pub const PET_BAR_BUTTONS: usize = PET_ACTION_BAR_SLOTS;
/// Clicking button `n` (0-based) emits `"{PET_ACTION_BUTTON_PREFIX}{n}"`.
pub const PET_ACTION_BUTTON_PREFIX: &str = "pet_action_button:";

pub const PET_BUTTON_SIZE: f32 = 30.0;
pub const PET_BUTTON_PADDING: f32 = 2.0;
pub const PET_BAR_W: f32 =
    PET_BAR_BUTTONS as f32 * PET_BUTTON_SIZE + (PET_BAR_BUTTONS as f32 - 1.0) * PET_BUTTON_PADDING;
/// Left edge from the screen's centre line: MainActionBar's left edge.
pub const PET_BAR_LEFT: f32 = -MAIN_BAR_W / 2.0;
/// `BOTTOM_ACTION_BARS_SPACER_Y` (Standard/EditModePresetLayoutConstants.lua:12).
const BOTTOM_ACTION_BARS_SPACER_Y: f32 = 5.0;
pub const PET_BAR_BOTTOM: f32 = BAR_BOTTOM + BUTTON_SIZE + BOTTOM_ACTION_BARS_SPACER_Y;
/// `SmallActionButtonMixin:UpdateButtonArt`: `NormalTexture`/`PushedTexture` 35×35 at TOPLEFT.
const FRAME_ART: f32 = 35.0;
/// `SmallActionButtonMixin_OnLoad`: `CheckedTexture`, `HighlightTexture` and `Flash` 31.6×30.9.
const OVERLAY_W: f32 = 31.6;
const OVERLAY_H: f32 = 30.9;
/// `HotKey` 32×10 at TOPRIGHT (`hotkeyX` -3, `hotkeyY` -4), `NumberFontNormalSmallGray`.
const HOTKEY_W: f32 = 32.0;
const HOTKEY_H: f32 = 10.0;
const HOTKEY_RIGHT: f32 = 3.0;
const HOTKEY_TOP: f32 = 4.0;
const HOTKEY_COLOR: &str = "0.6,0.6,0.6,1.0";
/// `ATTACK_BUTTON_FLASH_TIME` (Shared/ActionButton.lua:1).
pub const ATTACK_BUTTON_FLASH_TIME: f32 = 0.4;
/// "the checked texture looks a little confusing at full alpha" (PetActionBar.lua:168-169).
const ATTACK_CHECKED_ALPHA: f32 = 0.5;

/// UiTextureAtlas 1979 `interface/hud/uiactionbar.blp` (FDID 4613342, 256×1024).
const fn action_bar(rect: (f32, f32, f32, f32)) -> AtlasArt {
    AtlasArt {
        fdid: 4_613_342,
        atlas: (256.0, 1024.0),
        rect,
    }
}

/// `UI-HUD-ActionBar-IconFrame-Background`.
const SLOT_BACKGROUND: AtlasArt = action_bar((181.0, 227.0, 411.0, 456.0));
/// `ui-hud-actionbar-iconframe-slot`.
const SLOT_ART: AtlasArt = action_bar((181.0, 245.0, 136.0, 198.0));
/// `UI-HUD-ActionBar-IconFrame`.
const NORMAL: AtlasArt = action_bar((181.0, 227.0, 254.0, 299.0));
/// `UI-HUD-ActionBar-IconFrame-Down`.
const PUSHED: AtlasArt = action_bar((181.0, 227.0, 521.0, 566.0));
/// `UI-HUD-ActionBar-IconFrame-Mouseover`: both `HighlightTexture` and `CheckedTexture`.
const MOUSEOVER: AtlasArt = action_bar((181.0, 227.0, 643.0, 688.0));
/// `UI-HUD-ActionBar-IconFrame-Flash` (UiTextureAtlasMember 15803).
const FLASH: AtlasArt = action_bar((181.0, 227.0, 568.0, 613.0));

/// `PET_*_TEXTURE` (PetActionBar.lua:3-12) as FDIDs of `data/community-listfile.csv`.
pub const PET_ATTACK_TEXTURE: u32 = 132_152; // Interface\Icons\Ability_GhoulFrenzy
pub const PET_FOLLOW_TEXTURE: u32 = 132_328; // Interface\Icons\Ability_Tracking
pub const PET_WAIT_TEXTURE: u32 = 136_106; // Interface\Icons\Spell_Nature_TimeStop
pub const PET_MOVE_TO_TEXTURE: u32 = 457_329; // Interface\Icons\Ability_Hunter_Pet_Goto
pub const PET_ASSIST_TEXTURE: u32 = 524_348; // Interface\Icons\Ability_Hunter_Pet_Assist
pub const PET_DEFENSIVE_TEXTURE: u32 = 132_110; // Interface\Icons\Ability_Defend
pub const PET_PASSIVE_TEXTURE: u32 = 132_311; // Interface\Icons\Ability_Seal
pub const PET_AGGRESSIVE_TEXTURE: u32 = 132_277; // Interface\Icons\Ability_Racial_BloodRage

/// Every texture the bar draws besides spell icons, for hosts that copy art from local CASC.
pub const PET_BAR_ART_FDIDS: [u32; 9] = [
    4_613_342,
    PET_ATTACK_TEXTURE,
    PET_FOLLOW_TEXTURE,
    PET_WAIT_TEXTURE,
    PET_MOVE_TO_TEXTURE,
    PET_ASSIST_TEXTURE,
    PET_DEFENSIVE_TEXTURE,
    PET_PASSIVE_TEXTURE,
    PET_AGGRESSIVE_TEXTURE,
];

/// What one packed unit action button (`MAKE_UNIT_ACTION_BUTTON`) holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PetActionSlot {
    Empty,
    /// `ACT_COMMAND`: a `COMMAND_*`.
    Command(u32),
    /// `ACT_REACTION`: a `REACT_*`.
    Reaction(u32),
    /// `ACT_ENABLED`/`ACT_DISABLED`/`ACT_PASSIVE`: a pet spell.
    Spell(u32),
}

impl PetActionSlot {
    pub fn from_packed(packed: u32) -> Self {
        let action = pet_action_button_action(packed);
        match pet_action_button_type(packed) {
            _ if packed == 0 => Self::Empty,
            ACT_COMMAND => Self::Command(action),
            ACT_REACTION => Self::Reaction(action),
            _ => Self::Spell(action),
        }
    }

    /// The `PET_*_TEXTURE` token icon of a command or reaction.
    pub fn token_icon(self) -> Option<u32> {
        Some(match self {
            Self::Command(COMMAND_ATTACK) => PET_ATTACK_TEXTURE,
            Self::Command(COMMAND_FOLLOW) => PET_FOLLOW_TEXTURE,
            Self::Command(COMMAND_STAY) => PET_WAIT_TEXTURE,
            Self::Command(COMMAND_MOVE_TO) => PET_MOVE_TO_TEXTURE,
            Self::Reaction(REACT_ASSIST) => PET_ASSIST_TEXTURE,
            Self::Reaction(REACT_DEFENSIVE) => PET_DEFENSIVE_TEXTURE,
            Self::Reaction(REACT_PASSIVE) => PET_PASSIVE_TEXTURE,
            Self::Reaction(REACT_AGGRESSIVE) => PET_AGGRESSIVE_TEXTURE,
            _ => return None,
        })
    }
}

/// `GetPetActionInfo` isActive: Attack while the pet attacks (`UNIT_FLAG_PET_IN_COMBAT`),
/// another command matching the pet's `CommandState`, a reaction matching its `ReactState`.
pub fn pet_action_active(slot: PetActionSlot, spells: &PetSpells, pet_in_combat: bool) -> bool {
    match slot {
        PetActionSlot::Command(COMMAND_ATTACK) => pet_in_combat,
        PetActionSlot::Command(command) => command == spells.command_state,
        PetActionSlot::Reaction(react) => react == spells.react_state,
        PetActionSlot::Empty | PetActionSlot::Spell(_) => false,
    }
}

/// `PetActionButtonMixin_OnUpdate`: from `StartFlash` the Flash shows for
/// `ATTACK_BUTTON_FLASH_TIME`, hides for as long, and so on.
pub fn attack_flash_shown(seconds_since_start: f32) -> bool {
    (seconds_since_start / ATTACK_BUTTON_FLASH_TIME) as u32 % 2 == 0
}

/// One button's contents.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PetActionButtonView {
    /// Icon FDID; 0 for an empty slot.
    pub icon_fdid: u32,
    /// `SetChecked(isActive)`.
    pub checked: bool,
    /// `CheckedTexture` alpha: 0.5 on the flashing Attack button.
    pub checked_alpha: f32,
    /// The `Flash` texture is in its shown phase.
    pub flash: bool,
    /// `SetHotkeys`: the abbreviated `BONUSACTIONBUTTONn` binding.
    pub hotkey: String,
    /// Key held or button pressed: `PushedTexture` replaces `NormalTexture`.
    pub pushed: bool,
    /// Pointer over the button: `HighlightTexture`.
    pub hovered: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PetActionBarState {
    /// `PetHasActionBar() and UnitIsVisible("pet")`.
    pub visible: bool,
    pub buttons: [PetActionButtonView; PET_BAR_BUTTONS],
}

/// `PetActionBarMixin:Update` for the server's bar: `spell_icon` maps a spell id to its
/// icon FDID (0 unknown); `attack_flash` is the Flash phase of an attacking pet.
pub fn pet_bar_buttons(
    spells: &PetSpells,
    pet_in_combat: bool,
    attack_flash: bool,
    spell_icon: impl Fn(u32) -> u32,
    hotkeys: &[String; PET_BAR_BUTTONS],
) -> [PetActionButtonView; PET_BAR_BUTTONS] {
    std::array::from_fn(|index| {
        let slot = PetActionSlot::from_packed(spells.action_buttons[index]);
        let checked = pet_action_active(slot, spells, pet_in_combat);
        let attack = slot == PetActionSlot::Command(COMMAND_ATTACK) && checked;
        let icon_fdid = match slot {
            PetActionSlot::Spell(spell_id) => spell_icon(spell_id),
            slot => slot.token_icon().unwrap_or(0),
        };
        PetActionButtonView {
            icon_fdid,
            checked,
            checked_alpha: if attack { ATTACK_CHECKED_ALPHA } else { 1.0 },
            flash: attack && attack_flash,
            hotkey: hotkeys[index].clone(),
            pushed: false,
            hovered: false,
        }
    })
}

/// Retail `PetActionButton<n>` name of button `index` (0-based).
pub fn pet_action_button_name(index: usize) -> String {
    format!("PetActionButton{}", index + 1)
}

/// Button index of an action this screen emitted.
pub fn parse_pet_action_button(action: &str) -> Option<usize> {
    action
        .strip_prefix(PET_ACTION_BUTTON_PREFIX)?
        .parse()
        .ok()
        .filter(|&index| index < PET_BAR_BUTTONS)
}

struct DynName(String);

fn art(
    name: String,
    art: &AtlasArt,
    rect: (f32, f32, f32, f32),
    alpha: f32,
    hidden: bool,
) -> Element {
    let (x, y, width, height) = rect;
    let coords = art.tex_coords(1.0);
    let color = format!("1.0,1.0,1.0,{alpha}");
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            hidden,
            texture_fdid: {art.fdid},
            tex_coords: {coords.as_str()},
            vertex_color: {color.as_str()},
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}

fn icon(name: String, fdid: u32) -> Element {
    rsx! {
        texture {
            name: {DynName(name)},
            width: PET_BUTTON_SIZE,
            height: PET_BUTTON_SIZE,
            hidden: {fdid == 0},
            texture_fdid: {fdid},
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: 0.0,
        }
    }
}

fn hotkey(name: &str, text: &str) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("{name}HotKey"))},
            width: HOTKEY_W,
            height: HOTKEY_H,
            text,
            font: GameFont::ArialNarrow,
            font_size: 12.0,
            font_color: HOTKEY_COLOR,
            outline: "OUTLINE",
            justify_h: "RIGHT",
            pos_type: "absolute",
            right: HOTKEY_RIGHT,
            pos_y: HOTKEY_TOP,
        }
    }
}

fn button(index: usize, view: &PetActionButtonView) -> Element {
    let name = pet_action_button_name(index);
    let x = index as f32 * (PET_BUTTON_SIZE + PET_BUTTON_PADDING);
    let cell = (0.0, 0.0, PET_BUTTON_SIZE, PET_BUTTON_SIZE);
    let frame_art = (0.0, 0.0, FRAME_ART, FRAME_ART);
    let overlay = (0.0, 0.0, OVERLAY_W, OVERLAY_H);
    let children: Element = [
        art(
            format!("{name}SlotBackground"),
            &SLOT_BACKGROUND,
            cell,
            1.0,
            false,
        ),
        art(format!("{name}SlotArt"), &SLOT_ART, cell, 1.0, false),
        icon(format!("{name}Icon"), view.icon_fdid),
        art(format!("{name}Flash"), &FLASH, overlay, 1.0, !view.flash),
        art(
            format!("{name}NormalTexture"),
            &NORMAL,
            frame_art,
            1.0,
            view.pushed,
        ),
        art(
            format!("{name}PushedTexture"),
            &PUSHED,
            frame_art,
            1.0,
            !view.pushed,
        ),
        art(
            format!("{name}CheckedTexture"),
            &MOUSEOVER,
            overlay,
            view.checked_alpha,
            !view.checked,
        ),
        art(
            format!("{name}HighlightTexture"),
            &MOUSEOVER,
            overlay,
            1.0,
            !view.hovered,
        ),
        hotkey(&name, &view.hotkey),
    ]
    .into_iter()
    .flatten()
    .collect();
    rsx! {
        button {
            name: {DynName(name)},
            width: PET_BUTTON_SIZE,
            height: PET_BUTTON_SIZE,
            onclick: {format!("{PET_ACTION_BUTTON_PREFIX}{index}")},
            button_default_skin: false,
            pos_type: "absolute",
            pos_x: x,
            pos_y: 0.0,
            {children}
        }
    }
}

pub fn pet_action_bar_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<PetActionBarState>()
        .expect("PetActionBarState must be in SharedContext");
    let buttons: Element = state
        .buttons
        .iter()
        .enumerate()
        .flat_map(|(index, view)| button(index, view))
        .collect();
    let hidden = !state.visible;
    rsx! {
        r#frame {
            name: PET_ACTION_BAR,
            width: PET_BAR_W,
            height: PET_BUTTON_SIZE,
            hidden,
            strata: FrameStrata::Medium,
            pos_type: "absolute",
            left: "50%",
            margin_left: PET_BAR_LEFT,
            bottom: PET_BAR_BOTTOM,
            {buttons}
        }
    }
}
