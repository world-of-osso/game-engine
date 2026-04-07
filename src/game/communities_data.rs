use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemberRole {
    Owner,
    Officer,
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

impl Default for MemberRole {
    fn default() -> Self {
        Self::Member
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
}
