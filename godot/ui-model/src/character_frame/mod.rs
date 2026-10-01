//! Retail `CharacterFrame` with its `PaperDollFrame` (Blizzard_UIPanels_Game/Mainline/
//! CharacterFrame.xml / .lua cited as CF.xml / CF.lua, PaperDollFrame.xml / .lua as
//! PDF.xml / PDF.lua): the expanded 540×424 `ButtonFrameTemplate` window with the 18
//! paperdoll item buttons, the character model scene, the level line, the
//! `CharacterStatsPane` and the Character / Reputation tabs. Positions are top-left
//! offsets in frame space converted from the XML anchors. docs/specs/character-frame.md.

mod art;

use shared::protocol::{EquipmentSlot, ItemLocation};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::text_measure::measure_text;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

use art::{FULL, WHITE, WHITE_ICON_FRAME, atlas, texture};
pub use art::{class_background, race_background, race_overlay_alpha};

use crate::bag_data::InventoryState;
pub use crate::character_frame_component::{equipment_slot_action, parse_equipment_slot_action};
use crate::merchant_frame_component::{tab, tab_width};
use crate::quest_art::{DynName, HIGHLIGHT_FONT_COLOR, NORMAL_FONT_COLOR, window_chrome};
use crate::ui::strata::FrameStrata;

pub const FRAME_NAME: &str = "CharacterFrame";
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
        match self.column {
            Column::Left => (LEFT_COLUMN, COLUMN_TOP + row * SLOT_STEP),
            Column::Right => (RIGHT_COLUMN, COLUMN_TOP + row * SLOT_STEP),
            Column::Bottom if self.row == 0 => MAIN_HAND,
            Column::Bottom => (OFF_HAND_X, MAIN_HAND.1),
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

#[derive(Clone, Debug, PartialEq, Default)]
pub struct CharacterFrameView {
    pub visible: bool,
    /// `UnitPVPName("player")`.
    pub title: String,
    pub level: LevelLine,
    /// One per [`PAPERDOLL_BUTTONS`] entry, in that order.
    pub slots: Vec<PaperDollSlotView>,
    /// `STAT_AVERAGE_ITEM_LEVEL` value; `None` below [`MIN_LEVEL_FOR_ITEM_LEVEL`].
    pub item_level: Option<String>,
    pub race_id: u8,
    pub class_id: u8,
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
    let mut children = window_chrome(FRAME_NAME, (FRAME_W, FRAME_H), &view.title, ACTION_CLOSE);
    children.extend(art::inset_backgrounds(view.class_id));
    children.extend(art::race_backdrop(view.race_id));
    children.extend(art::inner_border());
    children.extend(model_scene_frame());
    children.extend(level_text(&view.level));
    children.extend(stats_pane(view.item_level.as_deref()));
    children.extend(slots(&view.slots));
    children.extend(tabs());
    rsx! {
        r#frame {
            name: {DynName(FRAME_NAME.into())},
            width: FRAME_W,
            height: FRAME_H,
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

/// Styles RSX attributes do not express: the white title
/// (`characterFrameDisplayInfo.Default.titleColor`) and the desaturated race backdrop
/// (`PaperDollBgDesaturate(true)`).
pub fn apply_character_frame_postsetup(registry: &mut FrameRegistry) {
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
    let (x, y, width, height) = MODEL;
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
    const SIZE: f32 = 12.0;
    let measure = |text: &str| {
        measure_text(text, GameFont::FrizQuadrata, SIZE).map_or(0.0, |(width, _)| width)
    };
    let (level_w, class_w) = (measure(&line.level), measure(&line.class_text));
    let start = FRAME_W / 2.0 - (level_w + class_w) / 2.0;
    let top = 42.0 - 12.0;
    let mut children = text(
        "CharacterLevelText".into(),
        &line.level,
        (start, top, level_w + 1.0, 24.0),
        (SIZE, NORMAL_FONT_COLOR),
        "LEFT",
    );
    children.extend(text(
        "CharacterLevelTextClass".into(),
        &line.class_text,
        (start + level_w, top, class_w + 1.0, 24.0),
        (SIZE, &line.class_color),
        "LEFT",
    ));
    children
}

/// `CharacterStatsPane`: `ItemLevelCategory` (TOP 0,-2) and `ItemLevelFrame` below it
/// (CF.xml). Attributes and Enhancements hide when they show no stat
/// (`catFrame:SetShown(numStatInCat > 0)`, PDF.lua:1985).
fn stats_pane(item_level: Option<&str>) -> Element {
    let Some(item_level) = item_level else {
        return Element::default();
    };
    let y = STATS.1 + 2.0;
    let mut children = item_level_category(STATS.0 + (STATS_W - 197.0) / 2.0, y);
    let frame_x = STATS.0 + (STATS_W - 187.0) / 2.0;
    let frame_y = y + 40.0;
    children.extend(atlas(
        "CharacterStatsPaneItemLevelFrameBackground".into(),
        &art::ITEM_LEVEL_BOUNCE,
        (frame_x + (187.0 - 162.0) / 2.0, frame_y, 162.0, 29.0),
        "1.0,1.0,1.0,0.3",
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

/// `CharacterStatFrameCategoryTemplate` 197×40 titled `STAT_AVERAGE_ITEM_LEVEL`.
fn item_level_category(x: f32, y: f32) -> Element {
    let mut children = atlas(
        "CharacterStatsPaneItemLevelCategoryBackground".into(),
        &art::CATEGORY_TITLE,
        (x, y, 196.0, 40.0),
        WHITE,
    );
    children.extend(text(
        "CharacterStatsPaneItemLevelCategoryTitle".into(),
        "Item Level",
        (x, y - 1.0, 197.0, 40.0),
        (13.0, HIGHLIGHT_FONT_COLOR),
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
/// Currency tab stays hidden (CF.xml).
fn tabs() -> Element {
    let top = FRAME_H - 2.0;
    let (character_w, reputation_w) = (tab_width("Character"), tab_width("Reputation"));
    let reputation = tab(
        "CharacterFrameTab2",
        "Reputation",
        (11.0 + character_w + 1.0, top, reputation_w),
        false,
        ACTION_TAB_REPUTATION,
    );
    let character = tab(
        "CharacterFrameTab1",
        "Character",
        (11.0, top, character_w),
        true,
        ACTION_TAB_CHARACTER,
    );
    // The selected tab draws over its neighbour.
    reputation.into_iter().chain(character).collect()
}
