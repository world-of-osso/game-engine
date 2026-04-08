use bevy::prelude::*;

/// A single member entry in the guild roster.
#[derive(Clone, Debug, PartialEq)]
pub struct GuildRosterEntry {
    pub name: String,
    pub level: u32,
    /// WoW class ID (1=Warrior..13=Evoker).
    pub class_id: u8,
    pub rank_index: u8,
    pub rank_name: String,
    pub zone: String,
    pub is_online: bool,
    pub public_note: String,
    pub officer_note: String,
    /// Human-readable time since last online (e.g. "2 days ago").
    pub last_online: String,
}

/// Column to sort the guild roster by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GuildRosterSort {
    #[default]
    Name,
    Level,
    Class,
    Rank,
    Zone,
    Status,
}

/// Runtime guild roster state.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct GuildRosterState {
    pub members: Vec<GuildRosterEntry>,
    pub guild_name: String,
    /// Message of the day.
    pub motd: String,
    pub sort: GuildRosterSort,
    pub sort_ascending: bool,
    pub show_offline: bool,
}

impl GuildRosterState {
    pub fn total_count(&self) -> usize {
        self.members.len()
    }

    pub fn online_count(&self) -> usize {
        self.members.iter().filter(|m| m.is_online).count()
    }

    /// Members filtered by the offline visibility toggle.
    pub fn visible_members(&self) -> Vec<&GuildRosterEntry> {
        self.members
            .iter()
            .filter(|m| self.show_offline || m.is_online)
            .collect()
    }

    /// Toggle sort direction or switch to a new sort column.
    pub fn set_sort(&mut self, field: GuildRosterSort) {
        if self.sort == field {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort = field;
            self.sort_ascending = true;
        }
    }

    pub fn toggle_offline(&mut self) {
        self.show_offline = !self.show_offline;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn online_member(name: &str, level: u32, rank: u8) -> GuildRosterEntry {
        GuildRosterEntry {
            name: name.into(),
            level,
            class_id: 1,
            rank_index: rank,
            rank_name: "Member".into(),
            zone: "Stormwind".into(),
            is_online: true,
            public_note: String::new(),
            officer_note: String::new(),
            last_online: String::new(),
        }
    }

    fn offline_member(name: &str) -> GuildRosterEntry {
        GuildRosterEntry {
            is_online: false,
            last_online: "3 days ago".into(),
            zone: String::new(),
            ..online_member(name, 60, 2)
        }
    }

    #[test]
    fn empty_roster() {
        let state = GuildRosterState::default();
        assert_eq!(state.total_count(), 0);
        assert_eq!(state.online_count(), 0);
        assert!(state.visible_members().is_empty());
    }

    #[test]
    fn total_and_online_counts() {
        let state = GuildRosterState {
            members: vec![
                online_member("Alice", 60, 0),
                online_member("Bob", 40, 1),
                offline_member("Charlie"),
            ],
            ..Default::default()
        };
        assert_eq!(state.total_count(), 3);
        assert_eq!(state.online_count(), 2);
    }

    #[test]
    fn visible_members_hides_offline() {
        let state = GuildRosterState {
            members: vec![
                online_member("Alice", 60, 0),
                offline_member("Bob"),
                online_member("Charlie", 30, 1),
            ],
            show_offline: false,
            ..Default::default()
        };
        let visible = state.visible_members();
        assert_eq!(visible.len(), 2);
        assert_eq!(visible[0].name, "Alice");
        assert_eq!(visible[1].name, "Charlie");
    }

    #[test]
    fn visible_members_shows_offline() {
        let state = GuildRosterState {
            members: vec![online_member("Alice", 60, 0), offline_member("Bob")],
            show_offline: true,
            ..Default::default()
        };
        assert_eq!(state.visible_members().len(), 2);
    }

    #[test]
    fn toggle_offline() {
        let mut state = GuildRosterState::default();
        assert!(!state.show_offline);
        state.toggle_offline();
        assert!(state.show_offline);
        state.toggle_offline();
        assert!(!state.show_offline);
    }

    #[test]
    fn set_sort_toggles_direction() {
        let mut state = GuildRosterState::default();
        state.set_sort(GuildRosterSort::Level);
        assert_eq!(state.sort, GuildRosterSort::Level);
        assert!(state.sort_ascending);

        state.set_sort(GuildRosterSort::Level);
        assert!(!state.sort_ascending);
    }

    #[test]
    fn set_sort_new_field_resets_direction() {
        let mut state = GuildRosterState {
            sort: GuildRosterSort::Level,
            sort_ascending: false,
            ..Default::default()
        };
        state.set_sort(GuildRosterSort::Rank);
        assert_eq!(state.sort, GuildRosterSort::Rank);
        assert!(state.sort_ascending);
    }

    #[test]
    fn default_sort_is_name() {
        assert_eq!(GuildRosterSort::default(), GuildRosterSort::Name);
    }

    #[test]
    fn motd_and_guild_name() {
        let state = GuildRosterState {
            guild_name: "Test Guild".into(),
            motd: "Welcome!".into(),
            ..Default::default()
        };
        assert_eq!(state.guild_name, "Test Guild");
        assert_eq!(state.motd, "Welcome!");
    }

    #[test]
    fn all_offline_with_filter() {
        let state = GuildRosterState {
            members: vec![offline_member("A"), offline_member("B")],
            show_offline: false,
            ..Default::default()
        };
        assert!(state.visible_members().is_empty());
        assert_eq!(state.total_count(), 2);
        assert_eq!(state.online_count(), 0);
    }
}
