//! Original cursor-item policy projected for standalone bag slots and stack splits.

use game_engine_ui_model::bag_data::InventoryRequest;
use game_engine_ui_model::bag_frame_component::{parse_bag_close_action, parse_bag_slot_action};
use game_engine_ui_model::character_frame::parse_equipment_slot_action;
use game_engine_ui_model::cursor_item::{CursorEffect, CursorItem, CursorTarget};
use game_engine_ui_model::cursor_item_component::{CursorItemFrameState, cursor_item_screen};
use game_engine_ui_model::item_catalog::item_catalog_entry_for;
use game_engine_ui_model::merchant::{Click, SplitKey};
use game_engine_ui_model::merchant_frame_component::{ACTION_FRAME, ACTION_ITEM_PREFIX};
use game_engine_ui_model::stack_split::{StackSplitOwner, StackSplitState};
use game_engine_ui_model::stack_split_frame_component::{
    ACTION_CANCEL, ACTION_LEFT, ACTION_OKAY, ACTION_RIGHT, FRAME_W, StackSplitFrameState,
    stack_split_frame_screen,
};
use game_engine_ui_model::trade_frame_component::ACTION_PLAYER_SLOT_PREFIX;
use godot::classes::Control;
use godot::global::Key;
use godot::prelude::*;
use shared::protocol::{EquipItem, ItemLocation, UseItem};
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::GameClient;
use crate::frame_error::FrameError;
use crate::ui::RegistryUi;
use crate::window_stack::UiHit;

const CURSOR_UI: &str = "CursorItemUI";
/// The original cursor icon is above every frame and ignores pointer input.
const CURSOR_LAYER: i32 = 100;
/// Original OnReceiveDrag threshold, measured in logical UI pixels.
const DRAG_THRESHOLD: f32 = 4.0;

pub(crate) enum BagInput {
    Click {
        owner: i64,
        action: String,
        click: Click,
        at: Option<Vector2>,
    },
    /// Every cursor canvas reports each release; only the pickup's canvas acts on it.
    Release {
        owner: i64,
        at: Vector2,
        physical_at: Vector2,
    },
}

pub(crate) struct SpellDrag {
    pub(crate) owner: i64,
    pub(crate) origin: Vector2,
    pub(crate) spell_id: u32,
    pub(crate) icon_fdid: u32,
}

#[derive(Default)]
pub(crate) struct BagCursor {
    pub(crate) item: CursorItem,
    pub(crate) spell_drag: Option<SpellDrag>,
    split: Option<StackSplitState>,
    picked_at: Option<(i64, Vector2, CursorTarget)>,
    pub(crate) ui: Option<Gd<RegistryUi>>,
}

#[derive(Clone, PartialEq)]
pub(crate) struct CursorView {
    icon: CursorItemFrameState,
    split: StackSplitFrameState,
}

pub(crate) fn cursor_screen(ctx: &SharedContext) -> Element {
    let view = ctx
        .get::<CursorView>()
        .expect("CursorView must be in SharedContext");
    let mut shared = SharedContext::new();
    shared.insert(view.icon.clone());
    shared.insert(view.split.clone());
    let mut elements = cursor_item_screen(&shared);
    elements.extend(stack_split_frame_screen(&shared));
    elements
}

impl BagCursor {
    pub(crate) fn reset(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
        self.item = CursorItem::Empty;
        self.spell_drag = None;
        self.split = None;
        self.picked_at = None;
    }
}

impl GameClient {
    pub(super) fn dispatch_bag_cursor_input(&mut self, input: BagInput) -> Result<(), FrameError> {
        match input {
            BagInput::Click {
                owner,
                action,
                click,
                at,
            } => self.send_cursor_press(owner, &action, click, at),
            BagInput::Release {
                owner,
                at,
                physical_at,
            } => {
                if self.character_frame_input_owner(owner) {
                    self.release_character_frame_pointer();
                }
                self.send_bag_drag_release(owner, at, physical_at)
            }
        }
    }

    fn send_cursor_press(
        &mut self,
        owner: i64,
        action: &str,
        click: Click,
        at: Option<Vector2>,
    ) -> Result<(), FrameError> {
        self.bags.cursor.picked_at = None;
        if self.toybox_cursor_press(owner, action, click, at)? {
            return Ok(());
        }
        if self.spell_cursor_press(owner, action, click, at)? {
            return Ok(());
        }
        let was_empty = self.bags.cursor.item.is_empty();
        let target = self
            .bank_cursor_target(owner, action)
            .or(cursor_action_target(action)?);
        if parse_bag_close_action(action).is_some() {
            // A container close button, also on a backpack an NPC canvas draws.
            self.dispatch_bag_action(action, click)?;
        } else if self.merchant_input_owner(owner) {
            self.merchant_cursor_click(action, click)?;
        } else if self.trade_input_owner(owner) {
            self.trade_cursor_click(action, target)?;
        } else if self.character_frame_input_owner(owner) {
            self.character_frame_click(action, click)?;
        } else if self.mail_input_owner(owner) {
            self.mail_cursor_click(action, click)?;
        } else if !self.bank_cursor_press(owner, action, click)? {
            self.dispatch_bag_action(action, click)?;
        }
        if was_empty && !self.bags.cursor.item.is_empty() {
            self.bags.cursor.picked_at = at.zip(target).map(|(at, target)| (owner, at, target));
        }
        Ok(())
    }

    fn send_bag_drag_release(
        &mut self,
        owner: i64,
        at: Vector2,
        physical_at: Vector2,
    ) -> Result<(), FrameError> {
        if self.finish_toy_drag(owner, at, physical_at)? {
            return Ok(());
        }
        if self.finish_spell_drag(owner, at, physical_at)? {
            return Ok(());
        }
        let Some((picked_at, picked_target)) = self.take_owned_drag_origin(owner) else {
            return Ok(());
        };
        let distance = picked_at.distance_to(at);
        if distance < DRAG_THRESHOLD {
            return Ok(());
        }
        // The drop target is the topmost frame of any canvas, not the pickup's own.
        let hit = self.ui_hit_at(physical_at)?;
        if let Some(action) = hit.as_ref().and_then(|hit| hit.action.as_deref())
            && self.assign_cursor_to_action_button(action)?
        {
            return Ok(());
        }
        if let Some(UiHit {
            owner,
            action: Some(action),
        }) = &hit
            && self.bank_cursor_drop(*owner, action)?
        {
            return Ok(());
        }
        let Some(target) = bag_release_target(hit.map(|hit| hit.action))? else {
            return Ok(());
        };
        if target == picked_target {
            return Ok(());
        }
        self.send_cursor_click(target)
    }

    fn take_owned_drag_origin(&mut self, owner: i64) -> Option<(Vector2, CursorTarget)> {
        let (picked_owner, _, _) = self.bags.cursor.picked_at.as_ref()?;
        if *picked_owner != owner {
            return None;
        }
        self.bags
            .cursor
            .picked_at
            .take()
            .map(|(_, at, target)| (at, target))
    }

    pub(super) fn send_cursor_click(&mut self, target: CursorTarget) -> Result<(), FrameError> {
        if let CursorTarget::TradeSlot(slot) = target {
            return Ok(self.place_trade_item(slot)?);
        }
        let session = &self.merchant.session;
        let effect = self
            .bags
            .cursor
            .item
            .click(target, &session.inventory, &session.merchant);
        self.send_cursor_effect(effect)
    }

    pub(super) fn clear_stale_bag_cursor(&mut self) {
        let session = &self.merchant.session;
        self.bags
            .cursor
            .item
            .clear_if_stale(&session.inventory, &session.merchant);
        if let Some(StackSplitOwner::Bag(location)) =
            self.bags.cursor.split.as_ref().map(|s| s.owner)
            && session
                .inventory
                .item_at(location)
                .is_none_or(|item| item.count < 2)
        {
            self.bags.cursor.split = None;
        }
    }

    pub(super) fn bag_cursor_click(
        &mut self,
        action: &str,
        click: Click,
    ) -> Result<(), FrameError> {
        let location = parse_bag_location(action)?;
        self.bags.cursor.split = None;
        if !click.right
            && let Some(effect) = self.merchant.session.repair_click(location)
        {
            return Ok(self.apply_merchant_effect(effect)?);
        }
        if click.right {
            // ContainerFrame.lua:1405-1406: with TradeFrame shown the item goes to the trade.
            if self.offer_trade_item(action)? {
                return Ok(());
            }
            return self.send_bag_use_request(location);
        }
        self.click_inventory_slot(location, click)
    }

    pub(super) fn clear_bag_split(&mut self) {
        self.bags.cursor.split = None;
    }

    /// Bank and bag slots share the original cursor and StackSplitFrame picker.
    pub(super) fn click_inventory_slot(
        &mut self,
        location: ItemLocation,
        click: Click,
    ) -> Result<(), FrameError> {
        if click.shift && self.bags.cursor.item.is_empty() {
            self.open_bag_split(location);
            return Ok(());
        }
        self.send_cursor_click(CursorTarget::Location(location))
    }

    /// `C_Container.UseContainerItem` (ContainerFrame.lua:1342): an equippable item is
    /// equipped, any other item used (its on-use spell, server-side).
    fn send_bag_use_request(&self, location: ItemLocation) -> Result<(), FrameError> {
        let Some(item) = self.merchant.session.inventory.item_at(location) else {
            return Ok(());
        };
        let can_equip = item_catalog_entry_for(item.definition_source, item.item_id)
            .is_some_and(|entry| entry.inventory_type != 0);
        let request = if can_equip {
            InventoryRequest::Equip(EquipItem { from: location })
        } else {
            InventoryRequest::Use(UseItem {
                location,
                target: None,
            })
        };
        self.account.send_inventory_request(&request)?;
        Ok(())
    }

    fn open_bag_split(&mut self, location: ItemLocation) {
        let count = self
            .merchant
            .session
            .inventory
            .item_at(location)
            .map_or(0, |item| item.count);
        self.bags.cursor.split = (count > 1)
            .then(|| StackSplitState::open(StackSplitOwner::Bag(location), count, 1))
            .flatten();
        if self.bags.cursor.split.is_some() {
            self.physical_input.clear_gameplay();
        }
    }

    pub(super) fn send_cursor_effect(
        &mut self,
        effect: Option<CursorEffect>,
    ) -> Result<(), FrameError> {
        match effect {
            None => Ok(()),
            Some(CursorEffect::Inventory(request)) => {
                Ok(self.account.send_inventory_request(&request)?)
            }
            Some(CursorEffect::Merchant(request)) => {
                let npc = self
                    .merchant
                    .session
                    .merchant
                    .npc
                    .ok_or("Cursor drop without vendor")?;
                Ok(self.account.send_merchant_request(npc, &request)?)
            }
            Some(CursorEffect::ConfirmDestroy(confirm)) => {
                self.group_frames
                    .popups
                    .push(game_engine_ui_model::cursor_item::destroy_popup(&confirm));
                self.physical_input.clear_gameplay();
                Ok(())
            }
        }
    }

    pub(super) fn bag_cursor_text_input(&self) -> bool {
        self.bags.cursor.split.is_some()
    }

    /// The split owns the keyboard; cursor Escape precedes every window Escape step.
    pub(super) fn bag_cursor_key(&mut self, key: Key) -> bool {
        if self.bag_cursor_text_input() {
            if let Some(key) = crate::merchant::split_key(key) {
                self.apply_bag_split_key(key);
            }
            return true;
        }
        if key == Key::ESCAPE && !self.bags.cursor.item.is_empty() {
            self.bags.cursor.item = CursorItem::Empty;
            return true;
        }
        false
    }

    fn apply_bag_split_key(&mut self, key: SplitKey) {
        match key {
            SplitKey::Enter => self.finish_bag_split(true),
            SplitKey::Escape => self.finish_bag_split(false),
            other => {
                let split = self.bags.cursor.split.as_mut().expect("bag split is open");
                match other {
                    SplitKey::Digit(digit) => split.type_digit(digit),
                    SplitKey::Backspace => split.backspace(),
                    SplitKey::Decrement => split.decrement(),
                    SplitKey::Increment => split.increment(),
                    SplitKey::Enter | SplitKey::Escape => unreachable!(),
                }
            }
        }
    }

    fn finish_bag_split(&mut self, accepted: bool) {
        let Some(split) = self.bags.cursor.split.take() else {
            return;
        };
        if accepted
            && self.bags.cursor.item.is_empty()
            && let StackSplitOwner::Bag(location) = split.owner
        {
            self.bags.cursor.item =
                CursorItem::split_from(&self.merchant.session.inventory, location, split.split);
        }
    }

    pub(super) fn sync_bag_cursor(&mut self) -> Result<(), String> {
        self.poll_bag_split_actions()?;
        if self.bags.cursor.ui.is_none()
            && self.bags.cursor.item.is_empty()
            && self.bags.cursor.spell_drag.is_none()
            && self.toybox.drag.is_none()
            && self.bags.cursor.split.is_none()
        {
            return Ok(());
        }
        let view = self.bag_cursor_view()?;
        let scale = self.effective_ui_scale();
        if self.bags.cursor.ui.is_none() {
            self.mount_bag_cursor(view.clone(), scale)?;
        }
        let mut ui = self
            .bags
            .cursor
            .ui
            .clone()
            .ok_or("Cursor item UI missing")?;
        ui.bind_mut().set_ui_scale(scale)?;
        ui.bind_mut().set_state(view)
    }

    fn poll_bag_split_actions(&mut self) -> Result<(), String> {
        let Some(mut ui) = self.bags.cursor.ui.clone() else {
            return Ok(());
        };
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            if self.bags.cursor.split.is_none() {
                continue;
            }
            match action.as_str() {
                ACTION_OKAY => self.finish_bag_split(true),
                ACTION_CANCEL => self.finish_bag_split(false),
                ACTION_LEFT => self.apply_bag_split_key(SplitKey::Decrement),
                ACTION_RIGHT => self.apply_bag_split_key(SplitKey::Increment),
                other => return Err(format!("Unknown bag split action: {other}")),
            }
        }
        Ok(())
    }

    fn bag_cursor_view(&self) -> Result<CursorView, String> {
        let pointer =
            Vector2::from_array(self.physical_input.pointer()) / self.effective_ui_scale();
        Ok(CursorView {
            icon: CursorItemFrameState {
                icon_fdid: self
                    .bags
                    .cursor
                    .spell_drag
                    .as_ref()
                    .map(|drag| drag.icon_fdid)
                    .or_else(|| self.toybox.drag.as_ref().map(|drag| drag.icon_fdid))
                    .or_else(|| self.bags.cursor.item.icon_fdid()),
                position: [pointer.x, pointer.y],
            },
            split: self.bag_split_view()?,
        })
    }

    fn bag_split_view(&self) -> Result<StackSplitFrameState, String> {
        let Some(split) = &self.bags.cursor.split else {
            return Ok(StackSplitFrameState::default());
        };
        let control = match split.owner {
            StackSplitOwner::Bag(ItemLocation::Bag { bag, slot }) => {
                self.find_bag_split_owner(bag, slot)?
            }
            StackSplitOwner::Bag(ItemLocation::Bank { slot, .. }) => {
                let name = format!("BankFrameItem{}", slot + 1);
                let ui = self
                    .banks
                    .bank_ui
                    .as_ref()
                    .ok_or("Split owner Bank UI missing")?;
                ui.bind()
                    .frame_control(&name)
                    .ok_or_else(|| format!("Split owner bank slot {name} missing"))?
            }
            _ => return Err("Standalone split owner is not a container slot".into()),
        };
        let rect = control.get_global_rect();
        let scale = self.effective_ui_scale();
        let mut state = StackSplitFrameState {
            visible: true,
            text: split.text(),
            total_text: split.total_text(),
            left_enabled: split.left_enabled(),
            right_enabled: split.right_enabled(),
            ..Default::default()
        };
        state.x = rect.end().x / scale - FRAME_W;
        state.y = rect.position.y / scale - state.height();
        Ok(state)
    }

    fn find_bag_split_owner(&self, bag: u8, slot: u8) -> Result<Gd<Control>, String> {
        let owner_name = format!("ContainerFrame{bag}Slot{slot}");
        if self.merchant.session.is_open() && bag == 0 {
            return self.merchant_frame_control(&owner_name);
        }
        let ui = self.bags.ui.as_ref().ok_or("Split owner Bags UI missing")?;
        ui.bind()
            .frame_control(&owner_name)
            .ok_or_else(|| format!("Split owner bag slot {owner_name} missing"))
    }

    fn mount_bag_cursor(&mut self, view: CursorView, scale: f32) -> Result<(), String> {
        let mut ui = RegistryUi::new_alloc();
        ui.set_name(CURSOR_UI);
        ui.set_layer(CURSOR_LAYER);
        self.base_mut().add_child(&ui);
        let shown = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_cursor_item(view))
        };
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.bags.cursor.ui = Some(ui);
        Ok(())
    }
}

fn bag_release_target(action: Option<Option<String>>) -> Result<Option<CursorTarget>, String> {
    match action {
        None => Ok(Some(CursorTarget::World)),
        Some(Some(action)) => cursor_action_target(&action),
        Some(None) => Ok(None),
    }
}

pub(super) fn cursor_action_target(action: &str) -> Result<Option<CursorTarget>, String> {
    if let Some(slot) =
        parse_equipment_slot_action(action).or_else(|| crate::bags::bag_slot_target(action))
    {
        return Ok(Some(CursorTarget::Location(ItemLocation::Equipment(slot))));
    }
    if action.starts_with(game_engine_ui_model::bag_frame_component::ACTION_BAG_SLOT_PREFIX) {
        return parse_bag_location(action)
            .map(CursorTarget::Location)
            .map(Some);
    }
    if let Some(index) = action.strip_prefix(ACTION_ITEM_PREFIX) {
        let index = index
            .parse()
            .map_err(|error| format!("Merchant cell {index}: {error}"))?;
        return Ok(Some(CursorTarget::MerchantItem(index)));
    }
    if let Some(slot) = action.strip_prefix(ACTION_PLAYER_SLOT_PREFIX) {
        let slot = slot
            .parse()
            .map_err(|error| format!("Trade slot {slot}: {error}"))?;
        return Ok(Some(CursorTarget::TradeSlot(slot)));
    }
    Ok((action == ACTION_FRAME).then_some(CursorTarget::MerchantFrame))
}

fn parse_bag_location(action: &str) -> Result<ItemLocation, String> {
    let (bag, slot) = parse_bag_slot_action(action)
        .ok_or_else(|| format!("Invalid bag slot action: {action}"))?;
    Ok(ItemLocation::Bag {
        bag: u8::try_from(bag).map_err(|error| format!("Bag index {bag}: {error}"))?,
        slot: u8::try_from(slot).map_err(|error| format!("Bag slot {slot}: {error}"))?,
    })
}
