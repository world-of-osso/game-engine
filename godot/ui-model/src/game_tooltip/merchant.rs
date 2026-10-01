//! Merchant and buyback tooltip content (docs/specs/merchant-frame.md).

#[cfg(test)]
mod tests {
    use shared::protocol::{BuybackItem, VendorInventory, VendorItem};

    use crate::game_tooltip::item::{item_game_tooltip, named_item};
    use crate::game_tooltip::{GameTooltip, TooltipRecord, TooltipScreen, place};
    use crate::merchant_data::{MerchantState, MerchantTab};
    use crate::tooltip_presentation::{TooltipLineState, item_id_line};

    fn vendor(count: u32, stock: Option<u32>) -> VendorInventory {
        VendorInventory {
            npc: 4294966979,
            can_repair: false,
            items: vec![VendorItem {
                slot: 0,
                item_id: 2589,
                name: "Fixture Linen Bundle".into(),
                quality: 2,
                price: 25,
                stack_count: count,
                max_stack: 20,
                num_available: stock,
                usable: true,
            }],
        }
    }

    fn tooltip_for_cell(merchant: &MerchantState) -> GameTooltip {
        super::super::set_test_data_root();
        let (item_id, name, quality, count, _stock) = merchant.cell_item(0).unwrap();
        // Exercise the current native merchant content route before its port.
        item_game_tooltip(&named_item(item_id, name, quality, count), Some(1))
    }

    fn assert_content(
        tooltip: GameTooltip,
        item_id: u32,
        name: &str,
        color: [f32; 4],
        rows: &[(&str, &str)],
    ) {
        assert!(tooltip.content.visible);
        assert_eq!(tooltip.content.title, name);
        assert_eq!(tooltip.content.title_color, color);
        assert_eq!(tooltip.record, Some(TooltipRecord::Item(item_id)));
        let mut expected: Vec<_> = rows
            .iter()
            .map(|&(label, value)| TooltipLineState::key_value(label, value))
            .collect();
        assert_eq!(tooltip.content.lines, expected);
        expected.push(item_id_line(item_id));
        let placed = place(
            tooltip,
            TooltipScreen {
                size: [1280.0, 720.0],
                cursor: [0.0, 0.0],
            },
        );
        assert_eq!(placed.lines, expected);
    }

    #[test]
    fn finite_bundle_has_stack_and_stock_rows_then_common_item_id() {
        let mut merchant = MerchantState::default();
        merchant.apply_inventory(vendor(5, Some(7)), "Fixture Vendor".into());
        assert_content(
            tooltip_for_cell(&merchant),
            2589,
            "Fixture Linen Bundle",
            [0.12, 1.0, 0.0, 1.0],
            &[("Stack Count", "5"), ("In Stock", "7")],
        );
    }

    #[test]
    fn single_unlimited_item_has_neither_stack_nor_stock_row() {
        let mut inventory = vendor(1, None);
        inventory.items[0].item_id = 4865;
        inventory.items[0].name = "Fixture Single Pelt".into();
        inventory.items[0].quality = 0;
        let mut merchant = MerchantState::default();
        merchant.apply_inventory(inventory, "Fixture Vendor".into());
        assert_content(
            tooltip_for_cell(&merchant),
            4865,
            "Fixture Single Pelt",
            [0.62, 0.62, 0.62, 1.0],
            &[],
        );
    }

    #[test]
    fn refreshed_bundle_uses_count_two_and_keeps_zero_stock_visible() {
        let mut merchant = MerchantState::default();
        merchant.apply_inventory(vendor(5, Some(7)), "Fixture Vendor".into());
        let before = tooltip_for_cell(&merchant);
        merchant.apply_inventory(vendor(2, Some(0)), String::new());
        assert_content(
            tooltip_for_cell(&merchant),
            2589,
            "Fixture Linen Bundle",
            [0.12, 1.0, 0.0, 1.0],
            &[("Stack Count", "2"), ("In Stock", "0")],
        );
        assert_content(
            before,
            2589,
            "Fixture Linen Bundle",
            [0.12, 1.0, 0.0, 1.0],
            &[("Stack Count", "5"), ("In Stock", "7")],
        );
    }

    #[test]
    fn buyback_three_uses_concrete_name_and_rare_quality_without_stock() {
        let mut merchant = MerchantState::default();
        merchant.apply_inventory(vendor(5, Some(7)), "Fixture Vendor".into());
        merchant.buyback.push(BuybackItem {
            slot: 0,
            item_id: 2589,
            name: "Fixture Returned Linen".into(),
            quality: 3,
            count: 3,
            price: 12,
        });
        merchant.set_tab(MerchantTab::Buyback);
        assert_content(
            tooltip_for_cell(&merchant),
            2589,
            "Fixture Returned Linen",
            [0.0, 0.44, 0.87, 1.0],
            &[("Stack Count", "3")],
        );
    }

    #[test]
    fn titles_keep_the_merchant_quality_map() {
        for (quality, color) in [
            (0, [0.62, 0.62, 0.62, 1.0]),
            (1, [1.0, 1.0, 1.0, 1.0]),
            (2, [0.12, 1.0, 0.0, 1.0]),
            (3, [0.0, 0.44, 0.87, 1.0]),
            (4, [0.64, 0.21, 0.93, 1.0]),
            (5, [1.0, 0.5, 0.0, 1.0]),
            (6, [0.9, 0.8, 0.5, 1.0]),
            (7, [0.9, 0.8, 0.5, 1.0]),
            (255, [1.0, 1.0, 1.0, 1.0]),
        ] {
            let mut inventory = vendor(1, None);
            inventory.items[0].quality = quality;
            let mut merchant = MerchantState::default();
            merchant.apply_inventory(inventory, "Fixture Vendor".into());
            assert_eq!(tooltip_for_cell(&merchant).content.title_color, color);
        }
    }
}
