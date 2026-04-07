use bevy::prelude::*;

/// Texture FDIDs for the LFG frame.
pub mod textures {
    /// LFG frame chrome.
    pub const FRAME: u32 = 337495;
    /// Role icons (tank/healer/dps combined, color).
    pub const ROLE_ICONS: u32 = 252190;
    /// Role icon backgrounds.
    pub const ROLE_BACKGROUNDS: u32 = 340817;
    /// Heroic difficulty icon.
    pub const ICON_HEROIC: u32 = 337496;
    /// Deadmines activity background.
    pub const BG_DEADMINES: u32 = 337488;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LFGRole {
    Tank,
    Healer,
    DPS,
}

impl LFGRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tank => "Tank",
            Self::Healer => "Healer",
            Self::DPS => "DPS",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActivityCategory {
    pub id: u32,
    pub name: &'static str,
}

pub static ACTIVITY_CATEGORIES: &[ActivityCategory] = &[
    ActivityCategory {
        id: 1,
        name: "Dungeons",
    },
    ActivityCategory {
        id: 2,
        name: "Raids",
    },
    ActivityCategory {
        id: 3,
        name: "Arenas",
    },
    ActivityCategory {
        id: 4,
        name: "Battlegrounds",
    },
    ActivityCategory {
        id: 5,
        name: "Questing",
    },
    ActivityCategory {
        id: 6,
        name: "Custom",
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationStatus {
    None,
    Applied,
    Invited,
    Declined,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GroupListing {
    pub id: u64,
    pub leader: String,
    pub member_count: u8,
    pub max_members: u8,
    pub activity: String,
    pub note: String,
    pub min_item_level: u32,
    pub voice_chat: bool,
}

impl GroupListing {
    pub fn members_display(&self) -> String {
        format!("{}/{}", self.member_count, self.max_members)
    }

    pub fn is_full(&self) -> bool {
        self.member_count >= self.max_members
    }
}

/// Runtime LFG state.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct LFGState {
    pub selected_roles: Vec<LFGRole>,
    pub selected_activity: Option<u32>,
    pub listings: Vec<GroupListing>,
    pub application_status: ApplicationStatus,
    pub applied_group_id: Option<u64>,
}

impl Default for ApplicationStatus {
    fn default() -> Self {
        Self::None
    }
}

impl LFGState {
    pub fn has_role(&self, role: LFGRole) -> bool {
        self.selected_roles.contains(&role)
    }

    pub fn toggle_role(&mut self, role: LFGRole) {
        if let Some(pos) = self.selected_roles.iter().position(|&r| r == role) {
            self.selected_roles.remove(pos);
        } else {
            self.selected_roles.push(role);
        }
    }

    pub fn selected_activity_name(&self) -> &str {
        self.selected_activity
            .and_then(|id| ACTIVITY_CATEGORIES.iter().find(|c| c.id == id))
            .map(|c| c.name)
            .unwrap_or("All Activities")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_labels() {
        assert_eq!(LFGRole::Tank.label(), "Tank");
        assert_eq!(LFGRole::Healer.label(), "Healer");
        assert_eq!(LFGRole::DPS.label(), "DPS");
    }

    #[test]
    fn toggle_role() {
        let mut state = LFGState::default();
        assert!(!state.has_role(LFGRole::Tank));
        state.toggle_role(LFGRole::Tank);
        assert!(state.has_role(LFGRole::Tank));
        state.toggle_role(LFGRole::Tank);
        assert!(!state.has_role(LFGRole::Tank));
    }

    #[test]
    fn group_listing_display() {
        let g = GroupListing {
            id: 1,
            leader: "Arthas".into(),
            member_count: 3,
            max_members: 5,
            activity: "Deadmines".into(),
            note: "Need healer".into(),
            min_item_level: 0,
            voice_chat: false,
        };
        assert_eq!(g.members_display(), "3/5");
        assert!(!g.is_full());
    }

    #[test]
    fn full_group() {
        let g = GroupListing {
            id: 2,
            leader: "X".into(),
            member_count: 5,
            max_members: 5,
            activity: "Raid".into(),
            note: String::new(),
            min_item_level: 0,
            voice_chat: false,
        };
        assert!(g.is_full());
    }

    #[test]
    fn activity_categories_exist() {
        assert!(ACTIVITY_CATEGORIES.len() >= 4);
    }

    #[test]
    fn selected_activity_name() {
        let mut state = LFGState::default();
        assert_eq!(state.selected_activity_name(), "All Activities");
        state.selected_activity = Some(1);
        assert_eq!(state.selected_activity_name(), "Dungeons");
    }

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::FRAME, 0);
        assert_ne!(textures::ROLE_ICONS, 0);
        assert_ne!(textures::ICON_HEROIC, 0);
        assert_ne!(textures::BG_DEADMINES, 0);
    }
}
