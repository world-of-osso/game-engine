use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PresenceState {
    Online,
    Away,
    Busy,
    #[default]
    Offline,
}

impl PresenceState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Online => "Online",
            Self::Away => "Away",
            Self::Busy => "Busy",
            Self::Offline => "Offline",
        }
    }

    pub fn is_online(self) -> bool {
        !matches!(self, Self::Offline)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BNetFriend {
    pub battletag: String,
    pub character_name: String,
    pub game: String,
    pub presence: PresenceState,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharacterFriend {
    pub name: String,
    pub level: u32,
    pub class: String,
    pub area: String,
    pub online: bool,
    pub note: String,
}

impl CharacterFriend {
    pub fn status_label(&self) -> &'static str {
        if self.online { "Online" } else { "Offline" }
    }
}

/// Runtime friends list state.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct FriendsState {
    pub bnet_friends: Vec<BNetFriend>,
    pub character_friends: Vec<CharacterFriend>,
}

impl FriendsState {
    pub fn online_bnet_count(&self) -> usize {
        self.bnet_friends
            .iter()
            .filter(|f| f.presence.is_online())
            .count()
    }

    pub fn online_character_count(&self) -> usize {
        self.character_friends.iter().filter(|f| f.online).count()
    }

    pub fn total_online(&self) -> usize {
        self.online_bnet_count() + self.online_character_count()
    }

    pub fn total_friends(&self) -> usize {
        self.bnet_friends.len() + self.character_friends.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_labels() {
        assert_eq!(PresenceState::Online.label(), "Online");
        assert_eq!(PresenceState::Away.label(), "Away");
        assert_eq!(PresenceState::Busy.label(), "Busy");
        assert_eq!(PresenceState::Offline.label(), "Offline");
    }

    #[test]
    fn presence_is_online() {
        assert!(PresenceState::Online.is_online());
        assert!(PresenceState::Away.is_online());
        assert!(PresenceState::Busy.is_online());
        assert!(!PresenceState::Offline.is_online());
    }

    #[test]
    fn character_friend_status() {
        let f = CharacterFriend {
            name: "Arthas".into(),
            level: 60,
            class: "Paladin".into(),
            area: "Stormwind".into(),
            online: true,
            note: String::new(),
        };
        assert_eq!(f.status_label(), "Online");
        let off = CharacterFriend { online: false, ..f };
        assert_eq!(off.status_label(), "Offline");
    }

    #[test]
    fn online_counts() {
        let mut state = FriendsState::default();
        state.bnet_friends.push(BNetFriend {
            battletag: "Alice#1234".into(),
            character_name: "Alicechar".into(),
            game: "World of Warcraft".into(),
            presence: PresenceState::Online,
            note: String::new(),
        });
        state.bnet_friends.push(BNetFriend {
            battletag: "Bob#5678".into(),
            character_name: String::new(),
            game: String::new(),
            presence: PresenceState::Offline,
            note: String::new(),
        });
        state.character_friends.push(CharacterFriend {
            name: "Charlie".into(),
            level: 40,
            class: "Mage".into(),
            area: "Ironforge".into(),
            online: true,
            note: "old friend".into(),
        });
        assert_eq!(state.online_bnet_count(), 1);
        assert_eq!(state.online_character_count(), 1);
        assert_eq!(state.total_online(), 2);
        assert_eq!(state.total_friends(), 3);
    }

    #[test]
    fn default_state_empty() {
        let state = FriendsState::default();
        assert_eq!(state.total_friends(), 0);
        assert_eq!(state.total_online(), 0);
    }
}
