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
//! StanceBar between them. `hud_layout` supplies the active preset's stack anchor; Forever
//! uses Camelot's main-bar BOTTOMLEFT + (30, scaled main height + 4), and FlareUI's
//! pet-button scale 1.06. Chrome atlas names resolve per skin. Autocastable spells carry the `AutoCastOverlay` (31×31 at
//! CENTER + (0.5, -0.5), `SmallActionButtonMixin_OnLoad`): its Corners, and while autocast is
//! on its rotating Shine, which the host composes into `shine_texture`
//! (`game_engine_core::pet_autocast_shine_data`).

use shared::protocol::{
    ACT_COMMAND, ACT_DISABLED, ACT_ENABLED, ACT_REACTION, COMMAND_ATTACK, COMMAND_FOLLOW,
    COMMAND_MOVE_TO, COMMAND_STAY, PET_ACTION_BAR_SLOTS, PetSpells, REACT_AGGRESSIVE, REACT_ASSIST,
    REACT_DEFENSIVE, REACT_PASSIVE, pet_action_button_action, pet_action_button_type,
};
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::texture::{DynamicTextureId, TextureSource};

use crate::hud_layout::{FOREVER_ACTION_BUTTON_SCALE, hud_layout};
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

/// `AutoCastOverlay` of a small button: 31×31 at CENTER + (0.5, -0.5) of the 30×30 button.
const AUTOCAST_OVERLAY: (f32, f32, f32, f32) = (0.0, 0.0, 31.0, 31.0);
/// Its Shine: TOPLEFT (-5, 5), BOTTOMRIGHT (5, -5) of the overlay.
const AUTOCAST_SHINE: (f32, f32, f32, f32) = (-5.0, -5.0, 41.0, 41.0);
/// `UI-HUD-ActionBar-PetAutoCast-Corners` (UiTextureAtlasMember 25944) in UiTextureAtlas
/// 2476 (FDID 5199404, 2048×1024).
const AUTOCAST_CORNERS: AtlasArt = AtlasArt {
    fdid: PET_AUTOCAST_ATLAS,
    atlas: (2048.0, 1024.0),
    rect: (1059.0, 1105.0, 498.0, 544.0),
};
pub const PET_AUTOCAST_ATLAS: u32 = 5_199_404;
/// `UI-HUD-ActionBar-PetAutoCast-Ants` (UiTextureAtlasMember 25943): `(left, right, top,
/// bottom)` in `PET_AUTOCAST_ATLAS`.
pub const PET_AUTOCAST_ANTS: (u32, u32, u32, u32) = (1036, 1113, 403, 480);
/// `UI-HUD-ActionBar-PetAutoCast-Mask` (UiTextureAtlasMember 25946): all of UiTextureAtlas
/// 2710, FDID 5550114 (32×32).
pub const PET_AUTOCAST_MASK: u32 = 5_550_114;

const SLOT_BACKGROUND: &str = "UI-HUD-ActionBar-IconFrame-Background";
const SLOT_ART: &str = "UI-HUD-ActionBar-IconFrame-Slot";
const NORMAL: &str = "UI-HUD-ActionBar-IconFrame";
const PUSHED: &str = "UI-HUD-ActionBar-IconFrame-Down";
/// Both `HighlightTexture` and `CheckedTexture`.
const MOUSEOVER: &str = "UI-HUD-ActionBar-IconFrame-Mouseover";
const FLASH: &str = "UI-HUD-ActionBar-IconFrame-Flash";

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
pub const PET_BAR_ART_FDIDS: [u32; 11] = [
    4_613_342,
    PET_AUTOCAST_ATLAS,
    PET_AUTOCAST_MASK,
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

/// `GetPetActionInfo` autoCastAllowed / autoCastEnabled of a button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PetAutocast {
    /// A command, a stance, an empty slot or a spell that cannot autocast (`ACT_PASSIVE`).
    #[default]
    Unavailable,
    /// `ACT_DISABLED`.
    Off,
    /// `ACT_ENABLED`.
    On,
}

impl PetAutocast {
    pub fn from_packed(packed: u32) -> Self {
        match pet_action_button_type(packed) {
            _ if packed == 0 => Self::Unavailable,
            ACT_ENABLED => Self::On,
            ACT_DISABLED => Self::Off,
            _ => Self::Unavailable,
        }
    }
}

/// `TogglePetAutocast`: the spell of an autocastable button and whether autocast turns on.
pub fn pet_autocast_toggle(packed: u32) -> Option<(u32, bool)> {
    let spell = pet_action_button_action(packed);
    match PetAutocast::from_packed(packed) {
        PetAutocast::On => Some((spell, false)),
        PetAutocast::Off => Some((spell, true)),
        PetAutocast::Unavailable => None,
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
    /// `AutoCastOverlay:SetShown(autoCastAllowed)`, `ShowAutoCastEnabled(autoCastEnabled)`.
    pub autocast: PetAutocast,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PetActionBarState {
    /// `PetHasActionBar() and UnitIsVisible("pet")`.
    pub visible: bool,
    pub buttons: [PetActionButtonView; PET_BAR_BUTTONS],
    /// The host's composed autocast Shine, drawn by every button with autocast on.
    pub shine_texture: Option<DynamicTextureId>,
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
            autocast: PetAutocast::from_packed(spells.action_buttons[index]),
        }
    })
}

/// Registry-only property the markup does not set: the Shines draw the host's composite.
pub fn apply_pet_action_bar_postsetup(state: &PetActionBarState, registry: &mut FrameRegistry) {
    let Some(shine) = state.shine_texture else {
        return;
    };
    for index in 0..PET_BAR_BUTTONS {
        let name = format!("{}AutoCastShine", pet_action_button_name(index));
        let Some(id) = registry.get_by_name(&name) else {
            continue;
        };
        if let Some(frame) = registry.get_mut(id)
            && let Some(WidgetData::Texture(texture)) = frame.widget_data.as_mut()
        {
            texture.source = TextureSource::Dynamic(shine);
        }
    }
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

fn art(name: String, atlas: &str, rect: (f32, f32, f32, f32), alpha: f32, hidden: bool) -> Element {
    let (x, y, width, height) = rect;
    let color = format!("1.0,1.0,1.0,{alpha}");
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            hidden,
            texture_atlas: atlas,
            vertex_color: {color.as_str()},
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}

/// The `AutoCastOverlay` frame's OVERLAY layer: Corners while autocast is allowed, the
/// Shine (its source set by `apply_pet_action_bar_postsetup`) while it is on.
fn autocast_overlay(name: &str, autocast: PetAutocast, scale: f32) -> Element {
    let corners = AUTOCAST_CORNERS.tex_coords(1.0);
    let (x, y, width, height) = AUTOCAST_OVERLAY;
    let (shine_x, shine_y, shine_w, shine_h) = AUTOCAST_SHINE;
    rsx! {
        texture {
            name: {DynName(format!("{name}AutoCastShine"))},
            width: {shine_w * scale},
            height: {shine_h * scale},
            hidden: {autocast != PetAutocast::On},
            draw_layer: "OVERLAY",
            pos_type: "absolute",
            pos_x: {shine_x * scale},
            pos_y: {shine_y * scale},
        }
        texture {
            name: {DynName(format!("{name}AutoCastCorners"))},
            width: {width * scale},
            height: {height * scale},
            hidden: {autocast == PetAutocast::Unavailable},
            texture_fdid: {AUTOCAST_CORNERS.fdid},
            tex_coords: {corners.as_str()},
            draw_layer: "OVERLAY",
            pos_type: "absolute",
            pos_x: {x * scale},
            pos_y: {y * scale},
        }
    }
}

fn icon(name: String, fdid: u32, size: f32) -> Element {
    rsx! {
        texture {
            name: {DynName(name)},
            width: size,
            height: size,
            hidden: {fdid == 0},
            texture_fdid: {fdid},
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: 0.0,
        }
    }
}

fn hotkey(name: &str, text: &str, scale: f32) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("{name}HotKey"))},
            width: {HOTKEY_W * scale},
            height: {HOTKEY_H * scale},
            text,
            font: GameFont::ArialNarrow,
            font_size: {12.0 * scale},
            font_color: HOTKEY_COLOR,
            outline: "OUTLINE",
            justify_h: "RIGHT",
            pos_type: "absolute",
            right: {HOTKEY_RIGHT * scale},
            pos_y: {HOTKEY_TOP * scale},
        }
    }
}

fn button(index: usize, view: &PetActionButtonView, scale: f32) -> Element {
    let name = pet_action_button_name(index);
    let size = PET_BUTTON_SIZE * scale;
    let x = index as f32 * (PET_BUTTON_SIZE + PET_BUTTON_PADDING) * scale;
    let cell = (0.0, 0.0, size, size);
    let frame_art = (0.0, 0.0, FRAME_ART * scale, FRAME_ART * scale);
    let overlay = (0.0, 0.0, OVERLAY_W * scale, OVERLAY_H * scale);
    let children: Element = [
        art(
            format!("{name}SlotBackground"),
            SLOT_BACKGROUND,
            cell,
            1.0,
            false,
        ),
        art(format!("{name}SlotArt"), SLOT_ART, cell, 1.0, false),
        icon(format!("{name}Icon"), view.icon_fdid, size),
        art(format!("{name}Flash"), FLASH, overlay, 1.0, !view.flash),
        art(
            format!("{name}NormalTexture"),
            NORMAL,
            frame_art,
            1.0,
            view.pushed,
        ),
        art(
            format!("{name}PushedTexture"),
            PUSHED,
            frame_art,
            1.0,
            !view.pushed,
        ),
        art(
            format!("{name}CheckedTexture"),
            MOUSEOVER,
            overlay,
            view.checked_alpha,
            !view.checked,
        ),
        art(
            format!("{name}HighlightTexture"),
            MOUSEOVER,
            overlay,
            1.0,
            !view.hovered,
        ),
        autocast_overlay(&name, view.autocast, scale),
        hotkey(&name, &view.hotkey, scale),
    ]
    .into_iter()
    .flatten()
    .collect();
    rsx! {
        button {
            name: {DynName(name)},
            width: size,
            height: size,
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
    let skin = *ctx
        .get::<ActiveSkin>()
        .expect("canvas carries the active skin");
    let scale = match skin {
        ActiveSkin::Modern => 1.0,
        ActiveSkin::Forever => FOREVER_ACTION_BUTTON_SCALE,
    };
    let size = (PET_BAR_W * scale, PET_BUTTON_SIZE * scale);
    let at = hud_layout(ctx).pet_action_bar.place(size);
    let buttons: Element = state
        .buttons
        .iter()
        .enumerate()
        .flat_map(|(index, view)| button(index, view, scale))
        .collect();
    let hidden = !state.visible;
    rsx! {
        r#frame {
            name: PET_ACTION_BAR,
            width: {size.0},
            height: {size.1},
            hidden,
            strata: FrameStrata::Medium,
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
