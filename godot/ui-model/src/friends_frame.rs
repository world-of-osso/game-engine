//! Native FriendsFrame state and projections for Retail's friends, Who and Raid tabs.

use crate::game::group_state::GroupState;
use shared::{
    components::PresenceStatus,
    protocol_snapshots::{FriendsSnapshot, WhoSnapshot},
};

pub const ACTION_SOCIAL_TOGGLE: &str = "social:toggle";
pub const ACTION_CLOSE: &str = "friends:close";
pub const ACTION_TAB_FRIENDS: &str = "friends:tab:friends";
pub const ACTION_TAB_WHO: &str = "friends:tab:who";
pub const ACTION_TAB_RAID: &str = "friends:tab:raid";
pub const ACTION_ADD_OPEN: &str = "friends:add_open";
pub const ACTION_ADD_SUBMIT: &str = "friends:add_submit";
pub const ACTION_ADD_CANCEL: &str = "friends:add_cancel";
pub const ACTION_SEND_MESSAGE: &str = "friends:send_message";
pub const ACTION_WHO_REFRESH: &str = "friends:who_refresh";
pub const ACTION_WHO_ADD: &str = "friends:who_add";
pub const ACTION_WHO_INVITE: &str = "friends:who_invite";
pub const ACTION_RAID_CONVERT: &str = "friends:raid_convert";
pub const ACTION_RAID_INFO: &str = "friends:raid_info";
pub const ACTION_CONTEXT_SEND: &str = "friends:context_send";
pub const ACTION_CONTEXT_ADD: &str = "friends:context_add";
pub const ACTION_CONTEXT_INVITE: &str = "friends:context_invite";
pub const FRIEND_ROW_PREFIX: &str = "friends:row:";
pub const WHO_ROW_PREFIX: &str = "friends:who_row:";

pub const FRIENDS_SCROLL_BOX: &str = "FriendsFrameFriendsScrollBox";
pub const WHO_SCROLL_BOX: &str = "FriendsFrameWhoScrollBox";
pub const RAID_SCROLL_BOX: &str = "FriendsFrameRaidScrollBox";
pub const ADD_FRIEND_EDIT_BOX: &str = "FriendsFrameAddFriendEditBox";
pub const WHO_QUERY_EDIT_BOX: &str = "WhoFrameEditBox";

const STATUS_ONLINE: u32 = 374_226;
const STATUS_AFK: u32 = 374_223;
const STATUS_DND: u32 = 374_224;
const STATUS_OFFLINE: u32 = 374_225;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FriendsTab {
    #[default]
    Friends,
    Who,
    Raid,
}

impl FriendsTab {
    pub fn title(self) -> &'static str {
        match self {
            Self::Friends => "Friends",
            Self::Who => "Who",
            Self::Raid => "Raid",
        }
    }

    pub fn action(self) -> &'static str {
        match self {
            Self::Friends => ACTION_TAB_FRIENDS,
            Self::Who => ACTION_TAB_WHO,
            Self::Raid => ACTION_TAB_RAID,
        }
    }

    pub fn from_action(action: &str) -> Option<Self> {
        [Self::Friends, Self::Who, Self::Raid]
            .into_iter()
            .find(|tab| tab.action() == action)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FriendsFrameState {
    pub open: bool,
    pub tab: FriendsTab,
    pub selected_friend: Option<String>,
    pub selected_who: Option<String>,
    pub who_query: String,
    pub add_friend_name: String,
    pub add_friend_open: bool,
    pub context_menu_open: bool,
}

impl FriendsFrameState {
    /// `ToggleFriendsFrame`: opening starts on Friends; the same action closes any selected tab.
    pub fn toggle(&mut self) -> bool {
        self.open = !self.open;
        self.add_friend_open = false;
        self.add_friend_name.clear();
        self.context_menu_open = false;
        if self.open {
            self.tab = FriendsTab::Friends;
            self.selected_friend = None;
            self.selected_who = None;
        }
        self.open
    }

    pub fn close(&mut self) -> bool {
        let was_open = self.open;
        self.open = false;
        self.add_friend_open = false;
        self.add_friend_name.clear();
        self.context_menu_open = false;
        was_open
    }

    pub fn select_tab(&mut self, tab: FriendsTab) {
        self.tab = tab;
        self.add_friend_open = false;
        self.context_menu_open = false;
        match tab {
            FriendsTab::Friends => self.selected_who = None,
            FriendsTab::Who => self.selected_friend = None,
            FriendsTab::Raid => {
                self.selected_friend = None;
                self.selected_who = None;
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FriendListRow {
    pub name: String,
    pub name_text: String,
    pub info_text: String,
    pub presence: PresenceStatus,
    pub status_icon: u32,
    pub name_color: &'static str,
    pub background_color: &'static str,
    pub can_send_message: bool,
}

pub fn project_friend_rows(snapshot: Option<&FriendsSnapshot>) -> Vec<FriendListRow> {
    snapshot
        .into_iter()
        .flat_map(|snapshot| &snapshot.entries)
        .map(|friend| {
            let online = friend.presence != PresenceStatus::Offline;
            let (status_icon, info_text) = match friend.presence {
                PresenceStatus::Online => (STATUS_ONLINE, friend.area.clone()),
                PresenceStatus::Afk => (STATUS_AFK, format!("{} - Away", friend.area)),
                PresenceStatus::Dnd => (STATUS_DND, format!("{} - Do Not Disturb", friend.area)),
                PresenceStatus::Offline => (STATUS_OFFLINE, "Offline".to_owned()),
            };
            let name_text = if online {
                format!(
                    "{}, Level {} {}",
                    friend.name, friend.level, friend.class_name
                )
            } else {
                friend.name.clone()
            };
            FriendListRow {
                name: friend.name.clone(),
                name_text,
                info_text,
                presence: friend.presence,
                status_icon,
                name_color: if online {
                    "0.51,0.77,1,1"
                } else {
                    "0.5,0.5,0.5,1"
                },
                background_color: if online {
                    "0.08,0.12,0.18,0.55"
                } else {
                    "0.08,0.08,0.08,0.25"
                },
                can_send_message: online,
            }
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhoListRow {
    pub name: String,
    pub level: u16,
    pub class_name: String,
    pub area: String,
}

pub fn project_who_rows(snapshot: Option<&WhoSnapshot>) -> Vec<WhoListRow> {
    snapshot
        .into_iter()
        .flat_map(|snapshot| &snapshot.entries)
        .map(|entry| WhoListRow {
            name: entry.name.clone(),
            level: entry.level,
            class_name: entry.class_name.clone(),
            area: entry.area.clone(),
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RaidMemberRow {
    pub name: String,
    pub subgroup: u8,
    pub online: bool,
    pub leader: bool,
}

pub fn project_raid_rows(group: &GroupState) -> Vec<RaidMemberRow> {
    group
        .members
        .iter()
        .map(|member| RaidMemberRow {
            name: member.name.clone(),
            subgroup: member.subgroup,
            online: member.online,
            leader: member.is_leader,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol_snapshots::{
        FriendCharacterSnapshot, FriendsSnapshot, GroupMemberSnapshot, GroupPortraitAppearance,
        WhoCharacterSnapshot, WhoSnapshot,
    };

    #[test]
    fn social_toggle_opens_on_friends_then_closes_the_current_tab() {
        let mut state = FriendsFrameState::default();
        assert!(state.toggle());
        assert_eq!(state.tab, FriendsTab::Friends);
        state.select_tab(FriendsTab::Who);
        assert!(!state.toggle());
        assert!(!state.open);
        assert!(state.toggle());
        assert_eq!(state.tab, FriendsTab::Friends);
    }

    #[test]
    fn friend_rows_project_online_and_offline_status_and_message_eligibility() {
        let snapshot = FriendsSnapshot {
            entries: vec![
                FriendCharacterSnapshot {
                    name: "Ari".into(),
                    level: 70,
                    class_name: "Mage".into(),
                    area: "Stormwind City".into(),
                    presence: PresenceStatus::Online,
                    note: "Raid lead".into(),
                },
                FriendCharacterSnapshot {
                    name: "Mira".into(),
                    level: 70,
                    class_name: "Druid".into(),
                    area: "Darnassus".into(),
                    presence: PresenceStatus::Afk,
                    note: String::new(),
                },
                FriendCharacterSnapshot {
                    name: "Taro".into(),
                    level: 69,
                    class_name: "Warrior".into(),
                    area: "Ironforge".into(),
                    presence: PresenceStatus::Dnd,
                    note: String::new(),
                },
                FriendCharacterSnapshot {
                    name: "Lysa".into(),
                    level: 68,
                    class_name: "Priest".into(),
                    area: "Unknown".into(),
                    presence: PresenceStatus::Offline,
                    note: String::new(),
                },
            ],
        };

        let rows = project_friend_rows(Some(&snapshot));

        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].name_text, "Ari, Level 70 Mage");
        assert_eq!(rows[0].info_text, "Stormwind City");
        assert_eq!(rows[0].status_icon, STATUS_ONLINE);
        assert!(rows[0].can_send_message);
        assert_eq!(rows[1].name, "Mira");
        assert_eq!(rows[1].info_text, "Darnassus - Away");
        assert_eq!(rows[1].status_icon, STATUS_AFK);
        assert!(rows[1].can_send_message);
        assert_eq!(rows[2].info_text, "Ironforge - Do Not Disturb");
        assert_eq!(rows[2].status_icon, STATUS_DND);
        assert!(rows[2].can_send_message);
        assert_eq!(rows[3].name_text, "Lysa");
        assert_eq!(rows[3].info_text, "Offline");
        assert_eq!(rows[3].status_icon, STATUS_OFFLINE);
        assert!(!rows[3].can_send_message);
    }

    #[test]
    fn who_rows_preserve_server_name_level_class_and_area() {
        let snapshot = WhoSnapshot {
            query: "Mage".into(),
            entries: vec![WhoCharacterSnapshot {
                name: "Ari".into(),
                level: 70,
                class_name: "Mage".into(),
                area: "Stormwind City".into(),
            }],
        };
        let rows = project_who_rows(Some(&snapshot));

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Ari");
        assert_eq!(rows[0].level, 70);
        assert_eq!(rows[0].class_name, "Mage");
        assert_eq!(rows[0].area, "Stormwind City");
    }

    #[test]
    fn raid_rows_preserve_roster_order_and_online_leader_state() {
        let group = GroupState {
            is_raid: true,
            members: vec![GroupMemberSnapshot {
                character_id: 7,
                name: "Ari".into(),
                role: shared::protocol_snapshots::GroupRoleSnapshot::Healer,
                is_leader: true,
                online: true,
                subgroup: 2,
                class: 8,
                level: 70,
                entity: Some(91),
                portrait: GroupPortraitAppearance::default(),
            }],
            ..GroupState::default()
        };
        let rows = project_raid_rows(&group);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Ari");
        assert_eq!(rows[0].subgroup, 2);
        assert!(rows[0].online);
        assert!(rows[0].leader);
    }
}
