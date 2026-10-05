//! Pure view models for the quest screens (objective tracker, quest log, quest giver
//! frame), built from [`QuestRuntime`]; shared by the Bevy and Godot hosts.

use std::collections::HashMap;

use crate::item_icons::item_icon_fdid;
use crate::quest_runtime::{
    QuestDialog, QuestDialogPage, QuestRuntime, QuestTextTokens, QuestUiState,
    substitute_quest_text,
};
use crate::ui::screens::quest_frame_component::{
    GossipOptionView, GreetingQuest, GreetingQuestKind, QuestFramePage, QuestFrameState,
    RewardItemView, RewardView,
};
use crate::ui::screens::quest_log_frame_component::{
    QuestDifficulty, QuestLogDetails, QuestLogFrameState, QuestLogGroup, QuestLogObjectiveLine,
    QuestLogRow,
};
use shared::protocol::{
    MAX_QUEST_LOG_SIZE, QuestEntrySnapshot, QuestGiverQuestDetails, QuestGiverQuestState,
    QuestRewardItem, QuestRewards,
};

/// Story text and rewards the quest giver showed this session, by quest id; the log
/// snapshot carries neither.
pub type QuestDetailsCache = HashMap<u32, QuestGiverQuestDetails>;

pub fn quest_frame_state(
    dialog: Option<&QuestDialog>,
    tokens: &QuestTextTokens,
) -> QuestFrameState {
    let Some(dialog) = dialog else {
        return QuestFrameState::default();
    };
    let text = |s: &str| substitute_quest_text(s, tokens);
    let page = match &dialog.page {
        QuestDialogPage::Greeting {
            text: greeting,
            options,
            quests,
        } => QuestFramePage::Greeting {
            text: text(greeting),
            options: options
                .iter()
                .map(|option| GossipOptionView {
                    option_id: option.option_id,
                    text: text(&option.text),
                })
                .collect(),
            quests: quests
                .iter()
                .enumerate()
                .map(|(index, quest)| GreetingQuest {
                    index,
                    title: quest.title.clone(),
                    kind: greeting_kind(quest.state),
                })
                .collect(),
        },
        QuestDialogPage::Detail(details) => QuestFramePage::Detail {
            title: details.title.clone(),
            description: text(&details.description),
            objectives_text: text(&details.objectives_text),
            rewards: reward_view(&details.rewards, None),
        },
        QuestDialogPage::Progress(request) => QuestFramePage::Progress {
            title: request.title.clone(),
            text: text(&request.completion_text),
            required: request.required_items.iter().map(item_view).collect(),
            can_complete: request.can_complete,
        },
        QuestDialogPage::Reward { offer, choice } => QuestFramePage::Reward {
            title: offer.title.clone(),
            text: text(&offer.reward_text),
            rewards: reward_view(&offer.rewards, choice.map(usize::from)),
        },
    };
    QuestFrameState {
        visible: true,
        npc_name: dialog.npc_name.clone(),
        page,
    }
}

fn greeting_kind(state: QuestGiverQuestState) -> GreetingQuestKind {
    match state {
        QuestGiverQuestState::Available | QuestGiverQuestState::LowLevelAvailable => {
            GreetingQuestKind::Available
        }
        QuestGiverQuestState::Complete => GreetingQuestKind::Complete,
        QuestGiverQuestState::Incomplete => GreetingQuestKind::Incomplete,
    }
}

fn item_view(item: &QuestRewardItem) -> RewardItemView {
    RewardItemView {
        name: item.name.clone(),
        count: item.count,
        icon_fdid: item_icon_fdid(item.item_id),
    }
}

fn reward_view(rewards: &QuestRewards, selected_choice: Option<usize>) -> RewardView {
    RewardView {
        money: rewards.money,
        items: rewards.items.iter().map(item_view).collect(),
        choices: rewards.choice_items.iter().map(item_view).collect(),
        selected_choice,
    }
}

/// `QuestInfoItem{n}` (1-based: the choices, then the fixed items) of `rewards`.
fn reward_item_at(rewards: &QuestRewards, n: usize) -> Option<&QuestRewardItem> {
    let index = n.checked_sub(1)?;
    rewards.choice_items.iter().chain(&rewards.items).nth(index)
}

/// The item behind the quest frame's `QuestInfoItem{n}` (`GameTooltip:SetQuestItem`).
pub fn frame_reward_item(dialog: Option<&QuestDialog>, n: usize) -> Option<&QuestRewardItem> {
    let rewards = match &dialog?.page {
        QuestDialogPage::Detail(details) => &details.rewards,
        QuestDialogPage::Reward { offer, .. } => &offer.rewards,
        QuestDialogPage::Greeting { .. } | QuestDialogPage::Progress(_) => return None,
    };
    reward_item_at(rewards, n)
}

/// The item behind the quest log's `QuestInfoItem{n}` for the selected quest
/// (`GameTooltip:SetQuestLogItem`), from the rewards its giver showed.
pub fn log_reward_item<'a>(
    runtime: &QuestRuntime,
    ui: &QuestUiState,
    cache: &'a QuestDetailsCache,
    n: usize,
) -> Option<&'a QuestRewardItem> {
    reward_item_at(&cache.get(&selected_quest(runtime, ui)?)?.rewards, n)
}

/// The selected quest, or the first one when the selection left the log.
pub fn selected_quest(runtime: &QuestRuntime, ui: &QuestUiState) -> Option<u32> {
    ui.log_selected
        .filter(|id| runtime.entry(*id).is_some())
        .or_else(|| runtime.log.first().map(|entry| entry.quest_id))
}

/// The quest log for a player of `player_level`: quest rows carry their displayed
/// level and difficulty against it.
pub fn quest_log_state(
    runtime: &QuestRuntime,
    ui: &QuestUiState,
    cache: &QuestDetailsCache,
    tokens: &QuestTextTokens,
    header_name: &mut dyn FnMut(i32) -> String,
    player_level: i32,
    visible: bool,
) -> QuestLogFrameState {
    let selected = selected_quest(runtime, ui);
    let mut groups: Vec<QuestLogGroup> = Vec::new();
    for entry in &runtime.log {
        // Level -1 scales to the player (`QuestEntrySnapshot::level`).
        let level = if entry.level < 0 {
            player_level
        } else {
            entry.level
        };
        let row = QuestLogRow {
            quest_id: entry.quest_id,
            title: entry.title.clone(),
            level,
            difficulty: QuestDifficulty::relative(player_level, level),
            complete: entry.completed,
            watched: runtime.is_watched(entry.quest_id),
            selected: selected == Some(entry.quest_id),
        };
        match groups
            .iter_mut()
            .find(|group| group.sort_id == entry.sort_id)
        {
            Some(group) => group.quests.push(row),
            None => groups.push(QuestLogGroup {
                sort_id: entry.sort_id,
                name: header_name(entry.sort_id),
                collapsed: ui.collapsed_headers.contains(&entry.sort_id),
                quests: vec![row],
            }),
        }
    }
    QuestLogFrameState {
        visible,
        quest_count: runtime.log.len(),
        max_quests: MAX_QUEST_LOG_SIZE,
        groups,
        details: selected
            .and_then(|id| runtime.entry(id))
            .map(|entry| log_details(entry, runtime, cache, tokens)),
    }
}

fn log_details(
    entry: &QuestEntrySnapshot,
    runtime: &QuestRuntime,
    cache: &QuestDetailsCache,
    tokens: &QuestTextTokens,
) -> QuestLogDetails {
    let cached = cache.get(&entry.quest_id);
    QuestLogDetails {
        quest_id: entry.quest_id,
        title: entry.title.clone(),
        objectives_text: substitute_quest_text(&entry.objectives_text, tokens),
        objectives: entry
            .objectives
            .iter()
            .map(|objective| QuestLogObjectiveLine {
                text: format!(
                    "{}/{} {}",
                    objective.current, objective.required, objective.text
                ),
                done: objective.completed,
            })
            .collect(),
        description: cached.map(|details| substitute_quest_text(&details.description, tokens)),
        rewards: cached
            .map(|details| reward_view(&details.rewards, None))
            .filter(|rewards| !rewards.is_empty()),
        watched: runtime.is_watched(entry.quest_id),
    }
}

#[cfg(test)]
#[path = "quest_view_tests.rs"]
mod tests;
