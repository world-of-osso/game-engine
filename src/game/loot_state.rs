//! Bevy loot markers and requests; shared window state and policy live in loot_data.

use bevy::prelude::*;

pub use crate::loot_data::{
    LootState, NpcRightClick, auto_loot, coin_icon_fdid, loot_chat_text, money_lines,
    npc_right_click,
};

impl Resource for LootState {}

/// The corpse has loot for the local player (`CorpseLootable`): sparkle, loot cursor,
/// right-click loots.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Lootable;

impl LootState {
    /// `LootSlot(i)` for every slot of the open window, in slot order.
    pub fn take_all(&self) -> Vec<LootRequest> {
        self.slots
            .iter()
            .map(|entry| LootRequest::Take { slot: entry.slot })
            .collect()
    }
}

/// Loot frame actions; the network layer sends them for the open corpse.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootRequest {
    /// Right-click on a lootable corpse (main-world entity).
    Open { corpse: Entity, auto: bool },
    /// `LootSlot(i)`.
    Take { slot: u8 },
    /// `CloseLoot()`.
    Release,
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::{LootContent, LootResponse, LootSlot};

    #[test]
    fn taking_all_asks_for_every_slot_still_on_the_corpse() {
        let mut state = LootState::default();
        assert!(state.take_all().is_empty(), "no window open");
        state.open(LootResponse {
            corpse: 42,
            auto: false,
            slots: vec![
                LootSlot {
                    slot: 0,
                    content: LootContent::Money { copper: 3 },
                },
                LootSlot {
                    slot: 1,
                    content: LootContent::Item {
                        item_id: 755,
                        name: "Melted Candle".into(),
                        quality: 0,
                        count: 1,
                    },
                },
            ],
        });
        assert_eq!(
            state.take_all(),
            vec![LootRequest::Take { slot: 0 }, LootRequest::Take { slot: 1 }]
        );
        state.remove(42, 0);
        assert_eq!(state.take_all(), vec![LootRequest::Take { slot: 1 }]);
    }
}
