pub(super) use super::*;

mod bank;
mod drag_drop;
mod inventory;
mod mutations;

pub(super) fn test_item(name: &str) -> InventorySlot {
    InventorySlot {
        icon_fdid: 1,
        count: 1,
        quality: ItemQuality::Common,
        name: name.into(),
    }
}
