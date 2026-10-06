//! Authoritative guild settings decisions; requests never optimistically edit state.
use shared::protocol::*;

pub const ROSTER_PAGE_SIZE: usize = 9;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GuildRanksSession {
    pub visible: bool,
    pub awaiting_state: bool,
    pub settings_open: bool,
    pub member_menu: Option<String>,
    pub error: Option<String>,
    state: Option<GuildRanksState>,
    selected_rank: usize,
    roster_page: usize,
}

impl GuildRanksSession {
    pub fn open(&mut self) -> GuildRankRequest {
        self.visible = true;
        self.awaiting_state = true;
        GuildRankRequest::Query
    }

    pub fn apply(&mut self, state: GuildRanksState) {
        self.awaiting_state = false;
        self.error = state.error.map(|error| error.message().to_owned());
        self.selected_rank = self.selected_rank.min(state.ranks.len().saturating_sub(1));
        self.roster_page = self
            .roster_page
            .min(state.members.len().saturating_sub(1) / ROSTER_PAGE_SIZE);
        self.state = Some(state);
    }

    pub fn state(&self) -> Option<&GuildRanksState> {
        self.state.as_ref()
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

    pub fn roster_page(&self) -> usize {
        self.roster_page
    }

    pub fn roster_has_next(&self) -> bool {
        self.state
            .as_ref()
            .is_some_and(|state| state.members.len() > (self.roster_page + 1) * ROSTER_PAGE_SIZE)
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
        let input = |name: &str| texts.get(name).map(String::as_str).unwrap_or_default();
        let request = match action {
            "guild:close" => {
                self.visible = false;
                self.settings_open = false;
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
            "guild:roster_previous" => {
                self.roster_page = self.roster_page.saturating_sub(1);
                self.member_menu = None;
                None
            }
            "guild:roster_next" => {
                if self.roster_has_next() {
                    self.roster_page += 1;
                }
                self.member_menu = None;
                None
            }
            "guild:add" => self.add_rank(input(crate::guild_rank_frame::NAME_BOX)),
            "guild:rename" => self.rename_rank(input(crate::guild_rank_frame::NAME_BOX)),
            "guild:remove" => self.remove_rank(),
            "guild:up" => self.move_rank(true),
            "guild:down" => self.move_rank(false),
            "guild:gold" => self.set_gold_limit(input(crate::guild_rank_frame::GOLD_BOX)),
            "guild:promote" => self.promote(self.member_menu.as_deref()?),
            "guild:demote" => self.demote(self.member_menu.as_deref()?),
            _ => self.indexed_click(action, texts),
        };
        if request.is_some() {
            self.awaiting_state = true;
        }
        request
    }

    fn indexed_click(
        &mut self,
        action: &str,
        texts: &crate::bank::InputTexts,
    ) -> Option<GuildRankRequest> {
        if let Some(name) = action.strip_prefix("guild:member:") {
            self.member_menu = Some(name.to_owned());
            return None;
        }
        let (kind, index) = action.strip_prefix("guild:")?.split_once(':')?;
        let index: usize = index.parse().ok()?;
        match kind {
            "rank" => {
                self.select_rank(index);
                None
            }
            "permission" => {
                let flag = 1u32.checked_shl(u32::try_from(index).ok()?)?;
                self.set_permission(flag, self.selected()?.rights & flag == 0)
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
        let (rank, settings) = self.editable_rank()?;
        let rights = if enabled {
            settings.rights | flag
        } else {
            settings.rights & !flag
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
