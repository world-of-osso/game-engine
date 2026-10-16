//! Separate news query/filter/sticky authority; never shares rank or lifecycle ACKs.
use shared::protocol::{GuildNewsEntry, GuildNewsKind, GuildNewsRequest, GuildNewsState};

#[derive(Clone, Debug, PartialEq)]
pub struct GuildNewsSession {
    pub state: Option<GuildNewsState>,
    pub waiting: bool,
    pub filters_open: bool,
    pub filters: [bool; 10],
    pub menu: Option<u64>,
    pub pending_sticky: Option<(u64, bool)>,
}

impl Default for GuildNewsSession {
    fn default() -> Self {
        Self {
            state: None,
            waiting: false,
            filters_open: false,
            filters: [true; 10],
            menu: None,
            pending_sticky: None,
        }
    }
}

impl GuildNewsSession {
    pub fn query(&mut self) -> Option<GuildNewsRequest> {
        if self.waiting {
            return None;
        }
        self.waiting = true;
        Some(GuildNewsRequest::Query)
    }

    pub fn apply(&mut self, state: GuildNewsState) {
        self.waiting = false;
        if state.error.is_some()
            || self.pending_sticky.is_some_and(|(id, desired)| {
                state
                    .entries
                    .iter()
                    .find(|entry| entry.id == id)
                    .is_none_or(|entry| entry.sticky == desired)
            })
        {
            self.pending_sticky = None;
        }
        if state.error == Some(shared::protocol::GuildNewsError::NotInGuild) {
            *self = Self::default();
            return;
        }
        if self
            .menu
            .is_some_and(|id| !state.entries.iter().any(|entry| entry.id == id))
        {
            self.menu = None;
        }
        self.state = Some(state);
    }

    pub fn sticky(&mut self, can_edit: bool) -> Option<GuildNewsRequest> {
        if !can_edit || self.pending_sticky.is_some() {
            return None;
        }
        let entry = self.entry(self.menu?)?;
        let (id, sticky) = (entry.id, !entry.sticky);
        self.pending_sticky = Some((id, sticky));
        self.menu = None;
        Some(GuildNewsRequest::SetSticky { id, sticky })
    }

    pub fn entry(&self, id: u64) -> Option<&GuildNewsEntry> {
        self.state
            .as_ref()?
            .entries
            .iter()
            .find(|entry| entry.id == id)
    }

    pub fn entries(&self) -> Vec<&GuildNewsEntry> {
        let Some(state) = &self.state else {
            return vec![];
        };
        let mut entries: Vec<_> = state
            .entries
            .iter()
            .filter(|entry| entry.sticky || self.filters[filter_id(&entry.kind)])
            .collect();
        entries.sort_by_key(|entry| std::cmp::Reverse((entry.sticky, entry.timestamp, entry.id)));
        entries
    }
}

pub fn filter_id(kind: &GuildNewsKind) -> usize {
    match kind {
        GuildNewsKind::PlayerAchievement { .. } => 2,
        GuildNewsKind::ItemLooted { .. } => 4,
        GuildNewsKind::ItemCrafted { .. } => 5,
        GuildNewsKind::ItemPurchased { .. } => 6,
    }
}
