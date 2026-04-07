use bevy::prelude::*;

/// Texture FDIDs for the help frame.
pub mod textures {
    /// Frame chrome top-left corner.
    pub const FRAME_TOP_LEFT: u32 = 132081;
    /// Frame chrome bottom-left corner.
    pub const FRAME_BOTTOM_LEFT: u32 = 132077;
    /// Frame chrome top border.
    pub const FRAME_TOP: u32 = 132080;
    /// Knowledge Base category icon.
    pub const ICON_KNOWLEDGE_BASE: u32 = 516770;
    /// Submit Ticket category icon.
    pub const ICON_OPEN_TICKET: u32 = 516771;
    /// Report Abuse / Bug Report category icon.
    pub const ICON_REPORT_ABUSE: u32 = 516772;
}

#[derive(Clone, Debug, PartialEq)]
pub struct HelpArticle {
    pub id: u32,
    pub title: &'static str,
    pub body: &'static str,
    pub category: &'static str,
}

pub static ARTICLES: &[HelpArticle] = &[
    HelpArticle {
        id: 1,
        title: "Getting Started",
        body: "Welcome! Use WASD to move and mouse to look around. Press M to open the map.",
        category: "Basics",
    },
    HelpArticle {
        id: 2,
        title: "Combat Basics",
        body: "Target an enemy by clicking on it. Use abilities on your action bar (keys 1-9) to attack.",
        category: "Basics",
    },
    HelpArticle {
        id: 3,
        title: "Grouping Up",
        body: "Invite players to your party by right-clicking their portrait and selecting Invite. Dungeons require a group of 5.",
        category: "Social",
    },
    HelpArticle {
        id: 4,
        title: "Professions",
        body: "Visit a profession trainer in any city to learn a trade skill. You can have two primary professions.",
        category: "Gameplay",
    },
    HelpArticle {
        id: 5,
        title: "Auction House",
        body: "Buy and sell items at the Auction House found in major cities. Use the Browse tab to search for items.",
        category: "Economy",
    },
    HelpArticle {
        id: 6,
        title: "Keybindings",
        body: "Open Key Bindings from the Game Menu (Escape) to customize your controls.",
        category: "Settings",
    },
];

pub static TICKET_CATEGORIES: &[&str] = &[
    "Stuck Character",
    "Bug Report",
    "Harassment",
    "Billing Issue",
    "Item Restoration",
    "Other",
];

pub fn articles_by_category(category: &str) -> Vec<&'static HelpArticle> {
    ARTICLES.iter().filter(|a| a.category == category).collect()
}

pub fn find_article(id: u32) -> Option<&'static HelpArticle> {
    ARTICLES.iter().find(|a| a.id == id)
}

/// Ticket lifecycle states.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TicketStatus {
    #[default]
    Draft,
    Submitted,
    InProgress,
    Resolved,
}

impl TicketStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Draft => "Draft",
            Self::Submitted => "Submitted",
            Self::InProgress => "In Progress",
            Self::Resolved => "Resolved",
        }
    }

    pub fn can_edit(self) -> bool {
        matches!(self, Self::Draft)
    }

    pub fn can_cancel(self) -> bool {
        matches!(self, Self::Draft | Self::Submitted)
    }
}

/// All unique categories present in the article database.
pub fn article_categories() -> Vec<&'static str> {
    let mut cats: Vec<&str> = ARTICLES.iter().map(|a| a.category).collect();
    cats.sort();
    cats.dedup();
    cats
}

/// Search articles by title (case-insensitive substring).
pub fn search_articles(query: &str) -> Vec<&'static HelpArticle> {
    let q = query.to_lowercase();
    ARTICLES
        .iter()
        .filter(|a| a.title.to_lowercase().contains(&q))
        .collect()
}

/// Runtime help system state.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct HelpState {
    pub selected_article: Option<u32>,
    pub ticket_category_index: usize,
    pub ticket_description: String,
    pub ticket_status: TicketStatus,
}

impl HelpState {
    pub fn selected_ticket_category(&self) -> &'static str {
        TICKET_CATEGORIES
            .get(self.ticket_category_index)
            .unwrap_or(&"Other")
    }

    /// Submit the ticket if it has a description and is still a draft.
    pub fn submit_ticket(&mut self) -> bool {
        if self.ticket_status != TicketStatus::Draft || self.ticket_description.is_empty() {
            return false;
        }
        self.ticket_status = TicketStatus::Submitted;
        true
    }

    /// Cancel the ticket (only if draft or submitted).
    pub fn cancel_ticket(&mut self) -> bool {
        if !self.ticket_status.can_cancel() {
            return false;
        }
        self.ticket_status = TicketStatus::Draft;
        self.ticket_description.clear();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn articles_exist() {
        assert!(!ARTICLES.is_empty());
        assert!(ARTICLES.len() >= 6);
    }

    #[test]
    fn articles_by_category_filters() {
        let basics = articles_by_category("Basics");
        assert_eq!(basics.len(), 2);
        for a in &basics {
            assert_eq!(a.category, "Basics");
        }
    }

    #[test]
    fn find_article_by_id() {
        let a = find_article(1).unwrap();
        assert_eq!(a.title, "Getting Started");
        assert!(find_article(999).is_none());
    }

    #[test]
    fn ticket_categories_nonempty() {
        assert!(TICKET_CATEGORIES.len() >= 4);
    }

    #[test]
    fn selected_ticket_category_default() {
        let state = HelpState::default();
        assert_eq!(state.selected_ticket_category(), "Stuck Character");
    }

    #[test]
    fn selected_ticket_category_out_of_bounds() {
        let state = HelpState {
            ticket_category_index: 999,
            ..Default::default()
        };
        assert_eq!(state.selected_ticket_category(), "Other");
    }

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::FRAME_TOP_LEFT, 0);
        assert_ne!(textures::FRAME_TOP, 0);
        assert_ne!(textures::ICON_KNOWLEDGE_BASE, 0);
        assert_ne!(textures::ICON_OPEN_TICKET, 0);
        assert_ne!(textures::ICON_REPORT_ABUSE, 0);
    }

    // --- Article category filtering ---

    #[test]
    fn article_categories_lists_all() {
        let cats = article_categories();
        assert!(cats.contains(&"Basics"));
        assert!(cats.contains(&"Social"));
        assert!(cats.contains(&"Gameplay"));
        assert!(cats.contains(&"Economy"));
        assert!(cats.contains(&"Settings"));
    }

    #[test]
    fn article_categories_no_duplicates() {
        let cats = article_categories();
        let count = cats.len();
        let mut deduped = cats.clone();
        deduped.dedup();
        assert_eq!(count, deduped.len());
    }

    #[test]
    fn articles_by_nonexistent_category() {
        assert!(articles_by_category("PvP").is_empty());
    }

    #[test]
    fn search_articles_finds_match() {
        let results = search_articles("combat");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Combat Basics");
    }

    #[test]
    fn search_articles_case_insensitive() {
        let results = search_articles("GETTING");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_articles_no_match() {
        assert!(search_articles("zzzzz").is_empty());
    }

    // --- Ticket state machine ---

    #[test]
    fn ticket_status_labels() {
        assert_eq!(TicketStatus::Draft.label(), "Draft");
        assert_eq!(TicketStatus::Submitted.label(), "Submitted");
        assert_eq!(TicketStatus::InProgress.label(), "In Progress");
        assert_eq!(TicketStatus::Resolved.label(), "Resolved");
    }

    #[test]
    fn ticket_submit_succeeds_with_description() {
        let mut state = HelpState {
            ticket_description: "I'm stuck".into(),
            ..Default::default()
        };
        assert!(state.submit_ticket());
        assert_eq!(state.ticket_status, TicketStatus::Submitted);
    }

    #[test]
    fn ticket_submit_fails_empty_description() {
        let mut state = HelpState::default();
        assert!(!state.submit_ticket());
        assert_eq!(state.ticket_status, TicketStatus::Draft);
    }

    #[test]
    fn ticket_submit_fails_if_already_submitted() {
        let mut state = HelpState {
            ticket_description: "bug".into(),
            ticket_status: TicketStatus::Submitted,
            ..Default::default()
        };
        assert!(!state.submit_ticket());
    }

    #[test]
    fn ticket_cancel_from_draft() {
        let mut state = HelpState {
            ticket_description: "draft text".into(),
            ..Default::default()
        };
        assert!(state.cancel_ticket());
        assert_eq!(state.ticket_status, TicketStatus::Draft);
        assert!(state.ticket_description.is_empty());
    }

    #[test]
    fn ticket_cancel_from_submitted() {
        let mut state = HelpState {
            ticket_description: "submitted".into(),
            ticket_status: TicketStatus::Submitted,
            ..Default::default()
        };
        assert!(state.cancel_ticket());
        assert_eq!(state.ticket_status, TicketStatus::Draft);
    }

    #[test]
    fn ticket_cancel_fails_in_progress() {
        let mut state = HelpState {
            ticket_status: TicketStatus::InProgress,
            ..Default::default()
        };
        assert!(!state.cancel_ticket());
        assert_eq!(state.ticket_status, TicketStatus::InProgress);
    }

    #[test]
    fn ticket_cancel_fails_resolved() {
        let mut state = HelpState {
            ticket_status: TicketStatus::Resolved,
            ..Default::default()
        };
        assert!(!state.cancel_ticket());
    }

    #[test]
    fn ticket_can_edit_only_in_draft() {
        assert!(TicketStatus::Draft.can_edit());
        assert!(!TicketStatus::Submitted.can_edit());
        assert!(!TicketStatus::InProgress.can_edit());
        assert!(!TicketStatus::Resolved.can_edit());
    }
}
