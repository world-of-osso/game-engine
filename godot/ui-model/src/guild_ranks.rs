//! Portable guild settings decisions. Native mounting/transport are tracked separately
//! in docs/specs/guild-ranks.md. Requests never optimistically edit server state.
use shared::protocol::*;

#[derive(Debug, Default)]
pub struct GuildRanksSession {
    pub visible: bool,
    pub error: Option<String>,
    state: Option<GuildRanksState>,
    selected_rank: usize,
}

impl GuildRanksSession {
    pub fn open(&mut self) -> GuildRankRequest {
        self.visible = true;
        GuildRankRequest::Query
    }

    pub fn apply(&mut self, state: GuildRanksState) {
        self.error = state.error.map(|error| error.message().to_owned());
        self.selected_rank = self.selected_rank.min(state.ranks.len().saturating_sub(1));
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

    pub fn selected(&self) -> Option<&GuildRankSettings> {
        self.state.as_ref()?.ranks.get(self.selected_rank)
    }

    fn editable_rank(&self) -> Option<(u8, &GuildRankSettings)> {
        let state = self.state.as_ref()?;
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
        settings.tabs.get(usize::from(tab))?;
        let stacks = u32::try_from(unsigned_input(stacks)?).ok()?;
        Some(GuildRankRequest::SetTab {
            rank,
            tab,
            view,
            deposit: view && deposit,
            withdrawals_per_day: if view { stacks } else { 0 },
        })
    }

    pub fn add_rank(&self, name: &str) -> Option<GuildRankRequest> {
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
