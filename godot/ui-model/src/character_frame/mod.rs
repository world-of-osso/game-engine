//! Retail `CharacterFrame` with its `PaperDollFrame` (Blizzard_UIPanels_Game/Mainline/
//! CharacterFrame.xml / .lua cited as CF.xml / CF.lua, PaperDollFrame.xml / .lua as
//! PDF.xml / PDF.lua): the expanded 540×424 `ButtonFrameTemplate` window with the 18
//! paperdoll item buttons, the character model scene, the level line, the
//! `CharacterStatsPane` and the Character / Reputation tabs; the Reputation tab swaps the
//! PaperDollFrame for the `ReputationFrame` ([`reputation`]). Positions are top-left
//! offsets in frame space converted from the XML anchors. docs/specs/character-frame.md.

mod art;
mod reputation;
mod reputation_catalog;
mod reputation_detail;

pub use reputation_catalog::enrich_reputation_rows;
pub use reputation_detail::{
    ACTION_REPUTATION_DETAIL_CLOSE, REPUTATION_DESCRIPTION_SCROLL,
    reputation_description_pan_extent, reputation_selection,
};

use game_engine_core::spell_catalog::PrimaryStat;
use shared::components::{CombatRatings, DerivedStats, UnitStats};
use shared::protocol::{EquipmentSlot, ItemLocation};
use ui_toolkit::atlas::{ActiveSkin, AtlasSource, active_skin, resolve_region};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::text_measure::measure_text;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;
use ui_toolkit::widgets::texture::TextureSource;

use art::{FULL, WHITE, WHITE_ICON_FRAME, atlas, texture};
pub use art::{class_background, race_background, race_overlay_alpha};
pub use reputation::{
    REPUTATION_SCROLL, ReputationRow, reputation_art_fdids, reputation_pan_extent,
    reputation_row_action, reputation_rows,
};

use crate::bag_data::InventoryState;
pub use crate::character_frame_component::{equipment_slot_action, parse_equipment_slot_action};
use crate::merchant_frame_component::{tab, tab_width};
use crate::quest_art::{
    DynName, HIGHLIGHT_FONT_COLOR, NORMAL_FONT_COLOR, portrait_border, window_chrome,
    window_portrait, window_portrait_slot,
};
use crate::ui::strata::FrameStrata;

pub const FRAME_NAME: &str = "CharacterFrame";
/// Player head rendered by the native unit-portrait host in either skin.
pub const PORTRAIT: crate::inworld_unit_frames_component::PortraitSlot =
    window_portrait_slot("CharacterFramePortrait");
/// `CHARACTERFRAME_EXPANDED_WIDTH` (CF.lua:2): PaperDollFrame_OnShow expands the frame.
pub const FRAME_W: f32 = 540.0;
/// `PANEL_DEFAULT_WIDTH` (Constants.lua:299): the PaperDollFrame inset keeps this width.
const PANEL_DEFAULT_WIDTH: f32 = 338.0;
/// `ButtonFrameBaseTemplate` height.
pub const FRAME_H: f32 = 424.0;
/// The Retail left UI panel slot, as the native MerchantFrame uses it.
pub const PANEL_LEFT: f32 = 16.0;
pub const PANEL_TOP: f32 = 104.0;

pub const ACTION_CLOSE: &str = "character_close";
pub const ACTION_TAB_CHARACTER: &str = "character_tab:character";
pub const ACTION_TAB_REPUTATION: &str = "character_tab:reputation";
/// The model scene: left-drag rotates it (`PanningModelSceneMixin`).
pub const ACTION_MODEL: &str = "character_model";
/// The PaperDollFrame outside its buttons (`enableMouse`): it blocks the world.
pub const ACTION_FRAME: &str = "character_frame";
/// Frame the host draws the model preview into.
pub const MODEL_SCENE: &str = "CharacterModelScene";

/// `Inset` TOPLEFT 4,-60 and, for the PaperDollFrame (CF.lua:132), BOTTOMRIGHT at
/// BOTTOMLEFT `PANEL_DEFAULT_WIDTH + PANEL_INSET_RIGHT_OFFSET`, `PANEL_INSET_BOTTOM_OFFSET`.
const INSET: (f32, f32, f32, f32) = (4.0, 60.0, PANEL_DEFAULT_WIDTH - 6.0, FRAME_H - 4.0);
/// `InsetRight` TOPLEFT Inset TOPRIGHT +1 / BOTTOMRIGHT -4,4 (CF.xml).
const INSET_RIGHT: (f32, f32, f32, f32) = (INSET.2 + 1.0, INSET.1, FRAME_W - 4.0, FRAME_H - 4.0);
/// `CharacterStatsPane` InsetRight +3,-3 / -3,2.
const STATS: (f32, f32) = (INSET_RIGHT.0 + 3.0, INSET_RIGHT.1 + 3.0);
const STATS_W: f32 = INSET_RIGHT.2 - 3.0 - STATS.0;

/// Intrinsic `ItemButton` size.
const SLOT: f32 = 37.0;
/// Slot buttons stack `BOTTOMLEFT 0,-4` below each other (PDF.xml:820-897).
const SLOT_STEP: f32 = SLOT + 4.0;
/// `CharacterHeadSlot` TOPLEFT Inset 4,-2; `CharacterHandsSlot` TOPRIGHT Inset -4,-2.
const LEFT_COLUMN: f32 = INSET.0 + 4.0;
const RIGHT_COLUMN: f32 = INSET.2 - 4.0 - SLOT;
const COLUMN_TOP: f32 = INSET.1 + 2.0;
/// `CharacterMainHandSlot` BOTTOMLEFT 130,16; the off hand TOPRIGHT +5.
const MAIN_HAND: (f32, f32) = (130.0, FRAME_H - 16.0 - SLOT);
const OFF_HAND_X: f32 = MAIN_HAND.0 + SLOT + 5.0;

/// `CharacterModelScene` 231×320 at TOPLEFT 52,-66.
const MODEL: (f32, f32, f32, f32) = (52.0, 66.0, 231.0, 320.0);

#[derive(Clone, Copy)]
struct CharacterLayout {
    size: (f32, f32),
    model: (f32, f32, f32, f32),
    stats: (f32, f32, f32),
    columns: (f32, f32, f32, f32),
    main_hand: (f32, f32),
    off_hand_x: f32,
}

fn read_character_layout() -> CharacterLayout {
    character_layout(active_skin())
}

/// Camelot CharacterFrameConstants.lua:4-5; CharacterFrame.xml:408-482;
/// PaperDollFrame.xml:607-609,773-850; PaperDollFrame.lua:1880 (existing two-hand row).
fn character_layout(skin: ActiveSkin) -> CharacterLayout {
    match skin {
        ActiveSkin::Modern => CharacterLayout {
            size: (FRAME_W, FRAME_H),
            model: MODEL,
            stats: (STATS.0, STATS.1, STATS_W),
            columns: (LEFT_COLUMN, RIGHT_COLUMN, COLUMN_TOP, SLOT_STEP),
            main_hand: MAIN_HAND,
            off_hand_x: OFF_HAND_X,
        },
        ActiveSkin::Forever => CharacterLayout {
            size: (631.0, 484.0),
            model: (0.0, 20.0, 398.0, 464.0),
            stats: (398.0, 105.0, 233.0),
            columns: (20.0, 398.0 - 20.0 - SLOT, 80.0, SLOT + 6.0),
            main_hand: (398.0 / 2.0 - 40.0 - SLOT / 2.0, 484.0 - 30.0 - SLOT),
            off_hand_x: 398.0 / 2.0 - 40.0 + SLOT / 2.0 + 6.0,
        },
    }
}

/// One paperdoll button: its Retail name, `<SLOT>SLOT` label, empty texture and column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaperDollButton {
    pub slot: EquipmentSlot,
    /// Registry name, as Retail names the buttons (`CharacterHeadSlot`).
    pub name: &'static str,
    /// `<SLOT>SLOT` GlobalString: the tooltip of an empty slot.
    pub label: &'static str,
    /// `Interface\PaperDoll\UI-PaperDoll-Slot-*` (`GetInventorySlotInfo`).
    pub empty_texture: u32,
    pub column: Column,
    /// Row in its column (0 top), or 0/1 for the weapon pair.
    pub row: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Column {
    Left,
    Right,
    Bottom,
}

const fn button(
    slot: EquipmentSlot,
    name: &'static str,
    label: &'static str,
    empty_texture: u32,
    (column, row): (Column, u8),
) -> PaperDollButton {
    PaperDollButton {
        slot,
        name,
        label,
        empty_texture,
        column,
        row,
    }
}

/// PDF.xml:820-930. Retail has no ranged slot button since Mists of Pandaria.
pub const PAPERDOLL_BUTTONS: [PaperDollButton; 18] = {
    use Column::*;
    use EquipmentSlot as S;
    [
        button(S::Head, "CharacterHeadSlot", "Head", 136_516, (Left, 0)),
        button(S::Neck, "CharacterNeckSlot", "Neck", 136_519, (Left, 1)),
        button(
            S::Shoulder,
            "CharacterShoulderSlot",
            "Shoulders",
            136_526,
            (Left, 2),
        ),
        // The back slot draws UI-PaperDoll-Slot-Chest.
        button(S::Back, "CharacterBackSlot", "Back", 136_512, (Left, 3)),
        button(S::Chest, "CharacterChestSlot", "Chest", 136_512, (Left, 4)),
        button(S::Shirt, "CharacterShirtSlot", "Shirt", 136_525, (Left, 5)),
        button(
            S::Tabard,
            "CharacterTabardSlot",
            "Tabard",
            136_527,
            (Left, 6),
        ),
        button(S::Wrist, "CharacterWristSlot", "Wrist", 136_530, (Left, 7)),
        button(S::Hands, "CharacterHandsSlot", "Hands", 136_515, (Right, 0)),
        button(S::Waist, "CharacterWaistSlot", "Waist", 136_529, (Right, 1)),
        button(S::Legs, "CharacterLegsSlot", "Legs", 136_517, (Right, 2)),
        button(S::Feet, "CharacterFeetSlot", "Feet", 136_513, (Right, 3)),
        button(
            S::Finger1,
            "CharacterFinger0Slot",
            "Finger",
            136_514,
            (Right, 4),
        ),
        button(
            S::Finger2,
            "CharacterFinger1Slot",
            "Finger",
            136_514,
            (Right, 5),
        ),
        button(
            S::Trinket1,
            "CharacterTrinket0Slot",
            "Trinket",
            136_528,
            (Right, 6),
        ),
        button(
            S::Trinket2,
            "CharacterTrinket1Slot",
            "Trinket",
            136_528,
            (Right, 7),
        ),
        button(
            S::MainHand,
            "CharacterMainHandSlot",
            "Main Hand",
            136_518,
            (Bottom, 0),
        ),
        button(
            S::OffHand,
            "CharacterSecondaryHandSlot",
            "Off Hand",
            136_524,
            (Bottom, 1),
        ),
    ]
};

pub fn paperdoll_button(slot: EquipmentSlot) -> Option<&'static PaperDollButton> {
    PAPERDOLL_BUTTONS.iter().find(|button| button.slot == slot)
}

impl PaperDollButton {
    /// Top-left of the button in CharacterFrame space.
    pub fn origin(&self) -> (f32, f32) {
        let row = f32::from(self.row);
        let layout = read_character_layout();
        let (left, right, top, step) = layout.columns;
        match self.column {
            Column::Left => (left, top + row * step),
            Column::Right => (right, top + row * step),
            Column::Bottom if self.row == 0 => layout.main_hand,
            Column::Bottom => (layout.off_hand_x, layout.main_hand.1),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct PaperDollSlotView {
    /// Equipped item icon; 0 shows the empty-slot texture.
    pub icon_fdid: u32,
    /// `BAG_ITEM_QUALITY_COLORS` `WhiteIconFrame` tint; empty for none.
    pub quality_border: &'static str,
    pub count: u32,
    /// On the cursor: the icon is dimmed.
    pub locked: bool,
}

/// `CharacterLevelText`: `PLAYER_LEVEL` "Level %s |c%s%s %s|r", the spec and class in the
/// class colour (`PLAYER_LEVEL_NO_SPEC` "Level %s |c%s%s|r" without a spec).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct LevelLine {
    pub level: String,
    pub class_text: String,
    pub class_color: String,
}

pub fn level_line(level: u8, spec: Option<&str>, class: &str, rgb: [f32; 3]) -> LevelLine {
    let class_text = match spec.filter(|spec| !spec.is_empty()) {
        Some(spec) => format!("{spec} {class}"),
        None => class.to_owned(),
    };
    LevelLine {
        level: format!("Level {level} "),
        class_text,
        class_color: format!("{},{},{},1.0", rgb[0], rgb[1], rgb[2]),
    }
}

/// The CharacterFrame subframe its tabs select (`CHARACTERFRAME_SUBFRAMES`,
/// Mainline/CharacterFrame.lua:1; `CharacterFrameTabButtonMixin:OnClick`, :394-406).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CharacterTab {
    /// `PaperDollFrame`, `CharacterFrameTab1`.
    #[default]
    PaperDoll,
    /// `ReputationFrame`, `CharacterFrameTab2`.
    Reputation,
}

impl CharacterTab {
    /// The tab a `character_tab:*` click selects.
    pub fn from_action(action: &str) -> Option<Self> {
        match action {
            ACTION_TAB_CHARACTER => Some(Self::PaperDoll),
            ACTION_TAB_REPUTATION => Some(Self::Reputation),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct CharacterFrameView {
    pub visible: bool,
    pub tab: CharacterTab,
    /// The ReputationFrame entries; shown only on [`CharacterTab::Reputation`].
    pub reputation: Vec<ReputationRow>,
    /// Selected faction ID, not its position in a mutable snapshot.
    pub selected_reputation: Option<u32>,
    /// `UnitPVPName("player")`.
    pub title: String,
    pub level: LevelLine,
    /// One per [`PAPERDOLL_BUTTONS`] entry, in that order.
    pub slots: Vec<PaperDollSlotView>,
    /// `STAT_AVERAGE_ITEM_LEVEL` value; `None` below [`MIN_LEVEL_FOR_ITEM_LEVEL`].
    pub item_level: Option<String>,
    /// `AttributesCategory` lines; empty hides the category.
    pub attributes: Vec<StatLine>,
    /// `EnhancementsCategory` lines; empty hides the category.
    pub enhancements: Vec<StatLine>,
    pub race_id: u8,
    pub class_id: u8,
}

/// One `CharacterStatFrameTemplate`: `STAT_FORMAT` label and its value text.
#[derive(Clone, Debug, PartialEq)]
pub struct StatLine {
    pub label: &'static str,
    pub value: String,
}

/// `PAPERDOLL_STATCATEGORIES` Attributes from the replicated sheet stats (PDF.lua:246-257):
/// Strength, Agility and Intellect, of which a specialization shows only its primary stat
/// (`stat.primary ~= primaryStat`, PDF.lua:1940-1945; all three without one), Stamina
/// (`UnitStat`, an integer), and Armor (`UnitArmor`). Stagger and mana regen need a
/// role, which the client does not know. `None` before the stats arrive.
pub fn attribute_lines(
    stats: Option<(&UnitStats, &CombatRatings)>,
    spec_primary: Option<PrimaryStat>,
) -> Vec<StatLine> {
    let Some((stats, ratings)) = stats else {
        return Vec::new();
    };
    let shown = |stat| spec_primary.is_none_or(|primary| primary == stat);
    [
        ("Strength:", stats.strength, shown(PrimaryStat::Strength)),
        ("Agility:", stats.agility, shown(PrimaryStat::Agility)),
        ("Intellect:", stats.intellect, shown(PrimaryStat::Intellect)),
        ("Stamina:", stats.stamina, true),
        ("Armor:", ratings.armor, true),
    ]
    .into_iter()
    .filter(|&(_, _, shown)| shown)
    .map(|(label, value, _)| StatLine {
        label,
        value: break_up_large_numbers(value.trunc() as i64),
    })
    .collect()
}

/// `PAPERDOLL_STATCATEGORIES` Enhancements (PDF.lua:259-273) from the replicated
/// `DerivedStats`: Critical Strike (`PaperDollFrame_SetCritChance`), Haste
/// (`PaperDollFrame_SetHaste`), Mastery (`GetMasteryEffect`), Versatility (its damage
/// done bonus), Leech, Avoidance and Speed (`GetLifesteal`, `GetAvoidance`, `GetSpeed`,
/// PDF.lua:1229-1275), each `format("%d%%", value + 0.5)` (`PaperDollFrame_SetLabelAndText`,
/// PDF.lua:1993-2002) and hidden at exactly 0 (`hideAt = 0`). `None` before the stats arrive.
pub fn enhancement_lines(derived: Option<&DerivedStats>) -> Vec<StatLine> {
    let Some(derived) = derived else {
        return Vec::new();
    };
    [
        ("Critical Strike:", derived.crit_pct),
        ("Haste:", derived.haste_pct),
        ("Mastery:", derived.mastery_pct),
        ("Versatility:", derived.versatility_pct),
        ("Leech:", derived.leech_pct),
        ("Avoidance:", derived.avoidance_pct),
        ("Speed:", derived.speed_pct),
    ]
    .into_iter()
    .filter(|&(_, value)| value != 0.0)
    .map(|(label, value)| StatLine {
        label,
        value: format!("{}%", (value + 0.5) as i64),
    })
    .collect()
}

/// `BreakUpLargeNumbers`: thousands separated by `LARGE_NUMBER_SEPERATOR` ",".
pub fn break_up_large_numbers(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut grouped = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    if value < 0 {
        grouped.insert(0, '-');
    }
    grouped
}

/// `MIN_PLAYER_LEVEL_FOR_ITEM_LEVEL_DISPLAY` (PDF.lua:77).
pub const MIN_LEVEL_FOR_ITEM_LEVEL: u8 = 10;

/// The paperdoll's slot states from the authoritative equipment.
pub fn paperdoll_slots(
    inventory: &InventoryState,
    cursor_source: Option<ItemLocation>,
) -> Vec<PaperDollSlotView> {
    PAPERDOLL_BUTTONS
        .iter()
        .map(|button| {
            let Some(item) = inventory.item_at(ItemLocation::Equipment(button.slot)) else {
                return PaperDollSlotView::default();
            };
            PaperDollSlotView {
                icon_fdid: item.icon_fdid,
                quality_border: item.quality.border_color(),
                count: item.count,
                locked: cursor_source == Some(ItemLocation::Equipment(button.slot)),
            }
        })
        .collect()
}

/// The 16 slots `GetAverageItemLevel` counts: every slot but shirt and tabard.
const ITEM_LEVEL_SLOTS: [EquipmentSlot; 16] = {
    use EquipmentSlot as S;
    [
        S::Head,
        S::Neck,
        S::Shoulder,
        S::Back,
        S::Chest,
        S::Wrist,
        S::Hands,
        S::Waist,
        S::Legs,
        S::Feet,
        S::Finger1,
        S::Finger2,
        S::Trinket1,
        S::Trinket2,
        S::MainHand,
        S::OffHand,
    ]
};

/// Inventory types that hold both hands: Two-Hand, Ranged, Ranged (right).
const TWO_HANDED: [u8; 3] = [17, 15, 26];

/// `GetAverageItemLevel` "equipped": item levels of [`ITEM_LEVEL_SLOTS`] over 16, a
/// two-handed main hand also counting for the empty off hand. `item` gives
/// `(item_level, inventory_type)` of an item id.
pub fn average_equipped_item_level(
    inventory: &InventoryState,
    item: impl Fn(u32) -> Option<(u16, u8)>,
) -> f32 {
    let level = |slot| {
        inventory
            .item_at(ItemLocation::Equipment(slot))
            .and_then(|slot| item(slot.item_id))
    };
    let mut total: u32 = ITEM_LEVEL_SLOTS
        .iter()
        .filter_map(|&slot| level(slot))
        .map(|(item_level, _)| u32::from(item_level))
        .sum();
    if level(EquipmentSlot::OffHand).is_none()
        && let Some((item_level, inventory_type)) = level(EquipmentSlot::MainHand)
        && TWO_HANDED.contains(&inventory_type)
    {
        total += u32::from(item_level);
    }
    total as f32 / ITEM_LEVEL_SLOTS.len() as f32
}

pub fn character_frame_screen(ctx: &SharedContext) -> Element {
    let view = ctx
        .get::<CharacterFrameView>()
        .expect("CharacterFrameView must be in SharedContext");
    let hide = !view.visible;
    let paperdoll = view.tab == CharacterTab::PaperDoll;
    let (width, height) = match paperdoll {
        true => read_character_layout().size,
        false => reputation::frame_size(read_character_layout().size),
    };
    // `UpdateTitle` (CharacterFrame.lua:117-121): the player's name or `REPUTATION`.
    let title = if paperdoll {
        &view.title
    } else {
        reputation::TITLE
    };
    let mut children = match active_skin() {
        ActiveSkin::Modern => window_chrome(FRAME_NAME, (width, height), title, ACTION_CLOSE),
        // Camelot CharacterFrame.xml:403 uses PortraitFrameBaseTemplate, not the rock fill.
        ActiveSkin::Forever => portrait_border(FRAME_NAME, (width, height), title, ACTION_CLOSE),
    };
    children.extend(window_portrait(&PORTRAIT));
    if paperdoll {
        children.extend(paperdoll_frame(view));
    } else {
        children.extend(reputation::backgrounds());
        children.extend(reputation::entries(
            &view.reputation,
            ctx.scroll_first_row(reputation::REPUTATION_SCROLL),
        ));
        let selected = view
            .reputation
            .iter()
            .find(|row| Some(row.faction_id) == view.selected_reputation);
        children.extend(reputation_detail::detail(selected, ctx));
    }
    children.extend(tabs(view.tab));
    rsx! {
        r#frame {
            name: {DynName(FRAME_NAME.into())},
            width,
            height,
            strata: FrameStrata::Medium,
            hidden: hide,
            mouse_enabled: true,
            onclick: ACTION_FRAME,
            pos_type: "absolute",
            left: PANEL_LEFT,
            top: PANEL_TOP,
            {children}
        }
    }
}

/// The `PaperDollFrame`: pane backgrounds, race backdrop, model scene, level line, stats
/// pane and the paperdoll item buttons.
fn paperdoll_frame(view: &CharacterFrameView) -> Element {
    let mut children = art::inset_backgrounds(view.class_id);
    children.extend(art::race_backdrop(view.race_id));
    children.extend(art::inner_border());
    children.extend(model_scene_frame());
    children.extend(level_text(&view.level));
    children.extend(stats_pane(
        view.item_level.as_deref(),
        &view.attributes,
        &view.enhancements,
    ));
    children.extend(slots(&view.slots));
    children
}

/// Styles RSX attributes do not express: the selected tab's art, the paper doll's white
/// title (`characterFrameDisplayInfo.Default.titleColor`; Reputation keeps
/// `NORMAL_FONT_COLOR`) and the desaturated race backdrop (`PaperDollBgDesaturate(true)`).
pub fn apply_character_frame_postsetup(registry: &mut FrameRegistry, tab: CharacterTab) {
    apply_named_chrome(registry, tab);
    if tab != CharacterTab::PaperDoll {
        return;
    }
    let title = format!("{FRAME_NAME}TitleText");
    if let Some(WidgetData::FontString(text)) = widget_mut(registry, &title) {
        text.color = [1.0, 1.0, 1.0, 1.0];
    }
    for quarter in art::MODEL_BACKGROUND_NAMES {
        if let Some(WidgetData::Texture(texture)) = widget_mut(registry, quarter) {
            texture.desaturated = true;
        }
    }
}

/// Name the art emitted by shared helpers without changing other windows.
fn apply_named_chrome(registry: &mut FrameRegistry, tab: CharacterTab) {
    apply_named_texture(
        registry,
        "CharacterFrameCloseButtonNormal",
        "RedButton-Exit",
    );
    if active_skin() == ActiveSkin::Modern {
        apply_named_texture(
            registry,
            "CharacterFrameTopTileStreaks",
            "_UI-Frame-TopTileStreaks",
        );
        for (index, tab_of) in [(1, CharacterTab::PaperDoll), (2, CharacterTab::Reputation)] {
            let prefix = if tab_of == tab {
                "uiframe-activetab"
            } else {
                "uiframe-tab"
            };
            for (part, suffix) in [("Left", "left"), ("Middle", "center"), ("Right", "right")] {
                let tile = if part == "Middle" { "_" } else { "" };
                apply_named_texture(
                    registry,
                    &format!("CharacterFrameTab{index}{part}"),
                    &format!("{tile}{prefix}-{suffix}"),
                );
            }
        }
    }
}

fn apply_named_texture(registry: &mut FrameRegistry, node: &str, atlas: &str) {
    let region = resolve_region(atlas, active_skin())
        .unwrap_or_else(|| panic!("CharacterFrame atlas missing: {atlas}"));
    let AtlasSource::FileDataId(fdid) = region.source else {
        panic!("CharacterFrame chrome is not a DB2 sheet: {atlas}");
    };
    let Some(WidgetData::Texture(texture)) = widget_mut(registry, node) else {
        panic!("CharacterFrame texture missing: {node}");
    };
    texture.source = TextureSource::FileDataId(fdid);
    texture.tex_coords = [region.left, region.right, region.top, region.bottom];
}

fn widget_mut<'a>(registry: &'a mut FrameRegistry, name: &str) -> Option<&'a mut WidgetData> {
    let id = registry.get_by_name(name)?;
    registry.get_mut(id)?.widget_data.as_mut()
}

fn text(
    name: String,
    value: &str,
    rect: (f32, f32, f32, f32),
    (size, color): (f32, &str),
    justify: &str,
) -> Element {
    let (x, y, width, height) = rect;
    rsx! {
        fontstring {
            name: {DynName(name)},
            width,
            height,
            text: value,
            font: GameFont::FrizQuadrata,
            font_size: size,
            font_color: color,
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h: justify,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

/// The scene frame the host renders the model into; it takes the rotate drag.
fn model_scene_frame() -> Element {
    let (x, y, width, height) = read_character_layout().model;
    rsx! {
        r#frame {
            name: {DynName(MODEL_SCENE.into())},
            width,
            height,
            mouse_enabled: true,
            onclick: ACTION_MODEL,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

/// `CharacterLevelText` 220×24 CENTER at PaperDollFrame TOP 0,-42 (PDF.lua:476),
/// `GameFontNormalSmall2`; the class part in its class colour.
fn level_text(line: &LevelLine) -> Element {
    let (size, center, top) = match active_skin() {
        ActiveSkin::Modern => (12.0, FRAME_W / 2.0, 42.0 - 12.0),
        // PaperDollFrame.xml:431,455,467: level line below the right-pane sidebar tabs.
        ActiveSkin::Forever => (16.0, 398.0 + 233.0 / 2.0, 54.0),
    };
    let measure = |text: &str| {
        measure_text(text, GameFont::FrizQuadrata, size).map_or(0.0, |(width, _)| width)
    };
    let (level_w, class_w) = (measure(&line.level), measure(&line.class_text));
    let start = center - (level_w + class_w) / 2.0;
    let (line_h, mut children) = match active_skin() {
        ActiveSkin::Modern => (24.0, Element::default()),
        ActiveSkin::Forever => {
            let art = art::resolve_art("UI-Character-Info-ItemLevel-Bounce");
            let width = art.size().0;
            (
                20.0,
                atlas(
                    "CharacterLevelTextBackground".into(),
                    &art,
                    (center - width / 2.0, top, width, 20.0),
                    WHITE,
                ),
            )
        }
    };
    children.extend(text(
        "CharacterLevelText".into(),
        &line.level,
        (start, top, level_w + 1.0, line_h),
        (size, NORMAL_FONT_COLOR),
        "LEFT",
    ));
    children.extend(text(
        "CharacterLevelTextClass".into(),
        &line.class_text,
        (start + level_w, top, class_w + 1.0, line_h),
        (size, &line.class_color),
        "LEFT",
    ));
    children
}

/// `CharacterStatsPane` (`PaperDollFrame_UpdateStats`, PDF.lua:1912): from level 10
/// `ItemLevelCategory` (TOP 0,-2) and `ItemLevelFrame` below it, then the
/// `AttributesCategory` under the item level frame, or at TOP 0,-2 with -5 between stats
/// below level 10. The Enhancements category follows the last shown stat
/// (`catFrame:SetPoint("TOP", lastAnchor, "BOTTOM", 0, categoryYOffset)`, -11 below level
/// 10). A category hides without a stat (`catFrame:SetShown(numStatInCat > 0)`,
/// PDF.lua:1985). Stat lines number on across categories, as Retail's shared frame pool.
fn stats_pane(
    item_level: Option<&str>,
    attributes: &[StatLine],
    enhancements: &[StatLine],
) -> Element {
    let (stats_x, stats_y, stats_w) = read_character_layout().stats;
    let y = stats_y + 2.0;
    let mut children = Element::default();
    let (mut category_y, stat_gap, category_gap) = match item_level {
        Some(item_level) => {
            children.extend(item_level_frames(item_level, y));
            (y + 40.0 + 29.0, 0.0, 0.0)
        }
        None => (y, 5.0, 11.0),
    };
    let mut first_stat = 1;
    for (name, title, lines) in [
        (
            "CharacterStatsPaneAttributesCategory",
            "Attributes",
            attributes,
        ),
        (
            "CharacterStatsPaneEnhancementsCategory",
            "Enhancements",
            enhancements,
        ),
    ] {
        if lines.is_empty() {
            continue;
        }
        children.extend(category(
            name,
            title,
            stats_x + (stats_w - 197.0) / 2.0,
            category_y,
        ));
        let top = category_y + 40.0 + 2.0;
        children.extend(stat_lines(lines, first_stat, top, stat_gap));
        first_stat += lines.len();
        let count = lines.len() as f32;
        category_y = top + count * 15.0 + (count - 1.0) * stat_gap + category_gap;
    }
    children
}

fn item_level_frames(item_level: &str, y: f32) -> Element {
    let (stats_x, _, stats_w) = read_character_layout().stats;
    let art = art::resolve_art("UI-Character-Info-ItemLevel-Bounce");
    let (w, h) = art.size();
    let mut children = category(
        "CharacterStatsPaneItemLevelCategory",
        "Item Level",
        stats_x + (stats_w - 197.0) / 2.0,
        y,
    );
    let frame_x = stats_x + (stats_w - 187.0) / 2.0;
    let frame_y = y + 40.0;
    let color = match active_skin() {
        ActiveSkin::Modern => "1.0,1.0,1.0,0.3",
        ActiveSkin::Forever => WHITE, // Camelot CharacterFrame.xml:507 has no alpha override.
    };
    children.extend(atlas(
        "CharacterStatsPaneItemLevelFrameBackground".into(),
        &art,
        (
            frame_x + (187.0 - w) / 2.0,
            frame_y + (29.0 - h) / 2.0,
            w,
            h,
        ),
        color,
    ));
    children.extend(text(
        "CharacterStatsPaneItemLevelFrameValue".into(),
        item_level,
        (frame_x, frame_y + 1.0, 187.0, 29.0),
        (15.0, HIGHLIGHT_FONT_COLOR),
        "CENTER",
    ));
    children
}

/// `CharacterStatFrameTemplate` 187×15 stacked TOP to BOTTOM: `Label`
/// (`GameFontNormalSmall`) LEFT 11, `Value` (`GameFontHighlightSmall`) RIGHT -8, and the
/// `UI-Character-Info-Line-Bounce` band (alpha 0.3) behind every second line.
fn stat_lines(lines: &[StatLine], first: usize, top: f32, gap: f32) -> Element {
    let size = match active_skin() {
        ActiveSkin::Modern => 10.0,
        ActiveSkin::Forever => 12.0, // Camelot CharacterFrame.xml:112,117; shared Fonts.xml:277.
    };
    let (stats_x, _, stats_w) = read_character_layout().stats;
    let x = stats_x + (stats_w - 187.0) / 2.0;
    let art = art::resolve_art("UI-Character-Info-Line-Bounce");
    let (w, h) = art.size();
    let color = match active_skin() {
        ActiveSkin::Modern => "1.0,1.0,1.0,0.3",
        ActiveSkin::Forever => WHITE, // Camelot CharacterFrame.xml:105 has no alpha override.
    };
    let mut children = Element::default();
    for (index, line) in lines.iter().enumerate() {
        let y = top + index as f32 * (15.0 + gap);
        let name = format!("CharacterStatsPaneStat{}", first + index);
        if index % 2 == 1 {
            children.extend(atlas(
                format!("{name}Background"),
                &art,
                (x + (187.0 - w) / 2.0, y + (15.0 - h) / 2.0, w, h),
                color,
            ));
        }
        children.extend(text(
            format!("{name}Label"),
            line.label,
            (x + 11.0, y, 187.0 - 19.0, 15.0),
            (size, NORMAL_FONT_COLOR),
            "LEFT",
        ));
        children.extend(text(
            format!("{name}Value"),
            &line.value,
            (x + 11.0, y, 187.0 - 19.0, 15.0),
            (size, HIGHLIGHT_FONT_COLOR),
            "RIGHT",
        ));
    }
    children
}

/// `CharacterStatFrameCategoryTemplate` 197×40: `UI-Character-Info-Title` and its
/// `GameFontHighlight` title CENTER 0,1.
fn category(name: &str, title: &str, x: f32, y: f32) -> Element {
    let art = art::resolve_art("UI-Character-Info-Title");
    let width = match active_skin() {
        ActiveSkin::Modern => 196.0,
        ActiveSkin::Forever => 197.0, // Camelot CharacterFrame.xml:83-90, both anchors.
    };
    let mut children = atlas(
        format!("{name}Background"),
        &art,
        (x, y, width, 40.0),
        WHITE,
    );
    let font_size = match active_skin() {
        ActiveSkin::Modern => 13.0,
        ActiveSkin::Forever => 12.0, // GameFontHighlight, shared Fonts.xml:277.
    };
    children.extend(text(
        format!("{name}Title"),
        title,
        (x, y - 1.0, 197.0, 40.0),
        (font_size, HIGHLIGHT_FONT_COLOR),
        "CENTER",
    ));
    children
}

fn slots(views: &[PaperDollSlotView]) -> Element {
    PAPERDOLL_BUTTONS
        .iter()
        .zip(views)
        .flat_map(|(button, view)| slot_button(button, view))
        .collect()
}

/// `PaperDollItemSlotButton{Left,Right,Bottom}Template`: the slot frame art behind the
/// button, the item or empty-slot icon, its quality border and stack count.
fn slot_button(button: &PaperDollButton, view: &PaperDollSlotView) -> Element {
    let (x, y) = button.origin();
    let mut children = art::slot_frame_art(button);
    children.extend(slot_icon(button, view));
    let action = equipment_slot_action(button.slot);
    rsx! {
        r#frame {
            name: {DynName(button.name.into())},
            width: SLOT,
            height: SLOT,
            mouse_enabled: true,
            onclick: {action.as_str()},
            pos_type: "absolute",
            left: x,
            top: y,
            {children}
        }
    }
}

fn slot_icon(button: &PaperDollButton, view: &PaperDollSlotView) -> Element {
    let name = button.name;
    let rect = (0.0, 0.0, SLOT, SLOT);
    if view.icon_fdid == 0 {
        return texture(
            format!("{name}IconTexture"),
            button.empty_texture,
            rect,
            FULL,
            WHITE,
        );
    }
    let color = if view.locked {
        crate::bag_frame_component::LOCKED_ICON
    } else {
        WHITE
    };
    let mut children = texture(
        format!("{name}IconTexture"),
        view.icon_fdid,
        rect,
        FULL,
        color,
    );
    if !view.quality_border.is_empty() {
        let border = format!("{name}IconBorder");
        children.extend(texture(
            border,
            WHITE_ICON_FRAME,
            rect,
            FULL,
            view.quality_border,
        ));
    }
    if view.count > 1 {
        children.extend(text(
            format!("{name}Count"),
            &view.count.to_string(),
            (0.0, SLOT - 16.0, SLOT - 5.0, 14.0),
            (14.0, WHITE),
            "RIGHT",
        ));
    }
    children
}

/// `CharacterFrameTab1` TOPLEFT at BOTTOMLEFT 11,2; Tab2 at Tab1 TOPRIGHT +1. The
/// Currency tab stays hidden (CF.xml): `ToggleTokenFrame` opens nothing while
/// `C_CurrencyInfo.GetCurrencyListSize() <= 0` (CharacterFrame.lua:67-73).
fn tabs(selected: CharacterTab) -> Element {
    if active_skin() == ActiveSkin::Forever {
        return forever_tabs(selected);
    }
    let top = FRAME_H - 2.0;
    let (character_w, reputation_w) = (tab_width("Character"), tab_width("Reputation"));
    let reputation = tab(
        "CharacterFrameTab2",
        "Reputation",
        (11.0 + character_w + 1.0, top, reputation_w),
        selected == CharacterTab::Reputation,
        ACTION_TAB_REPUTATION,
    );
    let character = tab(
        "CharacterFrameTab1",
        "Character",
        (11.0, top, character_w),
        selected == CharacterTab::PaperDoll,
        ACTION_TAB_CHARACTER,
    );
    // The selected tab draws over its neighbour.
    match selected {
        CharacterTab::PaperDoll => reputation.into_iter().chain(character).collect(),
        CharacterTab::Reputation => character.into_iter().chain(reputation).collect(),
    }
}

/// Existing Character/Reputation actions, with Camelot's vertical mode-tab chrome.
/// CharacterFrame.xml:569-589; CharacterFrame.lua:207; SharedUIPanelTemplates.lua:313.
/// Keep the existing Character/Reputation labels and approved tab art.
fn forever_tabs(selected: CharacterTab) -> Element {
    let background = art::resolve_art("common-sidetab");
    let (w, art_h) = background.size();
    let h = art_h - 5.0;
    let x = read_character_layout().size.0;
    [
        (
            1,
            "Character",
            CharacterTab::PaperDoll,
            ACTION_TAB_CHARACTER,
        ),
        (
            2,
            "Reputation",
            CharacterTab::Reputation,
            ACTION_TAB_REPUTATION,
        ),
    ]
    .into_iter()
    .flat_map(|(index, label, tab, action)| {
        let name = format!("CharacterFrameTab{index}");
        let y = 30.0 + (index - 1) as f32 * (h + 2.0);
        let mut children = atlas(
            format!("{name}Background"),
            &background,
            (0.0, (h - art_h) / 2.0, w, art_h),
            WHITE,
        );
        if tab == selected {
            children.extend(atlas(
                format!("{name}Selected"),
                &art::resolve_art("common-sidetab-selected"),
                (0.0, (h - art_h) / 2.0, w, art_h),
                WHITE,
            ));
        }
        // A full-height caption opts into native wrapping. Reserve only one line,
        // centered on the same tab, so longer native glyph runs are ellipsized.
        let caption_h = 13.0;
        children.extend(text(
            format!("{name}Text"),
            label,
            (0.0, (h - caption_h) / 2.0, w, caption_h),
            (10.0, HIGHLIGHT_FONT_COLOR),
            "CENTER",
        ));
        rsx! {
            r#frame {
                name: {DynName(name)},
                width: w,
                height: h,
                mouse_enabled: true,
                onclick: action,
                pos_type: "absolute",
                left: x,
                top: y,
                {children}
            }
        }
    })
    .collect()
}
