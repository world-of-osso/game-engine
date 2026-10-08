//! Client party/raid state: the server roster, each online member's live state and the
//! ready check, plus the [`GroupCommand`]s UI and chat send. Party and raid frames, the
//! invite popup, the ready check frame and IPC all read [`GroupState`].

use std::collections::HashMap;

use shared::protocol::{
    GroupMemberSnapshot, GroupMemberState, GroupRoleSnapshot, GroupRosterSnapshot,
    ReadyCheckAnswer, ReadyCheckUpdate,
};

/// Seconds ready check marks stay on the frames after the check ends
/// (`CUF_READY_CHECK_DECAY_TIME`, CompactUnitFrame.lua:3).
pub const READY_CHECK_DECAY_SECS: f32 = 11.0;

/// Retail `MAX_PARTY_MEMBERS + 1`: a raid of up to this size may convert to a party.
pub const MAX_PARTY_SIZE: usize = 5;

#[derive(Default, Debug, Clone, PartialEq)]
pub struct GroupState {
    pub is_raid: bool,
    /// Roster order from the server; empty when not in a group.
    pub members: Vec<GroupMemberSnapshot>,
    /// Live state of online members, by name.
    pub live: HashMap<String, GroupMemberState>,
    pub ready_check: Option<ReadyCheckView>,
    /// Inviter of the pending `PARTY_INVITE`, until answered or cancelled.
    pub pending_invite: Option<String>,
    pub last_server_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReadyCheckView {
    pub update: ReadyCheckUpdate,
    /// Seconds since the check finished; `None` while it runs.
    pub finished_for: Option<f32>,
}

/// Ready check mark on a member's frame (`READY_CHECK_*_TEXTURE_RAID`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadyMark {
    Waiting,
    Ready,
    NotReady,
}

impl GroupState {
    pub fn in_group(&self) -> bool {
        !self.members.is_empty()
    }

    pub fn member(&self, name: &str) -> Option<&GroupMemberSnapshot> {
        self.members
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(name))
    }

    pub fn is_leader(&self, name: &str) -> bool {
        self.member(name).is_some_and(|m| m.is_leader)
    }

    /// Replaces the roster. Live state of members who left or went offline is dropped;
    /// an empty roster (left the group) also ends any ready check.
    pub fn apply_roster(&mut self, roster: GroupRosterSnapshot) {
        self.is_raid = roster.is_raid;
        self.members = roster.members;
        let online: Vec<&str> = self
            .members
            .iter()
            .filter(|m| m.online)
            .map(|m| m.name.as_str())
            .collect();
        self.live.retain(|name, _| online.contains(&name.as_str()));
        if self.members.is_empty() {
            self.ready_check = None;
        }
    }

    /// Stores live states by name. The server sends states only of group members, but
    /// within one tick its per-type message queues may put a new member's state ahead of
    /// the roster that adds them; that state is kept (it is sent again only on change),
    /// and [`Self::apply_roster`] drops states of names that are not online members.
    pub fn apply_member_states(&mut self, states: Vec<GroupMemberState>) {
        for state in states {
            self.live.insert(state.name.clone(), state);
        }
    }

    pub fn apply_ready_check(&mut self, update: ReadyCheckUpdate) {
        let finished_for = update.finished.then_some(0.0);
        self.ready_check = Some(ReadyCheckView {
            update,
            finished_for,
        });
    }

    /// Ages a finished ready check and clears it after [`READY_CHECK_DECAY_SECS`].
    pub fn tick_ready_check(&mut self, dt_secs: f32) {
        let Some(view) = &mut self.ready_check else {
            return;
        };
        if let Some(elapsed) = &mut view.finished_for {
            *elapsed += dt_secs;
            if *elapsed >= READY_CHECK_DECAY_SECS {
                self.ready_check = None;
            }
        }
    }

    /// The member's ready check mark. After the check ends, members who never answered
    /// show as not ready (`CompactUnitFrame_FinishReadyCheck`).
    pub fn ready_mark(&self, name: &str) -> Option<ReadyMark> {
        let view = self.ready_check.as_ref()?;
        let answer = view.update.members.iter().find(|m| m.name == name)?.answer;
        Some(match (answer, view.update.finished) {
            (ReadyCheckAnswer::Ready, _) => ReadyMark::Ready,
            (ReadyCheckAnswer::NotReady, _) | (ReadyCheckAnswer::Pending, true) => {
                ReadyMark::NotReady
            }
            (ReadyCheckAnswer::Pending, false) => ReadyMark::Waiting,
        })
    }

    /// Whether `name` still has to answer the running ready check (the ReadyCheckFrame).
    pub fn awaits_ready_answer(&self, name: &str) -> bool {
        self.ready_check.as_ref().is_some_and(|view| {
            !view.update.finished
                && view
                    .update
                    .members
                    .iter()
                    .any(|m| m.name == name && m.answer == ReadyCheckAnswer::Pending)
        })
    }
}

/// A group request from UI, chat or IPC; networking sends it on `GroupChannel`.
#[derive(Clone, Debug, PartialEq)]
pub enum GroupCommand {
    Invite(String),
    Uninvite(String),
    Promote(String),
    Leave,
    ConvertToRaid,
    ConvertToParty,
    SetSubgroup {
        name: String,
        subgroup: u8,
    },
    SetRole {
        name: String,
        role: GroupRoleSnapshot,
    },
    StartReadyCheck,
    RespondReadyCheck(bool),
    RespondInvite(bool),
}

/// Group entries of a unit's right-click menu (Retail `UnitPopup` SELF/PARTY/PLAYER).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupMenuEntry {
    Invite,
    Promote,
    Uninvite,
    ConvertToRaid,
    ConvertToParty,
    ReadyCheck,
    SetRole(GroupRoleSnapshot),
    Leave,
}

const MENU_ACTION_PREFIX: &str = "group_menu:";

impl GroupMenuEntry {
    /// Retail `GlobalStrings` label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Invite => "Invite",
            Self::Promote => "Promote to Leader",
            Self::Uninvite => "Uninvite",
            Self::ConvertToRaid => "Convert To Raid",
            Self::ConvertToParty => "Convert To Party",
            Self::ReadyCheck => "Ready Check",
            Self::SetRole(GroupRoleSnapshot::Tank) => "Set Role: Tank",
            Self::SetRole(GroupRoleSnapshot::Healer) => "Set Role: Healer",
            Self::SetRole(GroupRoleSnapshot::Damage) => "Set Role: Damage",
            Self::SetRole(GroupRoleSnapshot::None) => "Set Role: None",
            Self::Leave => "Leave Party",
        }
    }

    /// Frame name suffix of the menu entry (`GroupContextMenu<Key>`, `UnitFrameContextMenu<Key>`).
    pub fn frame_key(self) -> &'static str {
        match self {
            Self::Invite => "Invite",
            Self::Promote => "Promote",
            Self::Uninvite => "Uninvite",
            Self::ConvertToRaid => "ConvertToRaid",
            Self::ConvertToParty => "ConvertToParty",
            Self::ReadyCheck => "ReadyCheck",
            Self::SetRole(GroupRoleSnapshot::Tank) => "RoleTank",
            Self::SetRole(GroupRoleSnapshot::Healer) => "RoleHealer",
            Self::SetRole(GroupRoleSnapshot::Damage) => "RoleDamage",
            Self::SetRole(GroupRoleSnapshot::None) => "RoleNone",
            Self::Leave => "Leave",
        }
    }

    pub fn action(self) -> &'static str {
        match self {
            Self::Invite => "group_menu:invite",
            Self::Promote => "group_menu:promote",
            Self::Uninvite => "group_menu:uninvite",
            Self::ConvertToRaid => "group_menu:convert_to_raid",
            Self::ConvertToParty => "group_menu:convert_to_party",
            Self::ReadyCheck => "group_menu:ready_check",
            Self::SetRole(GroupRoleSnapshot::Tank) => "group_menu:role_tank",
            Self::SetRole(GroupRoleSnapshot::Healer) => "group_menu:role_healer",
            Self::SetRole(GroupRoleSnapshot::Damage) => "group_menu:role_damage",
            Self::SetRole(GroupRoleSnapshot::None) => "group_menu:role_none",
            Self::Leave => "group_menu:leave",
        }
    }

    pub fn from_action(action: &str) -> Option<Self> {
        action.strip_prefix(MENU_ACTION_PREFIX)?;
        ALL_ENTRIES
            .into_iter()
            .find(|entry| entry.action() == action)
    }

    /// The command this entry sends for the menu's unit.
    pub fn command(self, unit_name: &str) -> GroupCommand {
        match self {
            Self::Invite => GroupCommand::Invite(unit_name.into()),
            Self::Promote => GroupCommand::Promote(unit_name.into()),
            Self::Uninvite => GroupCommand::Uninvite(unit_name.into()),
            Self::ConvertToRaid => GroupCommand::ConvertToRaid,
            Self::ConvertToParty => GroupCommand::ConvertToParty,
            Self::ReadyCheck => GroupCommand::StartReadyCheck,
            Self::SetRole(role) => GroupCommand::SetRole {
                name: unit_name.into(),
                role,
            },
            Self::Leave => GroupCommand::Leave,
        }
    }
}

const ALL_ENTRIES: [GroupMenuEntry; 11] = [
    GroupMenuEntry::Invite,
    GroupMenuEntry::Promote,
    GroupMenuEntry::Uninvite,
    GroupMenuEntry::ConvertToRaid,
    GroupMenuEntry::ConvertToParty,
    GroupMenuEntry::ReadyCheck,
    GroupMenuEntry::SetRole(GroupRoleSnapshot::Tank),
    GroupMenuEntry::SetRole(GroupRoleSnapshot::Healer),
    GroupMenuEntry::SetRole(GroupRoleSnapshot::Damage),
    GroupMenuEntry::SetRole(GroupRoleSnapshot::None),
    GroupMenuEntry::Leave,
];

const ROLE_ENTRIES: [GroupMenuEntry; 4] = [
    GroupMenuEntry::SetRole(GroupRoleSnapshot::Tank),
    GroupMenuEntry::SetRole(GroupRoleSnapshot::Healer),
    GroupMenuEntry::SetRole(GroupRoleSnapshot::Damage),
    GroupMenuEntry::SetRole(GroupRoleSnapshot::None),
];

/// Group entries for a right-click on the player `unit_name`, seen by `local_name`.
/// Self: roles, leader conversions and ready check, then Leave. Another player: Invite
/// when they can be invited, Promote and Uninvite for the leader's members.
pub fn group_menu_entries(
    state: &GroupState,
    local_name: &str,
    unit_name: &str,
) -> Vec<GroupMenuEntry> {
    let leader = state.is_leader(local_name);
    if unit_name.eq_ignore_ascii_case(local_name) {
        return self_menu_entries(state, leader);
    }
    if state.member(unit_name).is_some() {
        let mut entries = Vec::new();
        if leader {
            entries.extend([GroupMenuEntry::Promote, GroupMenuEntry::Uninvite]);
            entries.extend(ROLE_ENTRIES);
        }
        return entries;
    }
    if !state.in_group() || leader {
        vec![GroupMenuEntry::Invite]
    } else {
        Vec::new()
    }
}

fn self_menu_entries(state: &GroupState, leader: bool) -> Vec<GroupMenuEntry> {
    if !state.in_group() {
        return Vec::new();
    }
    let mut entries = ROLE_ENTRIES.to_vec();
    if leader {
        if !state.is_raid {
            entries.push(GroupMenuEntry::ConvertToRaid);
        } else if state.members.len() <= MAX_PARTY_SIZE {
            entries.push(GroupMenuEntry::ConvertToParty);
        }
        entries.push(GroupMenuEntry::ReadyCheck);
    }
    entries.push(GroupMenuEntry::Leave);
    entries
}

#[cfg(test)]
#[path = "group_state_tests.rs"]
mod tests;
