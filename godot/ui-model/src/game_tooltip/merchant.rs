//! Merchant and buyback tooltip content (docs/specs/merchant-frame.md).

use shared::protocol::{BuybackItem, ItemDurability};

use super::GameTooltip;
use super::item::{named_item, named_item_for};
use crate::bag_data::InventorySlot;
use crate::merchant_data::{MerchantState, MerchantTab};
use crate::tooltip_presentation::{
    TOOLTIP_DESCRIPTION_COLOR, TOOLTIP_WHITE, TooltipLineState, TooltipPresentation,
    description_lines,
};

/// The item of cell `index` as `MerchantItemButton_OnEnter` shows it (MF.lua:710-724):
/// `SetMerchantItem` on the merchant tab, the full item tooltip of one purchase with a
/// new item's durability (`DURABILITY_TEMPLATE`); `SetBuybackItem` on the buyback tab.
pub fn merchant_cell_item(merchant: &MerchantState, index: usize) -> Option<InventorySlot> {
    match merchant.tab {
        MerchantTab::Merchant => {
            let item = merchant.page_items().get(index)?;
            Some(InventorySlot {
                durability: item
                    .max_durability
                    .map(|max| ItemDurability { current: max, max }),
                ..named_item(item.item_id, &item.name, item.quality, item.stack_count)
            })
        }
        MerchantTab::Buyback => merchant.buyback.get(index).map(buyback_item),
    }
}

/// `SetBuybackItem`: the sold stack.
pub fn buyback_item(item: &BuybackItem) -> InventorySlot {
    named_item_for(
        item.definition_source,
        item.item_id,
        &item.name,
        item.quality,
        item.count,
        0,
    )
}

/// `RED_FONT_COLOR`.
const RED_FONT_COLOR: [f32; 4] = [1.0, 0.1, 0.1, 1.0];
/// `GUILDBANK_REPAIR_INSUFFICIENT_FUNDS`.
const INSUFFICIENT_FUNDS: &str = "Insufficient funds to repair all items";

/// A button tooltip: `GameTooltip:SetText(text)` in `HIGHLIGHT_FONT_COLOR`.
fn button_tooltip(title: &str, lines: Vec<TooltipLineState>) -> GameTooltip {
    GameTooltip::new(
        TooltipPresentation {
            title: title.to_owned(),
            title_color: TOOLTIP_WHITE,
            lines,
            ..TooltipPresentation::hidden()
        },
        None,
    )
}

/// `MerchantSellAllJunkButton` `OnEnter`: `SELL_ALL_JUNK_ITEMS` (MF.xml:207-211).
pub fn sell_all_junk_tooltip() -> GameTooltip {
    button_tooltip("Sell All Junk Items", Vec::new())
}

/// `MerchantRepairItemButton` `OnEnter`: `REPAIR_AN_ITEM` (MF.xml:300-303).
pub fn repair_item_tooltip() -> GameTooltip {
    button_tooltip("Repair an Item", Vec::new())
}

/// `MerchantRepairAllButton` `OnEnter` (MF.xml:240-252): with something to repair,
/// `REPAIR_ALL_ITEMS`, the cost (`GameTooltip_AddMoneyLine`) and, when it exceeds the
/// player's money, `GUILDBANK_REPAIR_INSUFFICIENT_FUNDS` in red. Nothing damaged shows
/// an empty tooltip, which is not drawn.
pub fn repair_all_tooltip(cost: u64, money: u64) -> Option<GameTooltip> {
    if cost == 0 {
        return None;
    }
    let mut lines = vec![TooltipLineState::money(String::new(), cost)];
    if cost > money {
        lines.push(TooltipLineState::colored(
            INSUFFICIENT_FUNDS,
            RED_FONT_COLOR,
        ));
    }
    Some(button_tooltip("Repair All Items", lines))
}

/// `MerchantGuildBankRepairButton` `OnEnter` (MF.xml:332-364): with something to repair,
/// `REPAIR_ALL_ITEMS` and the cost, then `GUILDBANK_REPAIR` (wrapped, `NORMAL_FONT_COLOR`)
/// over the guild money left for repairs as a highlight line. When the cost exceeds it,
/// `GUILDBANK_REPAIR_PERSONAL` and the difference if the player's money covers that,
/// else `GUILDBANK_REPAIR_INSUFFICIENT_FUNDS` in red. `guild_money` is the guild money the
/// player may spend (`min(GetGuildBankWithdrawMoney(), GetGuildBankMoney())`).
pub fn guild_repair_tooltip(cost: u64, guild_money: u64, money: u64) -> Option<GameTooltip> {
    if cost == 0 {
        return None;
    }
    let mut lines = vec![TooltipLineState::money(String::new(), cost)];
    lines.extend(description_lines(
        "Remaining amount for today's Guild Bank repairs:",
        TOOLTIP_DESCRIPTION_COLOR,
    ));
    lines.push(TooltipLineState::money(String::new(), guild_money));
    if cost > guild_money {
        let personal = cost - guild_money;
        if money >= personal {
            lines.extend(description_lines(
                "Personal amount to be spent:",
                TOOLTIP_DESCRIPTION_COLOR,
            ));
            lines.push(TooltipLineState::money(String::new(), personal));
        } else {
            lines.push(TooltipLineState::colored(
                INSUFFICIENT_FUNDS,
                RED_FONT_COLOR,
            ));
        }
    }
    Some(button_tooltip("Repair All Items", lines))
}

#[cfg(test)]
mod tests {
    use shared::protocol::{
        BuybackItem, EquipmentSlot, EquipmentSnapshot, EquippedItem, ItemStack, VendorInventory,
        VendorItem,
    };

    use super::{
        guild_repair_tooltip, merchant_cell_item, repair_all_tooltip, repair_item_tooltip,
        sell_all_junk_tooltip,
    };
    use crate::bag_data::InventoryState;
    use crate::game_tooltip::TooltipRecord;
    use crate::game_tooltip::item::{comparisons, item_game_tooltip};
    use crate::item_tooltip::GREEN_FONT_COLOR;
    use crate::merchant_data::{MerchantState, MerchantTab};
    use crate::tooltip_presentation::{
        TOOLTIP_DESCRIPTION_COLOR, TooltipLineState, description_lines,
    };

    fn vendor_item(item_id: u32, name: &str, count: u32, durability: Option<u32>) -> VendorItem {
        VendorItem {
            slot: 0,
            item_id,
            name: name.into(),
            quality: 1,
            price: 25,
            stack_count: count,
            max_stack: 20,
            num_available: Some(7),
            usable: true,
            max_durability: durability,
        }
    }

    fn merchant(items: Vec<VendorItem>) -> MerchantState {
        super::super::set_test_data_root();
        let mut merchant = MerchantState::default();
        merchant.apply_inventory(
            VendorInventory {
                npc: 4294966979,
                can_repair: false,
                guild_repair_money: None,
                items,
            },
            "Fixture Vendor".into(),
        );
        merchant
    }

    fn rows(lines: &[TooltipLineState]) -> Vec<(&str, &str)> {
        lines
            .iter()
            .map(|line| (line.left_text.as_str(), line.right_text.as_str()))
            .collect()
    }

    #[test]
    fn a_vendor_weapon_shows_its_full_item_tooltip_at_full_durability() {
        let merchant = merchant(vec![vendor_item(25, "Fixture Vendor Sword", 1, Some(20))]);
        let sword = merchant_cell_item(&merchant, 0).unwrap();
        let tooltip = item_game_tooltip(&sword, Some(1));
        assert_eq!(tooltip.content.title, "Fixture Vendor Sword");
        assert_eq!(tooltip.record, Some(TooltipRecord::Item(25)));
        // SetMerchantItem: a new, unbound item; ItemSparse 25 binds when equipped.
        assert_eq!(
            rows(&tooltip.content.lines),
            vec![
                ("Item Level 1", ""),
                ("Binds when equipped", ""),
                ("Main Hand", "Sword"),
                ("0 - 1 Damage", "Speed 2.60"),
                ("(0.4 damage per second)", ""),
                ("Durability 20 / 20", ""),
                ("Sell Price:", ""),
            ]
        );
        assert_eq!(tooltip.content.lines.last().unwrap().money, Some(3));
    }

    #[test]
    fn a_vendor_bundle_prices_its_whole_purchase_without_durability() {
        let merchant = merchant(vec![vendor_item(2589, "Fixture Linen Bundle", 5, None)]);
        let linen = merchant_cell_item(&merchant, 0).unwrap();
        assert_eq!(linen.durability, None);
        let tooltip = item_game_tooltip(&linen, None);
        assert_eq!(rows(&tooltip.content.lines), vec![("Sell Price:", "")]);
        assert_eq!(tooltip.content.lines[0].money, Some(13 * 5));
        assert_eq!(merchant_cell_item(&merchant, 1), None);
    }

    #[test]
    fn buyback_cells_show_the_sold_stack() {
        let mut merchant = merchant(vec![vendor_item(25, "Fixture Vendor Sword", 1, Some(20))]);
        merchant.buyback.push(BuybackItem {
            definition_source: shared::item_data::ItemDefinitionSource::Retail,
            slot: 0,
            item_id: 2589,
            name: "Fixture Returned Linen".into(),
            quality: 3,
            count: 3,
            price: 39,
        });
        merchant.set_tab(MerchantTab::Buyback);
        let linen = merchant_cell_item(&merchant, 0).unwrap();
        assert_eq!(
            (linen.item_id, linen.name.as_str(), linen.count),
            (2589, "Fixture Returned Linen", 3)
        );
        let tooltip = item_game_tooltip(&linen, None);
        assert_eq!(tooltip.content.lines[0].money, Some(13 * 3));
    }

    #[test]
    fn shift_compares_a_vendor_axe_with_the_equipped_sword() {
        let merchant = merchant(vec![vendor_item(3191, "Arced War Axe", 1, Some(75))]);
        let axe = merchant_cell_item(&merchant, 0).unwrap();
        let mut inventory = InventoryState::default();
        inventory.apply_equipment_snapshot(&EquipmentSnapshot {
            items: vec![EquippedItem {
                slot: EquipmentSlot::MainHand,
                item: ItemStack {
                    definition_source: shared::item_data::ItemDefinitionSource::Retail,
                    item_guid: 9,
                    item_id: 25,
                    count: 1,
                    durability: None,
                    soulbound: true,
                },
            }],
        });
        let shopping = comparisons(&axe, &inventory, Some(10));
        assert_eq!(shopping.len(), 1);
        assert_eq!(shopping[0].header, "Equipped");
        assert_eq!(shopping[0].tooltip.title, "Worn Shortsword");
        // At squished item levels the server applies 4-10 @ 3.6 (axe, 1.94 DPS) and
        // 0-1 @ 2.6 (sword, 0.19 DPS): a 1.75 DPS gain.
        let gain = shopping[0]
            .tooltip
            .lines
            .iter()
            .find(|line| line.left_text == "+1.8 Damage Per Second")
            .expect("damage gain");
        assert_eq!(gain.left_color, GREEN_FONT_COLOR);
    }

    #[test]
    fn guild_repair_shows_the_cost_and_the_guild_money_left() {
        assert!(guild_repair_tooltip(0, 984, 1000).is_none());
        let remaining = description_lines(
            "Remaining amount for today's Guild Bank repairs:",
            TOOLTIP_DESCRIPTION_COLOR,
        );
        let covered = guild_repair_tooltip(16, 984, 0).unwrap();
        assert_eq!(covered.content.title, "Repair All Items");
        let mut expected = vec![TooltipLineState::money(String::new(), 16)];
        expected.extend(remaining.clone());
        expected.push(TooltipLineState::money(String::new(), 984));
        assert_eq!(covered.content.lines, expected);

        // 10 copper of guild money: the player's 1100 covers the other 1006.
        let personal = guild_repair_tooltip(1016, 10, 1100).unwrap();
        let mut expected = vec![TooltipLineState::money(String::new(), 1016)];
        expected.extend(remaining.clone());
        expected.push(TooltipLineState::money(String::new(), 10));
        expected.extend(description_lines(
            "Personal amount to be spent:",
            TOOLTIP_DESCRIPTION_COLOR,
        ));
        expected.push(TooltipLineState::money(String::new(), 1006));
        assert_eq!(personal.content.lines, expected);

        let short = guild_repair_tooltip(1016, 10, 5).unwrap();
        assert_eq!(
            short.content.lines.last().unwrap(),
            &TooltipLineState::colored(
                "Insufficient funds to repair all items",
                [1.0, 0.1, 0.1, 1.0]
            )
        );
    }

    #[test]
    fn service_buttons_show_their_retail_strings_in_white() {
        for (tooltip, title) in [
            (sell_all_junk_tooltip(), "Sell All Junk Items"),
            (repair_item_tooltip(), "Repair an Item"),
        ] {
            assert!(tooltip.content.visible);
            assert_eq!(tooltip.content.title, title);
            assert_eq!(tooltip.content.title_color, [1.0, 1.0, 1.0, 1.0]);
            assert!(tooltip.content.lines.is_empty());
            assert_eq!(tooltip.record, None);
        }
    }

    #[test]
    fn repair_all_shows_cost_and_red_shortfall_only_when_damaged() {
        assert_eq!(repair_all_tooltip(0, 1000), None);
        let affordable = repair_all_tooltip(16, 1000).unwrap();
        assert_eq!(affordable.content.title, "Repair All Items");
        assert_eq!(
            affordable.content.lines,
            vec![TooltipLineState::money(String::new(), 16)]
        );
        let short = repair_all_tooltip(1016, 1000).unwrap();
        assert_eq!(
            short.content.lines,
            vec![
                TooltipLineState::money(String::new(), 1016),
                TooltipLineState::colored(
                    "Insufficient funds to repair all items",
                    [1.0, 0.1, 0.1, 1.0]
                ),
            ]
        );
    }
}
