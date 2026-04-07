use bevy::prelude::*;

/// Texture FDIDs for the communities frame.
pub mod textures {
    /// Main guild/community frame chrome.
    pub const FRAME_CHROME: u32 = 410251;
    /// Frame bottom-left corner.
    pub const CORNER_BOTTOM_LEFT: u32 = 131117;
    /// Frame bottom-right corner.
    pub const CORNER_BOTTOM_RIGHT: u32 = 131118;
    /// LFG role icons (tank/healer/dps combined).
    pub const ROLE_ICONS: u32 = 337499;
    /// Role icons sheet.
    pub const ROLE_ICONS_SHEET: u32 = 2134184;
    /// Default community/guild logo placeholder.
    pub const GUILD_LOGO_DEFAULT: u32 = 460904;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MemberRole {
    Owner,
    Officer,
    #[default]
    Member,
    Guest,
}

impl MemberRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::Owner => "Owner",
            Self::Officer => "Officer",
            Self::Member => "Member",
            Self::Guest => "Guest",
        }
    }

    pub fn can_kick(self) -> bool {
        matches!(self, Self::Owner | Self::Officer)
    }

    pub fn can_invite(self) -> bool {
        !matches!(self, Self::Guest)
    }

    pub fn can_edit_info(self) -> bool {
        matches!(self, Self::Owner | Self::Officer)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CommunityDef {
    pub id: u64,
    pub name: String,
    pub icon_fdid: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemberInfo {
    pub name: String,
    pub role: MemberRole,
    pub class: String,
    pub online: bool,
}

impl MemberInfo {
    pub fn status_label(&self) -> &'static str {
        if self.online { "Online" } else { "Offline" }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChatMsg {
    pub sender: String,
    pub text: String,
    pub timestamp: f64,
}

/// Runtime state for communities.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct CommunitiesState {
    pub communities: Vec<CommunityDef>,
    pub selected_community: Option<u64>,
    pub members: Vec<MemberInfo>,
    pub messages: Vec<ChatMsg>,
    pub player_role: MemberRole,
}

impl MemberRole {
    /// Sort priority (lower = higher rank).
    fn sort_priority(self) -> u8 {
        match self {
            Self::Owner => 0,
            Self::Officer => 1,
            Self::Member => 2,
            Self::Guest => 3,
        }
    }
}

impl CommunitiesState {
    pub fn selected_name(&self) -> &str {
        self.selected_community
            .and_then(|id| self.communities.iter().find(|c| c.id == id))
            .map(|c| c.name.as_str())
            .unwrap_or("")
    }

    pub fn online_count(&self) -> usize {
        self.members.iter().filter(|m| m.online).count()
    }

    /// Sort members: online first, then by role rank, then alphabetically.
    pub fn sort_members(&mut self) {
        self.members.sort_by(|a, b| {
            b.online
                .cmp(&a.online)
                .then(a.role.sort_priority().cmp(&b.role.sort_priority()))
                .then(a.name.cmp(&b.name))
        });
    }

    /// Sort messages by timestamp (oldest first).
    pub fn sort_messages(&mut self) {
        self.messages
            .sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap());
    }

    /// Set a member's role by name. Returns true if found.
    pub fn set_member_role(&mut self, name: &str, role: MemberRole) -> bool {
        if let Some(member) = self.members.iter_mut().find(|m| m.name == name) {
            member.role = role;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_labels() {
        assert_eq!(MemberRole::Owner.label(), "Owner");
        assert_eq!(MemberRole::Officer.label(), "Officer");
        assert_eq!(MemberRole::Member.label(), "Member");
        assert_eq!(MemberRole::Guest.label(), "Guest");
    }

    #[test]
    fn role_permissions() {
        assert!(MemberRole::Owner.can_kick());
        assert!(MemberRole::Officer.can_kick());
        assert!(!MemberRole::Member.can_kick());
        assert!(!MemberRole::Guest.can_kick());

        assert!(MemberRole::Owner.can_invite());
        assert!(MemberRole::Member.can_invite());
        assert!(!MemberRole::Guest.can_invite());

        assert!(MemberRole::Owner.can_edit_info());
        assert!(!MemberRole::Member.can_edit_info());
    }

    #[test]
    fn member_status_label() {
        let online = MemberInfo {
            name: "Alice".into(),
            role: MemberRole::Officer,
            class: "Paladin".into(),
            online: true,
        };
        assert_eq!(online.status_label(), "Online");
        let offline = MemberInfo {
            online: false,
            ..online
        };
        assert_eq!(offline.status_label(), "Offline");
    }

    #[test]
    fn selected_name_resolves() {
        let mut state = CommunitiesState::default();
        state.communities.push(CommunityDef {
            id: 1,
            name: "My Guild".into(),
            icon_fdid: 0,
        });
        state.selected_community = Some(1);
        assert_eq!(state.selected_name(), "My Guild");
    }

    #[test]
    fn selected_name_empty_when_none() {
        let state = CommunitiesState::default();
        assert_eq!(state.selected_name(), "");
    }

    #[test]
    fn online_count() {
        let mut state = CommunitiesState::default();
        state.members = vec![
            MemberInfo {
                name: "A".into(),
                role: MemberRole::Member,
                class: "Mage".into(),
                online: true,
            },
            MemberInfo {
                name: "B".into(),
                role: MemberRole::Member,
                class: "Rogue".into(),
                online: false,
            },
            MemberInfo {
                name: "C".into(),
                role: MemberRole::Officer,
                class: "Priest".into(),
                online: true,
            },
        ];
        assert_eq!(state.online_count(), 2);
    }

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::FRAME_CHROME, 0);
        assert_ne!(textures::ROLE_ICONS, 0);
        assert_ne!(textures::GUILD_LOGO_DEFAULT, 0);
    }

    // --- Member sorting ---

    fn member(name: &str, role: MemberRole, online: bool) -> MemberInfo {
        MemberInfo {
            name: name.into(),
            role,
            class: "Warrior".into(),
            online,
        }
    }

    #[test]
    fn sort_members_online_first() {
        let mut state = CommunitiesState::default();
        state.members = vec![
            member("Offline1", MemberRole::Member, false),
            member("Online1", MemberRole::Member, true),
        ];
        state.sort_members();
        assert_eq!(state.members[0].name, "Online1");
        assert_eq!(state.members[1].name, "Offline1");
    }

    #[test]
    fn sort_members_by_role_rank() {
        let mut state = CommunitiesState::default();
        state.members = vec![
            member("Guest1", MemberRole::Guest, true),
            member("Owner1", MemberRole::Owner, true),
            member("Officer1", MemberRole::Officer, true),
            member("Member1", MemberRole::Member, true),
        ];
        state.sort_members();
        assert_eq!(state.members[0].name, "Owner1");
        assert_eq!(state.members[1].name, "Officer1");
        assert_eq!(state.members[2].name, "Member1");
        assert_eq!(state.members[3].name, "Guest1");
    }

    #[test]
    fn sort_members_alphabetical_within_rank() {
        let mut state = CommunitiesState::default();
        state.members = vec![
            member("Charlie", MemberRole::Member, true),
            member("Alice", MemberRole::Member, true),
            member("Bob", MemberRole::Member, true),
        ];
        state.sort_members();
        assert_eq!(state.members[0].name, "Alice");
        assert_eq!(state.members[1].name, "Bob");
        assert_eq!(state.members[2].name, "Charlie");
    }

    #[test]
    fn sort_members_combined() {
        let mut state = CommunitiesState::default();
        state.members = vec![
            member("OfflineMember", MemberRole::Member, false),
            member("OnlineGuest", MemberRole::Guest, true),
            member("OnlineOwner", MemberRole::Owner, true),
            member("OfflineOfficer", MemberRole::Officer, false),
        ];
        state.sort_members();
        // Online first, then by rank
        assert_eq!(state.members[0].name, "OnlineOwner");
        assert_eq!(state.members[1].name, "OnlineGuest");
        // Offline, then by rank
        assert_eq!(state.members[2].name, "OfflineOfficer");
        assert_eq!(state.members[3].name, "OfflineMember");
    }

    // --- Chat message ordering ---

    #[test]
    fn sort_messages_by_timestamp() {
        let mut state = CommunitiesState::default();
        state.messages = vec![
            ChatMsg {
                sender: "B".into(),
                text: "second".into(),
                timestamp: 200.0,
            },
            ChatMsg {
                sender: "A".into(),
                text: "first".into(),
                timestamp: 100.0,
            },
            ChatMsg {
                sender: "C".into(),
                text: "third".into(),
                timestamp: 300.0,
            },
        ];
        state.sort_messages();
        assert_eq!(state.messages[0].text, "first");
        assert_eq!(state.messages[1].text, "second");
        assert_eq!(state.messages[2].text, "third");
    }

    // --- Role assignment ---

    #[test]
    fn set_member_role_promotes() {
        let mut state = CommunitiesState::default();
        state.members = vec![member("Alice", MemberRole::Member, true)];
        assert!(state.set_member_role("Alice", MemberRole::Officer));
        assert_eq!(state.members[0].role, MemberRole::Officer);
    }

    #[test]
    fn set_member_role_demotes() {
        let mut state = CommunitiesState::default();
        state.members = vec![member("Bob", MemberRole::Officer, true)];
        assert!(state.set_member_role("Bob", MemberRole::Guest));
        assert_eq!(state.members[0].role, MemberRole::Guest);
    }

    #[test]
    fn set_member_role_not_found() {
        let mut state = CommunitiesState::default();
        state.members = vec![member("Alice", MemberRole::Member, true)];
        assert!(!state.set_member_role("Unknown", MemberRole::Officer));
    }
}
