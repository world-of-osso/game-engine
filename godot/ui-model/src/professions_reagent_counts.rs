//! Owned recipe reagents (Retail `ItemUtil.GetCraftingReagentCount`).
use std::collections::BTreeMap;

use shared::item_data::ItemDefinitionSource;
use shared::protocol::BankContents;

use crate::bag_data::InventorySlot;

/// Retail-source reagents in the bags, by item id.
pub fn crafting_reagent_counts(
    bags: &[Vec<InventorySlot>],
    _banks: &[&BankContents],
) -> BTreeMap<u32, u32> {
    let mut counts = BTreeMap::<u32, u32>::new();
    for item in bags.iter().flatten() {
        if item.definition_source == ItemDefinitionSource::Retail {
            let count = counts.entry(item.item_id).or_default();
            *count = count.saturating_add(item.count);
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::{BankTabView, BankType, ItemStack};

    const LINEN: u32 = 2589;
    const SILK: u32 = 4306;

    fn bag_stack(item_id: u32, count: u32, source: ItemDefinitionSource) -> InventorySlot {
        InventorySlot {
            item_id,
            count,
            definition_source: source,
            ..Default::default()
        }
    }

    fn bank(bank: BankType, stacks: &[(u32, u32, ItemDefinitionSource)]) -> BankContents {
        let slots = stacks
            .iter()
            .enumerate()
            .map(|(guid, &(item_id, count, definition_source))| {
                Some(ItemStack {
                    item_guid: guid as u64 + 1,
                    item_id,
                    definition_source,
                    count,
                    durability: None,
                    soulbound: false,
                    crafting_origin: None,
                })
            })
            .chain([None])
            .collect();
        BankContents {
            bank,
            tabs: vec![BankTabView {
                name: "Tab 1".into(),
                icon: 0,
                deposit_flags: 0,
                slots,
            }],
            next_tab_cost: None,
            money: None,
        }
    }

    /// Retail counts the bags, the character bank and the Warband bank
    /// (`includeBank`, `includeReagentBank`, `includeAccountBank`); the Mainline UI
    /// never sets `characterInventoryOnly`.
    #[test]
    #[ignore = "RED: CRAFTING_REAGENTS_FROM_BANK"]
    fn crafting_reagent_counts_include_character_and_warband_banks() {
        use ItemDefinitionSource::{Forever70205, Retail};
        let bags = vec![
            vec![
                bag_stack(LINEN, 1, Retail),
                bag_stack(LINEN, 7, Forever70205),
            ],
            vec![InventorySlot::default(), bag_stack(SILK, 2, Retail)],
        ];
        let character = bank(BankType::Character, &[(LINEN, 2, Retail)]);
        let warband = bank(
            BankType::Account,
            &[
                (LINEN, 5, Retail),
                (LINEN, 3, Forever70205),
                (SILK, 1, Retail),
            ],
        );
        let counts = crafting_reagent_counts(&bags, &[&character, &warband]);
        assert_eq!(counts, BTreeMap::from([(LINEN, 8), (SILK, 3)]));
    }
}
