//! `GameTooltip:SetBagItem` / `SetInventoryItem` lines from the item catalog
//! (ItemSparse): name in its quality colour, item level, binding, uniqueness,
//! slot and type, durability, level requirement, flavor text and sell price, in
//! Retail's order. Stats, armor and weapon damage need the item-level budget
//! tables and are not shown.

use crate::bag_data::InventorySlot;
use crate::item_catalog::{ItemCatalogEntry, item_catalog_entry, item_subclass_name};
use crate::merchant_data::quality_color;

use crate::tooltip_presentation::{
    TOOLTIP_DESCRIPTION_COLOR, TOOLTIP_WHITE, TooltipLineState, TooltipPresentation,
    description_lines, parse_rgba,
};

/// `RED_FONT_COLOR`.
const RED_FONT_COLOR: [f32; 4] = [1.0, 0.125, 0.125, 1.0];
const ITEM_CLASS_WEAPON: u8 = 2;
const ITEM_CLASS_ARMOR: u8 = 4;
const ARMOR_SUBCLASS_MISCELLANEOUS: u8 = 0;
/// `Enum.InventoryType.IndexBagType`.
const INVTYPE_BAG: u8 = 18;

/// The item a tooltip describes and the viewer's level (`Requires Level` turns red
/// above it).
struct TooltipItem<'a> {
    pub slot: &'a InventorySlot,
    pub player_level: Option<u16>,
}

pub fn item_tooltip(slot: &InventorySlot, player_level: Option<u16>) -> TooltipPresentation {
    let item = TooltipItem { slot, player_level };
    let entry = item_catalog_entry(slot.item_id);
    TooltipPresentation {
        visible: true,
        x: 0.0,
        y: 0.0,
        title: slot.name.clone(),
        title_color: parse_rgba(quality_color(slot.quality.id())),
        lines: entry.map_or_else(Vec::new, |entry| item_lines(entry, &item)),
    }
}

fn item_lines(entry: &ItemCatalogEntry, item: &TooltipItem) -> Vec<TooltipLineState> {
    let slot = item.slot;
    let white = |text: String| TooltipLineState::colored(text, TOOLTIP_WHITE);
    let mut lines = Vec::new();
    let gear = matches!(entry.class_id, ITEM_CLASS_WEAPON | ITEM_CLASS_ARMOR);
    if gear && entry.inventory_type != 0 {
        // ITEM_LEVEL "Item Level %d".
        lines.push(TooltipLineState::colored(
            format!("Item Level {}", entry.item_level),
            TOOLTIP_DESCRIPTION_COLOR,
        ));
    }
    if let Some(binding) = binding_text(entry.bonding, slot.soulbound) {
        lines.push(white(binding.into()));
    }
    if entry.max_count == 1 {
        lines.push(white("Unique".into())); // ITEM_UNIQUE
    }
    if let Some(line) = slot_line(entry) {
        lines.push(line);
    }
    if let Some(durability) = slot.durability {
        // DURABILITY_TEMPLATE "Durability %d / %d".
        lines.push(white(format!(
            "Durability {} / {}",
            durability.current, durability.max
        )));
    }
    if entry.required_level > 1 {
        // ITEM_MIN_LEVEL, red while the viewer is below it.
        let met = item
            .player_level
            .is_none_or(|level| level >= entry.required_level);
        lines.push(TooltipLineState::colored(
            format!("Requires Level {}", entry.required_level),
            if met { TOOLTIP_WHITE } else { RED_FONT_COLOR },
        ));
    }
    if !entry.description.is_empty() {
        lines.extend(description_lines(
            &format!("\"{}\"", entry.description),
            TOOLTIP_DESCRIPTION_COLOR,
        ));
    }
    if entry.sell_price > 0 {
        // GameTooltip_OnTooltipAddMoney: `SELL_PRICE:` and the stack's price.
        lines.push(TooltipLineState::money(
            "Sell Price:",
            u64::from(entry.sell_price) * u64::from(slot.count.max(1)),
        ));
    }
    lines
}

/// `ITEM_SOULBOUND` once bound, else the item's `Bonding`
/// (`ITEM_BIND_ON_PICKUP` / `_ON_EQUIP` / `_ON_USE` / `ITEM_BIND_QUEST`).
fn binding_text(bonding: u8, soulbound: bool) -> Option<&'static str> {
    if soulbound {
        return Some("Soulbound");
    }
    match bonding {
        1 => Some("Binds when picked up"),
        2 => Some("Binds when equipped"),
        3 => Some("Binds when used"),
        4 => Some("Quest Item"),
        _ => None,
    }
}

/// The equip slot left and the subclass right ("Main Hand" / "Sword"); a bag
/// reads `CONTAINER_SLOTS` ("%d Slot %s").
fn slot_line(entry: &ItemCatalogEntry) -> Option<TooltipLineState> {
    let subclass = item_subclass_name(entry).unwrap_or_default();
    if entry.inventory_type == INVTYPE_BAG {
        return Some(TooltipLineState::colored(
            format!("{} Slot {subclass}", entry.container_slots),
            TOOLTIP_WHITE,
        ));
    }
    let slot = inventory_type_label(entry.inventory_type)?;
    let shows_subclass =
        !(entry.class_id == ITEM_CLASS_ARMOR && entry.subclass_id == ARMOR_SUBCLASS_MISCELLANEOUS);
    Some(TooltipLineState::pair(
        slot,
        if shows_subclass { subclass } else { "" },
    ))
}

/// `INVTYPE_*` GlobalStrings by `Enum.InventoryType`.
fn inventory_type_label(inventory_type: u8) -> Option<&'static str> {
    Some(match inventory_type {
        1 => "Head",
        2 => "Neck",
        3 => "Shoulder",
        4 => "Shirt",
        5 | 20 => "Chest",
        6 => "Waist",
        7 => "Legs",
        8 => "Feet",
        9 => "Wrist",
        10 => "Hands",
        11 => "Finger",
        12 => "Trinket",
        13 => "One-Hand",
        14 | 22 => "Off Hand",
        15 | 26 => "Ranged",
        16 => "Back",
        17 => "Two-Hand",
        19 => "Tabard",
        21 => "Main Hand",
        23 => "Held In Off-hand",
        24 => "Ammo",
        25 => "Thrown",
        27 => "Quiver",
        28 => "Relic",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use crate::bag_data::ItemQuality;
    use shared::protocol::ItemDurability;

    use super::*;

    fn configure_test_data() {
        #[cfg(godot_host)]
        crate::paths::set_data_root(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        )
        .expect("tooltip test data root");
    }

    fn texts(tooltip: &TooltipPresentation) -> Vec<(String, String)> {
        tooltip
            .lines
            .iter()
            .map(|line| (line.left_text.clone(), line.right_text.clone()))
            .collect()
    }

    fn pair(left: &str, right: &str) -> (String, String) {
        (left.into(), right.into())
    }

    #[test]
    fn worn_shortsword_reads_like_retail() {
        configure_test_data();
        let sword = InventorySlot {
            icon_fdid: 135_274,
            count: 1,
            quality: ItemQuality::Common,
            name: "Worn Shortsword".into(),
            item_id: 25,
            soulbound: true,
            durability: Some(ItemDurability {
                current: 18,
                max: 20,
            }),
            ..Default::default()
        };
        let tooltip = item_tooltip(&sword, Some(1));
        assert_eq!(tooltip.title, "Worn Shortsword");
        assert_eq!(tooltip.title_color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(
            texts(&tooltip),
            vec![
                pair("Item Level 1", ""),
                pair("Soulbound", ""),
                pair("Main Hand", "Sword"),
                pair("Durability 18 / 20", ""),
                pair("Sell Price:", ""),
            ]
        );
        assert_eq!(tooltip.lines.last().unwrap().money, Some(3));
    }

    #[test]
    fn trade_goods_show_the_stack_sell_price_and_grey_junk_its_title_colour() {
        configure_test_data();
        let linen = InventorySlot {
            count: 20,
            name: "Linen Cloth".into(),
            item_id: 2589,
            ..Default::default()
        };
        let tooltip = item_tooltip(&linen, None);
        assert_eq!(texts(&tooltip), vec![pair("Sell Price:", "")]);
        assert_eq!(tooltip.lines[0].money, Some(13 * 20));

        let pelt = InventorySlot {
            count: 1,
            quality: ItemQuality::Poor,
            name: "Ruined Pelt".into(),
            item_id: 4865,
            ..Default::default()
        };
        let tooltip = item_tooltip(&pelt, None);
        assert_eq!(tooltip.title_color, parse_rgba(quality_color(0)));
    }

    #[test]
    fn a_level_requirement_above_the_viewer_is_red() {
        configure_test_data();
        let entry = ItemCatalogEntry {
            class_id: ITEM_CLASS_ARMOR,
            subclass_id: 1,
            inventory_type: 20,
            item_level: 20,
            required_level: 15,
            bonding: 2,
            max_count: 1,
            description: "Soft as a kitten.".into(),
            ..Default::default()
        };
        let slot = InventorySlot {
            count: 1,
            ..Default::default()
        };
        let lines = item_lines(
            &entry,
            &TooltipItem {
                slot: &slot,
                player_level: Some(10),
            },
        );
        let text: Vec<&str> = lines.iter().map(|line| line.left_text.as_str()).collect();
        assert_eq!(
            text,
            [
                "Item Level 20",
                "Binds when equipped",
                "Unique",
                "Chest",
                "Requires Level 15",
                "\"Soft as a kitten.\"",
            ]
        );
        assert_eq!(lines[4].left_color, RED_FONT_COLOR);
        assert_eq!(lines[5].left_color, TOOLTIP_DESCRIPTION_COLOR);
    }
}
