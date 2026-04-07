use bevy::prelude::*;

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

/// Runtime help system state.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct HelpState {
    pub selected_article: Option<u32>,
    pub ticket_category_index: usize,
    pub ticket_description: String,
}

impl HelpState {
    pub fn selected_ticket_category(&self) -> &'static str {
        TICKET_CATEGORIES
            .get(self.ticket_category_index)
            .unwrap_or(&"Other")
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
}
