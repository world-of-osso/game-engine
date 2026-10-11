//! Native keystone slot/start and completion banner; the tracker consumes the same snapshot.
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_session::SessionScreen;
use game_engine_ui_model::challenge::{ChallengeView, challenge_screen, slot_cursor};
use godot::prelude::*;
use shared::protocol::{ChallengePhase, ChallengeRequest};
#[derive(Default)]
pub(crate) struct ChallengeUi {
    ui: Option<Gd<RegistryUi>>,
    queried: bool,
    dismissed: bool,
    last_snapshot: Option<shared::protocol::ChallengeSnapshot>,
}
impl ChallengeUi {
    fn free(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
    }
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        match &mut self.ui {
            Some(ui) => visit(ui),
            None => Ok(()),
        }
    }
}
impl GameClient {
    pub(super) fn update_challenge(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.challenge_ui.free();
            self.challenge_ui = ChallengeUi::default();
            self.account.challenge = Default::default();
            return Ok(());
        }
        if !self.challenge_ui.queried {
            self.account
                .send_challenge_request(ChallengeRequest::Query)?;
            self.challenge_ui.queried = true;
        }
        if self.challenge_ui.last_snapshot.as_ref() != Some(&self.account.challenge) {
            self.challenge_ui.last_snapshot = Some(self.account.challenge.clone());
            self.challenge_ui.dismissed = false;
        }
        if let Some(mut ui) = self.challenge_ui.ui.clone() {
            loop {
                let action = ui.bind_mut().pop_action().to_string();
                if action.is_empty() {
                    break;
                }
                self.apply_challenge_action(&action)?;
            }
        }
        let run = self.account.challenge.run.as_ref();
        let slotted = run.is_none_or(|run| run.phase == ChallengePhase::Slotted);
        let completed = run.is_some_and(|run| run.phase == ChallengePhase::Completed);
        let visible = (self.account.challenge.font.is_some() && slotted) || completed;
        if !visible || self.challenge_ui.dismissed {
            self.challenge_ui.free();
            return Ok(());
        }
        let scale = self.effective_ui_scale();
        let size = self
            .base()
            .get_viewport()
            .ok_or("Challenge viewport missing")?
            .get_visible_rect()
            .size;
        let view = ChallengeView {
            viewport: [size.x / scale, size.y / scale],
            snapshot: self.account.challenge.clone(),
        };
        self.extract_art(&crate::quests::screen_texture_fdids(
            view.clone(),
            challenge_screen,
        ));
        if self.challenge_ui.ui.is_none() {
            let mut ui = RegistryUi::new_alloc();
            ui.set_name("ChallengeUI");
            ui.set_layer(10);
            self.base_mut().add_child(&ui);
            let shown = {
                let mut host = ui.bind_mut();
                host.set_ui_scale(scale)
                    .and_then(|()| host.show_challenge(view.clone()))
            };
            if let Err(error) = shown {
                ui.free();
                return Err(error.into());
            }
            self.challenge_ui.ui = Some(ui);
        }
        let mut ui = self.challenge_ui.ui.clone().ok_or("Challenge UI missing")?;
        ui.bind_mut().set_state(view)?;
        Ok(())
    }
    fn apply_challenge_action(&mut self, action: &str) -> Result<(), FrameError> {
        match action {
            "challenge_close" => {
                self.challenge_ui.dismissed = true;
                self.challenge_ui.free();
            }
            "challenge_start" => self
                .account
                .send_challenge_request(ChallengeRequest::Start)?,
            "challenge_slot" => {
                let game_engine_ui_model::cursor_item::CursorItem::Inventory { from, .. } =
                    self.bags.cursor.item
                else {
                    return Ok(());
                };
                let Some(item) = self.merchant.session.inventory.item_at(from) else {
                    return Ok(());
                };
                if let Some(request) = slot_cursor(&self.account.challenge, item.item_guid) {
                    self.account.send_challenge_request(request)?;
                    self.bags.cursor.item = game_engine_ui_model::cursor_item::CursorItem::Empty;
                }
            }
            _ => return Err(format!("Unknown challenge action: {action}").into()),
        }
        Ok(())
    }
}
#[godot_api(secondary)]
impl GameClient {
    /// Native live fixture selects the ordinary Mythic preference before entering the dungeon.
    #[func]
    fn choose_mythic_difficulty(&self) -> GString {
        match self.account.send_set_dungeon_difficulty(23) {
            Ok(()) => "".into(),
            Err(error) => GString::from(&error.to_string()),
        }
    }
    #[func]
    fn challenge_state(&self) -> GString {
        GString::from(
            &serde_json::to_string(&self.account.challenge).expect("serialize challenge state"),
        )
    }
    /// Native automation uses the production authenticated request, not local UI injection.
    #[func]
    fn challenge_request(&self, json: GString) -> GString {
        let request = match serde_json::from_str::<ChallengeRequest>(&json.to_string()) {
            Ok(request) => request,
            Err(error) => return GString::from(&error.to_string()),
        };
        match self.account.send_challenge_request(request) {
            Ok(()) => "".into(),
            Err(error) => GString::from(&error.to_string()),
        }
    }
}
