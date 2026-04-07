use bevy::prelude::*;

pub mod textures {
    /// Quest log frame chrome.
    pub const QUEST_LOG_FRAME: u32 = 1064979;
    /// Quest type icon: normal quest bang.
    pub const QUEST_BANG_NORMAL: u32 = 132048;
    /// Quest type icon: daily quest.
    pub const QUEST_BANG_DAILY: u32 = 368364;
    /// Quest type icon: campaign/legendary.
    pub const QUEST_BANG_CAMPAIGN: u32 = 2032597;
    /// Quest complete turn-in question mark.
    pub const QUEST_TURNIN: u32 = 132049;
    /// NPC dialog frame chrome.
    pub const QUEST_DIALOG_FRAME: u32 = 1064979;
    /// NPC portrait border.
    pub const NPC_PORTRAIT_BORDER: u32 = 136040;
}

// --- Quest types ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum QuestType {
    #[default]
    Normal,
    Daily,
    Weekly,
    Campaign,
}

impl QuestType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Quest",
            Self::Daily => "Daily Quest",
            Self::Weekly => "Weekly Quest",
            Self::Campaign => "Campaign Quest",
        }
    }
}

// --- Objective ---

#[derive(Clone, Debug, PartialEq)]
pub struct QuestObjective {
    pub text: String,
    pub current: u32,
    pub required: u32,
}

impl QuestObjective {
    pub fn is_complete(&self) -> bool {
        self.current >= self.required
    }

    pub fn progress_text(&self) -> String {
        if self.required <= 1 {
            self.text.clone()
        } else {
            format!("{}: {}/{}", self.text, self.current, self.required)
        }
    }

    pub fn progress_fraction(&self) -> f32 {
        if self.required == 0 {
            return 1.0;
        }
        (self.current as f32 / self.required as f32).min(1.0)
    }
}

// --- Reward item ---

#[derive(Clone, Debug, PartialEq)]
pub struct QuestReward {
    pub name: String,
    pub icon_fdid: u32,
    pub quantity: u32,
}

impl QuestReward {
    pub fn display_name(&self) -> String {
        if self.quantity > 1 {
            format!("{} x{}", self.name, self.quantity)
        } else {
            self.name.clone()
        }
    }
}

// --- Quest entry ---

#[derive(Clone, Debug, PartialEq)]
pub struct QuestEntry {
    pub quest_id: u32,
    pub title: String,
    pub level: u32,
    pub zone: String,
    pub quest_type: QuestType,
    pub description: String,
    pub objectives: Vec<QuestObjective>,
    pub rewards: Vec<QuestReward>,
}

impl QuestEntry {
    pub fn is_complete(&self) -> bool {
        !self.objectives.is_empty() && self.objectives.iter().all(|o| o.is_complete())
    }

    pub fn objective_summary(&self) -> String {
        let done = self.objectives.iter().filter(|o| o.is_complete()).count();
        format!("{}/{}", done, self.objectives.len())
    }
}

// --- NPC dialog ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DialogMode {
    #[default]
    Offer,
    TurnIn,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RequiredItem {
    pub name: String,
    pub icon_fdid: u32,
    pub current: u32,
    pub required: u32,
}

impl RequiredItem {
    pub fn is_satisfied(&self) -> bool {
        self.current >= self.required
    }

    pub fn count_text(&self) -> String {
        format!("{}/{}", self.current, self.required)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct NPCDialogState {
    pub npc_name: String,
    pub npc_portrait_fdid: u32,
    pub quest_title: String,
    pub quest_text: String,
    pub mode: DialogMode,
    pub required_items: Vec<RequiredItem>,
}

impl NPCDialogState {
    pub fn all_requirements_met(&self) -> bool {
        self.required_items.iter().all(|r| r.is_satisfied())
    }
}

// --- Runtime quest log resource ---

/// Runtime quest log state, held as a Bevy Resource.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct QuestLogState {
    pub entries: Vec<QuestEntry>,
    pub selected_quest_id: Option<u32>,
    pub dialog: Option<NPCDialogState>,
}

impl QuestLogState {
    pub fn selected_entry(&self) -> Option<&QuestEntry> {
        let id = self.selected_quest_id?;
        self.entries.iter().find(|e| e.quest_id == id)
    }

    pub fn quest_count(&self) -> usize {
        self.entries.len()
    }

    pub fn completed_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_complete()).count()
    }

    pub fn zones(&self) -> Vec<&str> {
        let mut zones: Vec<&str> = Vec::new();
        for e in &self.entries {
            if !zones.contains(&e.zone.as_str()) {
                zones.push(&e.zone);
            }
        }
        zones
    }

    pub fn has_active_dialog(&self) -> bool {
        self.dialog.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_objective(current: u32, required: u32) -> QuestObjective {
        QuestObjective {
            text: "Kill mobs".into(),
            current,
            required,
        }
    }

    fn sample_entry(quest_id: u32, zone: &str, complete: bool) -> QuestEntry {
        let current = if complete { 5 } else { 2 };
        QuestEntry {
            quest_id,
            title: format!("Quest {quest_id}"),
            level: 25,
            zone: zone.into(),
            quest_type: QuestType::Normal,
            description: "A test quest.".into(),
            objectives: vec![sample_objective(current, 5)],
            rewards: vec![QuestReward {
                name: "Sword".into(),
                icon_fdid: 100001,
                quantity: 1,
            }],
        }
    }

    // --- QuestType ---

    #[test]
    fn quest_type_labels() {
        assert_eq!(QuestType::Normal.label(), "Quest");
        assert_eq!(QuestType::Daily.label(), "Daily Quest");
        assert_eq!(QuestType::Weekly.label(), "Weekly Quest");
        assert_eq!(QuestType::Campaign.label(), "Campaign Quest");
    }

    // --- QuestObjective ---

    #[test]
    fn objective_completion() {
        assert!(sample_objective(5, 5).is_complete());
        assert!(sample_objective(6, 5).is_complete());
        assert!(!sample_objective(4, 5).is_complete());
    }

    #[test]
    fn objective_progress_text() {
        let multi = QuestObjective {
            text: "Kill mobs".into(),
            current: 3,
            required: 5,
        };
        assert_eq!(multi.progress_text(), "Kill mobs: 3/5");

        let single = QuestObjective {
            text: "Talk to NPC".into(),
            current: 0,
            required: 1,
        };
        assert_eq!(single.progress_text(), "Talk to NPC");
    }

    #[test]
    fn objective_progress_fraction() {
        let obj = sample_objective(3, 10);
        assert!((obj.progress_fraction() - 0.3).abs() < 0.01);

        let zero = sample_objective(0, 0);
        assert_eq!(zero.progress_fraction(), 1.0);

        let over = sample_objective(12, 10);
        assert_eq!(over.progress_fraction(), 1.0);
    }

    // --- QuestReward ---

    #[test]
    fn reward_display_name() {
        let single = QuestReward {
            name: "Sword".into(),
            icon_fdid: 1,
            quantity: 1,
        };
        assert_eq!(single.display_name(), "Sword");

        let multi = QuestReward {
            name: "Gold Dust".into(),
            icon_fdid: 2,
            quantity: 5,
        };
        assert_eq!(multi.display_name(), "Gold Dust x5");
    }

    // --- QuestEntry ---

    #[test]
    fn quest_entry_completion() {
        assert!(sample_entry(1, "Z", true).is_complete());
        assert!(!sample_entry(2, "Z", false).is_complete());
    }

    #[test]
    fn quest_entry_objective_summary() {
        let entry = QuestEntry {
            objectives: vec![sample_objective(5, 5), sample_objective(2, 5)],
            ..sample_entry(1, "Z", false)
        };
        assert_eq!(entry.objective_summary(), "1/2");
    }

    // --- RequiredItem ---

    #[test]
    fn required_item_satisfaction() {
        let sat = RequiredItem {
            name: "A".into(),
            icon_fdid: 1,
            current: 5,
            required: 5,
        };
        assert!(sat.is_satisfied());

        let not = RequiredItem {
            name: "B".into(),
            icon_fdid: 2,
            current: 3,
            required: 5,
        };
        assert!(!not.is_satisfied());
    }

    #[test]
    fn required_item_count_text() {
        let item = RequiredItem {
            name: "X".into(),
            icon_fdid: 1,
            current: 3,
            required: 10,
        };
        assert_eq!(item.count_text(), "3/10");
    }

    // --- NPCDialogState ---

    #[test]
    fn dialog_all_requirements_met() {
        let dialog = NPCDialogState {
            required_items: vec![
                RequiredItem {
                    name: "A".into(),
                    icon_fdid: 1,
                    current: 5,
                    required: 5,
                },
                RequiredItem {
                    name: "B".into(),
                    icon_fdid: 2,
                    current: 3,
                    required: 3,
                },
            ],
            ..Default::default()
        };
        assert!(dialog.all_requirements_met());

        let partial = NPCDialogState {
            required_items: vec![RequiredItem {
                name: "C".into(),
                icon_fdid: 3,
                current: 1,
                required: 5,
            }],
            ..Default::default()
        };
        assert!(!partial.all_requirements_met());
    }

    #[test]
    fn dialog_empty_requirements_met() {
        let dialog = NPCDialogState::default();
        assert!(dialog.all_requirements_met());
    }

    // --- QuestLogState ---

    #[test]
    fn quest_log_selected_entry() {
        let state = QuestLogState {
            entries: vec![sample_entry(10, "Z", false), sample_entry(20, "Z", true)],
            selected_quest_id: Some(20),
            ..Default::default()
        };
        let selected = state.selected_entry().expect("should find quest 20");
        assert_eq!(selected.quest_id, 20);
    }

    #[test]
    fn quest_log_selected_none() {
        let state = QuestLogState::default();
        assert!(state.selected_entry().is_none());
    }

    #[test]
    fn quest_log_counts() {
        let state = QuestLogState {
            entries: vec![
                sample_entry(1, "A", true),
                sample_entry(2, "A", false),
                sample_entry(3, "B", true),
            ],
            ..Default::default()
        };
        assert_eq!(state.quest_count(), 3);
        assert_eq!(state.completed_count(), 2);
    }

    #[test]
    fn quest_log_zones() {
        let state = QuestLogState {
            entries: vec![
                sample_entry(1, "Elwynn Forest", false),
                sample_entry(2, "Elwynn Forest", false),
                sample_entry(3, "Westfall", false),
            ],
            ..Default::default()
        };
        let zones = state.zones();
        assert_eq!(zones, vec!["Elwynn Forest", "Westfall"]);
    }

    #[test]
    fn quest_log_dialog_presence() {
        let mut state = QuestLogState::default();
        assert!(!state.has_active_dialog());
        state.dialog = Some(NPCDialogState::default());
        assert!(state.has_active_dialog());
    }

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::QUEST_LOG_FRAME, 0);
        assert_ne!(textures::QUEST_BANG_NORMAL, 0);
        assert_ne!(textures::QUEST_BANG_DAILY, 0);
        assert_ne!(textures::QUEST_BANG_CAMPAIGN, 0);
        assert_ne!(textures::QUEST_TURNIN, 0);
        assert_ne!(textures::NPC_PORTRAIT_BORDER, 0);
    }
}
