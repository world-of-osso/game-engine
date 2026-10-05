//! Native mail host. Only actual mailbox use opens the authored MailFrame; the
//! portable session decides every request, and the server's replies change state.
use crate::replicated::UnitFields;
use crate::{
    GameClient,
    account::MailMessage,
    frame_error::{FrameError, SessionError},
    ui::RegistryUi,
    world_map::{WindowDrag, title_hit},
};
use game_engine_session::SessionScreen;
use game_engine_ui_model::{
    bag_frame_component::{ACTION_BAG_SLOT_PREFIX, parse_bag_slot_action},
    mail::{
        COD_POPUP, DELETE_MAIL_POPUP, DELETE_MONEY_POPUP, MailEffect, MailOutgoing, MailSession,
        MailTexts, NativeMailView, bag_item, can_use_mailbox, input_letters, input_names,
    },
    mail_frame_component::{ACTION_CLOSE, FRAME_NAME, MONEY_BOXES, MailFrameTab},
    merchant::Click,
    popup::{PopupOutcome, PopupResult},
};
use godot::{
    classes::{InputEvent, InputEventMouseButton, InputEventMouseMotion},
    global::MouseButton,
    prelude::*,
};
use shared::protocol::GameObjectInfo;
use ui_toolkit::frame::WidgetData;

#[derive(Default)]
pub(crate) struct Mailbox {
    pub session: MailSession,
    pub ui: Option<Gd<RegistryUi>>,
    texts: MailTexts,
    position: Option<[f32; 2]>,
    drag: Option<WindowDrag>,
}
impl Mailbox {
    fn free_ui(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
        self.texts.clear();
        self.drag = None;
    }
    /// A new connection: nothing of the last one's mail survives.
    pub fn reset(&mut self) {
        self.session.reset();
        self.free_ui();
        self.position = None;
    }
    /// Leaving the world view or its map closes the mailbox. The unread senders stay:
    /// the server sends `PendingMail` only when they change, and it can arrive while
    /// the world is still loading.
    pub fn close(&mut self) {
        self.session.close();
        self.free_ui();
    }
    pub fn close_for(&mut self, object: u64) {
        self.session.close_for(object);
        if !self.session.is_open() {
            self.free_ui();
        }
    }
    /// The Send Mail edit boxes as typed, cut to their Retail `letters` (money digits
    /// only); returns the boxes whose text had to be cut.
    /// Send Mail edit box texts, cut to their letters (money boxes digits only).
    fn read_inputs(&mut self) {
        let Some(ui) = self.ui.as_ref() else {
            return;
        };
        for name in input_names() {
            let Some(text) = editbox_text(ui, name) else {
                continue;
            };
            let money = MONEY_BOXES.all().contains(&name);
            let letters = input_letters(name).unwrap_or(usize::MAX);
            let text = text
                .chars()
                .filter(|c| !money || c.is_ascii_digit())
                .take(letters)
                .collect();
            self.texts.insert(name, text);
        }
    }
}
impl GameClient {
    /// The picker returned a real drawn mailbox. No SetTarget or NPC interaction for it.
    pub(crate) fn use_mailbox(&mut self, id: u64) -> Result<bool, FrameError> {
        let info = self
            .replica
            .unit(id)
            .and_then(|object| object.get::<GameObjectInfo>());
        let Some(info) = info.filter(|_| self.game_objects.contains(id)) else {
            return Ok(false);
        };
        let distance = self
            .world
            .local_player_transform()
            .zip(self.game_objects.position(id))
            .map_or(f32::INFINITY, |(player, position)| {
                player.origin.distance_to(position)
            });
        if can_use_mailbox(info, distance) {
            self.account.send_use_game_object(id)?;
            self.mailbox.session.expect_open(id);
        }
        Ok(true)
    }
    pub(crate) fn open_mailbox(&mut self, id: u64) {
        self.mailbox.session.open(id);
        if self.mailbox.session.is_open() {
            self.merchant.session.close();
            self.auction_interaction_closed_any();
        }
    }
    pub(crate) fn receive_mail(&mut self, message: MailMessage) -> Result<(), String> {
        match message {
            MailMessage::Contents(contents) => self.mailbox.session.receive_contents(contents),
            MailMessage::Failed(failed) => {
                if let Some(error) = self.mailbox.session.failed(failed) {
                    self.add_world_error(error)?;
                }
            }
            MailMessage::Sent(sent) => {
                if let Some(effect) = self.mailbox.session.mail_sent(sent) {
                    self.apply_mail_effect(effect)?;
                }
            }
            MailMessage::Pending(pending) => self.mailbox.session.pending_senders = pending.senders,
        }
        Ok(())
    }
    fn close_mailbox(&mut self) -> Result<(), SessionError> {
        let object = self.mailbox.session.object;
        self.mailbox.session.close();
        self.mailbox.free_ui();
        for key in game_engine_ui_model::mail::POPUP_KEYS {
            self.group_frames.popups.hide(key);
        }
        if let Some(object) = object {
            self.account.send_close_interaction(object)?;
        }
        Ok(())
    }
    /// Escape's `CloseAllWindows` hides MailFrame and an open letter together.
    pub(super) fn close_mailbox_window(&mut self) -> Result<bool, SessionError> {
        if !self.mailbox.session.is_open() {
            return Ok(false);
        }
        self.close_mailbox()?;
        Ok(true)
    }
    fn money(&self) -> u64 {
        self.world
            .local_player_id()
            .and_then(|id| self.replica.unit(id)?.gold())
            .unwrap_or(0)
    }
    pub(super) fn update_mailbox(&mut self) -> Result<(), FrameError> {
        self.hide_stale_mail_popups();
        if self.account.session.screen != SessionScreen::InWorld {
            self.mailbox.close();
            return Ok(());
        }
        if !self.mailbox.session.is_open() {
            self.mailbox.free_ui();
            return Ok(());
        }
        self.mailbox.read_inputs();
        let inventory = &self.merchant.session.inventory;
        self.mailbox.session.retain_attachments(inventory);
        let free = inventory.total_free_slots();
        if let Some(request) = self.mailbox.session.next_open_all(free) {
            self.account.send_mail_request(request)?;
        }
        Ok(self.sync_mailbox_ui()?)
    }
    fn sync_mailbox_ui(&mut self) -> Result<(), String> {
        self.read_bag_search(self.mailbox.ui.clone());
        let view = self.mailbox_view();
        let scale = self.effective_ui_scale();
        if self.mailbox.ui.is_none() {
            let mut ui = RegistryUi::new_alloc();
            ui.set_name("MailboxUI");
            self.base_mut().add_child(&ui);
            let shown = ui.bind_mut().show_mail(view.clone());
            if let Err(error) = shown {
                ui.free();
                return Err(error);
            }
            self.mailbox.ui = Some(ui);
        }
        let mut ui = self.mailbox.ui.clone().ok_or("Mailbox UI missing")?;
        ui.bind_mut().set_ui_scale(scale)?;
        ui.bind_mut().set_state(view)?;
        // Edit boxes show the form's texts: cut input, and texts set while their tab
        // was hidden (Reply fills To and Subject from the Inbox).
        for name in input_names() {
            let text = self.mailbox.texts.get(name).cloned().unwrap_or_default();
            if editbox_text(&ui, name).is_some_and(|shown| shown != text) {
                ui.bind_mut().set_editbox_text(name, &text)?;
            }
        }
        if let Some(position) = self.mailbox.position {
            ui.bind_mut().set_window_position(FRAME_NAME, position)?;
        }
        Ok(())
    }
    /// The frame, and the backpack with the attached items locked (`SetItemButtonDesaturated`).
    fn mailbox_view(&self) -> NativeMailView {
        let inventory = &self.merchant.session.inventory;
        let session = &self.mailbox.session;
        let mut bags = self.merchant.session.bag_state();
        for bag in &mut bags.bags {
            bag.visible = bag.bag_index == 0;
            let slots = inventory.slots.get(bag.bag_index);
            for (index, slot) in bag.slots.iter_mut().enumerate() {
                let guid = slots.and_then(|s| s.get(index)).map_or(0, |s| s.item_guid);
                slot.locked = guid != 0 && session.attachments.contains(&guid);
            }
        }
        NativeMailView {
            frame: session.view(self.money(), inventory, &self.mailbox.texts),
            bags,
        }
    }
    fn set_mail_text(
        &mut self,
        ui: &mut Gd<RegistryUi>,
        name: &'static str,
        text: &str,
    ) -> Result<(), String> {
        let present = ui
            .bind()
            .registry()
            .is_some_and(|reg| reg.get_by_name(name).is_some());
        if present {
            ui.bind_mut().set_editbox_text(name, text)?;
        }
        self.mailbox.texts.insert(name, text.to_string());
        Ok(())
    }
    pub(super) fn mail_input_owner(&self, owner: i64) -> bool {
        self.mailbox
            .ui
            .as_ref()
            .is_some_and(|ui| ui.instance_id().to_i64() == owner)
    }
    /// A click in the mail UI: Send Mail takes right-clicked bag items, other bag
    /// clicks keep their bag behavior, and the rest are MailFrame actions.
    pub(super) fn mail_cursor_click(
        &mut self,
        action: &str,
        click: Click,
    ) -> Result<(), FrameError> {
        if action.starts_with(ACTION_BAG_SLOT_PREFIX) {
            if let Some(handled) = self.mail_bag_slot_click(action, click) {
                return Ok(handled?);
            }
            return self.bag_cursor_click(action, click);
        }
        if action == ACTION_CLOSE {
            return Ok(self.close_mailbox()?);
        }
        let money = self.money();
        let effect = self.mailbox.session.click(
            action,
            &self.mailbox.texts,
            money,
            &self.merchant.session.inventory,
        );
        Ok(self.apply_mail_effect(effect)?)
    }
    /// A bag slot click, from any bag, while the mailbox is open: attached items stay
    /// locked, and a right-click while Send Mail shows attaches. None: the bags act.
    pub(super) fn mail_bag_slot_click(
        &mut self,
        action: &str,
        click: Click,
    ) -> Option<Result<(), String>> {
        if !self.mailbox.session.is_open() {
            return None;
        }
        let (bag, slot) = parse_bag_slot_action(action)?;
        let item = self.merchant.session.inventory.slot(bag, slot);
        if item.is_some_and(|item| {
            !item.is_empty() && self.mailbox.session.is_attached(item.item_guid)
        }) {
            return Some(Ok(()));
        }
        if click.right && self.mailbox.session.tab == MailFrameTab::Send {
            return Some(self.attach_bag_item(action));
        }
        None
    }
    /// Confirmations whose open mail went away (`OpenMailFrame_OnHide`).
    fn hide_stale_mail_popups(&mut self) {
        let confirming = self.mailbox.session.confirming();
        for key in [COD_POPUP, DELETE_MAIL_POPUP, DELETE_MONEY_POPUP] {
            if confirming != Some(key) {
                self.group_frames.popups.hide(key);
            }
        }
    }
    fn attach_bag_item(&mut self, action: &str) -> Result<(), String> {
        let (bag, slot) = parse_bag_slot_action(action)
            .ok_or_else(|| format!("Invalid bag slot action: {action}"))?;
        let inventory = &self.merchant.session.inventory;
        let Some(item) = inventory.slot(bag, slot).filter(|item| !item.is_empty()) else {
            return Ok(());
        };
        let item = item.clone();
        let effect = self
            .mailbox
            .session
            .attach(&item, &self.mailbox.texts, inventory);
        self.apply_mail_effect(effect)
    }
    fn apply_mail_effect(&mut self, effect: MailEffect) -> Result<(), String> {
        match effect.outgoing {
            Some(MailOutgoing::Request(request)) => {
                self.account.send_mail_request(request).map_err(|e| e.0)?
            }
            Some(MailOutgoing::Send(mail)) => self.account.send_mail(mail).map_err(|e| e.0)?,
            None => {}
        }
        if let Some(popup) = effect.popup {
            self.group_frames.popups.push(popup);
        }
        if let Some(mut ui) = self.mailbox.ui.clone() {
            for (name, text) in effect.edits {
                self.set_mail_text(&mut ui, name, &text)?;
            }
        } else {
            for (name, text) in effect.edits {
                self.mailbox.texts.insert(name, text);
            }
        }
        if let Some(error) = effect.error {
            self.add_world_error(error)?;
        }
        Ok(())
    }
    /// `COD_CONFIRMATION` / `DELETE_MAIL` / `DELETE_MONEY` answers.
    pub(super) fn dispatch_mail_popup_results(
        &mut self,
        results: &[PopupResult],
    ) -> Result<(), FrameError> {
        for result in results {
            let accepted = result.outcome == PopupOutcome::Accepted;
            match self.mailbox.session.popup_result(&result.key, accepted) {
                Some(MailOutgoing::Request(request)) => self.account.send_mail_request(request)?,
                Some(MailOutgoing::Send(mail)) => self.account.send_mail(mail)?,
                None => {}
            }
        }
        Ok(())
    }
    pub(super) fn mailbox_snapshot(&self) -> VarDictionary {
        let session = &self.mailbox.session;
        let mut state = VarDictionary::new();
        state.set("open", session.is_open());
        let mut objects = Array::<VarDictionary>::new();
        for unit in self.replica.units() {
            if let Some(info) = unit.get::<GameObjectInfo>() {
                let mut row = VarDictionary::new();
                row.set("id", unit.server_id as i64);
                row.set("entry", i64::from(info.entry));
                row.set("rendered", self.game_objects.contains(unit.server_id));
                objects.push(&row);
            }
        }
        state.set("replicated_objects", &objects);
        state.set(
            "object",
            &session
                .object
                .map(|id| (id as i64).to_variant())
                .unwrap_or_default(),
        );
        state.set("busy", session.busy());
        state.set("opening_all", session.opening_all());
        state.set("page", session.page as i64);
        state.set(
            "tab",
            match session.tab {
                MailFrameTab::Inbox => "inbox",
                MailFrameTab::Send => "send",
            },
        );
        state.set("cod_mode", session.cod_mode);
        state.set("attachments", &self.mail_draft_rows());
        let mut texts = VarDictionary::new();
        for (name, text) in &self.mailbox.texts {
            texts.set(*name, text.as_str());
        }
        state.set("texts", &texts);
        state.set(
            "selected",
            &session
                .selected
                .map(|id| (id as i64).to_variant())
                .unwrap_or_default(),
        );
        state.set(
            "money",
            &self
                .world
                .local_player_id()
                .and_then(|id| self.replica.unit(id)?.gold())
                .map(|v| (v as i64).to_variant())
                .unwrap_or_default(),
        );
        if let Some(contents) = &session.contents {
            state.set("now", contents.now as i64);
            state.set("mails", &mail_rows(&contents.mails));
        }

        let mut senders = VarArray::new();
        for sender in &session.pending_senders {
            senders.push(&sender.to_variant());
        }
        state.set("pending_senders", &senders);
        state
    }

    fn drag_mailbox(
        &mut self,
        motion: &Gd<InputEventMouseMotion>,
        rect: [f32; 4],
        scale: f32,
    ) -> bool {
        let Some(drag) = &self.mailbox.drag else {
            return false;
        };
        let Some(viewport) = self.base().get_viewport() else {
            return false;
        };
        let size = viewport.get_visible_rect().size / scale;
        self.mailbox.position = Some(drag.position(
            motion.get_position() / scale,
            [size.x, size.y],
            [rect[2] / scale, rect[3] / scale],
        ));
        true
    }

    fn mail_draft_rows(&self) -> VarArray {
        let mut attached = VarArray::new();
        for guid in &self.mailbox.session.attachments {
            let mut row = VarDictionary::new();
            row.set("guid", *guid as i64);
            let item = bag_item(&self.merchant.session.inventory, *guid);
            row.set("item_id", item.map_or(0, |item| i64::from(item.item_id)));
            row.set("count", item.map_or(0, |item| i64::from(item.count)));
            attached.push(&row.to_variant());
        }
        attached
    }

    pub(super) fn mailbox_pointer(&mut self, event: &Gd<InputEvent>) -> bool {
        if !self.mailbox.session.is_open() || self.game_menu_ui.is_some() {
            return false;
        }
        let Some(ui) = self.mailbox.ui.clone() else {
            return false;
        };
        let Some((rect, _)) = ui.bind().frame_rect(FRAME_NAME) else {
            return false;
        };
        let scale = self.effective_ui_scale();
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            return self.drag_mailbox(&motion, rect, scale);
        }
        let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() else {
            return false;
        };
        if button.get_button_index() != MouseButton::LEFT {
            return false;
        }
        if !button.is_pressed() {
            return self.mailbox.drag.take().is_some();
        }
        let rect = rect.map(|n| n / scale);
        let Some((close, _)) = ui.bind().frame_rect("MailFrameCloseButton") else {
            return false;
        };
        if !title_hit(
            rect,
            button.get_position(),
            scale,
            &[close.map(|n| n / scale)],
        ) {
            return false;
        }
        // A window raised over the title owns the press instead of the mailbox.
        match self.ui_owns_point(&ui, button.get_position()) {
            Ok(true) => {}
            Ok(false) => return false,
            Err(error) => {
                godot_error!("Mailbox title hit-test: {error}");
                return false;
            }
        }
        self.mailbox.drag = Some(WindowDrag::begin(
            button.get_position() / scale,
            [rect[0], rect[1]],
        ));
        true
    }
}

fn mail_rows(mails: &[shared::protocol::MailHeader]) -> VarArray {
    let mut rows = VarArray::new();
    for mail in mails {
        let mut row = VarDictionary::new();
        row.set("id", mail.mail_id as i64);
        row.set("sender", mail.sender.as_str());
        row.set("subject", mail.subject.as_str());
        row.set("body", mail.body.as_str());
        row.set("money", mail.money as i64);
        row.set("cod", mail.cod as i64);
        row.set("read", mail.read);
        row.set("returned", mail.returned);
        row.set("from_player", mail.from_player);
        row.set("can_delete", mail.can_delete());
        row.set("expires_at", mail.expires_at as i64);
        let mut attachments = VarArray::new();
        for attachment in &mail.attachments {
            let mut item = VarDictionary::new();
            item.set("slot", attachment.slot as i64);
            item.set("item_id", attachment.item.item_id as i64);
            item.set("guid", attachment.item.item_guid as i64);
            item.set("count", attachment.item.count as i64);
            attachments.push(&item.to_variant());
        }
        row.set("attachments", &attachments);
        rows.push(&row.to_variant());
    }
    rows
}

fn editbox_text(ui: &Gd<RegistryUi>, name: &str) -> Option<String> {
    let bound = ui.bind();
    let registry = bound.registry()?;
    let frame = registry.get(registry.get_by_name(name)?)?;
    match frame.widget_data.as_ref()? {
        WidgetData::EditBox(edit) => Some(edit.text.clone()),
        _ => None,
    }
}
