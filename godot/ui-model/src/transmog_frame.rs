//! Wardrobe service layout: shared paperdoll, top outfits, bottom Apply and Mainline
//! collection DressUpModel cells (Blizzard_Wardrobe.xml:5–98, lua:566–568).
//! Forever changes authored art, never slot positions, collection count or behavior.
use crate::bank_art::label;
use crate::merchant_frame_component::{MoneyAlign, money_colored};
use crate::quest_art::{DynName, panel_button, window_chrome};
use crate::transmog::TransmogBook;
use crate::ui::strata::FrameStrata;
use shared::protocol::{EquipmentSlot, TransmogChoice, TransmogSource};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const WIDTH: f32 = 960.0;
pub const HEIGHT: f32 = 580.0;
pub const MODEL_SCENE: &str = "TransmogCharacterModelScene";
const GOLD: &str = "1,0.8235294,0,1";
const WHITE: &str = "1,1,1,1";
const RED: &str = "1,0.1254902,0.1254902,1";
const COLLECTION_LEFT: f32 = 382.0;
const MODEL_WIDTH: f32 = 78.0;
const MODEL_HEIGHT: f32 = 104.0;
const COLUMN_STEP: f32 = 94.0;
const ROW_STEP: f32 = 132.0;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TransmogView {
    pub book: TransmogBook,
    pub viewport_width: f32,
    pub race_id: u8,
}

fn text(name: &str, value: &str, rect: (f32, f32, f32, f32), color: &str) -> Element {
    label(name.into(), value, rect, (12.0, color, "LEFT"))
}

fn atlas(name: String, art: &str, rect: (f32, f32, f32, f32)) -> Element {
    let (left, top, width, height) = rect;
    rsx! { texture { name: {DynName(name)}, width, height, left, top,
    pos_type: "absolute", texture_atlas: art } }
}

pub fn transmog_screen(ctx: &SharedContext) -> Element {
    let _ = ctx.get::<ui_toolkit::atlas::ActiveSkin>();
    let Some(view) = ctx
        .get::<TransmogView>()
        .filter(|view| view.book.state.is_some())
    else {
        return vec![];
    };
    let mut children = window_chrome(
        "TransmogFrame",
        (WIDTH, HEIGHT),
        "Transmogrify",
        "transmog:close",
    );
    children.extend(outfit_region());
    children.extend(character_region(view));
    children.extend(apply_region(&view.book));
    children.extend(wardrobe_region(&view.book));
    let left = ((view.viewport_width - WIDTH) * 0.5).max(0.0);
    rsx! { r#frame { name: "TransmogFrame", width: WIDTH, height: HEIGHT, left, top: 41.0,
    pos_type: "absolute", mouse_enabled: true, strata: FrameStrata::Dialog, {children} } }
}

fn outfit_region() -> Element {
    // No outfit protocol/persistence exists. Do not pretend a local selection is saved.
    let mut out = panel_button(
        "TransmogOutfitDropdown".into(),
        "Outfits unavailable",
        "transmog:outfits",
        false,
        (22.0, 48.0, 220.0, 26.0),
    );
    out.extend(panel_button(
        "TransmogSaveOutfitButton".into(),
        "Save",
        "transmog:save-outfit",
        false,
        (252.0, 48.0, 92.0, 26.0),
    ));
    out
}

fn apply_region(book: &TransmogBook) -> Element {
    let mut out = text(
        "TransmogPendingChanges",
        &format!("{} pending changes", book.pending.len()),
        (22.0, 486.0, 260.0, 22.0),
        GOLD,
    );
    match book.pending_cost() {
        Some(cost) => out.extend(money_colored(
            "TransmogPendingCost",
            cost,
            (34.0, 541.0),
            MoneyAlign::Left,
            WHITE,
        )),
        None => out.extend(text(
            "TransmogCostUnavailable",
            "Cost unavailable in local data",
            (22.0, 511.0, 320.0, 22.0),
            RED,
        )),
    }
    out.extend(panel_button(
        "TransmogApplyButton".into(),
        if book.applying {
            "Applying..."
        } else {
            "Apply"
        },
        "transmog:apply",
        book.can_apply(),
        (218.0, 534.0, 128.0, 28.0),
    ));
    out.extend(panel_button(
        "TransmogUndoButton".into(),
        "Undo",
        "transmog:undo",
        !book.applying && !book.pending.is_empty(),
        (252.0, 485.0, 94.0, 26.0),
    ));
    if !book.error.is_empty() {
        out.extend(text(
            "TransmogError",
            &book.error,
            (22.0, 76.0, 324.0, 24.0),
            RED,
        ));
    }
    out
}

fn character_region(view: &TransmogView) -> Element {
    let book = &view.book;
    let mut out = Vec::new();
    if let Some(background) = race_background_atlas(view.race_id) {
        out.extend(atlas(
            "TransmogRaceBackground".into(),
            background,
            (64.0, 100.0, 242.0, 367.0),
        ));
    }
    out.extend(
        rsx! { r#frame { name: {DynName(MODEL_SCENE.into())}, width: 242.0, height: 367.0,
        left: 64.0, top: 100.0, pos_type: "absolute" } },
    );
    let left = [
        EquipmentSlot::Head,
        EquipmentSlot::Shoulder,
        EquipmentSlot::Back,
        EquipmentSlot::Chest,
        EquipmentSlot::Shirt,
        EquipmentSlot::Tabard,
        EquipmentSlot::Wrist,
    ];
    let right = [
        EquipmentSlot::Hands,
        EquipmentSlot::Waist,
        EquipmentSlot::Legs,
        EquipmentSlot::Feet,
        EquipmentSlot::MainHand,
        EquipmentSlot::OffHand,
    ];
    for (column, slots) in [(22.0, left.as_slice()), (310.0, right.as_slice())] {
        for (row, slot) in slots.iter().enumerate() {
            out.extend(slot_button(book, *slot, column, 102.0 + row as f32 * 50.0));
        }
    }
    out.extend(preview_controls());
    out
}

fn preview_controls() -> Element {
    let mut out = panel_button(
        "TransmogRotateLeft".into(),
        "<",
        "transmog:rotate-left",
        true,
        (84.0, 456.0, 28.0, 24.0),
    );
    out.extend(panel_button(
        "TransmogSheatheButton".into(),
        "Sheathe / Draw",
        "transmog:sheathe",
        true,
        (120.0, 456.0, 160.0, 24.0),
    ));
    out.extend(panel_button(
        "TransmogRotateRight".into(),
        ">",
        "transmog:rotate-right",
        true,
        (288.0, 456.0, 28.0, 24.0),
    ));
    out
}

fn slot_button(book: &TransmogBook, slot: EquipmentSlot, x: f32, y: f32) -> Element {
    let name = format!("TransmogSlot{}", slot.inv_slot_id());
    let state = book
        .state
        .as_ref()
        .and_then(|state| state.slots.iter().find(|entry| entry.slot == slot));
    let mut children = atlas(
        format!("{name}EmptyIcon"),
        slot_atlas(slot),
        (0.0, 0.0, 37.0, 37.0),
    );
    if let Some(icon) = state.and_then(|slot| slot_icon(book, slot)) {
        children.extend(rsx! { texture { name: {DynName(format!("{name}Icon"))},
        width: 37.0, height: 37.0, texture_fdid: icon } });
    }
    if book.selected == Some(slot) {
        children.extend(atlas(
            format!("{name}Selected"),
            "transmog-nav-slot-selected",
            (-4.0, -4.0, 45.0, 47.0),
        ));
    }
    let caption = format!(
        "{}{}",
        slot_name(slot),
        if book.pending.contains_key(&slot) {
            " *"
        } else {
            ""
        }
    );
    children.extend(label(
        format!("{name}Name"),
        &caption,
        (-5.0, 37.0, 51.0, 12.0),
        (9.0, WHITE, "CENTER"),
    ));
    let action = format!("transmog:slot:{}", slot.inv_slot_id());
    let disabled = book.applying || state.is_none();
    rsx! { button { name: {DynName(name)}, width: 37.0, height: 37.0,
    left: x, top: y, pos_type: "absolute", disabled, onclick: {action.as_str()}, {children} } }
}

fn slot_icon(book: &TransmogBook, slot: &shared::protocol::TransmogSlot) -> Option<u32> {
    let choice = book
        .pending
        .get(&slot.slot)
        .copied()
        .unwrap_or(slot.applied);
    match choice {
        TransmogChoice::Source(id) => {
            let source = slot.sources.iter().find(|source| source.source_id == id)?;
            crate::item_catalog::item_catalog_entry_for(slot.definition_source, source.item_id)
                .map(|item| item.icon_fdid)
                .filter(|fdid| *fdid != 0)
        }
        TransmogChoice::Original => {
            crate::item_catalog::item_catalog_entry_for(slot.definition_source, slot.item_id)
                .map(|item| item.icon_fdid)
        }
        TransmogChoice::Hidden => None,
    }
}

fn wardrobe_region(book: &TransmogBook) -> Element {
    let mut out = text(
        "TransmogWardrobeHeading",
        "Appearances",
        (COLLECTION_LEFT, 49.0, 310.0, 24.0),
        GOLD,
    );
    if let Some(slot) = book.selected_slot() {
        out.extend(text(
            "TransmogSelectedSlot",
            slot_name(slot.slot),
            (COLLECTION_LEFT, 83.0, 250.0, 24.0),
            WHITE,
        ));
        out.extend(panel_button(
            "TransmogOriginalButton".into(),
            "Original",
            "transmog:original",
            !book.applying,
            (738.0, 82.0, 96.0, 24.0),
        ));
        if slot.can_hide {
            out.extend(panel_button(
                "TransmogHideButton".into(),
                "Hide slot",
                "transmog:hide",
                !book.applying,
                (842.0, 82.0, 96.0, 24.0),
            ));
        }
        if slot.sources.is_empty() {
            out.extend(text(
                "TransmogNoSources",
                "No collected appearances usable for this item.",
                (COLLECTION_LEFT, 140.0, 550.0, 48.0),
                WHITE,
            ));
        }
    }
    out.extend(
        book.page_sources()
            .iter()
            .enumerate()
            .flat_map(|(index, source)| source_button(book, source, index)),
    );
    out.extend(text(
        "TransmogPage",
        &format!("{} / {}", book.page + 1, book.pages()),
        (635.0, 537.0, 80.0, 24.0),
        WHITE,
    ));
    out.extend(panel_button(
        "TransmogPreviousPage".into(),
        "Previous",
        "transmog:previous",
        book.page > 0,
        (COLLECTION_LEFT, 534.0, 112.0, 28.0),
    ));
    out.extend(panel_button(
        "TransmogNextPage".into(),
        "Next",
        "transmog:next",
        book.page + 1 < book.pages(),
        (826.0, 534.0, 112.0, 28.0),
    ));
    out
}

pub fn source_model_scene(source_id: u32) -> String {
    format!("TransmogSource{source_id}ModelScene")
}

fn source_button(book: &TransmogBook, source: &TransmogSource, index: usize) -> Element {
    let x = COLLECTION_LEFT + (index % 6) as f32 * COLUMN_STEP;
    let y = 128.0 + (index / 6) as f32 * ROW_STEP;
    let prefix = format!("TransmogSource{}", source.source_id);
    let action = format!("transmog:source:{}", source.source_id);
    let mut children = rsx! { r#frame { name: {DynName(source_model_scene(source.source_id))}, width: MODEL_WIDTH, height: MODEL_HEIGHT,
    left: 0.0, top: 0.0, pos_type: "absolute", background_color: "0,0,0,1" } };
    children.extend(atlas(
        format!("{prefix}Border"),
        "transmog-wardrobe-border-collected",
        (-9.0, -9.0, 96.0, 122.0),
    ));
    let applied = book
        .selected_slot()
        .is_some_and(|slot| slot.applied == TransmogChoice::Source(source.source_id));
    let pending = book.selected.is_some_and(|slot| {
        book.pending.get(&slot) == Some(&TransmogChoice::Source(source.source_id))
    });
    if applied {
        children.extend(atlas(
            format!("{prefix}Applied"),
            "transmog-wardrobe-border-current",
            (-3.0, -3.0, 84.0, 110.0),
        ));
    }
    if pending {
        children.extend(atlas(
            format!("{prefix}Pending"),
            "transmog-wardrobe-border-selected",
            (-12.0, -12.0, 102.0, 128.0),
        ));
    }
    children.extend(label(
        format!("{prefix}Name"),
        &source.name,
        (0.0, 107.0, MODEL_WIDTH, 12.0),
        (9.0, GOLD, "CENTER"),
    ));
    let status = if pending {
        "Pending"
    } else if applied {
        "Applied"
    } else {
        "Collected"
    };
    children.extend(label(
        format!("{prefix}Status"),
        status,
        (0.0, 119.0, MODEL_WIDTH, 12.0),
        (9.0, WHITE, "CENTER"),
    ));
    let disabled = book.applying;
    rsx! { button { name: {DynName(prefix)}, width: MODEL_WIDTH, height: MODEL_HEIGHT,
    left: x, top: y, pos_type: "absolute", disabled, onclick: {action.as_str()}, {children} } }
}

fn slot_atlas(slot: EquipmentSlot) -> &'static str {
    match slot {
        EquipmentSlot::Head => "transmog-nav-slot-head",
        EquipmentSlot::Shoulder => "transmog-nav-slot-shoulder",
        EquipmentSlot::Back => "transmog-nav-slot-back",
        EquipmentSlot::Chest => "transmog-nav-slot-chest",
        EquipmentSlot::Shirt => "transmog-nav-slot-shirt",
        EquipmentSlot::Tabard => "transmog-nav-slot-tabard",
        EquipmentSlot::Wrist => "transmog-nav-slot-wrist",
        EquipmentSlot::Hands => "transmog-nav-slot-hands",
        EquipmentSlot::Waist => "transmog-nav-slot-waist",
        EquipmentSlot::Legs => "transmog-nav-slot-legs",
        EquipmentSlot::Feet => "transmog-nav-slot-feet",
        EquipmentSlot::MainHand => "transmog-nav-slot-mainhand",
        EquipmentSlot::OffHand => "transmog-nav-slot-secondaryhand",
        _ => unreachable!("Wardrobe has only visual slots"),
    }
}

fn race_background_atlas(race: u8) -> Option<&'static str> {
    Some(match race {
        1 => "transmog-background-race-human",
        2 => "transmog-background-race-orc",
        3 => "transmog-background-race-dwarf",
        4 => "transmog-background-race-nightelf",
        5 => "transmog-background-race-scourge",
        6 => "transmog-background-race-tauren",
        7 => "transmog-background-race-gnome",
        8 => "transmog-background-race-troll",
        9 => "transmog-background-race-goblin",
        10 => "transmog-background-race-bloodelf",
        11 => "transmog-background-race-draenei",
        22 => "transmog-background-race-worgen",
        24..=26 => "transmog-background-race-pandaren",
        27 => "transmog-background-race-nightborne",
        28 => "transmog-background-race-highmountaintauren",
        29 => "transmog-background-race-voidelf",
        30 => "transmog-background-race-lightforgeddraenei",
        31 => "transmog-background-race-ZandalariTroll",
        32 => "transmog-background-race-kultiran",
        34 => "transmog-background-race-darkirondwarf",
        35 => "transmog-background-race-vulpera",
        36 => "transmog-background-race-magharorc",
        37 => "transmog-background-race-mechagnome",
        52 | 70 => "transmog-background-race-dracthyr",
        84 | 85 => "transmog-background-race-earthen",
        _ => return None,
    })
}

pub fn slot_name(slot: EquipmentSlot) -> &'static str {
    match slot {
        EquipmentSlot::Head => "Head",
        EquipmentSlot::Shoulder => "Shoulders",
        EquipmentSlot::Back => "Back",
        EquipmentSlot::Chest => "Chest",
        EquipmentSlot::Shirt => "Shirt",
        EquipmentSlot::Tabard => "Tabard",
        EquipmentSlot::Wrist => "Wrists",
        EquipmentSlot::Hands => "Hands",
        EquipmentSlot::Waist => "Waist",
        EquipmentSlot::Legs => "Legs",
        EquipmentSlot::Feet => "Feet",
        EquipmentSlot::MainHand => "Main Hand",
        EquipmentSlot::OffHand => "Off Hand",
        EquipmentSlot::Ranged => "Ranged",
        _ => "Equipment",
    }
}
