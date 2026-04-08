use bevy::prelude::*;

// --- Gossip icon ---

/// Icon type for a gossip menu option, matching WoW gossip icon IDs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GossipIcon {
    /// Generic gossip/chat text.
    #[default]
    Chat,
    /// Opens merchant window.
    Vendor,
    /// Flight master / taxi.
    Taxi,
    /// Class or profession trainer.
    Trainer,
    /// Innkeeper (set hearthstone).
    Binder,
    /// Opens bank window.
    Banker,
    /// Guild or arena petition.
    Petition,
    /// Guild tabard vendor.
    Tabard,
    /// Battlemaster.
    Battle,
    /// Generic object interaction.
    Interact,
}

impl GossipIcon {
    /// Human-readable label for UI display.
    pub fn label(self) -> &'static str {
        match self {
            Self::Chat => "Chat",
            Self::Vendor => "Vendor",
            Self::Taxi => "Flight Master",
            Self::Trainer => "Trainer",
            Self::Binder => "Innkeeper",
            Self::Banker => "Banker",
            Self::Petition => "Petition",
            Self::Tabard => "Tabard",
            Self::Battle => "Battlemaster",
            Self::Interact => "Interact",
        }
    }
}

// --- Gossip option ---

/// A single selectable option in a gossip menu.
#[derive(Clone, Debug, PartialEq)]
pub struct GossipOption {
    pub option_id: u32,
    pub text: String,
    pub icon: GossipIcon,
}

// --- Quest entries in gossip ---

/// A quest entry shown alongside gossip options.
#[derive(Clone, Debug, PartialEq)]
pub struct GossipQuestEntry {
    pub quest_id: u32,
    pub title: String,
    pub level: u32,
    /// True when all objectives are complete and quest is ready for turn-in.
    pub is_complete: bool,
    /// True for daily/repeatable quests (shown with blue icon).
    pub is_daily: bool,
}

impl GossipQuestEntry {
    /// Display color for the quest title (yellow for available, gray for turn-in ready).
    pub fn title_color(&self) -> [f32; 4] {
        if self.is_daily {
            [0.3, 0.5, 1.0, 1.0] // blue
        } else if self.is_complete {
            [1.0, 1.0, 1.0, 1.0] // white — ready for turn-in
        } else {
            [1.0, 0.82, 0.0, 1.0] // yellow — available
        }
    }
}

// --- Gossip menu ---

/// A gossip menu sent by the server when the player interacts with an NPC.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct GossipMenu {
    pub npc_name: String,
    pub npc_portrait_fdid: u32,
    pub greeting_text: String,
    pub options: Vec<GossipOption>,
    pub available_quests: Vec<GossipQuestEntry>,
}

impl GossipMenu {
    /// True when the menu has no options and no quests (auto-close candidate).
    pub fn is_empty(&self) -> bool {
        self.options.is_empty() && self.available_quests.is_empty()
    }

    /// Total number of selectable items (options + quests).
    pub fn item_count(&self) -> usize {
        self.options.len() + self.available_quests.len()
    }
}

// --- Runtime gossip state ---

/// Runtime gossip interaction state, held as a Bevy Resource.
///
/// Supports multi-level dialog trees: [`navigate_to`](Self::navigate_to)
/// pushes the current menu onto a history stack so the player can
/// [`go_back`](Self::go_back) to the previous page.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct GossipState {
    /// The currently open gossip menu, if any.
    pub menu: Option<GossipMenu>,
    /// The server-side entity ID of the NPC being interacted with.
    pub interacting_npc: Option<u64>,
    /// History stack for dialog tree navigation (most recent on top).
    history: Vec<GossipMenu>,
}

impl GossipState {
    pub fn is_open(&self) -> bool {
        self.menu.is_some()
    }

    /// Start a new gossip interaction, clearing any previous history.
    pub fn open(&mut self, npc_entity_id: u64, menu: GossipMenu) {
        self.interacting_npc = Some(npc_entity_id);
        self.menu = Some(menu);
        self.history.clear();
    }

    /// Close the gossip dialog and clear all navigation history.
    pub fn close(&mut self) {
        self.menu = None;
        self.interacting_npc = None;
        self.history.clear();
    }

    /// Navigate to a sub-menu, pushing the current menu onto the history stack.
    /// No-op if no menu is currently open.
    pub fn navigate_to(&mut self, menu: GossipMenu) {
        if let Some(current) = self.menu.take() {
            self.history.push(current);
        }
        self.menu = Some(menu);
    }

    /// Go back to the previous menu in the dialog tree.
    /// Returns true if navigation occurred, false if already at root.
    pub fn go_back(&mut self) -> bool {
        let Some(previous) = self.history.pop() else {
            return false;
        };
        self.menu = Some(previous);
        true
    }

    /// Whether there is a previous menu to go back to.
    pub fn can_go_back(&self) -> bool {
        !self.history.is_empty()
    }

    /// How many levels deep in the dialog tree (0 = root menu).
    pub fn depth(&self) -> usize {
        self.history.len()
    }

    pub fn greeting(&self) -> &str {
        self.menu.as_ref().map_or("", |m| &m.greeting_text)
    }

    pub fn options(&self) -> &[GossipOption] {
        self.menu.as_ref().map_or(&[], |m| &m.options)
    }

    pub fn available_quests(&self) -> &[GossipQuestEntry] {
        self.menu.as_ref().map_or(&[], |m| &m.available_quests)
    }
}

// --- Client → server intents ---

/// A pending gossip action to send to the server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GossipIntent {
    /// Player wants to interact with an NPC (open gossip menu).
    Interact { npc_entity_id: u64 },
    /// Player selected a gossip menu option.
    SelectOption { option_id: u32 },
    /// Player navigated back to the previous dialog page.
    GoBack,
    /// Player closed the gossip dialog.
    Close,
}

/// Queue of gossip intents waiting to be sent to the server.
#[derive(Resource, Default)]
pub struct GossipIntentQueue {
    pub pending: Vec<GossipIntent>,
}

impl GossipIntentQueue {
    pub fn interact(&mut self, npc_entity_id: u64) {
        self.pending.push(GossipIntent::Interact { npc_entity_id });
    }

    pub fn select_option(&mut self, option_id: u32) {
        self.pending.push(GossipIntent::SelectOption { option_id });
    }

    pub fn go_back(&mut self) {
        self.pending.push(GossipIntent::GoBack);
    }

    pub fn close(&mut self) {
        self.pending.push(GossipIntent::Close);
    }

    pub fn drain(&mut self) -> Vec<GossipIntent> {
        std::mem::take(&mut self.pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- GossipIcon ---

    #[test]
    fn gossip_icon_labels_nonempty() {
        let icons = [
            GossipIcon::Chat,
            GossipIcon::Vendor,
            GossipIcon::Taxi,
            GossipIcon::Trainer,
            GossipIcon::Binder,
            GossipIcon::Banker,
            GossipIcon::Petition,
            GossipIcon::Tabard,
            GossipIcon::Battle,
            GossipIcon::Interact,
        ];
        for icon in icons {
            assert!(!icon.label().is_empty(), "{icon:?} has empty label");
        }
    }

    #[test]
    fn gossip_icon_default_is_chat() {
        assert_eq!(GossipIcon::default(), GossipIcon::Chat);
    }

    // --- GossipQuestEntry ---

    #[test]
    fn quest_entry_available_is_yellow() {
        let entry = GossipQuestEntry {
            quest_id: 1,
            title: "Test".into(),
            level: 10,
            is_complete: false,
            is_daily: false,
        };
        let color = entry.title_color();
        assert!((color[0] - 1.0).abs() < 0.01);
        assert!((color[1] - 0.82).abs() < 0.01);
    }

    #[test]
    fn quest_entry_complete_is_white() {
        let entry = GossipQuestEntry {
            quest_id: 1,
            title: "Test".into(),
            level: 10,
            is_complete: true,
            is_daily: false,
        };
        let color = entry.title_color();
        assert!((color[0] - 1.0).abs() < 0.01);
        assert!((color[1] - 1.0).abs() < 0.01);
        assert!((color[2] - 1.0).abs() < 0.01);
    }

    #[test]
    fn quest_entry_daily_is_blue() {
        let entry = GossipQuestEntry {
            quest_id: 1,
            title: "Test".into(),
            level: 10,
            is_complete: false,
            is_daily: true,
        };
        let color = entry.title_color();
        assert!(color[2] > 0.8); // blue channel dominant
    }

    // --- GossipMenu ---

    #[test]
    fn empty_menu() {
        let menu = GossipMenu::default();
        assert!(menu.is_empty());
        assert_eq!(menu.item_count(), 0);
    }

    #[test]
    fn menu_with_options() {
        let menu = GossipMenu {
            options: vec![GossipOption {
                option_id: 1,
                text: "Hello".into(),
                icon: GossipIcon::Chat,
            }],
            ..Default::default()
        };
        assert!(!menu.is_empty());
        assert_eq!(menu.item_count(), 1);
    }

    #[test]
    fn menu_with_quests_only() {
        let menu = GossipMenu {
            available_quests: vec![GossipQuestEntry {
                quest_id: 100,
                title: "Wolves".into(),
                level: 5,
                is_complete: false,
                is_daily: false,
            }],
            ..Default::default()
        };
        assert!(!menu.is_empty());
        assert_eq!(menu.item_count(), 1);
    }

    #[test]
    fn menu_item_count_sums_both() {
        let menu = GossipMenu {
            options: vec![
                GossipOption {
                    option_id: 1,
                    text: "Browse goods".into(),
                    icon: GossipIcon::Vendor,
                },
                GossipOption {
                    option_id: 2,
                    text: "Train me".into(),
                    icon: GossipIcon::Trainer,
                },
            ],
            available_quests: vec![GossipQuestEntry {
                quest_id: 100,
                title: "Wolves".into(),
                level: 5,
                is_complete: false,
                is_daily: false,
            }],
            ..Default::default()
        };
        assert_eq!(menu.item_count(), 3);
    }

    // --- GossipState ---

    #[test]
    fn state_starts_closed() {
        let state = GossipState::default();
        assert!(!state.is_open());
        assert!(state.interacting_npc.is_none());
        assert_eq!(state.greeting(), "");
        assert!(state.options().is_empty());
        assert!(state.available_quests().is_empty());
    }

    #[test]
    fn state_open_and_close() {
        let mut state = GossipState::default();
        let menu = GossipMenu {
            npc_name: "Innkeeper".into(),
            greeting_text: "Welcome!".into(),
            options: vec![GossipOption {
                option_id: 1,
                text: "Make this inn your home.".into(),
                icon: GossipIcon::Binder,
            }],
            ..Default::default()
        };
        state.open(42, menu);
        assert!(state.is_open());
        assert_eq!(state.interacting_npc, Some(42));
        assert_eq!(state.greeting(), "Welcome!");
        assert_eq!(state.options().len(), 1);

        state.close();
        assert!(!state.is_open());
        assert!(state.interacting_npc.is_none());
    }

    #[test]
    fn state_open_replaces_previous() {
        let mut state = GossipState::default();
        state.open(
            1,
            GossipMenu {
                greeting_text: "First".into(),
                ..Default::default()
            },
        );
        state.open(
            2,
            GossipMenu {
                greeting_text: "Second".into(),
                ..Default::default()
            },
        );
        assert_eq!(state.interacting_npc, Some(2));
        assert_eq!(state.greeting(), "Second");
    }

    // --- Dialog tree navigation ---

    #[test]
    fn navigate_to_sub_menu() {
        let mut state = GossipState::default();
        state.open(
            1,
            GossipMenu {
                greeting_text: "Root".into(),
                ..Default::default()
            },
        );
        state.navigate_to(GossipMenu {
            greeting_text: "Page 2".into(),
            ..Default::default()
        });
        assert_eq!(state.greeting(), "Page 2");
        assert!(state.can_go_back());
        assert_eq!(state.depth(), 1);
    }

    #[test]
    fn go_back_restores_previous() {
        let mut state = GossipState::default();
        state.open(
            1,
            GossipMenu {
                greeting_text: "Root".into(),
                ..Default::default()
            },
        );
        state.navigate_to(GossipMenu {
            greeting_text: "Sub".into(),
            ..Default::default()
        });
        assert!(state.go_back());
        assert_eq!(state.greeting(), "Root");
        assert!(!state.can_go_back());
        assert_eq!(state.depth(), 0);
    }

    #[test]
    fn go_back_at_root_returns_false() {
        let mut state = GossipState::default();
        state.open(
            1,
            GossipMenu {
                greeting_text: "Root".into(),
                ..Default::default()
            },
        );
        assert!(!state.go_back());
        assert_eq!(state.greeting(), "Root");
    }

    #[test]
    fn multi_level_navigation() {
        let mut state = GossipState::default();
        state.open(
            1,
            GossipMenu {
                greeting_text: "Level 0".into(),
                ..Default::default()
            },
        );
        state.navigate_to(GossipMenu {
            greeting_text: "Level 1".into(),
            ..Default::default()
        });
        state.navigate_to(GossipMenu {
            greeting_text: "Level 2".into(),
            ..Default::default()
        });
        assert_eq!(state.depth(), 2);
        assert_eq!(state.greeting(), "Level 2");

        assert!(state.go_back());
        assert_eq!(state.greeting(), "Level 1");
        assert_eq!(state.depth(), 1);

        assert!(state.go_back());
        assert_eq!(state.greeting(), "Level 0");
        assert_eq!(state.depth(), 0);

        assert!(!state.go_back());
    }

    #[test]
    fn open_clears_history() {
        let mut state = GossipState::default();
        state.open(
            1,
            GossipMenu {
                greeting_text: "A".into(),
                ..Default::default()
            },
        );
        state.navigate_to(GossipMenu {
            greeting_text: "B".into(),
            ..Default::default()
        });
        assert_eq!(state.depth(), 1);

        // Opening a new interaction clears the history.
        state.open(
            2,
            GossipMenu {
                greeting_text: "Fresh".into(),
                ..Default::default()
            },
        );
        assert_eq!(state.depth(), 0);
        assert!(!state.can_go_back());
    }

    #[test]
    fn close_clears_history() {
        let mut state = GossipState::default();
        state.open(
            1,
            GossipMenu {
                greeting_text: "A".into(),
                ..Default::default()
            },
        );
        state.navigate_to(GossipMenu {
            greeting_text: "B".into(),
            ..Default::default()
        });
        state.close();
        assert_eq!(state.depth(), 0);
        assert!(!state.is_open());
    }

    #[test]
    fn navigate_without_open_sets_menu() {
        let mut state = GossipState::default();
        state.navigate_to(GossipMenu {
            greeting_text: "Direct".into(),
            ..Default::default()
        });
        assert!(state.is_open());
        assert_eq!(state.greeting(), "Direct");
        // No previous menu was open, so nothing pushed to history.
        assert_eq!(state.depth(), 0);
    }

    // --- GossipIntentQueue ---

    #[test]
    fn intent_queue_interact() {
        let mut queue = GossipIntentQueue::default();
        queue.interact(100);
        let drained = queue.drain();
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0], GossipIntent::Interact { npc_entity_id: 100 });
    }

    #[test]
    fn intent_queue_select_option() {
        let mut queue = GossipIntentQueue::default();
        queue.select_option(5);
        let drained = queue.drain();
        assert_eq!(drained[0], GossipIntent::SelectOption { option_id: 5 });
    }

    #[test]
    fn intent_queue_go_back() {
        let mut queue = GossipIntentQueue::default();
        queue.go_back();
        let drained = queue.drain();
        assert_eq!(drained[0], GossipIntent::GoBack);
    }

    #[test]
    fn intent_queue_close() {
        let mut queue = GossipIntentQueue::default();
        queue.close();
        let drained = queue.drain();
        assert_eq!(drained[0], GossipIntent::Close);
    }

    #[test]
    fn intent_queue_drain_clears() {
        let mut queue = GossipIntentQueue::default();
        queue.interact(1);
        queue.select_option(2);
        queue.close();
        assert_eq!(queue.drain().len(), 3);
        assert!(queue.pending.is_empty());
    }

    #[test]
    fn intent_queue_multiple_interactions() {
        let mut queue = GossipIntentQueue::default();
        queue.interact(10);
        queue.interact(20);
        let drained = queue.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0], GossipIntent::Interact { npc_entity_id: 10 });
        assert_eq!(drained[1], GossipIntent::Interact { npc_entity_id: 20 });
    }
}
