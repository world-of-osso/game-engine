use std::collections::HashSet;

use bevy::prelude::*;

pub mod textures {
    // --- Quest log frame chrome ---
    /// Quest log dual-pane left background.
    pub const QUEST_LOG_PANE_LEFT: u32 = 309665;
    /// Quest log dual-pane right background.
    pub const QUEST_LOG_PANE_RIGHT: u32 = 309666;
    /// Quest log top-left corner.
    pub const QUEST_LOG_TOP_LEFT: u32 = 136804;
    /// Quest log top-right corner.
    pub const QUEST_LOG_TOP_RIGHT: u32 = 136805;
    /// Quest log bottom-left corner.
    pub const QUEST_LOG_BOT_LEFT: u32 = 136798;
    /// Quest log bottom-right corner.
    pub const QUEST_LOG_BOT_RIGHT: u32 = 136799;
    /// Quest log title highlight bar.
    pub const QUEST_LOG_TITLE_HIGHLIGHT: u32 = 136809;
    /// Quest log book icon.
    pub const QUEST_LOG_BOOK_ICON: u32 = 136797;
    /// Horizontal divider line.
    pub const HORIZONTAL_BREAK: u32 = 136783;
    /// Bullet point marker for objectives.
    pub const BULLET_POINT: u32 = 136788;

    // --- Quest type icons ---
    /// Active quest exclamation mark (normal).
    pub const QUEST_BANG_NORMAL: u32 = 132048;
    /// Available quest question mark / turn-in.
    pub const QUEST_TURNIN: u32 = 132049;
    /// Daily quest exclamation mark (blue).
    pub const QUEST_BANG_DAILY: u32 = 368364;
    /// Campaign/legendary quest active icon (orange).
    pub const QUEST_BANG_CAMPAIGN: u32 = 646979;
    /// Campaign/legendary quest available icon.
    pub const QUEST_TURNIN_CAMPAIGN: u32 = 646980;

    // --- NPC dialog ---
    /// Quest dialog top-left corner.
    pub const QUEST_DIALOG_TOP_LEFT: u32 = 136789;
    /// Quest dialog top-right corner.
    pub const QUEST_DIALOG_TOP_RIGHT: u32 = 136790;
    /// Quest dialog bottom-left corner.
    pub const QUEST_DIALOG_BOT_LEFT: u32 = 136784;
    /// Quest dialog bottom-right corner.
    pub const QUEST_DIALOG_BOT_RIGHT: u32 = 136786;
    /// NPC portrait ring with background.
    pub const NPC_PORTRAIT_RING: u32 = 652158;
    /// NPC portrait ring highlight (hover/selected).
    pub const NPC_PORTRAIT_RING_HIGHLIGHT: u32 = 652157;

    // --- Reward item icons (generic placeholders) ---
    /// Generic sword icon (melee weapon reward).
    pub const REWARD_ICON_SWORD: u32 = 135274;
    /// Generic shield icon (armor reward).
    pub const REWARD_ICON_SHIELD: u32 = 134948;
    /// Gold coin icon (money reward).
    pub const REWARD_ICON_GOLD: u32 = 133784;
    /// Experience icon (XP reward).
    pub const REWARD_ICON_XP: u32 = 132446;
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
        let mut seen = HashSet::new();
        let mut zones: Vec<&str> = Vec::new();
        for e in &self.entries {
            if seen.insert(e.zone.as_str()) {
                zones.push(&e.zone);
            }
        }
        zones
    }

    pub fn has_active_dialog(&self) -> bool {
        self.dialog.is_some()
    }

    /// Get quests filtered by zone name.
    pub fn quests_in_zone(&self, zone: &str) -> Vec<&QuestEntry> {
        self.entries.iter().filter(|e| e.zone == zone).collect()
    }

    /// Get quests filtered by type.
    pub fn quests_by_type(&self, qt: QuestType) -> Vec<&QuestEntry> {
        self.entries.iter().filter(|e| e.quest_type == qt).collect()
    }

    /// Accept a quest (add to log from dialog).
    pub fn accept_quest(&mut self, entry: QuestEntry) -> bool {
        if self.entries.iter().any(|e| e.quest_id == entry.quest_id) {
            return false; // already in log
        }
        self.entries.push(entry);
        true
    }

    /// Abandon a quest (remove from log).
    pub fn abandon_quest(&mut self, quest_id: u32) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.quest_id != quest_id);
        self.entries.len() < before
    }

    /// Turn in a quest (remove from log, marking complete).
    pub fn turn_in_quest(&mut self, quest_id: u32) -> bool {
        let entry = self.entries.iter().find(|e| e.quest_id == quest_id);
        let can_turn_in = entry.is_some_and(|e| e.is_complete());
        if can_turn_in {
            self.entries.retain(|e| e.quest_id != quest_id);
        }
        can_turn_in
    }
}

/// A pending quest interaction to send to the server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuestIntent {
    Accept { quest_id: u32, npc_id: u32 },
    TurnIn { quest_id: u32, npc_id: u32 },
    Abandon { quest_id: u32 },
}

/// Queue of quest intents waiting to be sent to the server.
#[derive(Resource, Default)]
pub struct QuestIntentQueue {
    pub pending: Vec<QuestIntent>,
}

impl QuestIntentQueue {
    pub fn accept(&mut self, quest_id: u32, npc_id: u32) {
        self.pending.push(QuestIntent::Accept { quest_id, npc_id });
    }

    pub fn turn_in(&mut self, quest_id: u32, npc_id: u32) {
        self.pending.push(QuestIntent::TurnIn { quest_id, npc_id });
    }

    pub fn abandon(&mut self, quest_id: u32) {
        self.pending.push(QuestIntent::Abandon { quest_id });
    }

    pub fn drain(&mut self) -> Vec<QuestIntent> {
        std::mem::take(&mut self.pending)
    }
}

#[cfg(test)]
#[path = "quest_data_tests/mod.rs"]
mod tests;
