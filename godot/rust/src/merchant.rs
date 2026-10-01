//! NPC interaction and the vendor frame, ported from the Bevy client's
//! `rendering/ui/target.rs` (`right_click_interact`), `rendering/ui/wow_cursor.rs`,
//! `networking/merchant.rs` and `scenes/merchant_frame`: right-clicking a unit targets
//! it and, within interact range, sends `InteractNpc`; the server's vendor list opens the
//! shared MerchantFrame with the backpack; frame and bag clicks become vendor requests;
//! Escape, the close button or the server's `InteractionClosed` close it.

use game_engine_core::input_bindings_data::{BindingMouseButton, InputState};
use game_engine_session::SessionScreen;
use game_engine_ui_model::cursor_item::CursorTarget;
use game_engine_ui_model::merchant::{Click, MerchantEffect, MerchantSession, SplitKey};
use game_engine_ui_model::merchant_data::MerchantTab;
use game_engine_ui_model::merchant_frame_component::{FRAME_H, FRAME_NAME, FRAME_W};
use game_engine_ui_model::wow_cursor_data::{ActiveWowCursor, NpcCursorView, npc_cursor};
use godot::classes::{
    ImageTexture, Input, InputEvent, InputEventMouseButton, InputEventMouseMotion,
};
use godot::global::Key;
use godot::global::MouseButton;
use godot::prelude::*;
use shared::components::{Health, Npc};
use shared::protocol::{InteractionKind, NpcFlags, NpcRole};

use crate::GameClient;
use crate::account::NpcMessage;
use crate::faction_reaction::{Reaction, reaction};
use crate::frame_error::{FrameError, SessionError, report_once};
use crate::replicated::UnitFields;
use crate::targeting::pick_unit;
use crate::ui::{MerchantStates, RegistryUi};
use crate::world_map::{WindowDrag, title_hit};

/// Bevy `INTERACT_RANGE`: the farthest a right-click interacts, in yards.
const INTERACT_RANGE: f32 = 5.0;
const MERCHANT_UI: &str = "MerchantUI";
const DEFAULT_POSITION: [f32; 2] = [16.0, 104.0];

#[derive(Default)]
pub(crate) struct Merchant {
    pub(crate) session: MerchantSession,
    ui: Option<Gd<RegistryUi>>,
    position: Option<[f32; 2]>,
    position_character: Option<u64>,
    drag: Option<WindowDrag>,
    cursor: Option<ActiveWowCursor>,
    /// Loaded cursor art; `None` caches a kind whose art failed to load.
    cursor_textures: Vec<(ActiveWowCursor, Option<Gd<ImageTexture>>)>,
    /// Test hook: every cursor kind reads this FDID instead of its Retail art.
    cursor_fdid_override: Option<u32>,
}

impl Merchant {
    pub(crate) fn override_cursor_fdid(&mut self, fdid: u32) {
        self.cursor_fdid_override = Some(fdid);
        self.cursor_textures.clear();
        self.cursor = None;
    }

    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(ui) = &mut self.ui {
            visit(ui)?;
        }
        Ok(())
    }

    fn free_ui(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
        self.position = None;
        self.position_character = None;
        self.drag = None;
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
    pub(super) fn merchant_frame_control(
        &self,
        name: &str,
    ) -> Result<Gd<godot::classes::Control>, String> {
        let ui = self
            .merchant
            .ui
            .as_ref()
            .ok_or_else(|| format!("MerchantUI is missing while resolving control '{name}'"))?;
        ui.bind()
            .frame_control(name)
            .ok_or_else(|| format!("MerchantUI control '{name}' is missing"))
    }

    pub(super) fn receive_npc_message(&mut self, message: NpcMessage) -> Result<(), String> {
        if let NpcMessage::Closed(npc) = &message {
            self.auction_interaction_closed(*npc);
            self.mailbox.close_for(*npc);
        }
        if let NpcMessage::Opened(opened) = &message {
            if opened.kind == InteractionKind::Role(NpcRole::Mailbox) {
                self.open_mailbox(opened.npc);
                return Ok(());
            }
            self.mailbox.session.close();
        }
        self.apply_npc_message(message)
    }

    fn trace_merchant_order(&self, event: &str, stage: &str) {
        if std::env::var_os("GODOT_MERCHANT_ORDER_TRACE").is_some() {
            let merchant = &self.merchant.session.merchant;
            eprintln!(
                "MERCHANT ORDER {stage} {event}: npc={:?} buyback_count={}",
                merchant.npc,
                merchant.buyback.len()
            );
        }
    }

    fn apply_npc_message(&mut self, message: NpcMessage) -> Result<(), String> {
        let event = match &message {
            NpcMessage::Vendor(_) => Some("Vendor"),
            NpcMessage::Buyback(_) => Some("Buyback"),
            NpcMessage::Opened(opened) if opened.kind == InteractionKind::Role(NpcRole::Vendor) => {
                Some("OpenedVendor")
            }
            NpcMessage::Closed(_) => Some("Closed"),
            _ => None,
        };
        if let Some(event) = event {
            self.trace_merchant_order(event, "before");
        }
        let session = &mut self.merchant.session;
        match message {
            NpcMessage::Opened(opened) => match opened.kind {
                // The vendor list follows the vendor role (`VendorInventory`).
                InteractionKind::Role(NpcRole::Vendor) => {
                    self.auction_interaction_closed_any();
                }
                InteractionKind::Role(NpcRole::AuctionHouse) => {
                    session.close();
                    self.auction.session.open(opened.npc);
                }
                InteractionKind::Gossip(menu) => self.show_auction_gossip(opened.npc, menu)?,
                other => godot_warn!("NPC frame {other:?} is not converted to Godot yet"),
            },
            NpcMessage::Closed(npc) => session.receive_interaction_closed(npc),
            NpcMessage::Vendor(inventory) => {
                let name = self
                    .replica
                    .unit(inventory.npc)
                    .and_then(|unit| unit.get::<Npc>())
                    .map(|npc| npc.name.clone())
                    .unwrap_or_default();
                session.receive_inventory(inventory, name);
            }
            NpcMessage::Buyback(list) => session.receive_buyback(list),
            NpcMessage::Inventory(snapshot) => session.receive_inventory_snapshot(&snapshot),
            NpcMessage::Equipment(snapshot) => {
                session.inventory.apply_equipment_snapshot(&snapshot)
            }
            NpcMessage::InventoryChanged(delta) => session.receive_inventory_delta(&delta),
            NpcMessage::RepairCost(cost) => session.repair_cost = cost,
            NpcMessage::InteractionError(failed) => {
                self.mailbox.close_for(failed.npc);
                self.add_world_error(failed.error.message())?;
            }
            NpcMessage::Error(error) => self.add_world_error(&error)?,
        }
        if let Some(event) = event {
            self.trace_merchant_order(event, "after");
        }
        Ok(())
    }

    /// Per frame, after targeting and the shared window input poll (`update_bags`):
    /// right-click interaction, the hover cursor, then the frame's presentation.
    pub(super) fn update_merchant(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.merchant.session.merchant.close();
            self.merchant.session.split = None;
            self.merchant.free_ui();
            self.set_world_cursor(None);
            return Ok(());
        }
        let money = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id)?.gold())
            .unwrap_or(0);
        self.merchant.session.money = money;
        let interactive =
            self.game_menu_ui.is_none() && self.account.session.gameplay_input_allowed();
        if interactive {
            self.right_click_interact()?;
        }
        let cursor = interactive.then(|| self.hover_cursor()).flatten();
        self.set_world_cursor(cursor);
        Ok(self.sync_merchant_ui()?)
    }

    /// Bevy `right_click_interact`: the unit under the pointer, else the current target.
    fn right_click_interact(&mut self) -> Result<(), FrameError> {
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
                if self.use_mailbox(unit)? {
                    return Ok(());
                }
                // Right-click targets, as Retail does.
                self.set_target(Some(unit));
                unit
            }
            None => match self.targeting_target() {
                Some(unit) => unit,
                None => return Ok(()),
            },
        };
        if self.send_corpse_loot(unit)? {
            return Ok(());
        }
        // Right-clicking an attackable unit attacks it (`CMSG_ATTACK_SWING`).
        if self.can_auto_attack(unit) {
            self.start_auto_attack(unit)?;
        } else if self.unit_right_click(unit) == RightClick::Interact {
            self.account.send_interact(unit)?;
        }
        Ok(())
    }

    fn unit_right_click(&self, id: u64) -> RightClick {
        let Some(unit) = self.replica.unit(id) else {
            return RightClick::Target;
        };
        let distance = self
            .world
            .local_player_transform()
            .zip(self.world.unit_node(id))
            .map_or(f32::INFINITY, |(player, node)| {
                player.origin.distance_to(node.get_global_position())
            });
        let dead = unit
            .get::<Health>()
            .is_some_and(|health| health.current <= 0.0);
        right_click(unit.has::<Npc>(), dead, distance)
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
        if self.game_objects.contains(id) {
            return Some(ActiveWowCursor::Mail);
        }
        let unit = self.replica.unit(id)?;
        if !unit.has::<Npc>() {
            return Some(ActiveWowCursor::Default);
        }
        let viewer = self
            .world
            .local_player_id()
            .and_then(|player| self.replica.unit(player)?.faction_template());
        let reaction = match self.nameplates.templates(&self.data_root) {
            Ok(templates) => reaction(
                unit.faction_template().and_then(|id| templates.get(&id)),
                viewer.and_then(|id| templates.get(&id)),
            ),
            Err(_) => Reaction::Neutral,
        };
        Some(npc_cursor(NpcCursorView {
            flags: NpcFlags(unit.npc_flags().unwrap_or(0)),
            dead: unit
                .get::<Health>()
                .is_some_and(|health| health.current <= 0.0),
            lootable: self.loot.lootable.contains(&id),
            reaction,
        }))
    }

    /// The Retail cursor art in world; the system cursor elsewhere, or when the art
    /// fails to load (Bevy `load_cursor_image`: logged, the cursor asset stays absent).
    pub(crate) fn set_world_cursor(&mut self, cursor: Option<ActiveWowCursor>) {
        if self.merchant.cursor == cursor {
            return;
        }
        self.merchant.cursor = cursor;
        let texture = cursor.and_then(|cursor| self.cursor_texture(cursor));
        let mut input = Input::singleton();
        match texture {
            Some(texture) => input.set_custom_mouse_cursor(&texture),
            None => input.set_custom_mouse_cursor(Gd::<godot::classes::Resource>::null_arg()),
        }
    }

    /// The cached cursor art; a load failure is reported once and cached as absent.
    fn cursor_texture(&mut self, cursor: ActiveWowCursor) -> Option<Gd<ImageTexture>> {
        if let Some((_, texture)) = self
            .merchant
            .cursor_textures
            .iter()
            .find(|(kind, _)| *kind == cursor)
        {
            return texture.clone();
        }
        let texture = self
            .load_cursor_texture(cursor)
            .inspect_err(|error| report_once(error))
            .ok();
        self.merchant
            .cursor_textures
            .push((cursor, texture.clone()));
        texture
    }

    fn load_cursor_texture(&self, cursor: ActiveWowCursor) -> Result<Gd<ImageTexture>, String> {
        let fdid = self
            .merchant
            .cursor_fdid_override
            .unwrap_or(cursor.texture_fdid());
        let path = self.data_root.join(format!("textures/{fdid}.blp"));
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
        .ok_or_else(|| format!("Godot rejected cursor image {}", path.display()))?;
        ImageTexture::create_from_image(&image)
            .ok_or_else(|| format!("Godot rejected cursor texture {}", path.display()))
    }

    pub(super) fn merchant_input_owner(&self, owner: i64) -> bool {
        self.merchant
            .ui
            .as_ref()
            .is_some_and(|ui| ui.instance_id().to_i64() == owner)
    }

    pub(super) fn merchant_cursor_click(
        &mut self,
        action: &str,
        click: Click,
    ) -> Result<(), FrameError> {
        if self.merchant.session.split.is_some() {
            return self.merchant_click(action, click);
        }
        if action.starts_with(game_engine_ui_model::bag_frame_component::ACTION_BAG_SLOT_PREFIX) {
            if click.right {
                return self.merchant_click(action, click);
            }
            return self.bag_cursor_click(action, click);
        }
        let Some(target) = self.merchant_cursor_target(action, click)? else {
            return self.merchant_click(action, click);
        };
        self.send_cursor_click(target)
    }

    fn merchant_cursor_target(
        &self,
        action: &str,
        click: Click,
    ) -> Result<Option<CursorTarget>, String> {
        if click.right || click.shift {
            return Ok(None);
        }
        if self.merchant.session.merchant.tab != MerchantTab::Merchant {
            return Ok(None);
        }
        crate::bag_cursor::cursor_action_target(action)
    }

    fn merchant_click(&mut self, action: &str, click: Click) -> Result<(), FrameError> {
        let session = &mut self.merchant.session;
        let effect = if action.starts_with("bag_slot:") {
            session.click_bag(action, click)
        } else {
            session.click_frame(action, click)
        };
        Ok(self.apply_merchant_effect(effect)?)
    }

    fn apply_merchant_effect(
        &mut self,
        effect: Option<MerchantEffect>,
    ) -> Result<(), SessionError> {
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

    /// Keys the open StackSplitFrame owns: digits, arrows, Backspace, Enter and Escape.
    pub(super) fn merchant_key(&mut self, key: Key) -> Result<bool, SessionError> {
        if let Some(key) = split_key(key)
            && let Some(effect) = self.merchant.session.split_key(key)
        {
            self.apply_merchant_effect(effect)?;
            return Ok(true);
        }
        Ok(false)
    }

    pub(super) fn close_merchant_window(&mut self) -> Result<bool, SessionError> {
        if !self.merchant.session.is_open() {
            return Ok(false);
        }
        let effect = self.merchant.session.close();
        self.apply_merchant_effect(effect)?;
        Ok(true)
    }

    fn sync_merchant_ui(&mut self) -> Result<(), String> {
        self.load_merchant_position()?;
        let states = self.merchant_states();
        let scale = self.effective_ui_scale();
        if let Some(ui) = self.merchant.ui.as_mut() {
            ui.bind_mut().set_ui_scale(scale)?;
            ui.bind_mut().set_merchant_states(states)?;
            return self.place_merchant();
        }
        if !self.merchant.session.is_open() {
            return Ok(());
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name(MERCHANT_UI);
        self.base_mut().add_child(&ui);
        ui.bind_mut().set_ui_scale(scale)?;
        let shown = ui.bind_mut().show_merchant(states);
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.merchant.ui = Some(ui);
        self.place_merchant()
    }

    fn merchant_states(&self) -> MerchantStates {
        let session = &self.merchant.session;
        let split = self
            .merchant
            .ui
            .as_ref()
            .and_then(|ui| {
                ui.bind()
                    .registry()
                    .map(|registry| session.split_state(registry))
            })
            .unwrap_or_default();
        let mut bags = session.bag_state();
        let source = self.bags.cursor.item.source();
        for bag in &mut bags.bags {
            for (index, slot) in bag.slots.iter_mut().enumerate() {
                slot.locked = source
                    == Some(shared::protocol::ItemLocation::Bag {
                        bag: bag.bag_index as u8,
                        slot: index as u8,
                    });
            }
        }
        MerchantStates {
            frame: session.frame_state(),
            bags,
            split,
        }
    }

    fn load_merchant_position(&mut self) -> Result<(), String> {
        if !self.merchant.session.is_open() {
            self.merchant.position = None;
            self.merchant.position_character = None;
            self.merchant.drag = None;
            return Ok(());
        }
        let id = self
            .account
            .session
            .selected_character_id
            .ok_or("Merchant requires selected server character ID")?;
        if self.merchant.position_character == Some(id) {
            return Ok(());
        }
        let path =
            game_engine_core::client_options_data::options_path().with_file_name("ui_layout.ron");
        self.merchant.position =
            game_engine_core::ui_layout_data::window_position(&path, id, FRAME_NAME)?;
        self.merchant.position_character = Some(id);
        self.merchant.drag = None;
        Ok(())
    }

    fn merchant_rect(&self) -> [f32; 4] {
        let size = self
            .base()
            .get_viewport()
            .map_or(Vector2::new(1280.0, 720.0), |viewport| {
                viewport.get_visible_rect().size
            });
        let scale = self.effective_ui_scale();
        let viewport = size / scale;
        let [x, y] = self.merchant.position.unwrap_or(DEFAULT_POSITION);
        [
            x.clamp(0.0, (viewport.x - FRAME_W).max(0.0)),
            y.clamp(0.0, (viewport.y - FRAME_H).max(0.0)),
            FRAME_W,
            FRAME_H,
        ]
    }

    fn place_merchant(&mut self) -> Result<(), String> {
        if !self.merchant.session.is_open() {
            return Ok(());
        }
        let [x, y, _, _] = self.merchant_rect();
        self.merchant
            .ui
            .as_mut()
            .ok_or("Merchant UI vanished")?
            .bind_mut()
            .set_window_position(FRAME_NAME, [x, y])
    }

    pub(super) fn reset_open_merchant_position(&mut self) -> Result<(), String> {
        self.merchant.position = None;
        self.merchant.drag = None;
        self.place_merchant()
    }

    pub(super) fn merchant_pointer(&mut self, event: &Gd<InputEvent>) -> bool {
        if !self.merchant.session.is_open() || self.game_menu_ui.is_some() {
            return false;
        }
        let rect = self.merchant_rect();
        let scale = self.effective_ui_scale();
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            return self.move_merchant(&motion, rect, scale);
        }
        let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() else {
            return false;
        };
        self.press_merchant_title(&button, rect, scale)
    }

    fn move_merchant(
        &mut self,
        motion: &Gd<InputEventMouseMotion>,
        rect: [f32; 4],
        scale: f32,
    ) -> bool {
        let Some(drag) = &self.merchant.drag else {
            return false;
        };
        let size = self
            .base()
            .get_viewport()
            .map_or(Vector2::new(1280.0, 720.0), |viewport| {
                viewport.get_visible_rect().size
            })
            / scale;
        self.merchant.position = Some(drag.position(
            motion.get_position() / scale,
            [size.x, size.y],
            [rect[2], rect[3]],
        ));
        if let Err(error) = self.place_merchant() {
            godot_error!("Merchant drag: {error}");
        }
        true
    }

    fn press_merchant_title(
        &mut self,
        button: &Gd<InputEventMouseButton>,
        rect: [f32; 4],
        scale: f32,
    ) -> bool {
        if button.get_button_index() != MouseButton::LEFT {
            return false;
        }
        if !button.is_pressed() && self.merchant.drag.take().is_some() {
            self.persist_merchant_position();
            return true;
        }
        if !button.is_pressed() {
            return false;
        }
        let buttons = self.merchant_close_rect(scale);
        if title_hit(rect, button.get_position(), scale, buttons.as_slice())
            && self.merchant_owns_point(button.get_position())
        {
            self.merchant.drag = Some(WindowDrag::begin(
                button.get_position() / scale,
                [rect[0], rect[1]],
            ));
            return true;
        }
        false
    }

    /// A window raised over the title owns the press instead of the merchant.
    fn merchant_owns_point(&mut self, at: Vector2) -> bool {
        let Some(ui) = self.merchant.ui.clone() else {
            return false;
        };
        self.ui_owns_point(&ui, at).unwrap_or_else(|error| {
            godot_error!("Merchant title hit-test: {error}");
            false
        })
    }

    fn merchant_close_rect(&self, scale: f32) -> Vec<[f32; 4]> {
        let Some(ui) = self.merchant.ui.as_ref() else {
            return Vec::new();
        };
        let Some(node) = ui
            .find_child_ex("MerchantFrameCloseButton")
            .owned(false)
            .done()
        else {
            return Vec::new();
        };
        let Ok(control) = node.try_cast::<godot::classes::Control>() else {
            return Vec::new();
        };
        let rect = control.get_global_rect();
        vec![[
            rect.position.x / scale,
            rect.position.y / scale,
            rect.size.x / scale,
            rect.size.y / scale,
        ]]
    }

    fn persist_merchant_position(&self) {
        let (Some(id), Some(position)) = (self.merchant.position_character, self.merchant.position)
        else {
            return;
        };
        let path =
            game_engine_core::client_options_data::options_path().with_file_name("ui_layout.ron");
        if let Err(error) =
            game_engine_core::ui_layout_data::save_window_position(&path, id, FRAME_NAME, position)
        {
            godot_error!("Merchant placement: {error}");
        }
    }

    /// Automation view of the vendor session: vendor, cells, buyback, bags, equipment, money.
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
        state.set("equipment", &equipment_items(&session.inventory));
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

/// Occupied equipment slots from the authoritative inventory state.
fn equipment_items(inventory: &game_engine_ui_model::bag_data::InventoryState) -> VarArray {
    inventory
        .equipment
        .iter()
        .map(|(slot, item)| {
            let mut entry = VarDictionary::new();
            entry.set("slot", format!("{slot:?}").as_str());
            entry.set("item_id", i64::from(item.item_id));
            entry.set("item_guid", item.item_guid as i64);
            entry.set("count", i64::from(item.count));
            entry.to_variant()
        })
        .collect()
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

pub(super) fn split_key(key: Key) -> Option<SplitKey> {
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
