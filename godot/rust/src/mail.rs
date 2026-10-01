//! Native receiving mail host. Only actual mailbox use can open the authored frame.
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
    mail::{MailSession, NativeMailView, can_use_mailbox},
    mail_frame_component::{ACTION_CLOSE, FRAME_NAME},
};
use godot::{
    classes::{InputEvent, InputEventMouseButton, InputEventMouseMotion},
    global::MouseButton,
    prelude::*,
};
use shared::protocol::GameObjectInfo;

#[derive(Default)]
pub(crate) struct Mailbox {
    pub session: MailSession,
    pub ui: Option<Gd<RegistryUi>>,
    position: Option<[f32; 2]>,
    drag: Option<WindowDrag>,
}
impl Mailbox {
    fn free_ui(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
        self.drag = None;
    }
    pub fn reset(&mut self) {
        self.session.reset();
        self.free_ui();
        self.position = None;
    }
    pub fn close_for(&mut self, object: u64) {
        self.session.close_for(object);
        if !self.session.is_open() {
            self.free_ui();
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
            MailMessage::Pending(pending) => self.mailbox.session.pending_senders = pending.senders,
        }
        Ok(())
    }
    fn close_mailbox(&mut self) -> Result<(), SessionError> {
        let object = self.mailbox.session.object;
        self.mailbox.session.close();
        self.mailbox.free_ui();
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
    pub(super) fn update_mailbox(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.mailbox.reset();
            return Ok(());
        }
        if !self.mailbox.session.is_open() {
            self.mailbox.free_ui();
            return Ok(());
        }
        if self.game_menu_ui.is_none() && self.account.session.gameplay_input_allowed() {
            self.poll_mailbox_input()?;
        }
        if !self.mailbox.session.is_open() {
            return Ok(());
        }
        let money = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id)?.gold())
            .unwrap_or(0);
        let mut bags = self.merchant.session.bag_state();
        for bag in &mut bags.bags {
            bag.visible = bag.bag_index == 0;
        }
        let view = NativeMailView {
            inbox: self.mailbox.session.view(money),
            bags,
        };
        let scale = self.effective_ui_scale();
        if self.mailbox.ui.is_none() {
            let mut ui = RegistryUi::new_alloc();
            ui.set_name("MailboxUI");
            self.base_mut().add_child(&ui);
            let shown = ui.bind_mut().show_mail(view.clone());
            if let Err(error) = shown {
                ui.free();
                return Err(error.into());
            }
            self.mailbox.ui = Some(ui);
        }
        let mut ui = self.mailbox.ui.clone().ok_or("Mailbox UI missing")?;
        ui.bind_mut().set_ui_scale(scale)?;
        ui.bind_mut().set_state(view)?;
        if let Some(position) = self.mailbox.position {
            ui.bind_mut().set_window_position(FRAME_NAME, position)?;
        }
        Ok(())
    }
    fn poll_mailbox_input(&mut self) -> Result<(), FrameError> {
        let Some(mut ui) = self.mailbox.ui.clone() else {
            return Ok(());
        };
        let error = ui.bind_mut().sync_input();
        if !error.is_empty() {
            return Err(error.to_string().into());
        }
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            if action == ACTION_CLOSE {
                self.close_mailbox()?;
                break;
            }
            if let Some(request) = self.mailbox.session.click(&action) {
                self.account.send_mail_request(request)?;
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
        state.set("page", session.page as i64);
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
        let mut mails = VarArray::new();
        if let Some(contents) = &session.contents {
            for mail in &contents.mails {
                let mut row = VarDictionary::new();
                row.set("id", mail.mail_id as i64);
                row.set("subject", mail.subject.as_str());
                row.set("money", mail.money as i64);
                row.set("cod", mail.cod as i64);
                row.set("read", mail.read);
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
                mails.push(&row.to_variant());
            }
        }
        state.set("mails", &mails);
        let mut senders = VarArray::new();
        for sender in &session.pending_senders {
            senders.push(&sender.to_variant());
        }
        state.set("pending_senders", &senders);
        state
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
            return true;
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
