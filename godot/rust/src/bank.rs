//! Native BankFrame and GuildBankFrame hosts, ported from the Bevy client's
//! `networking/bank` and `scenes/bank_frame`: the banker and Guild Vault roles open the
//! portable sessions, frame and bag clicks become `BankChannel` / `GuildBankChannel`
//! requests, and only the server's contents, inventory and Gold replies change what
//! the frames show (docs/specs/bank-frame.md, guild-bank-frame.md).

use game_engine_session::SessionScreen;
use game_engine_ui_model::bag_frame_component::ACTION_BAG_SLOT_PREFIX;
use game_engine_ui_model::bank::{
    BUY_BANK_TAB_POPUP, BagStacks, BankEffect, BankSession, InputTexts, ItemIcons,
};
use game_engine_ui_model::bank_frame_component::{self as bank_frame, MONEY_BOXES, TAB_NAME_BOX};
use game_engine_ui_model::cursor_item::CursorItem;
use game_engine_ui_model::guild_bank::{
    BUY_GUILD_BANK_TAB_POPUP, GuildBankEffect, GuildBankSession, NativeGuildBankView,
};
use game_engine_ui_model::guild_bank_frame_component::{self as guild_frame, INFO_BOX};
use game_engine_ui_model::item_icons::item_icon_fdid;
use game_engine_ui_model::merchant::Click;
use game_engine_ui_model::popup::{PopupOutcome, PopupResult, PopupSpec};
use godot::prelude::*;
use shared::protocol::{
    BankContents, GAMEOBJECT_TYPE_CHAIR, GAMEOBJECT_TYPE_GUILD_BANK, GameObjectInfo,
    GuildBankContents, InteractionKind, ItemLocation, NpcRole,
};

use crate::GameClient;
use crate::account::{BankMessage, NpcMessage};
use crate::frame_error::{FrameError, SessionError};
use crate::replicated::UnitFields;
use crate::ui::RegistryUi;

/// Server `GUILD_BANK_REACH`: the Guild Vault answers within five yards.
/// Game object interaction reach (`GAME_OBJECT_INTERACT_DISTANCE`), vaults and chairs.
const VAULT_RANGE: f32 = 5.0;

#[derive(Default)]
pub(crate) struct Banks {
    pub(crate) bank: BankSession,
    pub(crate) guild: GuildBankSession,
    /// The last Warband bank and guild vault contents the server sent, kept after the
    /// frames close (IPC `status warbank|guild-vault`).
    pub(crate) warbank_seen: Option<BankContents>,
    pub(crate) guild_vault_seen: Option<GuildBankContents>,
    pub(crate) bank_ui: Option<Gd<RegistryUi>>,
    guild_ui: Option<Gd<RegistryUi>>,
}

impl Banks {
    pub(crate) fn portrait_unit(&self) -> Option<u64> {
        self.bank.state.npc.filter(|_| self.bank.is_open())
    }

    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        for ui in [&mut self.bank_ui, &mut self.guild_ui]
            .into_iter()
            .flatten()
        {
            visit(ui)?;
        }
        Ok(())
    }

    fn free_bank_ui(&mut self) {
        if let Some(ui) = self.bank_ui.take() {
            ui.free();
        }
    }

    fn free_guild_ui(&mut self) {
        if let Some(ui) = self.guild_ui.take() {
            ui.free();
        }
    }

    fn reset(&mut self) {
        self.bank.close_frame();
        self.guild.close_frame();
        self.free_bank_ui();
        self.free_guild_ui();
    }
}

#[cfg(test)]
mod portrait_tests {
    use super::Banks;
    use shared::protocol::NpcRole;

    #[test]
    fn npcportraits_bank_follows_open_banker_and_clears_on_close() {
        let mut banks = Banks::default();
        assert_eq!(banks.portrait_unit(), None);
        assert!(!banks.bank.open_role(11, NpcRole::Vendor));
        assert_eq!(banks.portrait_unit(), None);
        assert!(banks.bank.open_role(42, NpcRole::Banker));
        assert_eq!(banks.portrait_unit(), Some(42));
        assert!(!banks.bank.close_for(11));
        assert_eq!(banks.portrait_unit(), Some(42));
        assert!(banks.bank.close_for(42));
        assert_eq!(banks.portrait_unit(), None);
        assert!(banks.bank.open_role(73, NpcRole::Banker));
        assert_eq!(banks.portrait_unit(), Some(73));
    }
}

/// Bank edit boxes whose text a click may read.
const BANK_INPUTS: [&str; 4] = [
    MONEY_BOXES.gold,
    MONEY_BOXES.silver,
    MONEY_BOXES.copper,
    TAB_NAME_BOX,
];
const GUILD_INPUTS: [&str; 4] = [
    guild_frame::MONEY_BOXES.gold,
    guild_frame::MONEY_BOXES.silver,
    guild_frame::MONEY_BOXES.copper,
    INFO_BOX,
];

fn read_texts(ui: &mut Gd<RegistryUi>, names: &[&str]) -> InputTexts {
    names
        .iter()
        .map(|name| {
            let text = ui.bind_mut().frame_text(GString::from(*name)).to_string();
            (name.to_string(), text)
        })
        .collect()
}

fn bag_slot(action: &str) -> Option<(u8, u8)> {
    let (bag, slot) = game_engine_ui_model::bag_frame_component::parse_bag_slot_action(action)?;
    Some((u8::try_from(bag).ok()?, u8::try_from(slot).ok()?))
}

/// A whole bag stack on the cursor, which a drop on a bank deposits.
fn cursor_bag_stack(item: &CursorItem) -> Option<(u8, u8)> {
    match item {
        CursorItem::Inventory {
            from: ItemLocation::Bag { bag, slot },
            split: false,
            ..
        } => Some((*bag, *slot)),
        _ => None,
    }
}

impl GameClient {
    /// The picker returned a drawn chair: `UseGameObject` within reach seats the player
    /// (TrinityCore `GameObject::Use` `GAMEOBJECT_TYPE_CHAIR`).
    pub(crate) fn use_chair(&mut self, id: u64) -> Result<bool, FrameError> {
        let is_chair = self
            .replica
            .unit(id)
            .and_then(|object| object.get::<GameObjectInfo>())
            .is_some_and(|info| info.go_type == GAMEOBJECT_TYPE_CHAIR);
        if !is_chair || !self.game_objects.contains(id) {
            return Ok(false);
        }
        let distance = self
            .world
            .local_player_transform()
            .zip(self.game_objects.position(id))
            .map_or(f32::INFINITY, |(player, position)| {
                player.origin.distance_to(position)
            });
        if distance <= VAULT_RANGE {
            self.account.send_use_game_object(id)?;
        }
        Ok(true)
    }

    /// The picker returned a drawn Guild Vault: `UseGameObject` within reach.
    pub(crate) fn use_guild_vault(&mut self, id: u64) -> Result<bool, FrameError> {
        let is_vault = self
            .replica
            .unit(id)
            .and_then(|object| object.get::<GameObjectInfo>())
            .is_some_and(|info| info.go_type == GAMEOBJECT_TYPE_GUILD_BANK);
        if !is_vault || !self.game_objects.contains(id) {
            return Ok(false);
        }
        let distance = self
            .world
            .local_player_transform()
            .zip(self.game_objects.position(id))
            .map_or(f32::INFINITY, |(player, position)| {
                player.origin.distance_to(position)
            });
        if distance <= VAULT_RANGE {
            self.account.send_use_game_object(id)?;
        }
        Ok(true)
    }

    /// Banker and Guild Vault roles open their frames, another role or the server's
    /// close ends them. `true` when the message opened a bank frame.
    pub(crate) fn bank_npc_message(&mut self, message: &NpcMessage) -> bool {
        match message {
            NpcMessage::Opened(opened) => {
                let role = match opened.kind {
                    InteractionKind::Role(role @ (NpcRole::Banker | NpcRole::GuildBanker)) => role,
                    _ => {
                        self.close_bank_frames();
                        return false;
                    }
                };
                self.merchant.session.close();
                self.mailbox.session.close();
                self.auction_interaction_closed_any();
                // Contents of this opening may already be here; only the other frame closes.
                if role == NpcRole::Banker {
                    if self.banks.guild.state.is_open() {
                        self.banks.guild.close_frame();
                    }
                    self.banks.bank.open_role(opened.npc, role)
                } else {
                    if self.banks.bank.is_open() {
                        self.banks.bank.close_frame();
                    }
                    self.banks.guild.open_role(opened.npc, role)
                }
            }
            NpcMessage::Closed(npc) => {
                self.banks.bank.close_for(*npc);
                self.banks.guild.close_for(*npc);
                false
            }
            _ => false,
        }
    }

    fn close_bank_frames(&mut self) {
        if self.banks.bank.is_open() {
            self.banks.bank.close_frame();
        }
        if self.banks.guild.state.is_open() {
            self.banks.guild.close_frame();
        }
    }

    pub(crate) fn receive_bank(&mut self, message: BankMessage) -> Result<(), FrameError> {
        match message {
            BankMessage::Contents(contents) => {
                self.banks.bank.apply_contents(contents);
                if let Some(account) = &self.banks.bank.state.account {
                    self.banks.warbank_seen = Some(account.clone());
                }
            }
            BankMessage::Failed(failed) => {
                if let Some(error) = self.banks.bank.apply_failed(failed) {
                    self.add_world_error(error)?;
                }
            }
            BankMessage::GuildContents(contents) => {
                let effects = self.banks.guild.apply_contents(contents);
                if let Some(seen) = &self.banks.guild.state.contents {
                    self.banks.guild_vault_seen = Some(seen.clone());
                }
                self.send_guild_effects(effects)?;
            }
            BankMessage::GuildLog(log) => self.banks.guild.apply_log(log),
            BankMessage::GuildFailed(failed) => {
                if let Some(error) = self.banks.guild.apply_failed(failed) {
                    self.add_world_error(error)?;
                }
            }
        }
        Ok(())
    }

    /// The bank opens the standalone backpack (Retail `OpenAllBags`) rather than
    /// embedding one; the guild bank embeds its own.
    pub(crate) fn bank_backpack_open(&self) -> bool {
        self.banks.bank.is_open()
    }

    pub(crate) fn guild_bank_backpack_embedded(&self) -> bool {
        self.banks.guild.state.is_open()
    }

    /// Right-click on a standalone bag slot with the bank open deposits it
    /// (`UseContainerItem` while `BankFrame` is shown).
    pub(crate) fn bank_bag_right_click(&mut self, action: &str) -> Result<bool, FrameError> {
        if !self.banks.bank.is_open() {
            return Ok(false);
        }
        let Some((bag, slot)) = bag_slot(action) else {
            return Ok(false);
        };
        let effects = self.banks.bank.deposit_bag(bag, slot);
        self.send_bank_effects(effects)?;
        Ok(true)
    }

    fn send_bank_effects(&mut self, effects: Vec<BankEffect>) -> Result<(), SessionError> {
        for effect in effects {
            match effect {
                BankEffect::Request { npc, request } => {
                    self.account.send_bank_request(npc, &request)?
                }
                BankEffect::CloseInteraction { npc } => {
                    self.banks.free_bank_ui();
                    self.account.send_close_interaction(npc)?;
                }
            }
        }
        Ok(())
    }

    fn send_guild_effects(&mut self, effects: Vec<GuildBankEffect>) -> Result<(), SessionError> {
        for effect in effects {
            match effect {
                GuildBankEffect::Request { object, request } => {
                    self.account.send_guild_bank_request(object, &request)?
                }
                GuildBankEffect::CloseInteraction { npc } => {
                    self.banks.free_guild_ui();
                    self.account.send_close_interaction(npc)?;
                }
            }
        }
        Ok(())
    }

    /// Escape's `CloseAllWindows` hides BankFrame and the bags it opened.
    pub(super) fn close_bank_window(&mut self) -> Result<bool, SessionError> {
        if !self.banks.bank.is_open() {
            return Ok(false);
        }
        let effects = self.banks.bank.close();
        self.send_bank_effects(effects)?;
        Ok(true)
    }

    /// Escape's `CloseAllWindows` hides GuildBankFrame and its embedded backpack.
    pub(super) fn close_guild_bank_window(&mut self) -> Result<bool, SessionError> {
        if !self.banks.guild.state.is_open() {
            return Ok(false);
        }
        let effects = self.banks.guild.close();
        self.send_guild_effects(effects)?;
        Ok(true)
    }

    /// `CONFIRM_BUY_BANK_TAB` / `CONFIRM_BUY_GUILDBANK_TAB` answers.
    pub(super) fn dispatch_bank_popup_results(
        &mut self,
        results: &[PopupResult],
    ) -> Result<(), FrameError> {
        for result in results {
            let accepted = result.outcome == PopupOutcome::Accepted;
            if result.key == BUY_BANK_TAB_POPUP {
                let effects = self.banks.bank.confirm_purchase(accepted);
                self.send_bank_effects(effects)?;
            } else if result.key == BUY_GUILD_BANK_TAB_POPUP {
                let effects = self.banks.guild.confirm_purchase(accepted);
                self.send_guild_effects(effects)?;
            }
        }
        Ok(())
    }

    fn sync_bank_popup(&mut self, key: &str, wanted: Option<PopupSpec>) {
        let popups = &mut self.group_frames.popups;
        match wanted {
            Some(spec) if !popups.contains(key) => {
                popups.push(spec);
                self.physical_input.clear_gameplay();
            }
            None if popups.contains(key) => {
                popups.hide(key);
            }
            _ => {}
        }
    }

    pub(super) fn update_banks(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.banks.reset();
            return Ok(());
        }
        self.feed_bank_sessions();
        self.sync_bank_popup(
            BUY_BANK_TAB_POPUP,
            self.banks.bank.purchase_confirmation.clone(),
        );
        self.sync_bank_popup(
            BUY_GUILD_BANK_TAB_POPUP,
            self.banks.guild.purchase_confirmation.clone(),
        );
        self.sync_bank_ui()?;
        Ok(self.sync_guild_bank_ui()?)
    }

    /// Bags, money and item icons the sessions read; the server owns all three.
    fn feed_bank_sessions(&mut self) {
        let money = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id)?.gold())
            .unwrap_or(0);
        let stacks = BagStacks::from_inventory(&self.merchant.session.inventory);
        let banks = &mut self.banks;
        banks.bank.inventory = stacks.clone();
        banks.bank.icons = ItemIcons(item_icon_fdid);
        banks.bank.money = money;
        banks.guild.inventory = stacks;
        banks.guild.icons = ItemIcons(item_icon_fdid);
        banks.guild.money = money;
    }

    fn is_ui(ui: &Option<Gd<RegistryUi>>, owner: i64) -> bool {
        ui.as_ref()
            .is_some_and(|ui| ui.instance_id().to_i64() == owner)
    }

    /// A press on a bank canvas from the shared cursor queue; `false` for other canvases.
    pub(super) fn bank_cursor_press(
        &mut self,
        owner: i64,
        action: &str,
        click: Click,
    ) -> Result<bool, FrameError> {
        if Self::is_ui(&self.banks.bank_ui, owner) {
            self.bank_press(action, click)?;
            return Ok(true);
        }
        if Self::is_ui(&self.banks.guild_ui, owner) {
            self.guild_bank_press(action, click)?;
            return Ok(true);
        }
        Ok(false)
    }

    /// A cursor stack released over a bank slot of any canvas deposits it.
    pub(super) fn bank_cursor_drop(
        &mut self,
        owner: i64,
        action: &str,
    ) -> Result<bool, FrameError> {
        if Self::is_ui(&self.banks.bank_ui, owner) {
            return self.drop_cursor_on_bank(action, Click::LEFT);
        }
        if Self::is_ui(&self.banks.guild_ui, owner) {
            return self.drop_cursor_on_guild_bank(action, Click::LEFT);
        }
        Ok(false)
    }

    fn bank_press(&mut self, action: &str, click: Click) -> Result<(), FrameError> {
        if self.drop_cursor_on_bank(action, click)? {
            return Ok(());
        }
        let mut ui = self.banks.bank_ui.clone().ok_or("Bank UI missing")?;
        let texts = read_texts(&mut ui, &BANK_INPUTS);
        let effects = self.banks.bank.click(action, click, &texts);
        Ok(self.send_bank_effects(effects)?)
    }

    /// A whole bag stack on the cursor clicked onto a bank slot is deposited.
    fn drop_cursor_on_bank(&mut self, action: &str, click: Click) -> Result<bool, FrameError> {
        if click.right || !action.starts_with(bank_frame::ACTION_SLOT_PREFIX) {
            return Ok(false);
        }
        let Some((bag, slot)) = cursor_bag_stack(&self.bags.cursor.item) else {
            return Ok(false);
        };
        let effects = self.banks.bank.deposit_bag(bag, slot);
        if !effects.is_empty() {
            self.bags.cursor.item = CursorItem::Empty;
        }
        self.send_bank_effects(effects)?;
        Ok(true)
    }

    fn guild_bank_press(&mut self, action: &str, click: Click) -> Result<(), FrameError> {
        if action.starts_with(ACTION_BAG_SLOT_PREFIX) {
            return self.guild_bag_click(action, click);
        }
        if self.drop_cursor_on_guild_bank(action, click)? {
            return Ok(());
        }
        let mut ui = self.banks.guild_ui.clone().ok_or("Guild bank UI missing")?;
        let texts = read_texts(&mut ui, &GUILD_INPUTS);
        let effects = self.banks.guild.click(action, click, &texts);
        Ok(self.send_guild_effects(effects)?)
    }

    fn drop_cursor_on_guild_bank(
        &mut self,
        action: &str,
        click: Click,
    ) -> Result<bool, FrameError> {
        if click.right || !action.starts_with(guild_frame::ACTION_SLOT_PREFIX) {
            return Ok(false);
        }
        let Some((bag, slot)) = cursor_bag_stack(&self.bags.cursor.item) else {
            return Ok(false);
        };
        let effects = self.banks.guild.deposit_bag(bag, slot);
        if !effects.is_empty() {
            self.bags.cursor.item = CursorItem::Empty;
        }
        self.send_guild_effects(effects)?;
        Ok(true)
    }

    /// Right-click on the embedded backpack deposits; other clicks keep the shared bag
    /// cursor behaviour.
    fn guild_bag_click(&mut self, action: &str, click: Click) -> Result<(), FrameError> {
        if !click.right {
            return self.dispatch_bag_action(action, click);
        }
        let Some((bag, slot)) = bag_slot(action) else {
            return Ok(());
        };
        let effects = self.banks.guild.deposit_bag(bag, slot);
        Ok(self.send_guild_effects(effects)?)
    }

    fn sync_bank_ui(&mut self) -> Result<(), String> {
        if !self.banks.bank.is_open() {
            self.banks.free_bank_ui();
            return Ok(());
        }
        let state = self.banks.bank.frame_state();
        let scale = self.effective_ui_scale();
        if self.banks.bank_ui.is_none() {
            let mut ui = RegistryUi::new_alloc();
            ui.set_name("BankUI");
            self.base_mut().add_child(&ui);
            let shown = {
                let mut host = ui.bind_mut();
                host.set_ui_scale(scale)
                    .and_then(|()| host.show_bank(state.clone()))
            };
            if let Err(error) = shown {
                ui.free();
                return Err(error);
            }
            self.banks.bank_ui = Some(ui);
        }
        let mut ui = self.banks.bank_ui.clone().ok_or("Bank UI missing")?;
        ui.bind_mut().set_ui_scale(scale)?;
        ui.bind_mut().set_state(state)?;
        for (name, text) in std::mem::take(&mut self.banks.bank.text_edits) {
            ui.bind_mut().set_editbox_text(name, &text)?;
        }
        Ok(())
    }

    fn sync_guild_bank_ui(&mut self) -> Result<(), String> {
        if !self.banks.guild.state.is_open() {
            self.banks.free_guild_ui();
            return Ok(());
        }
        self.read_bag_search(self.banks.guild_ui.clone());
        let mut bags = self.merchant.session.bag_state();
        for bag in &mut bags.bags {
            bag.visible = bag.bag_index == 0;
        }
        let view = NativeGuildBankView {
            frame: self.banks.guild.frame_state(),
            bags,
        };
        let scale = self.effective_ui_scale();
        if self.banks.guild_ui.is_none() {
            let mut ui = RegistryUi::new_alloc();
            ui.set_name("GuildBankUI");
            self.base_mut().add_child(&ui);
            let shown = {
                let mut host = ui.bind_mut();
                host.set_ui_scale(scale)
                    .and_then(|()| host.show_guild_bank(view.clone()))
            };
            if let Err(error) = shown {
                ui.free();
                return Err(error);
            }
            self.banks.guild_ui = Some(ui);
        }
        let mut ui = self.banks.guild_ui.clone().ok_or("Guild bank UI missing")?;
        ui.bind_mut().set_ui_scale(scale)?;
        ui.bind_mut().set_state(view)?;
        for (name, text) in std::mem::take(&mut self.banks.guild.text_edits) {
            ui.bind_mut().set_editbox_text(name, &text)?;
        }
        Ok(())
    }

    /// Read-only bank state for fixtures; requests only come from real frame input.
    pub(super) fn bank_snapshot(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        let bank = &self.banks.bank.state;
        state.set("open", bank.is_open());
        state.set(
            "npc",
            &bank
                .npc
                .map(|id| (id as i64).to_variant())
                .unwrap_or_default(),
        );
        let guild = &self.banks.guild.state;
        state.set("guild_open", guild.is_open());
        state.set(
            "guild_object",
            &guild
                .object
                .map(|id| (id as i64).to_variant())
                .unwrap_or_default(),
        );
        state.set(
            "guild_error",
            self.banks.guild.error.as_deref().unwrap_or_default(),
        );
        state
    }
}
