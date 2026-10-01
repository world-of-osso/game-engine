//! The item on the cursor (Retail `GetCursorInfo` "item" / "merchant"): a bag or
//! equipped item picked up with `PickupContainerItem` / `PickupInventoryItem` or
//! split off with `SplitContainerItem`, or a vendor item picked up with
//! `PickupMerchantItem`. Clicking a target with it turns into the server request
//! Retail sends for that drop; the server owns every move and answers with
//! inventory deltas.

#[cfg(not(godot_host))]
use bevy::prelude::*;
use shared::protocol::{DestroyItem, EquipmentSlot, ItemLocation, SplitItem, SwapItem};

use crate::bag_data::{InventoryRequest, InventoryState, ItemQuality};
use crate::merchant_data::{MerchantRequest, MerchantState, MerchantTab};
use crate::ui::popup::PopupSpec;

/// `StaticPopupDialogs["DELETE_ITEM"]` / `["DELETE_GOOD_ITEM"]`.
pub const DELETE_ITEM: &str = "DELETE_ITEM";
pub const DELETE_GOOD_ITEM: &str = "DELETE_GOOD_ITEM";

#[cfg_attr(not(godot_host), derive(Resource))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CursorItem {
    #[default]
    Empty,
    /// `count` items of the stack at `from`; `split` when fewer than all of it
    /// were split off (the drop sends `SplitItem`, not `SwapItem`).
    Inventory {
        from: ItemLocation,
        item_id: u32,
        icon_fdid: u32,
        count: u32,
        split: bool,
    },
    /// `purchases` of vendor slot `slot`; dropped on a bag slot it is bought there.
    Merchant {
        slot: u32,
        item_id: u32,
        icon_fdid: u32,
        purchases: u32,
    },
}

/// What a click with the cursor landed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorTarget {
    Location(ItemLocation),
    /// A vendor item cell (`MerchantItemButton_OnClick`, cell index on the page).
    MerchantItem(usize),
    /// The MerchantFrame outside its cells (`OnMouseUp` → `PickupMerchantItem(0)`).
    MerchantFrame,
    /// No frame under the cursor: the item is dropped on the world.
    World,
}

/// What a click asks for besides the new cursor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CursorEffect {
    Inventory(InventoryRequest),
    Merchant(MerchantRequest),
    /// `DELETE_ITEM_CONFIRM`: ask before destroying the cursor item.
    ConfirmDestroy(DestroyConfirm),
}

/// The popup `DELETE_ITEM_CONFIRM` raises (UIParent.lua:1460-1474).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DestroyConfirm {
    pub name: String,
    /// Rare and better, heirlooms excepted: `DELETE_GOOD_ITEM`, which makes the
    /// player type `DELETE`.
    pub good: bool,
}

/// `DELETE_ITEM` ("Do you want to destroy %s?"); Rare and better get
/// `DELETE_GOOD_ITEM`, where Yes waits for `DELETE` typed into its edit box.
pub fn destroy_popup(confirm: &DestroyConfirm) -> PopupSpec {
    let (key, text) = if confirm.good {
        (
            DELETE_GOOD_ITEM,
            format!(
                "Do you want to destroy {}?\n\nType \"DELETE\" into the field to confirm.",
                confirm.name
            ),
        )
    } else {
        (
            DELETE_ITEM,
            format!("Do you want to destroy {}?", confirm.name),
        )
    };
    PopupSpec {
        key: key.into(),
        text,
        accept_label: "Yes".into(),
        cancel_label: Some("No".into()),
        timeout: None,
        // DELETE_ITEM_CONFIRM_STRING.
        confirm_text: confirm.good.then(|| "DELETE".into()),
    }
}

impl CursorItem {
    pub fn is_empty(&self) -> bool {
        *self == Self::Empty
    }

    pub fn icon_fdid(&self) -> Option<u32> {
        match self {
            Self::Empty => None,
            Self::Inventory { icon_fdid, .. } | Self::Merchant { icon_fdid, .. } => {
                Some(*icon_fdid)
            }
        }
    }

    /// The bag or equipment location the cursor item was picked up from; Retail
    /// shows that slot locked while it is on the cursor.
    pub fn source(&self) -> Option<ItemLocation> {
        match self {
            Self::Inventory { from, .. } => Some(*from),
            _ => None,
        }
    }

    /// `SplitContainerItem`: `count` of the stack at `from` onto an empty cursor;
    /// all of it is the whole stack.
    pub fn split_from(inventory: &InventoryState, from: ItemLocation, count: u32) -> Self {
        let Some(item) = inventory
            .item_at(from)
            .filter(|item| (1..=item.count.max(1)).contains(&count))
        else {
            return Self::Empty;
        };
        Self::Inventory {
            from,
            item_id: item.item_id,
            icon_fdid: item.icon_fdid,
            count,
            split: count < item.count,
        }
    }

    /// `PickupMerchantItem` with the chosen number of purchases.
    pub fn from_merchant(merchant: &MerchantState, index: usize, purchases: u32) -> Self {
        let Some(item) = merchant.page_items().get(index) else {
            return Self::Empty;
        };
        Self::Merchant {
            slot: item.slot,
            item_id: item.item_id,
            icon_fdid: crate::item_icons::item_icon_fdid(item.item_id).unwrap_or(0),
            purchases,
        }
    }

    /// A left click on `target`: picks up, drops, swaps, buys or sells like
    /// Retail's `PickupContainerItem` / `PickupInventoryItem` / `PickupMerchantItem`.
    pub fn click(
        &mut self,
        target: CursorTarget,
        inventory: &InventoryState,
        merchant: &MerchantState,
    ) -> Option<CursorEffect> {
        match std::mem::take(self) {
            Self::Empty => {
                *self = pick_up(target, inventory, merchant);
                None
            }
            Self::Inventory {
                from,
                count,
                split,
                item_id,
                icon_fdid,
            } => {
                let held = Held { from, count, split };
                let effect = drop_inventory_item(held, target, inventory, merchant);
                if matches!(effect, Some(CursorEffect::ConfirmDestroy(_))) {
                    // The item stays on the cursor while the popup asks.
                    *self = Self::Inventory {
                        from,
                        count,
                        split,
                        item_id,
                        icon_fdid,
                    };
                }
                effect
            }
            Self::Merchant {
                slot,
                item_id,
                purchases,
                ..
            } => match target {
                CursorTarget::Location(location @ ItemLocation::Bag { .. }) => {
                    Some(CursorEffect::Merchant(MerchantRequest::Buy {
                        slot,
                        item_id,
                        count: purchases,
                        destination: Some(location),
                    }))
                }
                _ => None,
            },
        }
    }

    /// The accepted `DELETE_ITEM` popup (`DeleteCursorItem`).
    pub fn destroy(&mut self) -> Option<InventoryRequest> {
        match std::mem::take(self) {
            Self::Inventory {
                from, count, split, ..
            } => Some(InventoryRequest::Destroy(DestroyItem {
                location: from,
                count: if split { count } else { 0 },
            })),
            _ => None,
        }
    }

    /// Retail clears the cursor when its item is gone from where it was picked up
    /// (sold, moved by the server) or its vendor closed.
    pub fn clear_if_stale(&mut self, inventory: &InventoryState, merchant: &MerchantState) {
        let stale = match self {
            Self::Empty => false,
            Self::Inventory { from, item_id, .. } => inventory
                .item_at(*from)
                .is_none_or(|item| item.item_id != *item_id),
            Self::Merchant { .. } => !merchant.is_open(),
        };
        if stale {
            *self = Self::Empty;
        }
    }
}

#[derive(Clone, Copy)]
struct Held {
    from: ItemLocation,
    count: u32,
    split: bool,
}

fn pick_up(
    target: CursorTarget,
    inventory: &InventoryState,
    merchant: &MerchantState,
) -> CursorItem {
    match target {
        CursorTarget::Location(location) => {
            let Some(item) = inventory.item_at(location) else {
                return CursorItem::Empty;
            };
            CursorItem::Inventory {
                from: location,
                item_id: item.item_id,
                icon_fdid: item.icon_fdid,
                count: item.count.max(1),
                split: false,
            }
        }
        // The buyback tab buys back on click instead (the merchant frame's own input).
        CursorTarget::MerchantItem(index) if merchant.tab == MerchantTab::Merchant => {
            CursorItem::from_merchant(merchant, index, 1)
        }
        CursorTarget::MerchantItem(_) | CursorTarget::MerchantFrame | CursorTarget::World => {
            CursorItem::Empty
        }
    }
}

fn drop_inventory_item(
    held: Held,
    target: CursorTarget,
    inventory: &InventoryState,
    merchant: &MerchantState,
) -> Option<CursorEffect> {
    let request = match target {
        // Clicking the source slot puts the item back.
        CursorTarget::Location(to) if to == held.from => return None,
        CursorTarget::Location(to @ ItemLocation::Bag { .. }) if held.split => {
            InventoryRequest::Split(SplitItem {
                from: held.from,
                to,
                count: held.count,
            })
        }
        // Part of a stack cannot be equipped; the split returns to its stack.
        CursorTarget::Location(ItemLocation::Equipment(_)) if held.split => return None,
        CursorTarget::Location(to) => InventoryRequest::Swap(SwapItem {
            from: held.from,
            to,
        }),
        CursorTarget::MerchantItem(_) | CursorTarget::MerchantFrame => {
            return sell(held, inventory, merchant);
        }
        CursorTarget::World => {
            let item = inventory.item_at(held.from)?;
            return Some(CursorEffect::ConfirmDestroy(DestroyConfirm {
                name: item.name.clone(),
                good: matches!(
                    item.quality,
                    ItemQuality::Rare
                        | ItemQuality::Epic
                        | ItemQuality::Legendary
                        | ItemQuality::Artifact
                ),
            }));
        }
    };
    Some(CursorEffect::Inventory(request))
}

/// `PickupMerchantItem(0)` with an item on the cursor sells it (the split count,
/// or the whole stack).
fn sell(held: Held, inventory: &InventoryState, merchant: &MerchantState) -> Option<CursorEffect> {
    if !merchant.is_open() {
        return None;
    }
    let item = inventory.item_at(held.from)?;
    Some(CursorEffect::Merchant(MerchantRequest::Sell {
        item_guid: item.item_guid,
        count: if held.split { held.count } else { 0 },
    }))
}

/// `INVSLOT_*` order of [`EquipmentSlot::ALL`], the paperdoll's slot actions.
pub fn equipment_slot_index(slot: EquipmentSlot) -> usize {
    EquipmentSlot::ALL
        .iter()
        .position(|candidate| *candidate == slot)
        .expect("EquipmentSlot::ALL lists every slot")
}

#[cfg(test)]
#[path = "cursor_item_tests.rs"]
mod tests;
