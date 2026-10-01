//! Click actions of the quest screens (objective tracker, quest log, quest giver frame)
//! as pure state changes and host effects; shared by the Bevy and Godot hosts.

use shared::protocol::QuestGiverQuestState;

use crate::quest_runtime::{NpcInteractionRequest, QuestDialogPage, QuestRuntime, QuestUiState};
use crate::quest_view::selected_quest;
use crate::ui::popup::PopupSpec;
use crate::ui::screens::{
    objective_tracker_component as tracker, quest_frame_component as frame,
    quest_log_frame_component as log,
};

/// Retail `StaticPopupDialogs["ABANDON_QUEST"]`.
pub const ABANDON_QUEST_POPUP: &str = "ABANDON_QUEST";

/// What one quest screen action asks of the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuestUiEffect {
    Send(NpcInteractionRequest),
    OpenLog,
    CloseLog,
    /// Show the `ABANDON_QUEST` popup; accepting it abandons `quest_id`.
    ConfirmAbandon {
        quest_id: u32,
        popup: PopupSpec,
    },
}

/// Applies one screen action to the client quest state; returns its host effects.
pub fn quest_ui_action(
    action: &str,
    runtime: &mut QuestRuntime,
    ui: &mut QuestUiState,
) -> Vec<QuestUiEffect> {
    if action.starts_with("quest_tracker:") {
        tracker_action(action, ui)
    } else if action.starts_with("quest_log:") {
        log_action(action, runtime, ui)
    } else if action.starts_with("quest_frame:") {
        frame_action(action, runtime)
            .into_iter()
            .map(QuestUiEffect::Send)
            .collect()
    } else {
        Vec::new()
    }
}

fn tracker_action(action: &str, ui: &mut QuestUiState) -> Vec<QuestUiEffect> {
    if action == tracker::TOGGLE_ACTION {
        ui.tracker_collapsed = !ui.tracker_collapsed;
    } else if action == tracker::TOGGLE_QUESTS_ACTION {
        ui.quests_collapsed = !ui.quests_collapsed;
    } else if let Some(id) = parse_suffix::<u32>(action, tracker::OPEN_QUEST_PREFIX) {
        ui.log_selected = Some(id);
        return vec![QuestUiEffect::OpenLog];
    }
    Vec::new()
}

fn log_action(action: &str, runtime: &QuestRuntime, ui: &mut QuestUiState) -> Vec<QuestUiEffect> {
    let selected = selected_quest(runtime, ui);
    if action == log::CLOSE_ACTION {
        return vec![QuestUiEffect::CloseLog];
    } else if let Some(id) = parse_suffix::<u32>(action, log::SELECT_PREFIX) {
        ui.log_selected = Some(id);
    } else if let Some(sort_id) = parse_suffix::<i32>(action, log::HEADER_PREFIX) {
        if !ui.collapsed_headers.remove(&sort_id) {
            ui.collapsed_headers.insert(sort_id);
        }
    } else if action == log::TRACK_ACTION
        && let Some(quest_id) = selected
    {
        let watched = !runtime.is_watched(quest_id);
        return vec![QuestUiEffect::Send(NpcInteractionRequest::SetWatched {
            quest_id,
            watched,
        })];
    } else if action == log::ABANDON_ACTION
        && let Some(entry) = selected.and_then(|id| runtime.entry(id))
    {
        return vec![QuestUiEffect::ConfirmAbandon {
            quest_id: entry.quest_id,
            popup: abandon_popup(&entry.title),
        }];
    }
    Vec::new()
}

fn abandon_popup(title: &str) -> PopupSpec {
    PopupSpec {
        key: ABANDON_QUEST_POPUP.into(),
        // ABANDON_QUEST_CONFIRM
        text: format!("Abandon \"{title}\"?"),
        accept_label: "Yes".into(),
        cancel_label: Some("No".into()),
        timeout: None,
        confirm_text: None,
    }
}

/// Quest giver frame buttons (`QuestFrame.lua` handlers).
fn frame_action(action: &str, runtime: &mut QuestRuntime) -> Vec<NpcInteractionRequest> {
    use NpcInteractionRequest as R;
    let Some(dialog) = runtime.dialog.as_ref() else {
        return Vec::new();
    };
    let npc = dialog.npc;
    let quest_id = page_quest_id(&dialog.page);
    match action {
        frame::CLOSE_ACTION | frame::DECLINE_ACTION => {
            runtime.dialog = None;
            vec![R::Close { npc }]
        }
        frame::ACCEPT_ACTION => {
            runtime.dialog = None;
            quest_id
                .map(|quest_id| vec![R::Accept { npc, quest_id }, R::Close { npc }])
                .unwrap_or_default()
        }
        frame::CONTINUE_ACTION => quest_id
            .map(|quest_id| vec![R::Complete { npc, quest_id }])
            .unwrap_or_default(),
        frame::COMPLETE_ACTION => complete_request(&dialog.page, npc).into_iter().collect(),
        _ => frame_list_action(action, runtime, npc),
    }
}

fn frame_list_action(
    action: &str,
    runtime: &mut QuestRuntime,
    npc: u64,
) -> Vec<NpcInteractionRequest> {
    use NpcInteractionRequest as R;
    if let Some(index) = parse_suffix::<u8>(action, frame::CHOICE_ACTION_PREFIX) {
        runtime.choose_reward(index);
    } else if let Some(option_id) = parse_suffix::<u32>(action, frame::GOSSIP_ACTION_PREFIX) {
        return vec![R::SelectGossip { npc, option_id }];
    } else if let Some(index) = parse_suffix::<usize>(action, frame::QUEST_ACTION_PREFIX)
        && let Some(QuestDialogPage::Greeting { quests, .. }) =
            runtime.dialog.as_ref().map(|dialog| &dialog.page)
        && let Some(quest) = quests.get(index)
    {
        let quest_id = quest.quest_id;
        return vec![match quest.state {
            QuestGiverQuestState::Available | QuestGiverQuestState::LowLevelAvailable => {
                R::QueryQuest { npc, quest_id }
            }
            QuestGiverQuestState::Complete | QuestGiverQuestState::Incomplete => {
                R::Complete { npc, quest_id }
            }
        }];
    }
    Vec::new()
}

/// `QuestRewardCompleteButton_OnClick`: needs a choice when choices exist.
fn complete_request(page: &QuestDialogPage, npc: u64) -> Option<NpcInteractionRequest> {
    let QuestDialogPage::Reward { offer, choice } = page else {
        return None;
    };
    if !offer.rewards.choice_items.is_empty() && choice.is_none() {
        return None;
    }
    Some(NpcInteractionRequest::ChooseReward {
        npc,
        quest_id: offer.quest_id,
        choice: *choice,
    })
}

fn page_quest_id(page: &QuestDialogPage) -> Option<u32> {
    match page {
        QuestDialogPage::Greeting { .. } => None,
        QuestDialogPage::Detail(details) => Some(details.quest_id),
        QuestDialogPage::Progress(request) => Some(request.quest_id),
        QuestDialogPage::Reward { offer, .. } => Some(offer.quest_id),
    }
}

fn parse_suffix<T: std::str::FromStr>(action: &str, prefix: &str) -> Option<T> {
    action.strip_prefix(prefix)?.parse().ok()
}
