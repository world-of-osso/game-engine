//! GuildChannel rank requests/replies and the native guild/communities canvas.
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_session::SessionScreen;
use game_engine_ui_model::bank::InputTexts;
use game_engine_ui_model::guild_rank_frame::{GOLD_BOX, INVITE_BOX, PROFILE_BOX};
use game_engine_ui_model::guild_ranks::{GuildCommand, GuildRanksSession};
use godot::prelude::*;
use shared::protocol::{GuildRankRequest, GuildRanksState};

#[derive(Default)]
pub(crate) struct GuildRanks {
    session: GuildRanksSession,
    ui: Option<Gd<RegistryUi>>,
    invite_targets: std::collections::VecDeque<String>,
    query_info: bool,
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
    fn synchronize_session_screen(&mut self, screen: SessionScreen) {
        if !matches!(screen, SessionScreen::InWorld | SessionScreen::Loading) {
            self.session = GuildRanksSession::default();
            self.invite_targets.clear();
            self.query_info = false;
        }
    }
    fn free_ui(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
    }
}
impl GameClient {
    pub(crate) fn send_guild_command(&mut self, command: GuildCommand) -> Result<(), FrameError> {
        self.account.send_guild_command(command.clone())?;
        match command {
            GuildCommand::Invite(name) => self.guild_ranks.invite_targets.push_back(name),
            GuildCommand::DeclineInvite => {
                self.guild_ranks.session.invitation = None;
                self.guild_ranks.session.invitation_waiting = false;
            }
            GuildCommand::Query => self.guild_ranks.query_info = true,
            _ => {}
        }
        Ok(())
    }

    pub(super) fn receive_guild_invitation(
        &mut self,
        invitation: shared::protocol::GuildInvitation,
    ) {
        self.guild_ranks.session.invitation = Some(invitation);
        self.guild_ranks.session.invitation_waiting = false;
    }
    pub(super) fn receive_guild_invite_result(
        &mut self,
        result: shared::protocol::GuildInviteResponse,
    ) -> Result<(), FrameError> {
        let target = self.guild_ranks.invite_targets.pop_front();
        if let Some(error) = result.error {
            self.guild_notice(&error, true)?;
        } else if result.success
            && let Some(target) = target
        {
            self.guild_notice(
                &format!("You have invited {target} to join your guild."),
                false,
            )?;
        }
        Ok(())
    }
    pub(super) fn receive_guild_accepted(
        &mut self,
        result: shared::protocol::GuildAcceptInviteResponse,
    ) -> Result<(), FrameError> {
        self.guild_ranks.session.invitation_waiting = false;
        if let Some(error) = result.error {
            self.guild_notice(&error, true)?;
        } else if result.success {
            self.guild_ranks.session.invitation = None;
            self.account
                .send_guild_rank_request(GuildRankRequest::Query)?;
        }
        Ok(())
    }
    pub(super) fn receive_guild_profile(
        &mut self,
        update: shared::protocol::GuildStateUpdate,
    ) -> Result<(), FrameError> {
        self.guild_ranks.session.profile_waiting = false;
        if let Some(error) = update.error {
            self.guild_ranks.query_info = false;
            self.guild_notice(&error, true)?;
            return Ok(());
        }
        if let Some(profile) = &update.guild {
            let motd_changed = self
                .guild_ranks
                .session
                .profile
                .as_ref()
                .is_none_or(|old| old.motd != profile.motd);
            if motd_changed && !profile.motd.is_empty() {
                self.guild_notice(
                    &format!("Guild Message of the Day: \"{}\"", profile.motd),
                    false,
                )?;
            }
            if self.guild_ranks.query_info {
                let notice =
                    game_engine_ui_model::guild_ranks::guild_creation_notice(profile.created_at);
                self.guild_notice(&notice, false)?;
                self.guild_ranks.query_info = false;
            }
        }
        self.guild_ranks.session.profile = update.guild;
        Ok(())
    }
    pub(super) fn receive_guild_event_log(
        &mut self,
        log: shared::protocol::GuildEventLog,
    ) -> Result<(), FrameError> {
        if let Some(error) = log.error {
            self.guild_notice(error.message(), true)?;
        }
        self.guild_ranks.session.apply_event_log(log);
        Ok(())
    }

    pub(super) fn receive_guild_news(
        &mut self,
        news: shared::protocol::GuildNewsState,
    ) -> Result<(), FrameError> {
        if self
            .guild_ranks
            .session
            .state()
            .is_none_or(|state| state.ranks.is_empty())
        {
            return Ok(());
        }
        if let Some(error) = news.error {
            let message = match error {
                shared::protocol::GuildNewsError::NotInGuild => "You are not in a guild.",
                shared::protocol::GuildNewsError::Permissions => {
                    "You don't have permission to do that."
                }
                shared::protocol::GuildNewsError::EntryNotFound => {
                    "That guild news entry is no longer available."
                }
            };
            self.guild_notice(message, true)?;
        }
        self.guild_ranks.session.news.apply(news);
        Ok(())
    }

    fn guild_notice(&mut self, text: &str, error: bool) -> Result<(), FrameError> {
        game_engine_ui_model::chat_frame::add_system_line(&mut self.chat.model.log, text);
        if error {
            self.add_world_error(text)?;
        }
        Ok(())
    }

    pub(super) fn guild_ranks_pointer(&mut self, event: &Gd<godot::classes::InputEvent>) -> bool {
        if !guild_pointer_enabled(&self.guild_ranks.session, self.game_menu_ui.is_some()) {
            return false;
        }
        if let Ok(key) = event.clone().try_cast::<godot::classes::InputEventKey>()
            && key.is_pressed()
            && !key.is_echo()
            && key.get_keycode() == godot::global::Key::ESCAPE
            && self.guild_ranks.session.dismiss_subdialog()
        {
            self.mark_viewport_input_handled();
            return true;
        }
        let Some(ui) = self.guild_ranks.ui.as_mut() else {
            return false;
        };
        let taken = ui.bind_mut().scroll_list_input(event);
        match taken {
            Ok(true) => {
                self.mark_viewport_input_handled();
                true
            }
            Ok(false) => false,
            Err(error) => {
                crate::frame_error::report_once(&format!("Guild Control scroll: {error}"));
                false
            }
        }
    }

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
        let visible = self.guild_ranks.session.close();
        self.guild_ranks.free_ui();
        visible
    }
    pub(super) fn receive_guild_ranks(&mut self, state: GuildRanksState) -> Result<(), FrameError> {
        if let Some(error) = state.error {
            self.add_world_error(error.message())?;
        }
        let guilded = !state.ranks.is_empty();
        self.guild_ranks.session.apply(state);
        if guilded {
            self.account.send_guild_command(GuildCommand::Query)?;
        } else {
            self.guild_ranks.session.profile = None;
        }
        Ok(())
    }
    pub(super) fn update_guild_ranks(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.close_guild_ranks();
            self.guild_ranks
                .synchronize_session_screen(self.account.session.screen);
            return Ok(());
        }
        if !self.guild_ranks.session.visible && self.guild_ranks.session.invitation.is_none() {
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
            let previous_editor = self.guild_ranks.session.note_editor.clone();
            let texts = read_inputs(&mut ui, &self.guild_ranks.session);
            if let Some(request) = self.guild_ranks.session.click(&action, &texts) {
                self.send_guild_rank_request(request)?;
            }
            if previous_editor != self.guild_ranks.session.note_editor
                && self.guild_ranks.session.note_editor.is_some()
            {
                self.sync_guild_ranks()?;
                ui.bind_mut()
                    .focus_frame_named(game_engine_ui_model::guild_member_detail::NOTE_BOX)?;
            }
            if !self.guild_ranks.session.visible {
                break;
            }
        }
        let commands = std::mem::take(&mut self.guild_ranks.session.commands);
        for command in commands {
            if let Err(error) = self.send_guild_command(command) {
                self.guild_ranks.session.invitation_waiting = false;
                return Err(error);
            }
        }
        while let Some((action, right, _)) = ui.bind_mut().pop_alt_click() {
            if right && action.starts_with("guild:news_entry:") {
                self.guild_ranks.session.click(
                    &action.replacen("guild:news_entry:", "guild:news_menu:", 1),
                    &InputTexts::default(),
                );
            }
            if right && action.starts_with("guild:member:") {
                self.guild_ranks.session.click(
                    &action.replacen("guild:member:", "guild:member-menu:", 1),
                    &InputTexts::default(),
                );
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
    guild_input_names(session)
        .map(|name| {
            let text = ui
                .bind_mut()
                .frame_text(GString::from(name.as_str()))
                .to_string();
            (name, text)
        })
        .collect()
}

pub(crate) fn guild_pointer_enabled(session: &GuildRanksSession, game_menu_open: bool) -> bool {
    !game_menu_open && session.visible
}

pub(crate) fn read_inputs_for_registry(
    registry: &ui_toolkit::registry::FrameRegistry,
    session: &GuildRanksSession,
) -> InputTexts {
    use ui_toolkit::frame::WidgetData;
    guild_input_names(session)
        .map(|name| {
            let text = registry
                .get_by_name(&name)
                .and_then(|id| registry.get(id))
                .and_then(|frame| frame.widget_data.as_ref())
                .map_or_else(String::new, |data| match data {
                    WidgetData::EditBox(edit) => edit.text.clone(),
                    WidgetData::FontString(text) => text.text.clone(),
                    _ => String::new(),
                });
            (name, text)
        })
        .collect()
}

fn guild_input_names(session: &GuildRanksSession) -> impl Iterator<Item = String> {
    let tabs = session.state().map_or(0, |state| state.tab_names.len());
    let names = [
        GOLD_BOX.to_owned(),
        INVITE_BOX.to_owned(),
        PROFILE_BOX.to_owned(),
        game_engine_ui_model::guild_member_detail::NOTE_BOX.to_owned(),
    ]
    .into_iter()
    .chain((0..tabs).map(|index| format!("GuildTabItems{index}")))
    .chain(
        (0..session.state().map_or(0, |state| state.ranks.len()))
            .map(|index| format!("GuildRankOrderName{index}")),
    );
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guild_roster_pointer_is_enabled_while_settings_are_closed() {
        let mut session = GuildRanksSession::default();
        session.open();
        assert!(!session.settings_open);
        assert!(guild_pointer_enabled(&session, false));
    }

    #[test]
    fn guild_pointer_respects_closed_communities_and_game_menu() {
        let mut session = GuildRanksSession::default();
        assert!(!guild_pointer_enabled(&session, false));
        session.open();
        session.settings_open = true;
        assert!(guild_pointer_enabled(&session, false));
        assert!(!guild_pointer_enabled(&session, true));
        session.visible = false;
        assert!(!guild_pointer_enabled(&session, false));
    }

    #[test]
    fn guild_pending_invitation_survives_loading_but_not_logout() {
        let invitation = shared::protocol::GuildInvitation {
            guild_id: 7,
            guild_name: "Raid Team".into(),
            inviter_name: "Alice".into(),
        };
        let mut guild = GuildRanks::default();
        guild.session.invitation = Some(invitation.clone());
        // Server delivers Added<InWorld> while the native client still loads terrain.
        guild.synchronize_session_screen(SessionScreen::Loading);
        assert_eq!(guild.session.invitation, Some(invitation.clone()));
        guild.synchronize_session_screen(SessionScreen::InWorld);
        assert_eq!(guild.session.invitation, Some(invitation));
        guild.synchronize_session_screen(SessionScreen::Login);
        assert!(guild.session.invitation.is_none());
    }
}
