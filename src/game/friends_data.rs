use bevy::prelude::*;

/// Texture FDIDs for the friends frame.
pub mod textures {
    // Frame chrome
    pub const FRAME_TOP_LEFT: u32 = 131130;
    pub const FRAME_BOTTOM_LEFT: u32 = 131125;
    pub const FRAME_BOTTOM_RIGHT: u32 = 131126;
    pub const HIGHLIGHT_BAR: u32 = 131128;
    // Status icons
    pub const STATUS_ONLINE: u32 = 374226;
    pub const STATUS_AWAY: u32 = 374223;
    pub const STATUS_DND: u32 = 374224;
    pub const STATUS_OFFLINE: u32 = 374225;
    // Game icons
    pub const GAME_WOW: u32 = 374212;
}

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

    /// Sort key: Online=0, Away=1, Busy=2, Offline=3.
    fn sort_key(self) -> u8 {
        match self {
            Self::Online => 0,
            Self::Away => 1,
            Self::Busy => 2,
            Self::Offline => 3,
        }
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

    /// Sort BNet friends: online first (by presence priority), then alphabetical.
    pub fn sort_bnet_friends(&mut self) {
        self.bnet_friends.sort_by(|a, b| {
            a.presence
                .sort_key()
                .cmp(&b.presence.sort_key())
                .then(a.battletag.cmp(&b.battletag))
        });
    }

    /// Sort character friends: online first, then alphabetical.
    pub fn sort_character_friends(&mut self) {
        self.character_friends
            .sort_by(|a, b| b.online.cmp(&a.online).then(a.name.cmp(&b.name)));
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

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::FRAME_TOP_LEFT, 0);
        assert_ne!(textures::STATUS_ONLINE, 0);
        assert_ne!(textures::STATUS_OFFLINE, 0);
        assert_ne!(textures::GAME_WOW, 0);
    }

    // --- BNet presence ---

    fn bnet(tag: &str, presence: PresenceState) -> BNetFriend {
        BNetFriend {
            battletag: tag.into(),
            character_name: String::new(),
            game: String::new(),
            presence,
            note: String::new(),
        }
    }

    fn char_friend(name: &str, online: bool) -> CharacterFriend {
        CharacterFriend {
            name: name.into(),
            level: 60,
            class: "Warrior".into(),
            area: "Orgrimmar".into(),
            online,
            note: String::new(),
        }
    }

    #[test]
    fn bnet_away_and_busy_count_as_online() {
        let mut state = FriendsState::default();
        state.bnet_friends = vec![
            bnet("Away#1", PresenceState::Away),
            bnet("Busy#2", PresenceState::Busy),
            bnet("Off#3", PresenceState::Offline),
        ];
        assert_eq!(state.online_bnet_count(), 2);
    }

    #[test]
    fn sort_bnet_friends_by_presence() {
        let mut state = FriendsState::default();
        state.bnet_friends = vec![
            bnet("Off#1", PresenceState::Offline),
            bnet("Away#2", PresenceState::Away),
            bnet("Online#3", PresenceState::Online),
            bnet("Busy#4", PresenceState::Busy),
        ];
        state.sort_bnet_friends();
        assert_eq!(state.bnet_friends[0].battletag, "Online#3");
        assert_eq!(state.bnet_friends[1].battletag, "Away#2");
        assert_eq!(state.bnet_friends[2].battletag, "Busy#4");
        assert_eq!(state.bnet_friends[3].battletag, "Off#1");
    }

    #[test]
    fn sort_bnet_friends_alphabetical_within_status() {
        let mut state = FriendsState::default();
        state.bnet_friends = vec![
            bnet("Zoe#1", PresenceState::Online),
            bnet("Alice#2", PresenceState::Online),
        ];
        state.sort_bnet_friends();
        assert_eq!(state.bnet_friends[0].battletag, "Alice#2");
        assert_eq!(state.bnet_friends[1].battletag, "Zoe#1");
    }

    // --- Character friend online/offline ---

    #[test]
    fn sort_character_friends_online_first() {
        let mut state = FriendsState::default();
        state.character_friends = vec![
            char_friend("Offline1", false),
            char_friend("Online1", true),
            char_friend("Offline2", false),
            char_friend("Online2", true),
        ];
        state.sort_character_friends();
        assert!(state.character_friends[0].online);
        assert!(state.character_friends[1].online);
        assert!(!state.character_friends[2].online);
        assert!(!state.character_friends[3].online);
    }

    #[test]
    fn sort_character_friends_alpha_within_status() {
        let mut state = FriendsState::default();
        state.character_friends = vec![char_friend("Zack", true), char_friend("Alice", true)];
        state.sort_character_friends();
        assert_eq!(state.character_friends[0].name, "Alice");
        assert_eq!(state.character_friends[1].name, "Zack");
    }

    #[test]
    fn bnet_friend_with_game_and_character() {
        let f = BNetFriend {
            battletag: "Test#1234".into(),
            character_name: "Thrall".into(),
            game: "World of Warcraft".into(),
            presence: PresenceState::Online,
            note: "guild mate".into(),
        };
        assert!(f.presence.is_online());
        assert_eq!(f.character_name, "Thrall");
        assert_eq!(f.game, "World of Warcraft");
    }
}
