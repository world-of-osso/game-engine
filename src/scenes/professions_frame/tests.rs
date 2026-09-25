use game_engine::bag_data::InventorySlot;

use super::*;

fn slot(item_id: u32, count: u32) -> InventorySlot {
    InventorySlot {
        item_id,
        count,
        icon_fdid: 1,
        ..Default::default()
    }
}

#[test]
fn bag_counts_add_stacks_across_bags() {
    let mut bags = InventoryState::default();
    bags.set_item(0, 0, slot(2589, 3));
    bags.set_item(0, 5, slot(2589, 2));
    bags.set_item(0, 6, slot(2996, 1));
    assert_eq!(bag_count(Some(&bags), 2589), 5);
    assert_eq!(bag_count(Some(&bags), 2996), 1);
    assert_eq!(bag_count(Some(&bags), 2320), 0);
    assert_eq!(bag_count(None, 2589), 0);
}
