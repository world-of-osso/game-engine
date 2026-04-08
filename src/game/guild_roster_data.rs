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

// --- Client → server intents ---

/// A pending guild action to send to the server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GuildIntent {
    /// Invite a player by name.
    Invite { player_name: String },
    /// Remove a member from the guild.
    Kick { player_name: String },
    /// Promote a member to the next higher rank.
    Promote { player_name: String },
    /// Demote a member to the next lower rank.
    Demote { player_name: String },
    /// Update the guild message of the day.
    SetMotd { text: String },
    /// Update a member's public note.
    SetPublicNote { player_name: String, note: String },
    /// Update a member's officer note.
    SetOfficerNote { player_name: String, note: String },
    /// Leave the guild (self).
    Leave,
}

/// Queue of guild intents waiting to be sent to the server.
#[derive(Resource, Default)]
pub struct GuildIntentQueue {
    pub pending: Vec<GuildIntent>,
}

impl GuildIntentQueue {
    pub fn invite(&mut self, player_name: String) {
        self.pending.push(GuildIntent::Invite { player_name });
    }

    pub fn kick(&mut self, player_name: String) {
        self.pending.push(GuildIntent::Kick { player_name });
    }

    pub fn promote(&mut self, player_name: String) {
        self.pending.push(GuildIntent::Promote { player_name });
    }

    pub fn demote(&mut self, player_name: String) {
        self.pending.push(GuildIntent::Demote { player_name });
    }

    pub fn set_motd(&mut self, text: String) {
        self.pending.push(GuildIntent::SetMotd { text });
    }

    pub fn set_public_note(&mut self, player_name: String, note: String) {
        self.pending
            .push(GuildIntent::SetPublicNote { player_name, note });
    }

    pub fn set_officer_note(&mut self, player_name: String, note: String) {
        self.pending
            .push(GuildIntent::SetOfficerNote { player_name, note });
    }

    pub fn leave(&mut self) {
        self.pending.push(GuildIntent::Leave);
    }

    pub fn drain(&mut self) -> Vec<GuildIntent> {
        std::mem::take(&mut self.pending)
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

    // --- GuildIntentQueue ---

    #[test]
    fn intent_invite() {
        let mut queue = GuildIntentQueue::default();
        queue.invite("Alice".into());
        let drained = queue.drain();
        assert_eq!(
            drained[0],
            GuildIntent::Invite {
                player_name: "Alice".into()
            }
        );
    }

    #[test]
    fn intent_kick() {
        let mut queue = GuildIntentQueue::default();
        queue.kick("Bob".into());
        let drained = queue.drain();
        assert_eq!(
            drained[0],
            GuildIntent::Kick {
                player_name: "Bob".into()
            }
        );
    }

    #[test]
    fn intent_promote_demote() {
        let mut queue = GuildIntentQueue::default();
        queue.promote("Alice".into());
        queue.demote("Bob".into());
        let drained = queue.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(
            drained[0],
            GuildIntent::Promote {
                player_name: "Alice".into()
            }
        );
        assert_eq!(
            drained[1],
            GuildIntent::Demote {
                player_name: "Bob".into()
            }
        );
    }

    #[test]
    fn intent_set_motd() {
        let mut queue = GuildIntentQueue::default();
        queue.set_motd("Hello guild!".into());
        let drained = queue.drain();
        assert_eq!(
            drained[0],
            GuildIntent::SetMotd {
                text: "Hello guild!".into()
            }
        );
    }

    #[test]
    fn intent_set_notes() {
        let mut queue = GuildIntentQueue::default();
        queue.set_public_note("Alice".into(), "Main tank".into());
        queue.set_officer_note("Alice".into(), "Reliable".into());
        let drained = queue.drain();
        assert_eq!(
            drained[0],
            GuildIntent::SetPublicNote {
                player_name: "Alice".into(),
                note: "Main tank".into()
            }
        );
        assert_eq!(
            drained[1],
            GuildIntent::SetOfficerNote {
                player_name: "Alice".into(),
                note: "Reliable".into()
            }
        );
    }

    #[test]
    fn intent_leave() {
        let mut queue = GuildIntentQueue::default();
        queue.leave();
        let drained = queue.drain();
        assert_eq!(drained[0], GuildIntent::Leave);
    }

    #[test]
    fn intent_drain_clears() {
        let mut queue = GuildIntentQueue::default();
        queue.invite("A".into());
        queue.kick("B".into());
        queue.leave();
        assert_eq!(queue.drain().len(), 3);
        assert!(queue.pending.is_empty());
    }
}
