use bevy::prelude::*;

/// Texture FDIDs for the guild control frame.
pub mod textures {
    /// Checkbox check mark.
    pub const CHECKBOX_CHECK: u32 = 130751;
    /// Checkbox pressed/depressed state.
    pub const CHECKBOX_DOWN: u32 = 130752;
    /// Checkbox highlight on hover.
    pub const CHECKBOX_HIGHLIGHT: u32 = 130753;
    /// Checkbox normal (unchecked) state.
    pub const CHECKBOX_UP: u32 = 130755;
    /// Permission tab frame chrome.
    pub const PERMISSION_TAB: u32 = 132075;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GuildPermFlag {
    InviteMembers,
    RemoveMembers,
    PromoteMembers,
    DemoteMembers,
    SetMotd,
    EditPublicNote,
    EditOfficerNote,
    ModifyGuildInfo,
}

impl GuildPermFlag {
    pub fn label(self) -> &'static str {
        match self {
            Self::InviteMembers => "Invite Members",
            Self::RemoveMembers => "Remove Members",
            Self::PromoteMembers => "Promote Members",
            Self::DemoteMembers => "Demote Members",
            Self::SetMotd => "Set MOTD",
            Self::EditPublicNote => "Edit Public Note",
            Self::EditOfficerNote => "Edit Officer Note",
            Self::ModifyGuildInfo => "Modify Guild Info",
        }
    }

    pub const ALL: [GuildPermFlag; 8] = [
        Self::InviteMembers,
        Self::RemoveMembers,
        Self::PromoteMembers,
        Self::DemoteMembers,
        Self::SetMotd,
        Self::EditPublicNote,
        Self::EditOfficerNote,
        Self::ModifyGuildInfo,
    ];
}

#[derive(Clone, Debug, PartialEq)]
pub struct BankTabSettings {
    pub can_view: bool,
    pub can_deposit: bool,
    pub can_withdraw: bool,
    pub withdraw_limit: u32,
}

impl Default for BankTabSettings {
    fn default() -> Self {
        Self {
            can_view: true,
            can_deposit: false,
            can_withdraw: false,
            withdraw_limit: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GuildRankDef {
    pub name: String,
    pub index: usize,
    pub permissions: Vec<GuildPermFlag>,
    pub bank_tabs: Vec<BankTabSettings>,
}

/// Runtime guild control state.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct GuildControlDataState {
    pub ranks: Vec<GuildRankDef>,
    pub selected_rank: usize,
    pub bank_tab_count: usize,
}

impl GuildControlDataState {
    pub fn selected_rank_def(&self) -> Option<&GuildRankDef> {
        self.ranks.get(self.selected_rank)
    }

    pub fn rank_has_perm(&self, rank_index: usize, perm: GuildPermFlag) -> bool {
        self.ranks
            .get(rank_index)
            .is_some_and(|r| r.permissions.contains(&perm))
    }

    pub fn toggle_perm(&mut self, rank_index: usize, perm: GuildPermFlag) {
        let Some(rank) = self.ranks.get_mut(rank_index) else {
            return;
        };
        if let Some(pos) = rank.permissions.iter().position(|&p| p == perm) {
            rank.permissions.remove(pos);
        } else {
            rank.permissions.push(perm);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guild_master() -> GuildRankDef {
        GuildRankDef {
            name: "Guild Master".into(),
            index: 0,
            permissions: GuildPermFlag::ALL.to_vec(),
            bank_tabs: vec![BankTabSettings {
                can_view: true,
                can_deposit: true,
                can_withdraw: true,
                withdraw_limit: 100,
            }],
        }
    }

    fn member() -> GuildRankDef {
        GuildRankDef {
            name: "Member".into(),
            index: 1,
            permissions: vec![GuildPermFlag::EditPublicNote],
            bank_tabs: vec![BankTabSettings::default()],
        }
    }

    #[test]
    fn perm_flag_labels_all_unique() {
        let labels: Vec<&str> = GuildPermFlag::ALL.iter().map(|p| p.label()).collect();
        for (i, a) in labels.iter().enumerate() {
            for (j, b) in labels.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn selected_rank_def_resolves() {
        let state = GuildControlDataState {
            ranks: vec![guild_master(), member()],
            selected_rank: 1,
            bank_tab_count: 1,
        };
        assert_eq!(state.selected_rank_def().unwrap().name, "Member");
    }

    #[test]
    fn rank_has_perm_checks() {
        let state = GuildControlDataState {
            ranks: vec![guild_master(), member()],
            selected_rank: 0,
            bank_tab_count: 1,
        };
        assert!(state.rank_has_perm(0, GuildPermFlag::InviteMembers));
        assert!(!state.rank_has_perm(1, GuildPermFlag::InviteMembers));
        assert!(state.rank_has_perm(1, GuildPermFlag::EditPublicNote));
    }

    #[test]
    fn toggle_perm_adds_and_removes() {
        let mut state = GuildControlDataState {
            ranks: vec![member()],
            selected_rank: 0,
            bank_tab_count: 1,
        };
        assert!(!state.rank_has_perm(0, GuildPermFlag::InviteMembers));
        state.toggle_perm(0, GuildPermFlag::InviteMembers);
        assert!(state.rank_has_perm(0, GuildPermFlag::InviteMembers));
        state.toggle_perm(0, GuildPermFlag::InviteMembers);
        assert!(!state.rank_has_perm(0, GuildPermFlag::InviteMembers));
    }

    #[test]
    fn bank_tab_settings_default() {
        let s = BankTabSettings::default();
        assert!(s.can_view);
        assert!(!s.can_deposit);
        assert!(!s.can_withdraw);
        assert_eq!(s.withdraw_limit, 0);
    }

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::CHECKBOX_CHECK, 0);
        assert_ne!(textures::CHECKBOX_UP, 0);
        assert_ne!(textures::CHECKBOX_DOWN, 0);
        assert_ne!(textures::PERMISSION_TAB, 0);
    }
}
