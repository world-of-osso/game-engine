use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::faction_reaction::Reaction;
use crate::hud_layout::{HudAnchor, hud_layout};
use crate::status_text_data::StatusBarText;
use crate::ui::screens::menu_primitives::{
    ContextMenu, ContextMenuItem, context_menu, menu_height_for_items,
};
use crate::ui::strata::FrameStrata;
use crate::unit_frame_style::styled_frame;
use shared::components::CreatureClassification;
#[path = "class_bars/mod.rs"]
pub mod class_bars;
#[path = "inworld_unit_frames_art.rs"]
pub mod inworld_unit_frames_art;
#[path = "inworld_unit_frames_aura.rs"]
mod inworld_unit_frames_aura;
#[path = "inworld_unit_frames_flare.rs"]
pub mod inworld_unit_frames_flare;
#[path = "inworld_unit_frames_layout.rs"]
mod inworld_unit_frames_layout;
#[path = "inworld_unit_frames_parts.rs"]
mod inworld_unit_frames_parts;
#[path = "inworld_unit_frames_pet.rs"]
mod inworld_unit_frames_pet;
#[path = "inworld_unit_frames_power.rs"]
mod inworld_unit_frames_power;
#[path = "personal_resource_display.rs"]
pub mod personal_resource_display;
#[path = "portrait_party_frame_component.rs"]
pub mod portrait_party_frame_component;
use class_bars::{ClassBarView, TextureView};
use inworld_unit_frames_art::{
    BOSS_GOLD, BOSS_RARE_SILVER, BOSS_RARE_STAR, COMBAT_ICON, FRAME_PORTRAIT_OFF, HEALTH_BAR,
    PLAYER_PORTRAIT_ON, REACTION_STRIP, REST_FLIPBOOK, REST_ICON_COORDS, TARGET_HEALTH_BAR,
    TARGET_PORTRAIT_ON, power_bar_atlas, sized_atlas_texture,
};
use inworld_unit_frames_aura::target_auras;
pub use inworld_unit_frames_aura::{
    MAX_TARGET_BUFFS, MAX_TARGET_DEBUFFS, TargetAuraView, set_target_auras, target_aura_icon,
    target_frame_auras,
};
use inworld_unit_frames_flare::{
    FLARE_FOCUS, FLARE_PLAYER, FLARE_TARGET, FLARE_TARGET_OF_TARGET, FlareFrame, FlareUnit,
    flare_frame,
};
pub use inworld_unit_frames_layout::*;
use inworld_unit_frames_parts::{
    BarSpec, WHITE, art_root, art_texture, centred_art, portrait_slot, status_bar, unit_label,
};
pub use inworld_unit_frames_pet::PetFrameState;
use inworld_unit_frames_pet::pet_frame;
pub use inworld_unit_frames_power::{PowerBarState, power_bar_rgb};
use personal_resource_display::PersonalResourceDisplayState;

pub const ACTION_UNIT_MENU_SET_FOCUS: &str = "unit_menu_set_focus";
pub const ACTION_UNIT_MENU_CLEAR_FOCUS: &str = "unit_menu_clear_focus";
pub const ACTION_UNIT_MENU_CLOSE: &str = "unit_menu_close";
pub const ACTION_UNIT_MENU_INSPECT: &str = "unit_menu_inspect";
/// `UnitPopupTradeButtonMixin`.
pub const ACTION_UNIT_MENU_TRADE: &str = "unit_menu_trade";
/// `UnitPopupDungeonDifficultyButtonMixin`: opens the Dungeon Difficulty submenu.
pub const ACTION_UNIT_MENU_DUNGEON_DIFFICULTY: &str = "unit_menu_dungeon_difficulty";
/// `UnitPopupDungeonDifficulty1..3ButtonMixin:OnClick` → `SetDungeonDifficultyID(<id>)`.
pub const ACTION_UNIT_MENU_SET_DUNGEON_DIFFICULTY_PREFIX: &str = "unit_menu_dungeon_difficulty:";
pub const DIFFICULTY_MENU_W: f32 = 120.0;
const DIFFICULTY_ROW_H: f32 = 20.0;
const DIFFICULTY_MENU_TOP: f32 = 26.0;
/// `common-dropdown-tickradial` / `common-dropdown-icon-radialtick-yellow` on
/// UiTextureAtlas 2634 (FDID 5390329, 512x256), 18x18.
const RADIO_SHEET_FDID: u32 = 5_390_329;
const RADIO_EMPTY_COORDS: &str = "0.138671875,0.173828125,0.52734375,0.59765625";
const RADIO_CHECKED_COORDS: &str = "0.138671875,0.173828125,0.44921875,0.51953125";
const MENU_TEXT_ENABLED: &str = "1.0,1.0,1.0,1.0";
const MENU_TEXT_DISABLED: &str = "0.5,0.5,0.5,1.0";
pub const UNIT_MENU_W: f32 = 140.0;
const UNIT_MENU_CLOSE: ContextMenuItem<'static> = ContextMenuItem {
    name: "UnitFrameContextMenuClose",
    label: "Close",
    action: ACTION_UNIT_MENU_CLOSE,
};

/// Menu height with `items` entries before Close.
pub fn unit_menu_height(items: usize) -> f32 {
    menu_height_for_items(items + 1)
}

/// The menu's focus entry: `UnitPopupMenuFocus` (opened from FocusFrame) lists
/// `UnitPopupClearFocusButtonMixin`, every other unit menu `UnitPopupSetFocusButtonMixin`
/// (UnitPopupSharedMenus.lua:346-352, 316; UnitPopupSharedButtonMixins.lua:2169-2186).
pub fn focus_menu_item(from_focus_frame: bool) -> UnitMenuItem {
    let (name, label, action) = if from_focus_frame {
        ("ClearFocus", "Clear Focus", ACTION_UNIT_MENU_CLEAR_FOCUS)
    } else {
        ("SetFocus", "Set Focus", ACTION_UNIT_MENU_SET_FOCUS)
    };
    UnitMenuItem {
        name: format!("UnitFrameContextMenu{name}"),
        label: label.into(),
        action: action.into(),
    }
}

#[derive(Clone)]
pub(super) struct DynName(pub(super) String);

pub(super) fn dyn_name(name: String) -> DynName {
    DynName(name)
}

/// Reaction strip tint behind the unit name: Retail `TargetFrame` sets
/// `ReputationColor:SetVertexColor(UnitSelectionColor(unit))`, which is red, yellow or green
/// for hostile, neutral and friendly NPCs. Health bars keep their green art (`lockColor`).
pub fn reaction_color(reaction: Reaction) -> &'static str {
    match reaction {
        Reaction::Hostile => "1.0,0.0,0.0,1.0",
        Reaction::Neutral => "1.0,1.0,0.0,1.0",
        Reaction::Friendly => "0.0,1.0,0.0,1.0",
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnitFrameState {
    pub name: String,
    pub level_text: String,
    /// "r,g,b,a" of the level text: gold, or the target's difficulty colour.
    pub level_color: String,
    /// The health bar's status text (`TextStatusBar`).
    pub health_text: StatusBarText,
    /// Health fill fraction 0.0..=1.0.
    pub health_fraction: f32,
    /// `UnitHealth(unit) <= 0`: the bar's `DeadText` (`TargetFrameMixin:CheckDead`,
    /// TargetFrame.lua:467-480).
    pub dead: bool,
    pub reaction: Option<Reaction>,
    /// Replicated `Player.class` for FlareUI's player-only class tint.
    pub class_id: Option<u8>,
    /// `UnitClassification`: elite and rare art around the target portrait slot.
    pub classification: CreatureClassification,
    pub power: Option<PowerBarState>,
    /// The power bar's status text.
    pub power_text: StatusBarText,
    /// The player's class resource bar as drawn this frame ([`class_bars`]).
    pub class_bar: Option<ClassBarView>,
    pub show_combat_icon: bool,
    pub show_resting_icon: bool,
    /// Forever player-frame auras; harmful entries belong to the local player or pet.
    /// Filled from the same counted-down aura data as BuffFrame.
    pub player_auras: Vec<crate::aura_display_data::AuraInstance>,
    pub target_buffs: Vec<TargetAuraIconState>,
    pub target_debuffs: Vec<TargetAuraIconState>,
    /// Friendly target: buffs lead the aura container, else debuffs do.
    pub target_buffs_first: bool,
    /// `GetRaidTargetIndex(unit)`: raid target icon 1–8.
    pub raid_target: Option<u8>,
}

impl UnitFrameState {
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            level_text: String::new(),
            level_color: GOLD_TEXT.to_string(),
            health_text: StatusBarText::default(),
            health_fraction: 0.0,
            dead: false,
            reaction: None,
            class_id: None,
            classification: CreatureClassification::Normal,
            power: None,
            power_text: StatusBarText::default(),
            class_bar: None,
            show_combat_icon: false,
            show_resting_icon: false,
            player_auras: Vec::new(),
            target_buffs: Vec::new(),
            target_debuffs: Vec::new(),
            target_buffs_first: false,
            raid_target: None,
        }
    }
}

/// Target-of-target and focus: name, level and health; under Modern target of target
/// omits the level, and focus adds its power bar.
#[derive(Clone, Debug, PartialEq)]
pub struct SmallUnitFrameState {
    pub name: String,
    pub level: Option<(String, String)>,
    pub health_fraction: f32,
    /// `UnitHealth(unit) <= 0` (`TargetOfTargetMixin:CheckDead`, TargetFrame.lua:915-924).
    pub dead: bool,
    pub reaction: Option<Reaction>,
    pub class_id: Option<u8>,
    /// The focus frame's `ManaBar` (`TargetFrameTemplate`); target of target shows none.
    pub power: Option<PowerBarState>,
}

impl From<&UnitFrameState> for SmallUnitFrameState {
    fn from(unit: &UnitFrameState) -> Self {
        Self {
            name: unit.name.clone(),
            level: Some((unit.level_text.clone(), unit.level_color.clone())),
            health_fraction: unit.health_fraction,
            dead: unit.dead,
            reaction: unit.reaction,
            class_id: unit.class_id,
            power: unit.power.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct UnitFrameMenuState {
    pub visible: bool,
    pub title: String,
    pub x: f32,
    pub y: f32,
    /// The entries before Close: the focus entry, player-unit entries (`UnitPopup` Invite /
    /// Promote / Leave … then Trade) and the raid target icons.
    pub player_items: Vec<UnitMenuItem>,
    /// The Dungeon Difficulty submenu, open beside the menu.
    pub difficulty_menu: Option<DifficultyMenuState>,
}

/// `UnitPopupDungeonDifficultyButtonMixin:GetEntries`: Normal, Heroic, Mythic radios.
#[derive(Clone, Debug, PartialEq)]
pub struct DifficultyMenuState {
    pub x: f32,
    pub y: f32,
    pub entries: Vec<DifficultyMenuEntry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DifficultyMenuEntry {
    pub difficulty_id: u32,
    /// `PLAYER_DIFFICULTY1`, `PLAYER_DIFFICULTY2`, `PLAYER_DIFFICULTY6`.
    pub label: String,
    /// `IsChecked`: the player's dungeon difficulty.
    pub checked: bool,
    /// `IsEnabled` (`DifficultyUtil.IsDungeonDifficultyEnabled`).
    pub enabled: bool,
}

/// Height of the Dungeon Difficulty submenu with `rows` entries.
pub fn difficulty_menu_height(rows: usize) -> f32 {
    DIFFICULTY_MENU_TOP + rows as f32 * DIFFICULTY_ROW_H + 6.0
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnitMenuItem {
    pub name: String,
    pub label: String,
    pub action: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TargetAuraIconState {
    pub spell_id: u32,
    pub icon_fdid: u32,
    pub stacks: u32,
    /// Debuffs: the `DispelBorder` tint (`AuraUtil.SetAuraBorderColor`); buffs have none.
    pub dispel_color: Option<String>,
    /// Cast by the local player: `LargeAuraSize`.
    pub large: bool,
    /// Elapsed fraction of a timed aura, for the reverse cooldown swipe; `None` when
    /// permanent.
    pub elapsed: Option<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InWorldUnitFramesState {
    pub show_player_frame: bool,
    pub show_target_frame: bool,
    pub target_cast: Option<crate::casting_bar_frame_component::CastingBarState>,
    pub player: UnitFrameState,
    pub target: Option<UnitFrameState>,
    pub target_of_target: Option<SmallUnitFrameState>,
    pub focus: Option<SmallUnitFrameState>,
    /// The local player's pet, shown in PetFrame.
    pub pet: Option<PetFrameState>,
    /// `boss1..boss5` (`INSTANCE_ENCOUNTER_ENGAGE_UNIT`), Boss1TargetFrame first.
    pub bosses: Vec<UnitFrameState>,
    pub menu: UnitFrameMenuState,
    /// PersonalResourceDisplayFrame, when shown.
    pub personal_resource: Option<PersonalResourceDisplayState>,
}

pub fn fraction(current: f32, max: f32) -> f32 {
    if max <= 0.0 {
        return 0.0;
    }
    (current / max).clamp(0.0, 1.0)
}

/// Retail hides the level of units 10 or more levels above the player behind "??".
pub fn target_level_text(level: Option<u8>, player_level: Option<u8>) -> String {
    match (level, player_level) {
        (Some(level), Some(player)) if u16::from(level) >= u16::from(player) + 10 => "??".into(),
        (Some(level), _) => level.to_string(),
        (None, _) => String::new(),
    }
}

pub fn format_value_text(current: f32, max: f32) -> String {
    format!("{current:.0} / {max:.0}")
}

pub fn inworld_unit_frames_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<InWorldUnitFramesState>()
        .expect("InWorldUnitFramesState must be in SharedContext");
    let layout = hud_layout(ctx);
    let skin = *ctx
        .get::<ActiveSkin>()
        .expect("canvas carries the active skin");
    let player = player_frame(&state.player, state.show_player_frame, &layout.player, skin);
    let pet = pet_frame(
        state.pet.as_ref().filter(|_| state.show_player_frame),
        &layout.pet,
        skin,
    );
    let target = target_frame(
        state.target.as_ref(),
        state.show_target_frame,
        &layout.target,
        skin,
    );
    let target_of_target = small_unit_frame(
        SmallFrameSpec::TARGET_OF_TARGET,
        visible_target_of(state),
        &layout.target_of_target,
        skin,
    );
    let focus = small_unit_frame(
        SmallFrameSpec::FOCUS,
        state.focus.as_ref(),
        &layout.focus,
        skin,
    );
    rsx! {
        r#frame {
            name: "InWorldUnitFramesRoot",
            width: "fill",
            height: "fill",
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: 0.0,
            strata: FrameStrata::Dialog,
            background_color: "0.0,0.0,0.0,0.0",
            {styled_frame(player, &layout.player_style, &layout.player)}
            {styled_frame(pet, &layout.pet_style, &layout.pet)}
            {styled_frame(target, &layout.target_style, &layout.target)}
            {crate::casting_bar_frame_component::target_cast_bar_frame(ctx, state)}
            {styled_frame(target_of_target, &layout.target_style, &layout.target_of_target)}
            {styled_frame(focus, &layout.focus_style, &layout.focus)}
            {boss_frames(&state.bosses, skin)}
            {personal_resource_display::frame(state.personal_resource.as_ref())}
            {unit_frame_menu(&state.menu)}
            {difficulty_menu(state.menu.difficulty_menu.as_ref())}
        }
    }
}

pub(super) fn modern_target_cast_offset(units: &InWorldUnitFramesState) -> (f32, f32) {
    // TargetFrame.lua:556-573,819-838: >0 aura rows without ToT, >2 with ToT.
    let have_tot = units.target_of_target.is_some();
    let (rows, bottom) =
        inworld_unit_frames_aura::target_aura_rows(units.target.as_ref().expect("shown target"));
    let anchor_to_auras = rows > if have_tot { 2 } else { 0 };
    if anchor_to_auras {
        return (TARGET_AURAS_LEFT + 18.0, TARGET_AURAS_TOP + bottom + 10.0);
    }
    (43.0, UNIT_FRAME_H + if have_tot { 46.0 } else { -5.0 })
}

fn visible_target_of(state: &InWorldUnitFramesState) -> Option<&SmallUnitFrameState> {
    let target_shown = state.show_target_frame && state.target.is_some();
    state.target_of_target.as_ref().filter(|_| target_shown)
}

const UNIT_FRAME_SIZE: (f32, f32) = (UNIT_FRAME_W, UNIT_FRAME_H);

fn player_frame(
    state: &UnitFrameState,
    visible: bool,
    anchor: &HudAnchor,
    skin: ActiveSkin,
) -> Element {
    if skin == ActiveSkin::Forever {
        let unit = FlareUnit {
            aura_state: Some(state),
            ..state.into()
        };
        return flare_frame(&FLARE_PLAYER, Some(unit), !visible, anchor);
    }
    let content = rsx! {
        {unit_frame_contents("Player", state, &PLAYER_SLOTS, HEALTH_BAR)}
        {class_bar(state.class_bar.as_ref())}
        {status_icons(state, skin)}
    };
    art_root(
        dyn_name("PlayerFrame".into()),
        UNIT_FRAME_SIZE,
        anchor,
        !visible,
        centred_art("PlayerFrame", PLAYER_PORTRAIT_ON, UNIT_FRAME_SIZE, skin),
        portrait_slot(&PLAYER_PORTRAIT),
        content,
    )
}

fn target_frame(
    target: Option<&UnitFrameState>,
    visible: bool,
    anchor: &HudAnchor,
    skin: ActiveSkin,
) -> Element {
    if skin == ActiveSkin::Forever {
        return flare_frame(
            &FLARE_TARGET,
            target.map(|state| FlareUnit {
                aura_state: Some(state),
                ..state.into()
            }),
            !visible,
            anchor,
        );
    }
    let content = target
        .map(|target| target_frame_contents(target, skin))
        .unwrap_or_default();
    art_root(
        dyn_name("TargetFrame".into()),
        UNIT_FRAME_SIZE,
        anchor,
        target.is_none() || !visible,
        centred_art("TargetFrame", TARGET_PORTRAIT_ON, UNIT_FRAME_SIZE, skin),
        portrait_slot(&TARGET_PORTRAIT),
        content,
    )
}

fn target_frame_contents(state: &UnitFrameState, skin: ActiveSkin) -> Element {
    let (strip_x, strip_y) = TARGET_REPUTATION;
    let strip = move |(width, height)| (strip_x, strip_y, width, height);
    rsx! {
        {reaction_strip("Target", state.reaction, skin, strip)}
        {unit_frame_contents("Target", state, &TARGET_SLOTS, TARGET_HEALTH_BAR)}
        {dead_text("Target", TARGET_SLOTS.health, UNIT_FONT_SIZE, state.dead)}
        {classification_art(state.classification, skin)}
        {raid_target_icon(state.raid_target)}
        {target_auras(state, (TARGET_AURAS_LEFT, TARGET_AURAS_TOP))}
    }
}

/// `TargetFrameMixin:CheckClassification` (TargetFrame.lua:436-445): the gold dragon for
/// elites, the silver one for rare elites. The winged `UnitIsBossMob` dragon is not drawn.
pub fn boss_portrait_atlas(classification: CreatureClassification) -> Option<&'static str> {
    match classification {
        CreatureClassification::RareElite => Some(BOSS_RARE_SILVER),
        CreatureClassification::Elite => Some(BOSS_GOLD),
        _ => None,
    }
}

/// The rare star `BossIcon` (TargetFrame.lua:457-462).
pub fn shows_rare_star(classification: CreatureClassification) -> bool {
    matches!(
        classification,
        CreatureClassification::Rare | CreatureClassification::RareElite
    )
}

/// The dragon at its atlas size TOPRIGHT of the frame frames the portrait; the star,
/// at its atlas size, is centred on the portrait's bottom edge.
fn classification_art(classification: CreatureClassification, skin: ActiveSkin) -> Element {
    let portrait = boss_portrait_atlas(classification);
    let (right, top) = TARGET_BOSS_PORTRAIT_TOPRIGHT;
    let (star_x, star_y) = TARGET_BOSS_ICON_CENTRE;
    rsx! {
        {sized_atlas_texture("TargetBossPortraitFrameTexture".into(), portrait.unwrap_or(BOSS_GOLD), skin, |(width, height)| (UNIT_FRAME_W - right - width, top, width, height), WHITE, portrait.is_none())}
        {sized_atlas_texture("TargetBossIcon".into(), BOSS_RARE_STAR, skin, |(width, height)| (star_x - width / 2.0, star_y - height / 2.0, width, height), WHITE, !shows_rare_star(classification))}
    }
}

/// `Interface\TargetingFrame\UI-RaidTargetingIcons`: a 4×4 sheet whose first two rows
/// hold Star, Circle, Diamond, Triangle, Moon, Square, Cross, Skull.
pub const RAID_TARGET_ICONS_FDID: u32 = 137_009;
/// Retail `RAID_TARGET_TEXTURE_ROWS` / `RAID_TARGET_TEXTURE_COLUMNS` (TargetFrame.lua:682-683).
const RAID_TARGET_TEXTURE_CELLS: u8 = 4;
const TARGET_RAID_TARGET_ICON_SIZE: f32 = 26.0;

/// `SetRaidTargetIconTexture` → `SetSpriteSheetCell(index, 4, 4)`: left, right, top,
/// bottom of raid target icon `index` (1–8).
pub fn raid_target_tex_coords(index: u8) -> [f32; 4] {
    let cell = index - 1;
    let cells = f32::from(RAID_TARGET_TEXTURE_CELLS);
    let column = f32::from(cell % RAID_TARGET_TEXTURE_CELLS);
    let row = f32::from(cell / RAID_TARGET_TEXTURE_CELLS);
    [
        column / cells,
        (column + 1.0) / cells,
        row / cells,
        (row + 1.0) / cells,
    ]
}

/// `TargetFrameMixin:UpdateRaidTargetIcon` (TargetFrame.lua:672-680).
fn raid_target_icon(raid_target: Option<u8>) -> Element {
    let [left, right, top, bottom] = raid_target_tex_coords(raid_target.unwrap_or(1));
    let coords = format!("{left},{right},{top},{bottom}");
    let (centre_x, centre_y) = TARGET_RAID_TARGET_ICON_CENTRE;
    let size = TARGET_RAID_TARGET_ICON_SIZE;
    let hidden = raid_target.is_none();
    rsx! {
        texture {
            name: {dyn_name("TargetRaidTargetIcon".into())},
            width: size,
            height: size,
            hidden,
            texture_fdid: RAID_TARGET_ICONS_FDID,
            tex_coords: {coords.as_str()},
            pos_type: "absolute",
            pos_x: {centre_x - size / 2.0},
            pos_y: {centre_y - size / 2.0},
        }
    }
}

/// Retail `MAX_BOSS_FRAMES`.
pub const MAX_BOSS_FRAMES: usize = 5;
/// `BossTargetFrameContainer` (Blizzard_UnitFrame/Mainline/TargetFrame.xml): a vertical
/// right-managed stack, `spacing` 10. Its right-side slot below the minimap is placed
/// at the reference resolution; the boss flair art is not drawn.
const BOSS_FRAME_RIGHT: f32 = 60.0;
const BOSS_FRAME_TOP: f32 = 300.0;
const BOSS_FRAME_SPACING: f32 = 10.0;

pub fn boss_frame_name(index: usize) -> String {
    format!("Boss{}TargetFrame", index + 1)
}

fn boss_frames(bosses: &[UnitFrameState], skin: ActiveSkin) -> Element {
    (0..MAX_BOSS_FRAMES)
        .flat_map(|index| boss_frame(index, bosses.get(index), skin))
        .collect()
}

fn boss_frame(index: usize, boss: Option<&UnitFrameState>, skin: ActiveSkin) -> Element {
    let name = boss_frame_name(index);
    let prefix = format!("Boss{}", index + 1);
    let content = boss
        .map(|state| {
            rsx! {
                {reaction_strip(&prefix, state.reaction, skin, portrait_off_strip(1.0))}
                {unit_frame_contents(&prefix, state, &PORTRAIT_OFF_SLOTS, HEALTH_BAR)}
                {dead_text(&prefix, PORTRAIT_OFF_SLOTS.health, UNIT_FONT_SIZE, state.dead)}
            }
        })
        .unwrap_or_default();
    let art = art_texture(
        dyn_name(format!("{name}Art")),
        FRAME_PORTRAIT_OFF,
        (0.0, 0.0, FRAME_W, FRAME_H),
        false,
    );
    let hidden = boss.is_none();
    let top = BOSS_FRAME_TOP + index as f32 * (FRAME_H + BOSS_FRAME_SPACING);
    rsx! {
        r#frame {
            name: {dyn_name(name)},
            width: FRAME_W,
            height: FRAME_H,
            hidden,
            mouse_enabled: true,
            pos_type: "absolute",
            right: BOSS_FRAME_RIGHT,
            top,
            {art}
            {content}
        }
    }
}

/// `scale` maps the authored 133×51 art slots onto smaller frames.
fn scaled((x, y, width, height): Rect, scale: f32) -> Rect {
    (x * scale, y * scale, width * scale, height * scale)
}

/// The reaction strip over the portrait-off name tab, at its atlas height. Authored 18px
/// tall: a 13px band that fades out above its transparent lower rows.
fn portrait_off_strip(scale: f32) -> impl FnOnce((f32, f32)) -> Rect {
    move |(_, height)| scaled((BAR_X, NAME_Y, BAR_W, height), scale)
}

fn reaction_strip(
    prefix: &str,
    reaction: Option<Reaction>,
    skin: ActiveSkin,
    place: impl FnOnce((f32, f32)) -> Rect,
) -> Element {
    let Some(reaction) = reaction else {
        return Element::default();
    };
    sized_atlas_texture(
        format!("{prefix}ReputationColor"),
        REACTION_STRIP,
        skin,
        place,
        reaction_color(reaction),
        false,
    )
}

fn unit_frame_contents(
    prefix: &str,
    state: &UnitFrameState,
    slots: &FrameSlots,
    health_art: &'static str,
) -> Element {
    let power_fraction = state.power.as_ref().map_or(0.0, |power| {
        fraction(power.current as f32, power.max as f32)
    });
    rsx! {
        {unit_label(dyn_name(format!("{prefix}Name")), &state.name, slots.name, (GOLD_TEXT, UNIT_FONT_SIZE), "LEFT")}
        {unit_label(dyn_name(format!("{prefix}LevelText")), &state.level_text, slots.level, (&state.level_color, UNIT_FONT_SIZE), slots.level_justify)}
        {status_bar(BarSpec {
            name: format!("{prefix}HealthBar"),
            rect: slots.health,
            fraction: state.health_fraction,
            art: Some(health_art),
            text: &state.health_text,
            anchors: slots.health_text,
            font_size: UNIT_FONT_SIZE,
            hidden: false,
        })}
        {status_bar(BarSpec {
            name: format!("{prefix}ManaBar"),
            rect: slots.power,
            fraction: power_fraction,
            art: state.power.as_ref().and_then(|power| power_bar_atlas(power.power)),
            text: &state.power_text,
            anchors: slots.power_text,
            font_size: UNIT_FONT_SIZE - 1.0,
            hidden: state.power.is_none(),
        })}
    }
}

/// Retail `VerticalLayoutMixin` places a centred child at the container top plus its
/// `topPadding`, shifted right by half its `leftPadding` (LayoutFrame.lua:340,346-348).
fn class_bar(view: Option<&ClassBarView>) -> Element {
    let Some(view) = view else {
        return Element::default();
    };
    let (width, height) = view.size;
    let (top_padding, left_padding) = view.padding;
    let textures: Element = view
        .textures
        .iter()
        .flat_map(|texture| class_bar_texture("", texture))
        .collect();
    rsx! {
        r#frame {
            name: "PlayerSecondaryResourceRow",
            width,
            height,
            pos_type: "absolute",
            pos_x: {CLASS_BAR_CENTRE_X + left_padding / 2.0 - width / 2.0},
            pos_y: {CLASS_BAR_TOP + top_padding},
            {textures}
        }
    }
}

/// One class bar texture at its alpha, named `prefix` + its name; hidden while not shown
/// or fully transparent. A Cooldown swipe is an empty frame the client fills with its
/// radial swipe.
fn class_bar_texture(prefix: &str, texture: &TextureView) -> Element {
    let name = dyn_name(format!("{prefix}{}", texture.name));
    if texture.swipe.is_some() {
        let (x, y, width, height) = texture.rect;
        let hidden = !texture.shown;
        return rsx! {
            r#frame {
                name,
                width,
                height,
                hidden,
                pos_type: "absolute",
                pos_x: x,
                pos_y: y,
            }
        };
    }
    let (x, y, width, height) = texture.rect;
    let coords = texture.art.tex_coords(1.0);
    let color = format!("1.0,1.0,1.0,{}", texture.alpha);
    let hidden = !texture.shown || texture.alpha <= 0.0;
    rsx! {
        texture {
            name,
            width,
            height,
            hidden,
            texture_fdid: {texture.art.fdid},
            tex_coords: {coords.as_str()},
            vertex_color: {color.as_str()},
            rotation: {texture.rotation},
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}

/// Retail `AttackIcon` and the rest flipbook's first cell beside the portrait.
fn status_icons(state: &UnitFrameState, skin: ActiveSkin) -> Element {
    let (combat_x, combat_y) = PLAYER_ATTACK_ICON;
    let (rest_x, rest_y, rest_w, rest_h) = PLAYER_REST_ICON;
    let rest_hidden = !state.show_resting_icon;
    rsx! {
        {sized_atlas_texture("PlayerCombatIcon".into(), COMBAT_ICON, skin, |(width, height)| (combat_x, combat_y, width, height), WHITE, !state.show_combat_icon)}
        texture {
            name: {dyn_name("PlayerRestingIcon".into())},
            width: rest_w,
            height: rest_h,
            hidden: rest_hidden,
            texture_atlas: REST_FLIPBOOK,
            tex_coords: REST_ICON_COORDS,
            pos_type: "absolute",
            pos_x: rest_x,
            pos_y: rest_y,
        }
    }
}

struct SmallFrameSpec {
    flare: &'static FlareFrame,
    root: &'static str,
    prefix: &'static str,
    /// Retail's focus frame is a `TargetFrameTemplate` with the reaction strip; target of
    /// target has none.
    reaction_strip: bool,
    /// `TargetFrameTemplate`'s `LevelText` and `ManaBar`, which FocusFrame keeps at its
    /// small size (`FocusFrameMixin:SetSmallSize`, TargetFrame.lua:1148-1176).
    level_and_power: bool,
}

impl SmallFrameSpec {
    const TARGET_OF_TARGET: Self = Self {
        flare: &FLARE_TARGET_OF_TARGET,
        root: "TargetOfTargetFrame",
        prefix: "TargetOfTarget",
        reaction_strip: false,
        level_and_power: false,
    };
    const FOCUS: Self = Self {
        flare: &FLARE_FOCUS,
        root: "FocusFrame",
        prefix: "Focus",
        reaction_strip: true,
        level_and_power: true,
    };
}

fn small_unit_frame(
    spec: SmallFrameSpec,
    state: Option<&SmallUnitFrameState>,
    anchor: &HudAnchor,
    skin: ActiveSkin,
) -> Element {
    if skin == ActiveSkin::Forever {
        return flare_frame(spec.flare, state.map(FlareUnit::from), false, anchor);
    }
    let content = state
        .map(|unit| small_unit_contents(&spec, unit, skin))
        .unwrap_or_default();
    art_root(
        dyn_name(spec.root.into()),
        (TOT_W, TOT_H),
        anchor,
        state.is_none(),
        art_texture(
            dyn_name(format!("{}Art", spec.root)),
            FRAME_PORTRAIT_OFF,
            (0.0, 0.0, TOT_W, TOT_H),
            false,
        ),
        Element::default(),
        content,
    )
}

fn small_unit_contents(
    spec: &SmallFrameSpec,
    unit: &SmallUnitFrameState,
    skin: ActiveSkin,
) -> Element {
    let scale = SMALL_ART_SCALE;
    let strip = unit.reaction.filter(|_| spec.reaction_strip);
    rsx! {
        {reaction_strip(spec.prefix, strip, skin, portrait_off_strip(scale))}
        {unit_label(dyn_name(format!("{}Name", spec.prefix)), &unit.name, scaled(PORTRAIT_OFF_SLOTS.name, scale), (GOLD_TEXT, UNIT_FONT_SIZE * scale), "LEFT")}
        {small_level_and_power(spec, unit, scale)}
        {status_bar(BarSpec {
            name: format!("{}HealthBar", spec.prefix),
            rect: scaled(PORTRAIT_OFF_SLOTS.health, scale),
            fraction: unit.health_fraction,
            art: Some(HEALTH_BAR),
            text: &StatusBarText::default(),
            anchors: PORTRAIT_OFF_SLOTS.health_text,
            font_size: UNIT_FONT_SIZE * scale,
            hidden: false,
        })}
        {dead_text(spec.prefix, scaled(PORTRAIT_OFF_SLOTS.health, scale), UNIT_FONT_SIZE * scale, unit.dead)}
    }
}

/// `{prefix}DeadText`: `DEAD` in `GameFontNormalSmall`, centred on the health bar, while
/// the unit is dead (TargetFrame.xml:182-186, 441-445). The player frame has none.
fn dead_text(prefix: &str, rect: Rect, font_size: f32, dead: bool) -> Element {
    if !dead {
        return Element::default();
    }
    unit_label(
        dyn_name(format!("{prefix}DeadText")),
        "Dead",
        rect,
        (GOLD_TEXT, font_size),
        "CENTER",
    )
}

/// FocusFrame's level text and mana bar on the portrait-off art, scaled like the rest.
fn small_level_and_power(spec: &SmallFrameSpec, unit: &SmallUnitFrameState, scale: f32) -> Element {
    if !spec.level_and_power {
        return Element::default();
    }
    let (level, color) = unit
        .level
        .as_ref()
        .map_or(("", GOLD_TEXT), |(text, color)| {
            (text.as_str(), color.as_str())
        });
    let power = unit.power.as_ref();
    rsx! {
        {unit_label(dyn_name(format!("{}LevelText", spec.prefix)), level, scaled(PORTRAIT_OFF_SLOTS.level, scale), (color, UNIT_FONT_SIZE * scale), PORTRAIT_OFF_SLOTS.level_justify)}
        {status_bar(BarSpec {
            name: format!("{}ManaBar", spec.prefix),
            rect: scaled(PORTRAIT_OFF_SLOTS.power, scale),
            fraction: power.map_or(0.0, |power| fraction(power.current as f32, power.max as f32)),
            art: power.and_then(|power| power_bar_atlas(power.power)),
            text: &StatusBarText::default(),
            anchors: PORTRAIT_OFF_SLOTS.power_text,
            font_size: (UNIT_FONT_SIZE - 1.0) * scale,
            hidden: power.is_none(),
        })}
    }
}

fn unit_frame_menu(state: &UnitFrameMenuState) -> Element {
    let mut items: Vec<ContextMenuItem<'_>> = state
        .player_items
        .iter()
        .map(|item| ContextMenuItem {
            name: &item.name,
            label: &item.label,
            action: &item.action,
        })
        .collect();
    items.push(UNIT_MENU_CLOSE);
    context_menu(ContextMenu {
        frame_name: "UnitFrameContextMenu",
        title_name: "UnitFrameContextMenuTitle",
        divider_name: "UnitFrameContextMenuDivider",
        hidden: !state.visible,
        title: state.title.as_str(),
        width: UNIT_MENU_W,
        x: state.x,
        y: state.y,
        items: &items,
    })
}

/// The rows the Dungeon Difficulty submenu always has (Normal, Heroic, Mythic), so they
/// exist, laid out, before it first opens.
const DIFFICULTY_MENU_ROWS: [u32; 3] = [1, 2, 23];

/// The Dungeon Difficulty submenu (`DUNGEON_DIFFICULTY`): a radio row per difficulty,
/// grey while disabled (its click is ignored, `unit_frames::UnitFrameClick`).
fn difficulty_menu(state: Option<&DifficultyMenuState>) -> Element {
    let hidden = state.is_none();
    let (x, y) = state.map_or((0.0, 0.0), |menu| (menu.x, menu.y));
    let height = difficulty_menu_height(DIFFICULTY_MENU_ROWS.len());
    let rows: Element = DIFFICULTY_MENU_ROWS
        .iter()
        .enumerate()
        .flat_map(|(index, &difficulty_id)| {
            let entry = state
                .and_then(|menu| {
                    menu.entries
                        .iter()
                        .find(|entry| entry.difficulty_id == difficulty_id)
                })
                .cloned()
                .unwrap_or(DifficultyMenuEntry {
                    difficulty_id,
                    label: String::new(),
                    checked: false,
                    enabled: false,
                });
            difficulty_row(index, &entry)
        })
        .collect();
    rsx! {
        r#frame {
            name: "UnitFrameDifficultyMenu",
            width: {DIFFICULTY_MENU_W},
            height: {height},
            hidden: hidden,
            strata: FrameStrata::Dialog,
            frame_level: 61.0,
            background_color: "0.03,0.03,0.03,0.96",
            pos_type: "absolute",
            left: {x},
            top: {y},
            fontstring {
                name: "UnitFrameDifficultyMenuTitle",
                width: {DIFFICULTY_MENU_W - 12.0},
                height: 14.0,
                text: "Dungeon Difficulty",
                font_size: 10.0,
                font_color: "1.0,0.82,0.0,1.0",
                justify_h: "LEFT",
                pos_type: "absolute",
                left: 6.0,
                top: 6.0,
            }
            {rows}
        }
    }
}

fn difficulty_row(index: usize, entry: &DifficultyMenuEntry) -> Element {
    let row_name = dyn_name(format!("UnitFrameDifficultyMenu{}", entry.difficulty_id));
    let radio_name = dyn_name(format!(
        "UnitFrameDifficultyMenu{}Radio",
        entry.difficulty_id
    ));
    let text_name = dyn_name(format!(
        "UnitFrameDifficultyMenu{}Text",
        entry.difficulty_id
    ));
    let top = DIFFICULTY_MENU_TOP + index as f32 * DIFFICULTY_ROW_H;
    let coords = if entry.checked {
        RADIO_CHECKED_COORDS
    } else {
        RADIO_EMPTY_COORDS
    };
    let color = if entry.enabled {
        MENU_TEXT_ENABLED
    } else {
        MENU_TEXT_DISABLED
    };
    let action = format!(
        "{ACTION_UNIT_MENU_SET_DUNGEON_DIFFICULTY_PREFIX}{}",
        entry.difficulty_id
    );
    rsx! {
        r#frame {
            name: row_name,
            width: {DIFFICULTY_MENU_W - 12.0},
            height: {DIFFICULTY_ROW_H},
            onclick: action.as_str(),
            pos_type: "absolute",
            left: 6.0,
            top: {top},
            texture {
                name: radio_name,
                width: 18.0,
                height: 18.0,
                texture_fdid: RADIO_SHEET_FDID,
                tex_coords: coords,
                pos_type: "absolute",
                left: 0.0,
                top: 1.0,
            }
            fontstring {
                name: text_name,
                width: {DIFFICULTY_MENU_W - 32.0},
                height: 20.0,
                text: entry.label.as_str(),
                font_size: 10.0,
                font_color: color,
                justify_h: "LEFT",
                pos_type: "absolute",
                left: 19.0,
                top: 0.0,
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/class_bars_tests.rs"]
mod class_bars_tests;
