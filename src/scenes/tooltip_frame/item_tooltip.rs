//! Bevy owner/record adapter around the shared original item formatter.

use super::{TooltipAnchor, TooltipFrameState, TooltipRecord};
use game_engine::bag_data::InventorySlot;

pub(super) struct TooltipItem<'a> {
    pub slot: &'a InventorySlot,
    pub player_level: Option<u16>,
}

pub(super) fn item_tooltip(item: TooltipItem) -> TooltipFrameState {
    let presentation = game_engine::item_tooltip::item_tooltip(item.slot, item.player_level);
    TooltipFrameState {
        visible: presentation.visible,
        x: presentation.x,
        y: presentation.y,
        title: presentation.title,
        title_color: presentation.title_color,
        lines: presentation.lines,
        record: Some(TooltipRecord::Item(item.slot.item_id)),
        anchor: TooltipAnchor::Default,
    }
}
