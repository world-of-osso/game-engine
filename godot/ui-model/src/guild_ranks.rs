//! Authoritative guild settings decisions; requests never optimistically edit state.
use shared::protocol::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CommunitiesTab {
    Chat,
    #[default]
    Roster,
    Benefits,
    Info,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuildRosterSort {
    Name,
    Rank,
    Level,
    Class,
    Zone,
    Note,
}

pub fn guild_creation_notice(created_at: Option<u64>) -> String {
    let Some(seconds) = created_at else {
        return "Guild creation date is unavailable.".into();
    };
    let seconds = i64::try_from(seconds).expect("guild creation time fits Unix seconds");
    let date = chrono::DateTime::from_timestamp(seconds, 0).expect("valid guild creation date");
    format!("Guild created on {} (UTC).", date.format("%B %-d, %Y"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GuildCommand {
    Invite(String),
    GroupInvite(String),
    AcceptInvite,
    DeclineInvite,
    SetMotd(String),
    SetInfo(String),
    Query,
    QueryEventLog,
    News(GuildNewsRequest),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuildProfileField {
    Motd,
    Info,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GuildRanksSession {
    pub visible: bool,
    pub awaiting_state: bool,
    pub settings_open: bool,
    pub control_page: usize,
    pub control_menu_open: bool,
    pub rank_menu_open: bool,
    pub member_menu: Option<String>,
    pub member_detail: Option<String>,
    pub member_rank_menu_open: bool,
    pub removal_target: Option<String>,
    /// CONFIRM_GUILD_PROMOTE data, independent of later roster selection.
    pub leadership_target: Option<String>,
    pub disband_open: bool,
    disband_query_pending: bool,
    /// StaticPopup data captures the target identity independently of roster selection.
    pub note_editor: Option<(String, bool)>,
    pub error: Option<String>,
    pub invitation: Option<GuildInvitation>,
    pub invitation_waiting: bool,
    invitation_elapsed: f64,
    pub profile: Option<GuildSnapshot>,
    pub commands: Vec<GuildCommand>,
    /// Chat edit-box prefill requested by roster Whisper; the host opens chat once.
    pub chat_open: Option<String>,
    pub communities_tab: CommunitiesTab,
    pub roster_sort: Option<GuildRosterSort>,
    pub roster_sort_descending: bool,
    /// Presentation-only inverse of persisted communitiesShowOffline.
    pub roster_online_only: bool,
    pub invite_open: bool,
    pub profile_editor: Option<GuildProfileField>,
    pub profile_waiting: bool,
    pub log_open: bool,
    pub log_waiting: bool,
    pub event_log: Option<GuildEventLog>,
    pub news: crate::guild_news_state::GuildNewsSession,
    state: Option<GuildRanksState>,
    selected_rank: usize,
}

impl GuildRanksSession {
    pub fn receive_invitation(&mut self, invitation: GuildInvitation) {
        self.invitation = Some(invitation);
        self.invitation_waiting = false;
        self.invitation_elapsed = 0.0;
    }

    /// GuildInviteFrame.xml OnUpdate hides strictly after 60 displayed seconds.
    pub fn advance_invitation(&mut self, delta: f64) -> Option<GuildCommand> {
        if self.invitation.is_none() || self.invitation_waiting {
            return None;
        }
        self.invitation_elapsed += delta;
        const INVITATION_TIMEOUT_SECONDS: f64 = 60.0;
        if self.invitation_elapsed > INVITATION_TIMEOUT_SECONDS {
            return self.dismiss_invitation();
        }
        None
    }

    /// GuildInviteFrame OnHide declines unless Join has already sent acceptance.
    pub fn dismiss_invitation(&mut self) -> Option<GuildCommand> {
        if self.invitation_waiting {
            return None;
        }
        self.invitation.take()?;
        Some(GuildCommand::DeclineInvite)
    }

    pub fn open(&mut self) -> GuildRankRequest {
        self.visible = true;
        self.awaiting_state = true;
        GuildRankRequest::Query
    }

    pub fn close(&mut self) -> bool {
        let visible = self.visible;
        self.visible = false;
        self.settings_open = false;
        self.invite_open = false;
        self.profile_editor = None;
        self.member_menu = None;
        self.member_detail = None;
        self.member_rank_menu_open = false;
        self.note_editor = None;
        self.removal_target = None;
        self.leadership_target = None;
        self.disband_open = false;
        self.disband_query_pending = false;
        self.log_open = false;
        self.news.filters_open = false;
        self.news.menu = None;
        visible
    }

    pub fn dismiss_subdialog(&mut self) -> bool {
        if std::mem::take(&mut self.disband_open) {
            return true;
        }
        if self.leadership_target.take().is_some() {
            return true;
        }
        if self.member_rank_menu_open {
            self.member_rank_menu_open = false;
            return true;
        }
        if self.removal_target.take().is_some() {
            return true;
        }
        if self.note_editor.take().is_some() {
            return true;
        }
        if self.news.menu.take().is_some() {
            return true;
        }
        if self.news.filters_open {
            self.news.filters_open = false;
            return true;
        }
        let log_open = self.log_open;
        self.log_open = false;
        log_open
    }

    pub fn apply_event_log(&mut self, log: GuildEventLog) {
        self.log_waiting = false;
        if log.error.is_some() {
            self.log_open = false;
        }
        self.event_log = Some(log);
    }

    pub fn apply(&mut self, state: GuildRanksState) {
        self.awaiting_state = false;
        if state.ranks.is_empty() {
            self.log_open = false;
            self.log_waiting = false;
            self.event_log = None;
            self.news = Default::default();
        }
        if state.error.is_some() {
            self.profile_waiting = false;
        }
        self.error = state.error.map(|error| error.message().to_owned());
        self.selected_rank = self.selected_rank.min(state.ranks.len().saturating_sub(1));
        self.state = Some(state);
        if !self.is_guild_master() {
            self.disband_open = false;
        }
        if std::mem::take(&mut self.disband_query_pending) {
            self.open_disband_confirmation();
        }
    }

    pub fn state(&self) -> Option<&GuildRanksState> {
        self.state.as_ref()
    }

    pub fn can(&self, right: u32) -> bool {
        self.state.as_ref().is_some_and(|state| {
            state
                .ranks
                .get(usize::from(state.own_rank))
                .is_some_and(|rank| state.own_rank == 0 || rank.rights & right == right)
        })
    }
    pub fn apply_profile(&mut self, update: GuildStateUpdate) {
        self.profile_waiting = false;
        if update.error.is_none() {
            let previous = self.profile.as_ref().map(|guild| guild.guild_id);
            let current = update.guild.as_ref().map(|guild| guild.guild_id);
            if previous.is_some() && previous != current {
                self.disband_open = false;
            }
            self.profile = update.guild;
        }
    }

    pub fn is_guild_master(&self) -> bool {
        self.state
            .as_ref()
            .is_some_and(|state| state.own_rank == 0 && !state.ranks.is_empty())
    }

    fn open_disband_confirmation(&mut self) {
        if self.is_guild_master() && !self.awaiting_state {
            self.disband_open = true;
        }
    }
    fn click_profile(&mut self, action: &str, texts: &crate::bank::InputTexts) {
        match action {
            "guild:info_open" => {
                self.communities_tab = CommunitiesTab::Info;
                if let Some(request) = self.news.query() {
                    self.commands.push(GuildCommand::News(request));
                }
            }
            "guild:news_filters" => self.news.filters_open = true,
            "guild:news_filters_close" => self.news.filters_open = false,
            "guild:news_sticky" => {
                let can_edit = self.can(GUILD_RIGHT_EDIT_MOTD);
                if let Some(request) = self.news.sticky(can_edit) {
                    self.commands.push(GuildCommand::News(request));
                }
            }
            _ if action.starts_with("guild:news_filter:") => {
                if let Ok(id) = action
                    .trim_start_matches("guild:news_filter:")
                    .parse::<usize>()
                {
                    if let Some(filter) = self.news.filters.get_mut(id) {
                        *filter = !*filter;
                    }
                }
            }
            _ if action.starts_with("guild:news_menu:") => {
                self.news.menu = action
                    .trim_start_matches("guild:news_menu:")
                    .parse::<u64>()
                    .ok()
                    .filter(|&id| self.news.entry(id).is_some());
            }
            "guild:log_open" => {
                self.log_open = true;
                if !self.log_waiting {
                    self.log_waiting = true;
                    self.commands.push(GuildCommand::QueryEventLog);
                }
            }
            "guild:log_close" => self.log_open = false,
            "guild:roster_open" => self.communities_tab = CommunitiesTab::Roster,
            "guild:chat_open" => self.communities_tab = CommunitiesTab::Chat,
            "guild:benefits_open" => self.communities_tab = CommunitiesTab::Benefits,
            "guild:roster-sort:name" => self.sort_roster(GuildRosterSort::Name),
            "guild:roster-sort:rank" => self.sort_roster(GuildRosterSort::Rank),
            "guild:roster-sort:level" => self.sort_roster(GuildRosterSort::Level),
            "guild:roster-sort:class" => self.sort_roster(GuildRosterSort::Class),
            "guild:roster-sort:zone" => self.sort_roster(GuildRosterSort::Zone),
            "guild:roster-sort:note" => self.sort_roster(GuildRosterSort::Note),
            "guild:invite_open" if self.can(GUILD_RIGHT_INVITE) => self.invite_open = true,
            "guild:invite_close" => self.invite_open = false,
            "guild:invite_send" if self.invite_open && self.can(GUILD_RIGHT_INVITE) => {
                if let Some(name) = texts
                    .get(crate::guild_rank_frame::INVITE_BOX)
                    .filter(|text| !text.trim().is_empty())
                {
                    self.commands.push(GuildCommand::Invite(name.trim().into()));
                    self.invite_open = false;
                }
            }
            "guild:edit_motd" if self.can(GUILD_RIGHT_EDIT_MOTD) && !self.profile_waiting => {
                self.profile_editor = Some(GuildProfileField::Motd)
            }
            "guild:edit_info" if self.can(GUILD_RIGHT_EDIT_INFO) && !self.profile_waiting => {
                self.profile_editor = Some(GuildProfileField::Info)
            }
            "guild:profile_close" => self.profile_editor = None,
            "guild:profile_accept" => self.queue_profile_edit(texts),
            _ => {}
        }
    }
    fn queue_profile_edit(&mut self, texts: &crate::bank::InputTexts) {
        let Some(field) = self.profile_editor else {
            return;
        };
        let (right, limit) = match field {
            GuildProfileField::Motd => (GUILD_RIGHT_EDIT_MOTD, 255),
            GuildProfileField::Info => (GUILD_RIGHT_EDIT_INFO, 499),
        };
        if self.profile_waiting || !self.can(right) {
            return;
        }
        let Some(text) = texts.get(crate::guild_rank_frame::PROFILE_BOX) else {
            return;
        };
        if text.chars().count() > limit {
            return;
        }
        let command = match field {
            GuildProfileField::Motd => GuildCommand::SetMotd(text.clone()),
            GuildProfileField::Info => GuildCommand::SetInfo(text.clone()),
        };
        self.commands.push(command);
        self.profile_waiting = true;
        self.profile_editor = None;
    }

    pub fn select_rank(&mut self, rank: usize) {
        if self
            .state
            .as_ref()
            .is_some_and(|state| rank < state.ranks.len())
        {
            self.selected_rank = rank;
        }
    }

    fn sort_roster(&mut self, column: GuildRosterSort) {
        self.roster_sort_descending = if self.roster_sort == Some(column) {
            !self.roster_sort_descending
        } else {
            // Mainline's first string-attribute click swaps comparator arguments.
            column == GuildRosterSort::Note
        };
        self.roster_sort = Some(column);
        self.member_menu = None;
    }

    pub fn saved_input_text(&self, name: &str) -> Option<String> {
        if let Some(index) = name
            .strip_prefix("GuildRankOrderName")
            .and_then(|value| value.parse::<usize>().ok())
        {
            return Some(self.state()?.ranks.get(index)?.name.clone());
        }
        if let Some(index) = name
            .strip_prefix("GuildTabItems")
            .and_then(|value| value.parse::<usize>().ok())
        {
            return Some(
                self.selected()?
                    .tabs
                    .get(index)?
                    .withdrawals_per_day
                    .to_string(),
            );
        }
        (name == crate::guild_rank_frame::GOLD_BOX)
            .then(|| {
                self.selected()
                    .map(|rank| (rank.gold_per_day / 10_000).to_string())
            })
            .flatten()
    }

    pub fn selected_index(&self) -> usize {
        self.selected_rank
    }

    /// Native buttons read current editbox values, then produce one wire request.
    pub fn click(
        &mut self,
        action: &str,
        texts: &crate::bank::InputTexts,
    ) -> Option<GuildRankRequest> {
        self.click_profile(action, texts);
        let input = |name: &str| texts.get(name).map(String::as_str).unwrap_or_default();
        let request = match action {
            "guild:accept_invite" | "guild:decline_invite" => {
                if self.invitation.is_some() && !self.invitation_waiting {
                    self.invitation_waiting = true;
                    self.commands.push(if action == "guild:accept_invite" {
                        GuildCommand::AcceptInvite
                    } else {
                        GuildCommand::DeclineInvite
                    });
                }
                None
            }
            "guild:show_offline" if self.communities_tab == CommunitiesTab::Roster => {
                self.roster_online_only = !self.roster_online_only;
                None
            }
            "guild:disband_request" => {
                if self.awaiting_state || self.disband_query_pending {
                    return None;
                }
                self.disband_query_pending = true;
                Some(GuildRankRequest::Query)
            }
            "guild:disband" => {
                self.open_disband_confirmation();
                None
            }
            "guild:disband_cancel" => {
                self.disband_open = false;
                if std::mem::take(&mut self.disband_query_pending) {
                    self.awaiting_state = false;
                }
                None
            }
            "guild:disband_confirm" => {
                if !self.disband_open || self.awaiting_state || !self.is_guild_master() {
                    return None;
                }
                self.disband_open = false;
                Some(GuildRankRequest::Disband)
            }
            "guild:close" => {
                self.close();
                None
            }
            "guild:settings" => {
                self.settings_open = true;
                None
            }
            "guild:settings_close" => {
                self.settings_open = false;
                None
            }
            "guild:add" => self.add_rank("New Rank"),
            "guild:control-menu" => {
                self.control_menu_open = !self.control_menu_open;
                self.rank_menu_open = false;
                None
            }
            "guild:rank-menu" => {
                self.rank_menu_open = !self.rank_menu_open;
                self.control_menu_open = false;
                None
            }
            "guild:detail_group_invite" => {
                if let Some(member) = self.detail_member().filter(|member| member.is_online) {
                    self.commands
                        .push(GuildCommand::GroupInvite(member.character_name.clone()));
                }
                None
            }
            "guild:detail_remove" => {
                self.open_member_removal(self.member_detail.clone()?);
                None
            }
            "guild:remove_confirm" => {
                let request = self.remove_member(self.removal_target.as_deref()?)?;
                self.removal_target = None;
                self.member_detail = None;
                Some(request)
            }
            "guild:remove_cancel" => {
                self.removal_target = None;
                None
            }
            "guild:detail_rank_menu" => {
                let eligible = self
                    .member_detail
                    .as_deref()
                    .is_some_and(|name| self.member_rank_range(name).is_some());
                let waiting = self.awaiting_state || self.profile_waiting;
                if eligible && !waiting {
                    self.member_rank_menu_open = !self.member_rank_menu_open;
                    self.member_menu = None;
                }
                None
            }
            "guild:detail_close" => {
                self.member_detail = None;
                self.member_rank_menu_open = false;
                self.note_editor = None;
                None
            }
            "guild:note_public" | "guild:note_officer" => {
                let officer = action == "guild:note_officer";
                if self.can_edit_note(officer) && !self.awaiting_state && !self.profile_waiting {
                    self.removal_target = None;
                    self.leadership_target = None;
                    self.note_editor = self.member_detail.clone().map(|name| (name, officer));
                }
                None
            }
            "guild:note_close" => {
                self.note_editor = None;
                None
            }
            "guild:note_accept" => self.submit_note(input(crate::guild_member_detail::NOTE_BOX)),
            "guild:gold" => self.set_gold_limit(input(crate::guild_rank_frame::GOLD_BOX)),
            "guild:transfer_leadership" => {
                self.open_leadership_confirmation(self.member_menu.clone()?);
                None
            }
            "guild:transfer_leadership_confirm" => {
                if self.awaiting_state {
                    return None;
                }
                self.leadership_target
                    .take()
                    .map(|character_name| GuildRankRequest::TransferLeadership { character_name })
            }
            "guild:transfer_leadership_cancel" => {
                self.leadership_target = None;
                None
            }
            "guild:whisper" => {
                let name = self.member_menu.clone()?;
                if self.can_whisper(&name) {
                    // CHAT_WHISPER_HEADER_ABSENT: Retail /cw + ParseText header mode is not represented.
                    self.chat_open = Some(format!("/w {name} "));
                    self.member_menu = None;
                }
                None
            }
            "guild:promote" => self.promote(self.member_menu.as_deref()?),
            "guild:demote" => self.demote(self.member_menu.as_deref()?),
            "guild:remove_member" => {
                self.open_member_removal(self.member_menu.clone()?);
                None
            }
            _ => self.indexed_click(action, texts),
        };
        if request.is_some() {
            self.awaiting_state = true;
        }
        request
    }

    fn open_leadership_confirmation(&mut self, name: String) {
        if self.can_transfer_leadership(&name) {
            self.leadership_target = Some(name);
            self.member_menu = None;
            self.member_rank_menu_open = false;
            self.note_editor = None;
            self.removal_target = None;
        }
    }

    fn open_member_removal(&mut self, name: String) {
        if self.remove_member(&name).is_some() {
            self.removal_target = Some(name);
            self.member_menu = None;
            self.note_editor = None;
            self.leadership_target = None;
        }
    }

    pub fn detail_member(&self) -> Option<&GuildMemberSnapshot> {
        let name = self.member_detail.as_ref()?;
        self.profile
            .as_ref()?
            .members
            .iter()
            .find(|member| &member.character_name == name)
    }

    pub fn can_edit_note(&self, officer: bool) -> bool {
        let Some(member) = self.detail_member() else {
            return false;
        };
        self.can_edit_member_note(member, officer)
    }

    fn can_edit_member_note(&self, member: &GuildMemberSnapshot, officer: bool) -> bool {
        if officer {
            self.can(GUILD_RIGHT_EDIT_OFFICER_NOTE) && self.can(GUILD_RIGHT_VIEW_OFFICER_NOTE)
        } else {
            member.is_self || self.can(GUILD_RIGHT_EDIT_PUBLIC_NOTE)
        }
    }

    pub fn note_member(&self) -> Option<&GuildMemberSnapshot> {
        let (name, _) = self.note_editor.as_ref()?;
        self.profile
            .as_ref()?
            .members
            .iter()
            .find(|member| &member.character_name == name)
    }

    fn submit_note(&mut self, note: &str) -> Option<GuildRankRequest> {
        let (character_name, officer) = self.note_editor.clone()?;
        if self.awaiting_state
            || self.profile_waiting
            || !self.can_edit_member_note(self.note_member()?, officer)
            || !valid_guild_member_note(note)
        {
            return None;
        }
        self.note_editor = None;
        self.profile_waiting = true;
        Some(GuildRankRequest::SetMemberNote {
            character_name,
            note: note.to_owned(),
            officer,
        })
    }

    fn indexed_click(
        &mut self,
        action: &str,
        texts: &crate::bank::InputTexts,
    ) -> Option<GuildRankRequest> {
        if let Some(value) = action.strip_prefix("guild:detail_rank:") {
            let (name, rank) = value.rsplit_once(':')?;
            let rank = rank.parse().ok()?;
            let request = self.build_member_rank_assignment(name, rank);
            self.member_rank_menu_open = false;
            return request;
        }
        if let Some(name) = action.strip_prefix("guild:member:") {
            self.member_rank_menu_open = false;
            self.member_detail = Some(name.to_owned());
            self.member_menu = None;
            return None;
        }
        if let Some(name) = action.strip_prefix("guild:member-menu:") {
            self.member_rank_menu_open = false;
            self.member_menu = Some(name.to_owned());
            self.member_detail = None;
            return None;
        }
        let (kind, index) = action.strip_prefix("guild:")?.split_once(':')?;
        let index: usize = index.parse().ok()?;
        match kind {
            "rank" => {
                self.select_rank(index);
                self.rank_menu_open = false;
                None
            }
            "control-page" if index < crate::guild_control::PAGES.len() => {
                self.control_page = index;
                self.control_menu_open = false;
                self.rank_menu_open = false;
                if index != 0 && self.selected_rank == 0 {
                    self.select_rank(1);
                }
                None
            }
            "retail-permission" => {
                let mask = crate::guild_control::permission_right(index)?;
                self.set_permission_mask(mask, self.selected()?.rights & mask == 0)
            }
            "rename" | "remove" | "up" | "down" => {
                self.state()?.ranks.get(index)?;
                self.select_rank(index);
                match kind {
                    "rename" => self.rename_rank(texts.get(&format!("GuildRankOrderName{index}"))?),
                    "remove" => self.remove_rank(),
                    "up" => self.move_rank(true),
                    _ => self.move_rank(false),
                }
            }
            "view" | "deposit" | "items" => self.click_tab(kind, index, texts),
            _ => None,
        }
    }

    fn click_tab(
        &self,
        kind: &str,
        index: usize,
        texts: &crate::bank::InputTexts,
    ) -> Option<GuildRankRequest> {
        let tab = self.selected()?.tabs.get(index)?;
        let view = if kind == "view" { !tab.view } else { tab.view };
        let deposit = if kind == "deposit" {
            !tab.deposit
        } else {
            tab.deposit
        };
        let stacks = if kind == "items" {
            texts.get(&format!("GuildTabItems{index}"))?.clone()
        } else {
            tab.withdrawals_per_day.to_string()
        };
        self.set_tab(u8::try_from(index).ok()?, view, deposit, &stacks)
    }

    pub fn selected(&self) -> Option<&GuildRankSettings> {
        self.state.as_ref()?.ranks.get(self.selected_rank)
    }

    fn editable_rank(&self) -> Option<(u8, &GuildRankSettings)> {
        let state = self.state.as_ref()?;
        if self.awaiting_state {
            return None;
        }
        if state.own_rank != 0 || self.selected_rank == 0 {
            return None;
        }
        Some((self.selected_rank as u8, self.selected()?))
    }

    pub fn set_permission(&self, flag: u32, enabled: bool) -> Option<GuildRankRequest> {
        if !flag.is_power_of_two() || flag & !GUILD_RIGHT_ALL != 0 {
            return None;
        }
        self.set_permission_mask(flag, enabled)
    }

    fn set_permission_mask(&self, mask: u32, enabled: bool) -> Option<GuildRankRequest> {
        let (rank, settings) = self.editable_rank()?;
        let rights = if enabled {
            settings.rights | mask
        } else {
            settings.rights & !mask
        };
        Some(GuildRankRequest::SetPermissions {
            rank,
            rights,
            gold_per_day: settings.gold_per_day,
        })
    }

    /// Retail GoldBox takes whole gold; the wire carries copper.
    pub fn set_gold_limit(&self, gold: &str) -> Option<GuildRankRequest> {
        let (rank, settings) = self.editable_rank()?;
        let gold = unsigned_input(gold)?;
        Some(GuildRankRequest::SetPermissions {
            rank,
            rights: settings.rights,
            gold_per_day: gold.checked_mul(10_000)?,
        })
    }

    pub fn set_tab(
        &self,
        tab: u8,
        view: bool,
        deposit: bool,
        stacks: &str,
    ) -> Option<GuildRankRequest> {
        let (rank, settings) = self.editable_rank()?;
        self.state.as_ref()?.tab_names.get(usize::from(tab))?;
        settings.tabs.get(usize::from(tab))?;
        let stacks = u32::try_from(unsigned_input(stacks)?).ok()?;
        Some(GuildRankRequest::SetTab {
            rank,
            tab,
            view,
            deposit,
            withdrawals_per_day: stacks,
        })
    }

    pub fn add_rank(&self, name: &str) -> Option<GuildRankRequest> {
        if self.awaiting_state {
            return None;
        }
        let state = self.state.as_ref()?;
        if state.own_rank != 0 || state.ranks.len() >= GUILD_MAX_RANKS {
            return None;
        }
        Some(GuildRankRequest::Add {
            name: valid_name(name)?,
        })
    }

    pub fn rename_rank(&self, name: &str) -> Option<GuildRankRequest> {
        let (rank, _) = self.editable_rank()?;
        Some(GuildRankRequest::Rename {
            rank,
            name: valid_name(name)?,
        })
    }

    pub fn remove_rank(&self) -> Option<GuildRankRequest> {
        let (rank, _) = self.editable_rank()?;
        let state = self.state.as_ref()?;
        if state.ranks.len() <= GUILD_MIN_RANKS
            || state.members.iter().any(|member| member.rank == rank)
        {
            return None;
        }
        Some(GuildRankRequest::Remove { rank })
    }

    pub fn move_rank(&self, up: bool) -> Option<GuildRankRequest> {
        let (rank, _) = self.editable_rank()?;
        let state = self.state.as_ref()?;
        if (up && rank <= 1) || (!up && usize::from(rank) + 1 >= state.ranks.len()) {
            return None;
        }
        Some(GuildRankRequest::Move { rank, up })
    }

    /// Mainline Whisper CanShow: online, not the local player; independent of guild rights.
    pub fn can_whisper(&self, name: &str) -> bool {
        self.profile.as_ref().is_some_and(|profile| {
            profile
                .members
                .iter()
                .any(|member| member.character_name == name && member.is_online && !member.is_self)
        })
    }

    pub fn can_transfer_leadership(&self, name: &str) -> bool {
        if self.awaiting_state || self.profile_waiting {
            return false;
        }
        let Some(state) = self.state.as_ref().filter(|state| state.own_rank == 0) else {
            return false;
        };
        let known_member = state
            .members
            .iter()
            .any(|member| member.character_name == name);
        let other_member = self.profile.as_ref().is_some_and(|profile| {
            profile
                .members
                .iter()
                .any(|member| member.character_name == name && !member.is_self)
        });
        known_member && other_member
    }

    pub fn remove_member(&self, name: &str) -> Option<GuildRankRequest> {
        if self.awaiting_state || !self.can(GUILD_RIGHT_REMOVE) {
            return None;
        }
        let state = self.state.as_ref()?;
        let member = state
            .members
            .iter()
            .find(|member| member.character_name.eq_ignore_ascii_case(name))?;
        if member.rank <= state.own_rank {
            return None;
        }
        Some(GuildRankRequest::RemoveMember {
            character_name: member.character_name.clone(),
        })
    }

    pub fn promote(&self, name: &str) -> Option<GuildRankRequest> {
        self.change_member_rank(name, true)
    }

    pub fn demote(&self, name: &str) -> Option<GuildRankRequest> {
        self.change_member_rank(name, false)
    }

    fn change_member_rank(&self, name: &str, promote: bool) -> Option<GuildRankRequest> {
        if self.awaiting_state {
            return None;
        }
        let state = self.state.as_ref()?;
        let own = state.ranks.get(usize::from(state.own_rank))?;
        let right = if promote {
            GUILD_RIGHT_PROMOTE
        } else {
            GUILD_RIGHT_DEMOTE
        };
        if state.own_rank != 0 && own.rights & right == 0 {
            return None;
        }
        let member = state
            .members
            .iter()
            .find(|member| member.character_name.eq_ignore_ascii_case(name))?;
        if member.rank <= state.own_rank {
            return None;
        }
        let character_name = member.character_name.clone();
        if promote {
            if member.rank <= state.own_rank + 1 {
                return None;
            }
            Some(GuildRankRequest::Promote { character_name })
        } else {
            if usize::from(member.rank) + 1 >= state.ranks.len() {
                return None;
            }
            Some(GuildRankRequest::Demote { character_name })
        }
    }
}

fn unsigned_input(text: &str) -> Option<u64> {
    let text = text.trim();
    if text.is_empty() || !text.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

fn valid_name(name: &str) -> Option<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 15 || name.chars().any(char::is_control) {
        return None;
    }
    Some(name.to_owned())
}
