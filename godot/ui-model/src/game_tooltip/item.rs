//! Item tooltips for every item source (bags, merchant and buyback cells, loot slots, mail
//! attachments, auction rows) over the shared catalog formatter, and the Shift comparison
//! with the equipped items (`TooltipComparisonManager`, `C_TooltipComparison`).

use shared::item_data::ItemDefinitionSource;
use shared::protocol::EquipmentSlot;

use super::{GameTooltip, TooltipRecord};
use crate::auction::AuctionHouseState;
use crate::auction_house_frame_component::{
    ACTION_BROWSE_ITEM_PREFIX, ACTION_SELECT_AUCTION_PREFIX, ACTION_SELL_ITEM_PREFIX,
};
use crate::bag_data::{InventorySlot, InventoryState, ItemQuality};
use crate::item_catalog::{
    ItemCatalogEntry, SourceItemCatalogs, item_catalog_entry, item_catalogs,
};
use crate::item_stats::{item_armor_for, item_stats_for, weapon_damage_for};
use crate::item_tooltip::{GREEN_FONT_COLOR, RED_FONT_COLOR, item_tooltip, stat_name, stat_text};
use crate::tooltip_presentation::{
    TOOLTIP_DESCRIPTION_COLOR, TooltipLineState, TooltipPresentation, description_lines,
    item_id_line,
};

/// `ITEM_DELTA_DESCRIPTION`.
const DELTA_HEADER: &str = "If you replace this item, the following stat changes will occur:";
/// `EQUIPPED`: the comparison header.
pub const EQUIPPED_HEADER: &str = "Equipped";

/// A stack the server names: what merchant, loot, mail and auction items carry.
pub fn named_item(item_id: u32, name: &str, quality: u8, count: u32) -> InventorySlot {
    InventorySlot {
        icon_fdid: item_catalog_entry(item_id).map_or(0, |entry| entry.icon_fdid),
        count,
        quality: ItemQuality::from_id(quality),
        name: name.to_owned(),
        item_id,
        ..Default::default()
    }
}

pub fn named_item_for(
    source: ItemDefinitionSource,
    item_id: u32,
    name: &str,
    quality: u8,
    count: u32,
    item_guid: u64,
) -> InventorySlot {
    InventorySlot {
        icon_fdid: crate::item_icons::item_icon_fdid_for(source, item_id).unwrap_or(0),
        definition_source: source,
        item_id,
        item_guid,
        name: name.to_owned(),
        quality: ItemQuality::from_id(quality),
        count,
        ..Default::default()
    }
}

/// `GameTooltip:SetBagItem` and the other item setters: the catalog lines and the item ID.
pub fn item_game_tooltip(slot: &InventorySlot, player_level: Option<u16>) -> GameTooltip {
    GameTooltip::new(
        item_tooltip(slot, player_level),
        Some(TooltipRecord::Item(slot.item_id)),
    )
}

/// The auction house's item tooltips (`AuctionHouseUtil.SetAuctionHouseTooltip`, `ANCHOR_RIGHT`)
/// hide the vendor price.
pub fn without_sell_price(mut tooltip: GameTooltip) -> GameTooltip {
    tooltip.content.lines.retain(|line| line.money.is_none());
    tooltip
}

/// The item of an auction house row by its click action: a browse result, a listing of the
/// item buy, auctions or bids lists, or a sell-tab inventory item.
pub fn auction_row_item(action: &str, net: &AuctionHouseState) -> Option<InventorySlot> {
    if let Some(item_id) = action.strip_prefix(ACTION_BROWSE_ITEM_PREFIX) {
        let item_id: u32 = item_id.parse().ok()?;
        let item = net
            .browse_results
            .iter()
            .find(|item| item.item_id == item_id)?;
        return Some(named_item_for(
            item.definition_source,
            item.item_id,
            &item.name,
            item.quality,
            1,
            0,
        ));
    }
    if let Some(auction_id) = action.strip_prefix(ACTION_SELECT_AUCTION_PREFIX) {
        let auction_id: u64 = auction_id.parse().ok()?;
        let listing = [&net.search_results, &net.owned_results, &net.bid_results]
            .into_iter()
            .flatten()
            .find(|listing| listing.auction_id == auction_id)?;
        let item = &listing.item;
        return Some(named_item_for(
            item.definition_source,
            item.item_id,
            &item.name,
            item.quality,
            listing.stack_count,
            item.item_guid,
        ));
    }
    let guid: u64 = action.strip_prefix(ACTION_SELL_ITEM_PREFIX)?.parse().ok()?;
    let item = net
        .inventory
        .as_ref()?
        .items
        .iter()
        .find(|item| item.item_guid == guid)?;
    Some(named_item_for(
        item.definition_source,
        item.item_id,
        &item.name,
        item.quality,
        item.stack_count,
        guid,
    ))
}

/// A comparison tooltip: the `CompareHeader` label and the equipped item's tooltip.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ShoppingTooltip {
    pub header: String,
    pub tooltip: TooltipPresentation,
}

impl ShoppingTooltip {
    pub fn hidden() -> Self {
        Self {
            header: String::new(),
            tooltip: TooltipPresentation::hidden(),
        }
    }
}

/// The equipment slots an item of `inventory_type` replaces (`C_TooltipComparison`
/// `GetItemComparisonInfo`): both rings, both trinkets, and both hands for one-handers.
pub fn comparison_slots(inventory_type: u8) -> &'static [EquipmentSlot] {
    use EquipmentSlot::*;
    match inventory_type {
        1 => &[Head],
        2 => &[Neck],
        3 => &[Shoulder],
        4 => &[Shirt],
        5 | 20 => &[Chest],
        6 => &[Waist],
        7 => &[Legs],
        8 => &[Feet],
        9 => &[Wrist],
        10 => &[Hands],
        11 => &[Finger1, Finger2],
        12 => &[Trinket1, Trinket2],
        13 => &[MainHand, OffHand],
        14 | 22 | 23 => &[OffHand],
        15 | 25 | 26 => &[Ranged, MainHand],
        16 => &[Back],
        17 | 21 => &[MainHand],
        19 => &[Tabard],
        _ => &[],
    }
}

/// Shift comparison of `hovered` (`TooltipUtil.ShouldDoItemComparison`: the
/// `COMPAREITEMS` modifier) with the items equipped in the slots it replaces, at most two.
/// Each shows the equipped item and, below it, `ITEM_DELTA_DESCRIPTION` and the stat
/// changes swapping it for `hovered` would make.
pub fn comparisons(
    hovered: &InventorySlot,
    inventory: &InventoryState,
    player_level: Option<u16>,
) -> Vec<ShoppingTooltip> {
    let Some(catalogs) = item_catalogs() else {
        return Vec::new();
    };
    comparisons_in_catalogs(hovered, inventory, player_level, catalogs)
}

pub fn comparisons_in_catalogs(
    hovered: &InventorySlot,
    inventory: &InventoryState,
    player_level: Option<u16>,
    catalogs: &SourceItemCatalogs,
) -> Vec<ShoppingTooltip> {
    let Some(new) = catalogs
        .catalog(hovered.definition_source)
        .ok()
        .and_then(|catalog| catalog.get(hovered.item_id))
    else {
        return Vec::new();
    };
    comparison_slots(new.inventory_type)
        .iter()
        .filter_map(|slot| inventory.equipped(*slot))
        .filter(|equipped| !equipped.is_empty() && equipped.item_guid != hovered.item_guid)
        .take(2)
        .map(|equipped| {
            shopping_tooltip(
                equipped,
                new,
                hovered.definition_source,
                player_level,
                catalogs,
            )
        })
        .collect()
}

fn shopping_tooltip(
    equipped: &InventorySlot,
    new: &ItemCatalogEntry,
    source: ItemDefinitionSource,
    player_level: Option<u16>,
    catalogs: &SourceItemCatalogs,
) -> ShoppingTooltip {
    let catalog = catalogs.catalog(equipped.definition_source).ok();
    let mut tooltip = crate::item_tooltip::item_tooltip_in_catalog(equipped, player_level, catalog);
    tooltip.lines.push(item_id_line(equipped.item_id));
    let deltas = catalog
        .and_then(|catalog| catalog.get(equipped.item_id))
        .map(|old| stat_deltas_for(source, new, equipped.definition_source, old))
        .unwrap_or_default();
    if !deltas.is_empty() {
        tooltip.lines.push(TooltipLineState::new(String::new()));
        tooltip
            .lines
            .extend(description_lines(DELTA_HEADER, TOOLTIP_DESCRIPTION_COLOR));
        tooltip.lines.extend(deltas);
    }
    ShoppingTooltip {
        header: EQUIPPED_HEADER.to_owned(),
        tooltip,
    }
}

/// `C_TooltipComparison.GetItemComparisonDelta`: what equipping `new` in place of `old`
/// changes: damage per second, armor, then each stat; gains green, losses red.
pub fn stat_deltas(new: &ItemCatalogEntry, old: &ItemCatalogEntry) -> Vec<TooltipLineState> {
    stat_deltas_for(
        ItemDefinitionSource::Retail,
        new,
        ItemDefinitionSource::Retail,
        old,
    )
}

pub fn stat_deltas_for(
    new_source: ItemDefinitionSource,
    new: &ItemCatalogEntry,
    old_source: ItemDefinitionSource,
    old: &ItemCatalogEntry,
) -> Vec<TooltipLineState> {
    let mut lines = Vec::new();
    let dps = |source, entry| weapon_damage_for(source, entry).map_or(0.0, |damage| damage.dps);
    let dps_delta = dps(new_source, new) - dps(old_source, old);
    if (dps_delta * 10.0).round() != 0.0 {
        lines.push(delta_line(
            format!("{dps_delta:+.1} Damage Per Second"),
            dps_delta > 0.0,
        ));
    }
    let armor = item_armor_for(new_source, new) as i32 - item_armor_for(old_source, old) as i32;
    if armor != 0 {
        lines.push(delta_line(format!("{armor:+} Armor"), armor > 0));
    }
    let (new_stats, old_stats) = (
        item_stats_for(new_source, new),
        item_stats_for(old_source, old),
    );
    let value = |stats: &[crate::item_stats::ItemStat], stat| {
        stats
            .iter()
            .filter(|entry| entry.stat == stat)
            .map(|entry| entry.value)
            .sum::<i32>()
    };
    let mut seen = Vec::new();
    for stat in new_stats.iter().chain(&old_stats).map(|entry| entry.stat) {
        if seen.contains(&stat) {
            continue;
        }
        seen.push(stat);
        let delta = value(&new_stats, stat) - value(&old_stats, stat);
        if let (Some((name, _)), true) = (stat_name(stat), delta != 0) {
            let change = crate::item_stats::ItemStat { stat, value: delta };
            lines.push(delta_line(stat_text(change, name), delta > 0));
        }
    }
    lines
}

fn delta_line(text: String, gain: bool) -> TooltipLineState {
    TooltipLineState::colored(
        text,
        if gain {
            GREEN_FONT_COLOR
        } else {
            RED_FONT_COLOR
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::{EquipmentSnapshot, EquippedItem, ItemStack};

    fn rows(tooltip: &TooltipPresentation) -> Vec<&str> {
        tooltip
            .lines
            .iter()
            .map(|line| line.left_text.as_str())
            .collect()
    }

    fn equipped(items: &[(EquipmentSlot, u64, u32)]) -> InventoryState {
        let mut inventory = InventoryState::default();
        inventory.apply_equipment_snapshot(&EquipmentSnapshot {
            items: items
                .iter()
                .map(|&(slot, item_guid, item_id)| EquippedItem {
                    slot,
                    item: ItemStack {
                        definition_source: shared::item_data::ItemDefinitionSource::Retail,
                        item_guid,
                        item_id,
                        count: 1,
                        durability: None,
                        soulbound: true,
                    },
                })
                .collect(),
        });
        inventory
    }

    #[test]
    fn a_merchant_item_reads_its_catalog_lines_and_id() {
        super::super::set_test_data_root();
        let shoes = named_item(2117, "Thin Cloth Shoes", 1, 1);
        assert_eq!(shoes.icon_fdid, item_catalog_entry(2117).unwrap().icon_fdid);
        let tooltip = item_game_tooltip(&shoes, Some(1));
        assert_eq!(tooltip.content.title, "Thin Cloth Shoes");
        assert_eq!(tooltip.record, Some(TooltipRecord::Item(2117)));
        assert!(rows(&tooltip.content).contains(&"1 Armor"));
    }

    #[test]
    fn the_rapier_compares_with_the_equipped_shortsword_and_shows_its_gains() {
        super::super::set_test_data_root();
        let inventory = equipped(&[(EquipmentSlot::MainHand, 9, 25)]);
        let rapier = InventorySlot {
            item_guid: 12,
            ..named_item(1925, "Defias Rapier", 3, 1)
        };
        let shopping = comparisons(&rapier, &inventory, Some(10));
        assert_eq!(shopping.len(), 1);
        let compare = &shopping[0];
        assert_eq!(compare.header, "Equipped");
        assert_eq!(compare.tooltip.title, "Worn Shortsword");
        let lines = rows(&compare.tooltip);
        let header = lines
            .iter()
            .position(|line| line.starts_with("If you replace this item"))
            .expect("delta header");
        assert_eq!(lines[header - 1], "");
        assert_eq!(lines[header - 2], "Item ID: 25");
        // The header wraps to the tooltip width; the changes follow it.
        let deltas = &lines[lines.len() - 3..];
        assert_eq!(
            deltas,
            ["+1.0 Damage Per Second", "+2 Agility", "+2 Critical Strike"]
        );
        assert_eq!(lines[header..lines.len() - 3].join(" "), DELTA_HEADER);
        let first = compare.tooltip.lines.len() - 3;
        assert_eq!(compare.tooltip.lines[first].left_color, GREEN_FONT_COLOR);
    }

    #[test]
    fn rings_compare_with_both_fingers_and_the_hovered_item_itself_never() {
        super::super::set_test_data_root();
        assert_eq!(
            comparison_slots(11),
            [EquipmentSlot::Finger1, EquipmentSlot::Finger2]
        );
        assert_eq!(
            comparison_slots(13),
            [EquipmentSlot::MainHand, EquipmentSlot::OffHand]
        );
        assert!(comparison_slots(0).is_empty());
        let inventory = equipped(&[(EquipmentSlot::MainHand, 9, 25)]);
        let same = InventorySlot {
            item_guid: 9,
            ..named_item(25, "Worn Shortsword", 1, 1)
        };
        assert!(comparisons(&same, &inventory, None).is_empty());
        let reagent = named_item(2589, "Linen Cloth", 1, 20);
        assert!(comparisons(&reagent, &inventory, None).is_empty());
    }

    #[test]
    fn auction_rows_resolve_their_item_and_hide_the_vendor_price() {
        super::super::set_test_data_root();
        use shared::protocol::{AuctionBrowseItem, AuctionInventoryItem, AuctionInventorySnapshot};
        let net = AuctionHouseState {
            browse_results: vec![AuctionBrowseItem {
                definition_source: shared::item_data::ItemDefinitionSource::Retail,
                item_id: 2589,
                name: "Linen Cloth".into(),
                quality: 1,
                required_level: 0,
                lowest_unit_price: 40,
                total_quantity: 60,
            }],
            inventory: Some(AuctionInventorySnapshot {
                gold: 0,
                items: vec![AuctionInventoryItem {
                    definition_source: shared::item_data::ItemDefinitionSource::Retail,
                    item_guid: 77,
                    item_id: 25,
                    name: "Worn Shortsword".into(),
                    quality: 1,
                    required_level: 1,
                    stack_count: 1,
                    vendor_sell_price: 3,
                }],
            }),
            ..Default::default()
        };
        let linen = auction_row_item("auction_browse_item:2589", &net).expect("browse row");
        assert_eq!((linen.item_id, linen.name.as_str()), (2589, "Linen Cloth"));
        let sword = auction_row_item("auction_sell_item:77", &net).expect("sell row");
        assert_eq!((sword.item_id, sword.item_guid), (25, 77));
        assert_eq!(auction_row_item("auction_select:9", &net), None);
        let tooltip = without_sell_price(item_game_tooltip(&linen, None));
        assert!(
            tooltip
                .content
                .lines
                .iter()
                .all(|line| line.money.is_none())
        );
    }

    #[test]
    fn losses_are_red() {
        super::super::set_test_data_root();
        let axe = item_catalog_entry(3191).unwrap();
        let sword = item_catalog_entry(25).unwrap();
        let lines = stat_deltas(sword, axe);
        assert!(lines.iter().all(|line| line.left_color == RED_FONT_COLOR));
        assert_eq!(lines[0].left_text, "-1.6 Damage Per Second");
        assert!(
            rows(&TooltipPresentation {
                lines: lines.clone(),
                ..TooltipPresentation::hidden()
            })
            .contains(&"-4 Stamina")
        );
    }
}
