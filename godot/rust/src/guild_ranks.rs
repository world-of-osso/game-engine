//! GuildChannel rank requests/replies and the native guild/communities canvas.
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_session::SessionScreen;
use game_engine_ui_model::bank::InputTexts;
use game_engine_ui_model::guild_rank_frame::{GOLD_BOX, NAME_BOX};
use game_engine_ui_model::guild_ranks::GuildRanksSession;
use godot::prelude::*;
use shared::protocol::{GuildRankRequest, GuildRanksState};

#[derive(Default)]
pub(crate) struct GuildRanks {
    session: GuildRanksSession,
    ui: Option<Gd<RegistryUi>>,
}
impl GuildRanks {
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
    pub(super) fn toggle_guild_ranks(&mut self) -> Result<(), FrameError> {
        if self.guild_ranks.session.visible {
            self.close_guild_ranks();
        } else {
            let request = self.guild_ranks.session.open();
            self.send_guild_rank_request(request)?;
        }
        Ok(())
    }
    fn send_guild_rank_request(&mut self, request: GuildRankRequest) -> Result<(), FrameError> {
        if let Err(error) = self.account.send_guild_rank_request(request) {
            self.guild_ranks.session.awaiting_state = false;
            return Err(error.into());
        }
        Ok(())
    }

    pub(super) fn close_guild_ranks(&mut self) -> bool {
        let visible = self.guild_ranks.session.visible;
        self.guild_ranks.session.visible = false;
        self.guild_ranks.session.settings_open = false;
        self.guild_ranks.session.member_menu = None;
        self.guild_ranks.free_ui();
        visible
    }
    pub(super) fn receive_guild_ranks(&mut self, state: GuildRanksState) -> Result<(), FrameError> {
        if let Some(error) = state.error {
            self.add_world_error(error.message())?;
        }
        self.guild_ranks.session.apply(state);
        Ok(())
    }
    pub(super) fn update_guild_ranks(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.close_guild_ranks();
            self.guild_ranks.session = GuildRanksSession::default();
            return Ok(());
        }
        if !self.guild_ranks.session.visible {
            self.guild_ranks.free_ui();
            return Ok(());
        }
        self.sync_guild_ranks()?;
        self.poll_guild_rank_inputs()?;
        Ok(())
    }
    fn poll_guild_rank_inputs(&mut self) -> Result<(), FrameError> {
        let Some(mut ui) = self.guild_ranks.ui.clone() else {
            return Ok(());
        };
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            let texts = read_inputs(&mut ui, &self.guild_ranks.session);
            if let Some(request) = self.guild_ranks.session.click(&action, &texts) {
                self.send_guild_rank_request(request)?;
            }
            if !self.guild_ranks.session.visible {
                break;
            }
        }
        while let Some((action, right, _)) = ui.bind_mut().pop_alt_click() {
            if right && action.starts_with("guild:member:") {
                self.guild_ranks
                    .session
                    .click(&action, &InputTexts::default());
            }
        }
        Ok(())
    }
    fn sync_guild_ranks(&mut self) -> Result<(), String> {
        let scale = self.effective_ui_scale();
        let state = self.guild_ranks.session.clone();
        if self.guild_ranks.ui.is_none() {
            let mut ui = RegistryUi::new_alloc();
            ui.set_name("GuildRanksUI");
            self.base_mut().add_child(&ui);
            let shown = {
                let mut host = ui.bind_mut();
                host.set_ui_scale(scale)
                    .and_then(|()| host.show_guild_ranks(state.clone()))
            };
            if let Err(error) = shown {
                ui.free();
                return Err(error);
            }
            self.guild_ranks.ui = Some(ui);
        }
        let mut ui = self
            .guild_ranks
            .ui
            .clone()
            .ok_or("Guild ranks UI missing")?;
        ui.bind_mut().set_ui_scale(scale)?;
        ui.bind_mut().set_state(state)
    }
}
fn read_inputs(ui: &mut Gd<RegistryUi>, session: &GuildRanksSession) -> InputTexts {
    let tabs = session.state().map_or(0, |state| state.tab_names.len());
    let names = [NAME_BOX.to_owned(), GOLD_BOX.to_owned()]
        .into_iter()
        .chain((0..tabs).map(|index| format!("GuildTabItems{index}")));
    names
        .map(|name| {
            let text = ui
                .bind_mut()
                .frame_text(GString::from(name.as_str()))
                .to_string();
            (name, text)
        })
        .collect()
}
