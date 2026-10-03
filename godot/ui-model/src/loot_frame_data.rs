//! Shared authored loot cards, cursor placement and click policy.

use crate::item_icons::item_icon_fdid;
use crate::loot_data::{LootState, coin_icon_fdid, money_lines};
use crate::merchant_data::quality_color;
use crate::ui::screens::loot_frame_component::{
    ACTION_CLOSE, ACTION_SLOT_PREFIX, FRAME_W, LootFrameRow, LootFrameState, frame_height,
    quality_description,
};
use shared::protocol::{LootContent, LootSlot};

/// Retail's authored question-mark identity for an item absent from item data.
/// This is not a replacement for an icon that fails extraction.
const UNKNOWN_ICON_FDID: u32 = 134_400;

/// Runtime-independent authored frame actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootFrameAction {
    Take { slot: u8 },
    Release,
}

/// Retail `lootUnderMouse` (LootFrame.lua:180-190): TOPLEFT at the cursor x - 30 and
/// 50 above it, never lower than 350 above the screen bottom; `clampedToScreen`.
pub fn anchor_under_cursor(cursor: [f32; 2], screen: [f32; 2], rows: usize) -> [f32; 2] {
    let height = frame_height(rows);
    let left = (cursor[0] - 30.0).clamp(0.0, (screen[0] - FRAME_W).max(0.0));
    let top = (cursor[1] - 50.0)
        .min(screen[1] - 350.0)
        .clamp(0.0, (screen[1] - height).max(0.0));
    [left, top]
}

fn row(slot: &LootSlot) -> LootFrameRow {
    match &slot.content {
        LootContent::Money { copper } => LootFrameRow {
            slot: slot.slot,
            icon_fdid: coin_icon_fdid(*copper),
            name: money_lines(*copper),
            color: quality_color(1),
            quality_text: None,
            count: 1,
        },
        LootContent::Item {
            item_id,
            name,
            quality,
            count,
        } => LootFrameRow {
            slot: slot.slot,
            icon_fdid: item_icon_fdid(*item_id).unwrap_or(UNKNOWN_ICON_FDID),
            name: name.clone(),
            color: quality_color(*quality),
            quality_text: Some(quality_description(*quality)),
            count: *count,
        },
    }
}

pub fn build_state(loot: &LootState, anchor: [f32; 2]) -> LootFrameState {
    LootFrameState {
        visible: loot.is_open() && !loot.slots.is_empty(),
        rows: loot.slots.iter().map(row).collect(),
        left: anchor[0],
        top: anchor[1],
    }
}

/// The request a frame click sends.
pub fn request_for_action(action: &str) -> Option<LootFrameAction> {
    if action == ACTION_CLOSE {
        return Some(LootFrameAction::Release);
    }
    let slot = action.strip_prefix(ACTION_SLOT_PREFIX)?.parse().ok()?;
    Some(LootFrameAction::Take { slot })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_frame_opens_up_and_left_of_the_cursor_and_stays_on_screen() {
        let screen = [1920.0, 1080.0];
        assert_eq!(
            anchor_under_cursor([900.0, 500.0], screen, 2),
            [870.0, 450.0]
        );
        // Low on the screen: kept at least 350 above the bottom.
        assert_eq!(
            anchor_under_cursor([900.0, 1000.0], screen, 2),
            [870.0, 730.0]
        );
        // Corners: clamped inside.
        assert_eq!(anchor_under_cursor([10.0, 20.0], screen, 2), [0.0, 0.0]);
        assert_eq!(anchor_under_cursor([1910.0, 500.0], screen, 2)[0], 1700.0);
    }

    #[test]
    fn card_clicks_loot_their_slot_and_the_close_button_releases() {
        assert_eq!(
            request_for_action("loot_slot:3"),
            Some(LootFrameAction::Take { slot: 3 })
        );
        assert_eq!(
            request_for_action("loot_close"),
            Some(LootFrameAction::Release)
        );
        assert_eq!(request_for_action("merchant_close"), None);
        assert_eq!(request_for_action("loot_slot:"), None);
        assert_eq!(request_for_action("loot_slot:256"), None);
    }

    #[test]
    fn money_and_items_become_retail_cards() {
        let loot = LootState {
            corpse: Some(7),
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
        };
        let state = build_state(&loot, [400.0, 300.0]);
        assert!(state.visible);
        assert_eq!((state.left, state.top), (400.0, 300.0));
        assert_eq!(state.rows[0].name, "3 Copper");
        assert_eq!(state.rows[0].icon_fdid, 133_788);
        assert_eq!(state.rows[0].quality_text, None);
        assert_eq!(state.rows[1].quality_text, Some("Poor"));
        assert_eq!(state.rows[1].color, quality_color(0));
        assert!(!build_state(&LootState::default(), [0.0, 0.0]).visible);
        let empty_corpse = LootState {
            corpse: Some(7),
            ..LootState::default()
        };
        assert!(!build_state(&empty_corpse, [400.0, 300.0]).visible);
    }
}
