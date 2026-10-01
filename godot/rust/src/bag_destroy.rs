//! Original world-drop confirmation using the shared StaticPopup host.

use game_engine_session::SessionScreen;
use game_engine_ui_model::cursor_item::{CursorItem, CursorTarget};
use game_engine_ui_model::popup::{PopupOutcome, PopupResult};
use godot::classes::{InputEvent, InputEventMouseButton};
use godot::global::MouseButton;
use godot::prelude::*;

use crate::{GameClient, frame_error::FrameError};

pub(crate) const DELETE_ITEM: &str = "DELETE_ITEM";
pub(crate) const DELETE_GOOD_ITEM: &str = "DELETE_GOOD_ITEM";

fn world_drop_pressed(event: &Gd<InputEvent>) -> bool {
    let Ok(mouse) = event.clone().try_cast::<InputEventMouseButton>() else {
        return false;
    };
    mouse.is_pressed() && mouse.get_button_index() == MouseButton::LEFT
}

impl GameClient {
    /// Called after GUI handling: a blocking frame is never a world target.
    pub(super) fn bag_cursor_world_pointer(
        &mut self,
        event: &Gd<InputEvent>,
    ) -> Result<bool, FrameError> {
        if self.account.session.screen != SessionScreen::InWorld
            || self.game_menu_ui.is_some()
            || self.bags.cursor.item.is_empty()
        {
            return Ok(false);
        }
        if !world_drop_pressed(event) {
            return Ok(false);
        }
        let Some(viewport) = self.base().get_viewport() else {
            return Ok(false);
        };
        if viewport.gui_get_hovered_control().is_some() {
            return Ok(false);
        }
        let session = &self.merchant.session;
        let effect =
            self.bags
                .cursor
                .item
                .click(CursorTarget::World, &session.inventory, &session.merchant);
        self.send_cursor_effect(effect)?;
        Ok(true)
    }

    pub(super) fn resolve_bag_destroy_results(
        &mut self,
        results: &[PopupResult],
    ) -> Result<(), FrameError> {
        for result in results {
            if result.key != DELETE_ITEM && result.key != DELETE_GOOD_ITEM {
                continue;
            }
            match result.outcome {
                PopupOutcome::Accepted => {
                    if let Some(request) = self.bags.cursor.item.destroy() {
                        self.account.send_inventory_request(&request)?;
                    }
                }
                PopupOutcome::Cancelled | PopupOutcome::TimedOut => {
                    self.bags.cursor.item = CursorItem::Empty;
                }
            }
        }
        Ok(())
    }

    pub(super) fn hide_stale_bag_destroy_popups(&mut self) {
        if self.bags.cursor.item.is_empty() {
            for key in [DELETE_ITEM, DELETE_GOOD_ITEM] {
                self.group_frames.popups.hide(key);
            }
        }
    }
}
