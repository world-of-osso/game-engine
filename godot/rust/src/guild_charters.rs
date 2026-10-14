//! Native Mainline registrar and owned Guild Charter canvas.
use crate::{GameClient, account::NpcMessage, frame_error::FrameError, ui::RegistryUi};
use game_engine_session::SessionScreen;
use game_engine_ui_model::{
    bank::InputTexts,
    guild_charters::{GuildCharters as View, PURCHASE_NAME, RENAME_NAME},
};
use godot::prelude::*;
use shared::{
    components::Player,
    protocol::{GuildCharterReply, GuildCharterRequest, InteractionKind, NpcRole},
};

#[derive(Default)]
pub(crate) struct GuildCharters {
    view: View,
    pub(crate) ui: Option<Gd<RegistryUi>>,
}
impl GuildCharters {
    pub(crate) fn portrait_unit(&self) -> Option<u64> {
        self.view.registrar.as_ref().map(|r| r.npc)
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
    }
}
impl GameClient {
    pub(super) fn charter_npc_message(&mut self, message: &NpcMessage) -> Result<bool, FrameError> {
        match message {
            NpcMessage::Opened(opened)
                if opened.kind == InteractionKind::Role(NpcRole::Petitioner) =>
            {
                self.auction_interaction_closed(opened.npc);
                self.close_trainer();
                self.merchant.session.close();
                self.account
                    .send_guild_charter(GuildCharterRequest::Registrar { npc: opened.npc })?;
                Ok(true)
            }
            NpcMessage::Closed(npc)
                if self
                    .guild_charters
                    .view
                    .registrar
                    .as_ref()
                    .is_some_and(|r| r.npc == *npc) =>
            {
                self.guild_charters.view.registrar = None;
                self.guild_charters.free_ui();
                Ok(true)
            }
            _ => Ok(false),
        }
    }
    pub(super) fn receive_guild_charter(
        &mut self,
        reply: GuildCharterReply,
    ) -> Result<(), FrameError> {
        match &reply {
            GuildCharterReply::Failed { message } => self.add_world_error(message)?,
            GuildCharterReply::Created { guild_name, .. } => {
                game_engine_ui_model::chat_frame::add_system_line(
                    &mut self.chat.model.log,
                    &format!("Your guild, {guild_name}, has been created."),
                );
                self.account
                    .send_guild_rank_request(shared::protocol::GuildRankRequest::Query)?;
            }
            _ => {}
        }
        self.guild_charters.view.receive(reply);
        Ok(())
    }
    /// The server already switched to the same vendor's tabard role; do not close that new interaction.
    pub(super) fn hide_guild_charter_canvas(&mut self) {
        self.guild_charters.view = View::default();
        self.guild_charters.free_ui();
    }

    pub(super) fn close_guild_charter(&mut self) -> Result<bool, FrameError> {
        let visible = self.guild_charters.view.visible();
        if let Some(registrar) = &self.guild_charters.view.registrar {
            self.account.send_close_interaction(registrar.npc)?;
        }
        self.guild_charters.view = View::default();
        self.guild_charters.free_ui();
        Ok(visible)
    }
    pub(super) fn update_guild_charters(&mut self) -> Result<(), FrameError> {
        if !matches!(
            self.account.session.screen,
            SessionScreen::InWorld | SessionScreen::Loading
        ) {
            self.guild_charters.view = View::default();
            self.guild_charters.free_ui();
            return Ok(());
        }
        if self.account.session.screen != SessionScreen::InWorld {
            return Ok(());
        }
        if !self.guild_charters.view.visible() {
            self.guild_charters.free_ui();
            return Ok(());
        }
        self.sync_guild_charters()?;
        let Some(mut ui) = self.guild_charters.ui.clone() else {
            return Ok(());
        };
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            if action == "charter:close" {
                self.close_guild_charter()?;
                break;
            }
            let texts: InputTexts = [PURCHASE_NAME, RENAME_NAME]
                .into_iter()
                .map(|name| {
                    (
                        name.into(),
                        ui.bind_mut().frame_text(name.into()).to_string(),
                    )
                })
                .collect();
            let target = self
                .targeting_target()
                .and_then(|id| self.replica.unit(id))
                .and_then(|unit| unit.get::<Player>())
                .map(|player| player.name.clone());
            if let Some(request) = self.guild_charters.view.click(&action, &texts, target) {
                if let Err(error) = self.account.send_guild_charter(request) {
                    self.guild_charters.view.waiting = false;
                    return Err(error.into());
                }
            }
        }
        Ok(())
    }
    fn sync_guild_charters(&mut self) -> Result<(), String> {
        let state = self.guild_charters.view.clone();
        let scale = self.effective_ui_scale();
        if let Some(ui) = &mut self.guild_charters.ui {
            ui.bind_mut().set_ui_scale(scale)?;
            return ui.bind_mut().set_state(state);
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("GuildCharterUI");
        self.base_mut().add_child(&ui);
        let result = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_guild_charter(state))
        };
        if let Err(error) = result {
            ui.free();
            return Err(error);
        }
        self.guild_charters.ui = Some(ui);
        Ok(())
    }
}
