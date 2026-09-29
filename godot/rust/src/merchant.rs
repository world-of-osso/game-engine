//! NPC interaction and the vendor frame, ported from the Bevy client's
//! `rendering/ui/target.rs` (`right_click_interact`), `rendering/ui/wow_cursor.rs`,
//! `networking/merchant.rs` and `scenes/merchant_frame`: right-clicking a unit targets
//! it and, within interact range, sends `InteractNpc`; the server's vendor list opens the
//! shared MerchantFrame with the backpack; frame and bag clicks become vendor requests;
//! Escape, the close button or the server's `InteractionClosed` close it.

use game_engine_core::input_bindings_data::{BindingMouseButton, InputState};
use game_engine_session::SessionScreen;
use game_engine_ui_model::merchant::{Click, MerchantEffect, MerchantSession, SplitKey};
use game_engine_ui_model::merchant_frame_component::FRAME_NAME;
use game_engine_ui_model::wow_cursor_data::{ActiveWowCursor, NpcCursorView, npc_cursor};
use godot::classes::{ImageTexture, Input};
use godot::global::Key;
use godot::prelude::*;
use shared::protocol::{InteractionKind, NpcFlags, NpcRole};

use crate::GameClient;
use crate::account::NpcMessage;
use crate::faction_reaction::{Reaction, reaction};
use crate::targeting::pick_unit;
use crate::ui::{MerchantStates, RegistryUi};

/// Bevy `INTERACT_RANGE`: the farthest a right-click interacts, in yards.
const INTERACT_RANGE: f32 = 5.0;
const MERCHANT_UI: &str = "MerchantUI";

#[derive(Default)]
pub(crate) struct Merchant {
    session: MerchantSession,
    ui: Option<Gd<RegistryUi>>,
    cursor: Option<ActiveWowCursor>,
    cursor_textures: Vec<(ActiveWowCursor, Gd<ImageTexture>)>,
}

impl Merchant {
    fn free_ui(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
    }
}

/// What a right-click on a unit does (Bevy `interact_with_clicked_npc` / `npc_right_click`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RightClick {
    /// Target it only: a player, a corpse, or an NPC out of range.
    Target,
    /// Target it and send `InteractNpc`.
    Interact,
}

pub(crate) fn right_click(is_npc: bool, dead: bool, distance: f32) -> RightClick {
    if is_npc && !dead && distance <= INTERACT_RANGE {
        RightClick::Interact
    } else {
        RightClick::Target
    }
}

impl GameClient {
    pub(super) fn receive_npc_message(&mut self, message: NpcMessage) -> Result<(), String> {
        let session = &mut self.merchant.session;
        match message {
            NpcMessage::Opened(opened) => match opened.kind {
                // The vendor list follows the vendor role (`VendorInventory`).
                InteractionKind::Role(NpcRole::Vendor) => {}
                other => godot_warn!("NPC frame {other:?} is not converted to Godot yet"),
            },
            NpcMessage::Closed(npc) => session.receive_interaction_closed(npc),
            NpcMessage::Vendor(inventory) => {
                let name = self
                    .units
                    .get(&inventory.npc)
                    .and_then(|unit| unit.npc.as_ref())
                    .map(|npc| npc.name.clone())
                    .unwrap_or_default();
                session.receive_inventory(inventory, name);
            }
            NpcMessage::Buyback(list) => session.receive_buyback(list),
            NpcMessage::Inventory(snapshot) => session.receive_inventory_snapshot(&snapshot),
            NpcMessage::InventoryChanged(delta) => session.receive_inventory_delta(&delta),
            NpcMessage::RepairCost(cost) => session.repair_cost = cost,
            NpcMessage::Error(error) => self.add_world_error(&error)?,
        }
        Ok(())
    }

    /// Per frame, after targeting: right-click interaction, the hover cursor, frame input,
    /// then the frame's presentation.
    pub(super) fn update_merchant(&mut self) -> Result<(), String> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.merchant.session.merchant.close();
            self.merchant.session.split = None;
            self.merchant.free_ui();
            self.set_world_cursor(None)?;
            return Ok(());
        }
        let money = self
            .world
            .local_player_id()
            .and_then(|id| self.units.get(&id)?.gold)
            .unwrap_or(0);
        self.merchant.session.money = money;
        let interactive =
            self.game_menu_ui.is_none() && self.account.session.gameplay_input_allowed();
        if interactive {
            self.right_click_interact()?;
            self.poll_merchant_input()?;
        }
        let cursor = interactive.then(|| self.hover_cursor()).flatten();
        self.set_world_cursor(cursor)?;
        self.sync_merchant_ui()
    }

    /// Bevy `right_click_interact`: the unit under the pointer, else the current target.
    fn right_click_interact(&mut self) -> Result<(), String> {
        if !self
            .physical_input
            .mouse_just_pressed(BindingMouseButton::Right)
        {
            return Ok(());
        }
        let Some(viewport) = self.base().get_viewport() else {
            return Ok(());
        };
        if viewport.gui_get_hovered_control().is_some() {
            return Ok(());
        }
        let clicked = viewport.get_camera_3d().and_then(|camera| {
            pick_unit(&camera, Vector2::from_array(self.physical_input.pointer()))
        });
        let unit = match clicked {
            Some(unit) => {
                // Right-click targets, as Retail does.
                self.set_target(Some(unit));
                unit
            }
            None => match self.targeting_target() {
                Some(unit) => unit,
                None => return Ok(()),
            },
        };
        if self.unit_right_click(unit) == RightClick::Interact {
            self.account.send_interact(unit)?;
        }
        Ok(())
    }

    fn unit_right_click(&self, id: u64) -> RightClick {
        let Some(unit) = self.units.get(&id) else {
            return RightClick::Target;
        };
        let distance = self
            .world
            .local_player_transform()
            .zip(self.world.unit_node(id))
            .map_or(f32::INFINITY, |(player, node)| {
                player.origin.distance_to(node.get_global_position())
            });
        let dead = unit.health.is_some_and(|health| health.current <= 0.0);
        right_click(unit.npc.is_some(), dead, distance)
    }

    /// Bevy `pick_desired_cursor` for units: the cursor of the NPC under the pointer.
    fn hover_cursor(&mut self) -> Option<ActiveWowCursor> {
        let viewport = self.base().get_viewport()?;
        if viewport.gui_get_hovered_control().is_some() {
            return Some(ActiveWowCursor::Default);
        }
        let camera = viewport.get_camera_3d()?;
        let Some(id) = pick_unit(&camera, Vector2::from_array(self.physical_input.pointer()))
        else {
            return Some(ActiveWowCursor::Default);
        };
        let unit = self.units.get(&id)?;
        if unit.npc.is_none() {
            return Some(ActiveWowCursor::Default);
        }
        let viewer = self
            .world
            .local_player_id()
            .and_then(|player| self.units.get(&player)?.faction_template);
        let reaction = match self.nameplates.templates(&self.data_root) {
            Ok(templates) => reaction(
                unit.faction_template.and_then(|id| templates.get(&id)),
                viewer.and_then(|id| templates.get(&id)),
            ),
            Err(_) => Reaction::Neutral,
        };
        Some(npc_cursor(NpcCursorView {
            flags: NpcFlags(unit.npc_flags.unwrap_or(0)),
            dead: unit.health.is_some_and(|health| health.current <= 0.0),
            lootable: false,
            reaction,
        }))
    }

    /// The Retail cursor art in world; the system cursor elsewhere.
    fn set_world_cursor(&mut self, cursor: Option<ActiveWowCursor>) -> Result<(), String> {
        if self.merchant.cursor == cursor {
            return Ok(());
        }
        self.merchant.cursor = cursor;
        let mut input = Input::singleton();
        let Some(cursor) = cursor else {
            input.set_custom_mouse_cursor(Gd::<godot::classes::Resource>::null_arg());
            return Ok(());
        };
        let texture = self.cursor_texture(cursor)?;
        input.set_custom_mouse_cursor(&texture);
        Ok(())
    }

    fn cursor_texture(&mut self, cursor: ActiveWowCursor) -> Result<Gd<ImageTexture>, String> {
        if let Some((_, texture)) = self
            .merchant
            .cursor_textures
            .iter()
            .find(|(kind, _)| *kind == cursor)
        {
            return Ok(texture.clone());
        }
        let path = self
            .data_root
            .join(format!("textures/{}.blp", cursor.texture_fdid()));
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("Read cursor {}: {error}", path.display()))?;
        let image = game_engine_core::blp::decode_rgba(&bytes)
            .map_err(|error| format!("Decode cursor {}: {error}", path.display()))?;
        let image = godot::classes::Image::create_from_data(
            image.width as i32,
            image.height as i32,
            false,
            godot::classes::image::Format::RGBA8,
            &PackedByteArray::from(image.pixels.as_slice()),
        )
        .ok_or("Godot rejected a cursor image")?;
        let texture =
            ImageTexture::create_from_image(&image).ok_or("Godot rejected a cursor texture")?;
        self.merchant
            .cursor_textures
            .push((cursor, texture.clone()));
        Ok(texture)
    }

    fn poll_merchant_input(&mut self) -> Result<(), String> {
        let Some(mut ui) = self.merchant.ui.clone() else {
            return Ok(());
        };
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            self.merchant_click(&action, Click::LEFT)?;
        }
        while let Some((action, right, shift)) = ui.bind_mut().pop_alt_click() {
            self.merchant_click(&action, Click { right, shift })?;
        }
        Ok(())
    }

    fn merchant_click(&mut self, action: &str, click: Click) -> Result<(), String> {
        let session = &mut self.merchant.session;
        let effect = if action.starts_with("bag_slot:") {
            session.click_bag(action, click)
        } else {
            session.click_frame(action, click)
        };
        self.apply_merchant_effect(effect)
    }

    fn apply_merchant_effect(&mut self, effect: Option<MerchantEffect>) -> Result<(), String> {
        match effect {
            None => Ok(()),
            Some(MerchantEffect::Request { npc, request }) => {
                self.account.send_merchant_request(npc, &request)
            }
            Some(MerchantEffect::CloseInteraction { npc }) => {
                self.account.send_close_interaction(npc)
            }
        }
    }

    /// Keys the open frames own: the StackSplitFrame takes digits, arrows, Backspace,
    /// Enter and Escape; Escape then closes the MerchantFrame (`CloseAllWindows`).
    pub(super) fn merchant_key(&mut self, key: Key) -> Result<bool, String> {
        if let Some(key) = split_key(key)
            && let Some(effect) = self.merchant.session.split_key(key)
        {
            self.apply_merchant_effect(effect)?;
            return Ok(true);
        }
        if key == Key::ESCAPE && self.merchant.session.is_open() {
            let effect = self.merchant.session.close();
            self.apply_merchant_effect(effect)?;
            return Ok(true);
        }
        Ok(false)
    }

    fn sync_merchant_ui(&mut self) -> Result<(), String> {
        let session = &self.merchant.session;
        let split = match self.merchant.ui.as_ref() {
            Some(ui) => ui
                .bind()
                .registry()
                .map(|registry| session.split_state(registry))
                .unwrap_or_default(),
            None => Default::default(),
        };
        let states = MerchantStates {
            frame: session.frame_state(),
            bags: session.bag_state(),
            split,
        };
        if let Some(ui) = self.merchant.ui.as_mut() {
            return ui.bind_mut().set_merchant_states(states);
        }
        if !session.is_open() {
            return Ok(());
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name(MERCHANT_UI);
        self.base_mut().add_child(&ui);
        let shown = ui.bind_mut().show_merchant(states);
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.merchant.ui = Some(ui);
        Ok(())
    }

    /// Automation view of the vendor session: open vendor, cells, buyback, bags, money.
    pub(super) fn merchant_snapshot(&self) -> VarDictionary {
        let session = &self.merchant.session;
        let merchant = &session.merchant;
        let mut state = VarDictionary::new();
        state.set("open", session.is_open());
        state.set(
            "npc",
            &merchant
                .npc
                .map(|npc| (npc as i64).to_variant())
                .unwrap_or_default(),
        );
        state.set("vendor_name", merchant.vendor_name.as_str());
        state.set(
            "items",
            &string_array(merchant.items.iter().map(|item| &item.name)),
        );
        state.set(
            "buyback",
            &string_array(merchant.buyback.iter().map(|item| &item.name)),
        );
        state.set("bags", &bag_items(&session.inventory));
        state.set("money", session.money as i64);
        state.set("repair_cost", i64::from(session.repair_cost));
        state.set("split_open", session.split.is_some());
        state.set(
            "cursor",
            self.merchant
                .cursor
                .map(|cursor| format!("{cursor:?}"))
                .unwrap_or_default()
                .as_str(),
        );
        state.set("frame", FRAME_NAME);
        state
    }
}

fn string_array<'a>(items: impl Iterator<Item = &'a String>) -> VarArray {
    let mut array = VarArray::new();
    for item in items {
        array.push(&item.to_variant());
    }
    array
}

/// Occupied bag slots: bag, slot, item id, catalog name and count.
fn bag_items(inventory: &game_engine_ui_model::bag_data::InventoryState) -> VarArray {
    let mut bags = VarArray::new();
    for (bag, slots) in inventory.slots.iter().enumerate() {
        for (slot, item) in slots
            .iter()
            .enumerate()
            .filter(|(_, item)| !item.is_empty())
        {
            let mut entry = VarDictionary::new();
            entry.set("bag", bag as i64);
            entry.set("slot", slot as i64);
            entry.set("item_id", i64::from(item.item_id));
            entry.set("name", item.name.as_str());
            entry.set("count", i64::from(item.count));
            bags.push(&entry.to_variant());
        }
    }
    bags
}

fn split_key(key: Key) -> Option<SplitKey> {
    Some(match key {
        Key::ENTER | Key::KP_ENTER => SplitKey::Enter,
        Key::ESCAPE => SplitKey::Escape,
        Key::BACKSPACE | Key::DELETE => SplitKey::Backspace,
        Key::LEFT | Key::DOWN => SplitKey::Decrement,
        Key::RIGHT | Key::UP => SplitKey::Increment,
        _ => {
            let digit = (key.ord() - Key::KEY_0.ord()) as u32;
            let keypad = (key.ord() - Key::KP_0.ord()) as u32;
            SplitKey::Digit(if digit <= 9 {
                digit
            } else if keypad <= 9 {
                keypad
            } else {
                return None;
            })
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_click_interacts_only_with_a_living_npc_in_range() {
        assert_eq!(right_click(true, false, 4.9), RightClick::Interact);
        assert_eq!(right_click(true, false, 5.0), RightClick::Interact);
        assert_eq!(right_click(true, false, 5.1), RightClick::Target);
        assert_eq!(right_click(true, true, 1.0), RightClick::Target);
        assert_eq!(right_click(false, false, 1.0), RightClick::Target);
    }

    #[test]
    fn split_frame_keys_follow_retail_bindings() {
        assert_eq!(split_key(Key::KEY_7), Some(SplitKey::Digit(7)));
        assert_eq!(split_key(Key::KP_3), Some(SplitKey::Digit(3)));
        assert_eq!(split_key(Key::KP_ENTER), Some(SplitKey::Enter));
        assert_eq!(split_key(Key::UP), Some(SplitKey::Increment));
        assert_eq!(split_key(Key::A), None);
    }
}
