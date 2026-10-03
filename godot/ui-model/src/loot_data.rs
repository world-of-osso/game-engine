//! Shared loot window state and Retail loot policies, independent of the runtime.

use shared::protocol::{LootContent, LootResponse, LootSlot};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LootState {
    /// Server entity bits of the open corpse.
    pub corpse: Option<u64>,
    /// Opened by auto-loot (`LOOT_OPENED isAutoLoot`).
    pub auto: bool,
    pub slots: Vec<LootSlot>,
}

impl LootState {
    pub fn is_open(&self) -> bool {
        self.corpse.is_some()
    }

    pub fn open(&mut self, response: LootResponse) {
        self.corpse = Some(response.corpse);
        self.auto = response.auto;
        self.slots = response.slots;
    }

    /// `LOOT_SLOT_CLEARED`: the taken slot's content.
    pub fn remove(&mut self, corpse: u64, slot: u8) -> Option<LootContent> {
        if self.corpse != Some(corpse) {
            return None;
        }
        let index = self.slots.iter().position(|entry| entry.slot == slot)?;
        Some(self.slots.remove(index).content)
    }

    /// `LOOT_CLOSED` for `corpse`.
    pub fn close(&mut self, corpse: u64) {
        if self.corpse == Some(corpse) {
            *self = Self::default();
        }
    }
}

/// What a right-click on an NPC does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcRightClick {
    /// A corpse with loot for the player opens it.
    Loot { auto: bool },
    /// Any other corpse is only targeted.
    Target,
    /// A living NPC is interacted with (gossip, vendor, ...).
    Interact,
}

pub fn npc_right_click(dead: bool, lootable: bool, auto: bool) -> NpcRightClick {
    if lootable {
        NpcRightClick::Loot { auto }
    } else if dead {
        NpcRightClick::Target
    } else {
        NpcRightClick::Interact
    }
}

/// Retail `AUTOLOOTTOGGLE` (default Shift) inverts `autoLootDefault`.
pub fn auto_loot(auto_loot_default: bool, toggle_held: bool) -> bool {
    auto_loot_default != toggle_held
}

/// `COPPER_PER_SILVER`, `COPPER_PER_GOLD`.
const COPPER_PER_SILVER: u64 = 100;
const COPPER_PER_GOLD: u64 = 10_000;

/// A money slot's name: one `GOLD_AMOUNT` / `SILVER_AMOUNT` / `COPPER_AMOUNT` line
/// per non-zero coin.
pub fn money_lines(copper: u64) -> String {
    [
        (copper / COPPER_PER_GOLD, "Gold"),
        (copper / COPPER_PER_SILVER % 100, "Silver"),
        (copper % COPPER_PER_SILVER, "Copper"),
    ]
    .into_iter()
    .filter(|(amount, _)| *amount > 0)
    .map(|(amount, unit)| format!("{amount} {unit}"))
    .collect::<Vec<_>>()
    .join("\n")
}

/// `interface/icons/inv_misc_coin_01` / `_03` / `_05`: the money slot icon by its
/// largest coin (FrameXML `GetCoinIcon`).
pub fn coin_icon_fdid(copper: u64) -> u32 {
    if copper >= COPPER_PER_GOLD {
        133_784
    } else if copper >= COPPER_PER_SILVER {
        133_786
    } else {
        133_788
    }
}

/// The chat line for a taken slot: `LOOT_ITEM_SELF` ("You receive loot: %s",
/// `LOOT_ITEM_SELF_MULTIPLE` "%sx%d") or `YOU_LOOT_MONEY` ("You loot %s").
pub fn loot_chat_text(content: &LootContent) -> String {
    match content {
        LootContent::Money { copper } => {
            format!("You loot {}", money_lines(*copper).replace('\n', ", "))
        }
        LootContent::Item { name, count, .. } if *count > 1 => {
            format!("You receive loot: [{name}]x{count}")
        }
        LootContent::Item { name, .. } => format!("You receive loot: [{name}]"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle() -> LootContent {
        LootContent::Item {
            item_id: 755,
            name: "Melted Candle".into(),
            quality: 0,
            count: 1,
        }
    }

    fn response() -> LootResponse {
        LootResponse {
            corpse: 42,
            auto: false,
            slots: vec![
                LootSlot {
                    slot: 0,
                    content: LootContent::Money { copper: 3 },
                },
                LootSlot {
                    slot: 1,
                    content: candle(),
                },
            ],
        }
    }

    #[test]
    fn taking_slots_keeps_the_rest_until_the_server_closes() {
        let mut state = LootState::default();
        state.open(response());
        assert_eq!(state.remove(7, 0), None, "another corpse");
        assert_eq!(state.remove(42, 1), Some(candle()));
        assert_eq!(state.slots.len(), 1);
        assert_eq!(state.slots[0].slot, 0);
        state.close(7);
        assert!(state.is_open());
        state.close(42);
        assert_eq!(state, LootState::default());
    }

    #[test]
    fn right_clicks_loot_corpses_with_loot_and_leave_empty_ones_targeted() {
        assert_eq!(
            npc_right_click(true, true, true),
            NpcRightClick::Loot { auto: true }
        );
        assert_eq!(npc_right_click(true, false, false), NpcRightClick::Target);
        assert_eq!(
            npc_right_click(false, false, false),
            NpcRightClick::Interact
        );
    }

    #[test]
    fn shift_inverts_the_auto_loot_setting() {
        assert!(auto_loot(false, true));
        assert!(!auto_loot(true, true));
        assert!(auto_loot(true, false));
        assert!(!auto_loot(false, false));
    }

    #[test]
    fn money_reads_one_coin_per_line_and_picks_the_biggest_coin_icon() {
        assert_eq!(money_lines(3), "3 Copper");
        assert_eq!(money_lines(10_502), "1 Gold\n5 Silver\n2 Copper");
        assert_eq!(money_lines(120), "1 Silver\n20 Copper");
        assert_eq!(coin_icon_fdid(3), 133_788);
        assert_eq!(coin_icon_fdid(120), 133_786);
        assert_eq!(coin_icon_fdid(10_502), 133_784);
    }

    #[test]
    fn taken_slots_print_retail_loot_lines() {
        assert_eq!(
            loot_chat_text(&candle()),
            "You receive loot: [Melted Candle]"
        );
        assert_eq!(
            loot_chat_text(&LootContent::Money { copper: 120 }),
            "You loot 1 Silver, 20 Copper"
        );
        let apples = LootContent::Item {
            item_id: 4536,
            name: "Shiny Red Apple".into(),
            quality: 1,
            count: 2,
        };
        assert_eq!(
            loot_chat_text(&apples),
            "You receive loot: [Shiny Red Apple]x2"
        );
    }
}
