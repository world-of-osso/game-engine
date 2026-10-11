//! TransmogFrame's three authored Mainline regions (Blizzard_Transmog.xml:4-8,
//! 15-17,200-202,456-458). Both skins share this tree and application state.
use crate::bank_art::label;
use crate::merchant_frame_component::{MoneyAlign, money_colored};
use crate::quest_art::{DynName, panel_button, window_chrome};
use crate::transmog::TransmogBook;
use crate::ui::strata::FrameStrata;
use shared::protocol::{EquipmentSlot, TransmogChoice, TransmogSource};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const WIDTH: f32 = 1618.0;
pub const HEIGHT: f32 = 883.0;
pub const MODEL_SCENE: &str = "TransmogCharacterModelScene";
const GOLD: &str = "1,0.8235294,0,1";
const WHITE: &str = "1,1,1,1";
const RED: &str = "1,0.1254902,0.1254902,1";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TransmogView {
    pub book: TransmogBook,
    pub viewport_width: f32,
}

fn text(name: &str, value: &str, rect: (f32, f32, f32, f32), color: &str) -> Element {
    label(name.into(), value, rect, (12.0, color, "LEFT"))
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
    children.extend(outfit_region(&view.book));
    children.extend(character_region(&view.book));
    children.extend(wardrobe_region(&view.book));
    let left = ((view.viewport_width - WIDTH) * 0.5).max(0.0);
    rsx! { r#frame { name: "TransmogFrame", width: WIDTH, height: HEIGHT, left, top: 41.0,
    pos_type: "absolute", mouse_enabled: true, strata: FrameStrata::Dialog, {children} } }
}

fn outfit_region(book: &TransmogBook) -> Element {
    let mut out = text(
        "TransmogOutfitHeading",
        "Outfits",
        (16.0, 50.0, 280.0, 24.0),
        GOLD,
    );
    out.extend(text(
        "TransmogOutfitEmpty",
        "No saved outfits",
        (16.0, 86.0, 280.0, 24.0),
        WHITE,
    ));
    let pending = format!("{} pending changes", book.pending.len());
    out.extend(text(
        "TransmogPendingChanges",
        &pending,
        (16.0, 730.0, 280.0, 24.0),
        GOLD,
    ));
    match book.pending_cost() {
        Some(cost) => out.extend(money_colored(
            "TransmogPendingCost",
            cost,
            (147.0, 835.0),
            MoneyAlign::Left,
            WHITE,
        )),
        None => out.extend(text(
            "TransmogCostUnavailable",
            "Cost unavailable in local data",
            (16.0, 766.0, 280.0, 36.0),
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
        (11.0, 827.0, 128.0, 28.0),
    ));
    out.extend(panel_button(
        "TransmogUndoButton".into(),
        "Undo",
        "transmog:undo",
        !book.applying && !book.pending.is_empty(),
        (11.0, 787.0, 128.0, 28.0),
    ));
    if !book.error.is_empty() {
        out.extend(text(
            "TransmogError",
            &book.error,
            (16.0, 670.0, 280.0, 58.0),
            RED,
        ));
    }
    out
}

fn character_region(book: &TransmogBook) -> Element {
    let mut out = text(
        "TransmogCharacterHeading",
        "Appearance",
        (330.0, 50.0, 620.0, 24.0),
        GOLD,
    );
    out.extend(
        rsx! { r#frame { name: {DynName(MODEL_SCENE.into())}, width: 370.0, height: 688.0,
        left: 458.0, top: 95.0, pos_type: "absolute" } },
    );
    if let Some(state) = &book.state {
        out.extend(state.slots.iter().enumerate().flat_map(|(index, slot)| {
            let right = index >= 7;
            let row = if right { index - 7 } else { index };
            let x = if right { 839.0 } else { 325.0 };
            let pending = book.pending.contains_key(&slot.slot);
            let name = format!("TransmogSlot{}", slot.slot.inv_slot_id());
            let caption = format!(
                "{}{}{}",
                if Some(slot.slot) == book.selected {
                    "> "
                } else {
                    ""
                },
                slot_name(slot.slot),
                if pending { " *" } else { "" }
            );
            let action = format!("transmog:slot:{}", slot.slot.inv_slot_id());
            panel_button(
                name,
                &caption,
                &action,
                !book.applying,
                (x, 110.0 + row as f32 * 78.0, 118.0, 44.0),
            )
        }));
    }
    out
}

fn wardrobe_region(book: &TransmogBook) -> Element {
    let mut out = text(
        "TransmogWardrobeHeading",
        "Items",
        (992.0, 50.0, 590.0, 24.0),
        GOLD,
    );
    if let Some(slot) = book.selected_slot() {
        out.extend(text(
            "TransmogSelectedSlot",
            slot_name(slot.slot),
            (992.0, 86.0, 300.0, 24.0),
            WHITE,
        ));
        out.extend(panel_button(
            "TransmogOriginalButton".into(),
            "Original",
            "transmog:original",
            !book.applying,
            (992.0, 122.0, 140.0, 28.0),
        ));
        if slot.can_hide {
            out.extend(panel_button(
                "TransmogHideButton".into(),
                "Hide slot",
                "transmog:hide",
                !book.applying,
                (1142.0, 122.0, 140.0, 28.0),
            ));
        }
        if slot.sources.is_empty() {
            out.extend(text(
                "TransmogNoSources",
                "No collected appearances usable for this item.",
                (992.0, 180.0, 590.0, 48.0),
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
    let page = format!("{} / {}", book.page + 1, book.pages());
    out.extend(text(
        "TransmogPage",
        &page,
        (1240.0, 784.0, 90.0, 24.0),
        WHITE,
    ));
    out.extend(panel_button(
        "TransmogPreviousPage".into(),
        "Previous",
        "transmog:previous",
        book.page > 0,
        (992.0, 784.0, 140.0, 28.0),
    ));
    out.extend(panel_button(
        "TransmogNextPage".into(),
        "Next",
        "transmog:next",
        book.page + 1 < book.pages(),
        (1450.0, 784.0, 140.0, 28.0),
    ));
    out
}

fn source_button(book: &TransmogBook, source: &TransmogSource, index: usize) -> Element {
    let x = 992.0 + (index % 4) as f32 * 150.0;
    let y = 180.0 + (index / 4) as f32 * 192.0;
    let prefix = format!("TransmogSource{}", source.source_id);
    let action = format!("transmog:source:{}", source.source_id);
    let mut children = rsx! { texture { name: {DynName(format!("{prefix}Icon"))}, width: 56.0, height: 56.0,
    left: 44.0, top: 18.0, pos_type: "absolute", texture_fdid: source.icon_fdid } };
    children.extend(text(
        &format!("{prefix}Name"),
        &source.name,
        (6.0, 90.0, 132.0, 46.0),
        GOLD,
    ));
    let applied = book
        .selected_slot()
        .is_some_and(|slot| slot.applied == TransmogChoice::Source(source.source_id));
    let pending = book.selected.is_some_and(|slot| {
        book.pending.get(&slot) == Some(&TransmogChoice::Source(source.source_id))
    });
    let status = if pending {
        "Pending"
    } else if applied {
        "Applied"
    } else {
        "Collected"
    };
    children.extend(text(
        &format!("{prefix}Status"),
        status,
        (6.0, 142.0, 132.0, 22.0),
        WHITE,
    ));
    let disabled = book.applying;
    rsx! { button { name: {DynName(prefix)}, width: 144.0, height: 180.0, left: x, top: y,
    pos_type: "absolute", disabled, onclick: {action.as_str()}, {children} } }
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
