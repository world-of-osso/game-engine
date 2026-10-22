//! EncounterItemTemplate XML1220–1271 and EncounterJournalItemMixin Lua189–233.
use game_engine_core::adventure_guide::Item;
use ui_toolkit::{rsx, widget_def::Element};

use crate::{
    adventure_guide::{View, paper_text},
    adventure_guide_art::{Art, art},
    bank_art::{WHITE, label, texture},
    quest_art::DynName,
};

const INK: &str = "0.25,0.1484375,0.02,1";
const BOSS_HEIGHT: f32 = 45.;
const INSTANCE_HEIGHT: f32 = 64.;
const WHITE_ICON_FRAME: u32 = 651_080;
const RELIC_ICON_FRAME: u32 = 1_313_680;
const ITEM_CLASS_GEM: u8 = 3;
const ARTIFACT_RELIC_SUBCLASS: u8 = 11;

pub(crate) fn rows(view: &View) -> (Element, f32) {
    let items = view.loot();
    let instance_wide = view.encounter.is_none();
    let height = if instance_wide {
        INSTANCE_HEIGHT
    } else {
        BOSS_HEIGHT
    };
    let bosses = view.loot_boss_names();
    let content = items
        .iter()
        .enumerate()
        .flat_map(|(index, item)| {
            let names = instance_wide.then(|| bosses[&item.id].as_slice());
            loot_row(item, index as f32 * height, height, names)
        })
        .collect();
    (content, items.len() as f32 * height)
}

fn loot_row(item: &Item, top: f32, height: f32, bosses: Option<&[&str]>) -> Element {
    let name = format!("JournalLoot{}", item.id);
    let mut content = read_item_icon(item, &name);
    let (border, border_width) = if bosses.is_some() {
        (Art::DungeonLootFrame, 369.)
    } else {
        (Art::LootFrame, 321.)
    };
    content.extend(art(
        format!("{name}Border"),
        border,
        (0., 0., border_width, height),
    ));
    content.extend(label(
        format!("{name}Name"),
        &item.name,
        (51., 9., 250., 12.),
        (
            14.,
            crate::merchant_data::quality_color(item.quality as u8),
            "LEFT",
        ),
    ));
    content.extend(item_type_labels(item, &name));
    content.extend(read_quality_border(item, &name));
    if let Some(bosses) = bosses {
        content.extend(paper_text(
            &format!("{name}Boss"),
            &boss_text(bosses),
            (2., 47., 315., 12.),
        ));
    }
    let action = format!("journal:loot:{}", item.id);
    rsx! {
        button {
            name: {DynName(name)}, left: 0., top, width: 321., height,
            pos_type: "absolute", onclick: {action.as_str()}, button_default_skin: false,
            tooltip_provider: "journal_loot", tooltip_anchor: "ANCHOR_RIGHT", {content}
        }
    }
}

fn read_item_icon(item: &Item, name: &str) -> Element {
    // C_Item.GetItemIconByID: Item.IconFileDataID alone is not an appearance icon.
    let Some(icon) = crate::item_icons::item_icon_fdid(item.id) else {
        return vec![];
    };
    texture(format!("{name}Icon"), icon, (2., 2., 42., 42.), WHITE)
}

fn read_quality_border(item: &Item, name: &str) -> Element {
    // ItemButtonTemplate.lua210–237; ColorConstants.lua21–30 has colors only1..8.
    if !(1..=8).contains(&item.quality) {
        return vec![];
    }
    let Some(entry) = crate::item_catalog::item_catalog_entry(item.id) else {
        return vec![];
    };
    let frame = if entry.class_id == ITEM_CLASS_GEM && entry.subclass_id == ARTIFACT_RELIC_SUBCLASS
    {
        RELIC_ICON_FRAME
    } else {
        WHITE_ICON_FRAME
    };
    let quality = crate::bag_data::ItemQuality::from_id(item.quality as u8);
    texture(
        format!("{name}IconBorder"),
        frame,
        (2., 2., 42., 42.),
        quality.border_color(),
    )
}

fn item_type_labels(item: &Item, name: &str) -> Element {
    let armor = read_armor_type(item.id);
    let slot = crate::item_tooltip::inventory_type_label(item.slot as u8).unwrap_or_default();
    // XML: Slot LEFT51, ArmorClass RIGHT315,15px between; ArmorClass has intrinsic x0.
    // Native font metrics, not a second backend's fixed width, resolve these constraints.
    rsx! {
        frame {
            name: {DynName(format!("{name}TypeLine"))},
            left: 51., top: 27., width: 264., height: 12., pos_type: "absolute",
            layout: "flex-row", align: "start", gap: 15.,
            fontstring {
                name: {DynName(format!("{name}Slot"))}, text: slot,
                width: "fill", height: 12., pos_type: "relative",
                font_size: 12., font_color: INK, justify_h: "LEFT"
            }
            fontstring {
                name: {DynName(format!("{name}ArmorClass"))}, text: armor,
                width: "auto", height: 12., pos_type: "relative",
                font_size: 12., font_color: INK, justify_h: "RIGHT"
            }
        }
    }
}

fn read_armor_type(item: u32) -> &'static str {
    let Some(entry) = crate::item_catalog::item_catalog_entry(item) else {
        return "";
    };
    // Same GetItemInfo subtype exclusion as the shared item tooltip's slot line.
    if entry.class_id == 4 && entry.subclass_id == 0 {
        return "";
    }
    crate::item_catalog::item_subclass_name(entry).unwrap_or_default()
}

fn boss_text(bosses: &[&str]) -> String {
    let key = match bosses.len() {
        0 => return String::new(),
        1 => "BOSS_INFO_STRING",
        2 => "BOSS_INFO_STRING_TWO",
        _ => "BOSS_INFO_STRING_MANY",
    };
    bosses
        .iter()
        .take(2)
        .fold(crate::auction_mail::text(key).to_owned(), |text, boss| {
            text.replacen("%s", boss, 1)
        })
}
