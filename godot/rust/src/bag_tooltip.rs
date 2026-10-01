//! Original item tooltip content and authored UI beside a standalone bag slot.

use game_engine_ui_model::bag_data::InventorySlot;
use game_engine_ui_model::bag_frame_component::parse_bag_slot_action;
use game_engine_ui_model::item_tooltip::item_tooltip;
use game_engine_ui_model::tooltip_presentation::{TOOLTIP_W, TooltipPresentation, append_item_id};
use godot::prelude::*;
use shared::components::UnitLevel;

use crate::GameClient;
use crate::ui::RegistryUi;

const TOOLTIP_UI: &str = "BagTooltipUI";
const TOOLTIP_LAYER: i32 = 8;

/// ContainerFrameItemButton_CalculateItemTooltipAnchors, then clampedToScreen.
fn bag_tooltip_position(owner: Rect2, screen: Vector2, height: f32) -> [f32; 2] {
    let right_edge = owner.end().x;
    let x = if right_edge < screen.x / 2.0 {
        right_edge
    } else {
        owner.position.x - TOOLTIP_W
    };
    let y = owner.position.y - height;
    [
        x.clamp(0.0, (screen.x - TOOLTIP_W).max(0.0)),
        y.clamp(0.0, (screen.y - height).max(0.0)),
    ]
}

impl GameClient {
    fn hovered_bag_item(&self) -> Option<(&InventorySlot, Rect2, Vector2)> {
        let ui = self.bags.ui.as_ref()?.bind();
        let pointer = Vector2::from_array(self.physical_input.pointer());
        let action = ui.pointer_action_at(pointer)??;
        let (bag, slot) = parse_bag_slot_action(&action)?;
        let item = self.merchant.session.inventory.slot(bag, slot)?;
        if item.is_empty() {
            return None;
        }
        let control = ui.frame_control(&format!("ContainerFrame{bag}Slot{slot}"))?;
        let scale = self.effective_ui_scale();
        let rect = control.get_global_rect();
        let owner = Rect2::new(rect.position / scale, rect.size / scale);
        let registry = ui.registry()?;
        let screen = Vector2::new(registry.screen_width, registry.screen_height);
        Some((item, owner, screen))
    }

    fn bag_tooltip_state(&self) -> TooltipPresentation {
        let Some((item, owner, screen)) = self.hovered_bag_item() else {
            return self.minimap_mail_tooltip().unwrap_or_default();
        };
        let level = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id)?.get::<UnitLevel>())
            .map(|level| u16::from(level.0));
        let mut state = item_tooltip(item, level);
        append_item_id(&mut state, item.item_id);
        [state.x, state.y] = bag_tooltip_position(owner, screen, state.height());
        state
    }

    pub(super) fn sync_bag_tooltip(&mut self) -> Result<(), String> {
        let state = self.bag_tooltip_state();
        let scale = self.effective_ui_scale();
        if let Some(ui) = self.bags.tooltip_ui.as_mut() {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)?;
            return host.set_state(state);
        }
        if !state.visible {
            return Ok(());
        }
        self.mount_bag_tooltip(state, scale)
    }

    fn mount_bag_tooltip(&mut self, state: TooltipPresentation, scale: f32) -> Result<(), String> {
        let mut ui = RegistryUi::new_alloc();
        ui.set_name(TOOLTIP_UI);
        ui.set_layer(TOOLTIP_LAYER);
        self.base_mut().add_child(&ui);
        let shown = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_item_tooltip(state))
        };
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.bags.tooltip_ui = Some(ui);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standalone_slot_uses_original_side_and_above_owner_anchor() {
        let screen = Vector2::new(1920.0, 1080.0);
        let left = Rect2::new(Vector2::new(100.0, 700.0), Vector2::splat(32.0));
        let right = Rect2::new(Vector2::new(1800.0, 700.0), Vector2::splat(32.0));
        assert_eq!(bag_tooltip_position(left, screen, 60.0), [132.0, 640.0]);
        assert_eq!(bag_tooltip_position(right, screen, 60.0), [1540.0, 640.0]);
        let midpoint = Rect2::new(Vector2::new(928.0, 700.0), Vector2::splat(32.0));
        assert_eq!(bag_tooltip_position(midpoint, screen, 60.0), [668.0, 640.0]);
    }

    #[test]
    fn standalone_tooltip_clamps_to_viewport_edges() {
        let screen = Vector2::new(300.0, 100.0);
        let top = Rect2::new(Vector2::new(145.0, 10.0), Vector2::splat(32.0));
        let bottom = Rect2::new(Vector2::new(10.0, 200.0), Vector2::splat(32.0));
        assert_eq!(bag_tooltip_position(top, screen, 60.0), [0.0, 0.0]);
        assert_eq!(bag_tooltip_position(bottom, screen, 60.0), [40.0, 40.0]);
    }
}
