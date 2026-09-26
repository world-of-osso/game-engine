//! Click actions of the quest screens and the abandon confirmation popup.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::quest_runtime::{QuestDialogPage, QuestRuntime, QuestUiState};
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::UiState;
use game_engine::ui::popup::{PopupOutcome, PopupResult, PopupSpec, PopupStack};
use game_engine::ui::screens::{
    objective_tracker_component as tracker, quest_frame_component as frame,
    quest_log_frame_component as log,
};
use shared::protocol::QuestGiverQuestState;

use crate::networking_quests::NpcInteractionRequest;
use crate::ui_input::walk_up_for_onclick;
use crate::window_manager::{WindowId, WindowManager};

/// Retail `StaticPopupDialogs["ABANDON_QUEST"]`.
pub const ABANDON_QUEST_POPUP: &str = "ABANDON_QUEST";

/// Quest the open abandon popup is about.
#[derive(Resource, Default)]
pub(super) struct PendingAbandon(Option<u32>);

#[derive(SystemParam)]
pub(super) struct QuestUiContext<'w> {
    runtime: ResMut<'w, QuestRuntime>,
    quest_ui: ResMut<'w, QuestUiState>,
    manager: ResMut<'w, WindowManager>,
    popups: ResMut<'w, PopupStack>,
    pending_abandon: ResMut<'w, PendingAbandon>,
}

#[derive(SystemParam)]
pub(super) struct PointerClick<'w, 's> {
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    mouse: Option<Res<'w, ButtonInput<MouseButton>>>,
    reconnect: Option<Res<'w, crate::networking::ReconnectState>>,
    modal_open: Option<Res<'w, crate::scenes::game_menu::UiModalOpen>>,
}

impl PointerClick<'_, '_> {
    /// `onclick` action under a fresh left click, if gameplay input is allowed.
    fn action(self, ui: &UiState) -> Option<String> {
        let pressed = self
            .mouse
            .as_ref()
            .is_some_and(|mouse| mouse.just_pressed(MouseButton::Left));
        if !pressed || self.modal_open.is_some() {
            return None;
        }
        if !crate::networking::gameplay_input_allowed(self.reconnect) {
            return None;
        }
        let window = self.windows.single().ok()?;
        let cursor = ui_cursor_position(&ui.registry, window)?;
        let frame_id = find_frame_at(&ui.registry, cursor.x, cursor.y)?;
        walk_up_for_onclick(&ui.registry, frame_id)
    }
}

pub(super) fn handle_quest_ui_clicks(
    pointer: PointerClick,
    ui: Res<UiState>,
    mut context: QuestUiContext,
    mut requests: MessageWriter<NpcInteractionRequest>,
) {
    let Some(action) = pointer.action(&ui) else {
        return;
    };
    for request in dispatch_action(&action, &mut context) {
        requests.write(request);
    }
}

/// Applies one screen action; returns the server requests it makes.
fn dispatch_action(action: &str, context: &mut QuestUiContext) -> Vec<NpcInteractionRequest> {
    if action.starts_with("quest_tracker:") {
        tracker_action(action, context);
        Vec::new()
    } else if action.starts_with("quest_log:") {
        log_action(action, context)
    } else if action.starts_with("quest_frame:") {
        frame_action(action, &mut context.runtime)
    } else {
        Vec::new()
    }
}

fn tracker_action(action: &str, context: &mut QuestUiContext) {
    if action == tracker::TOGGLE_ACTION {
        context.quest_ui.tracker_collapsed = !context.quest_ui.tracker_collapsed;
    } else if action == tracker::TOGGLE_QUESTS_ACTION {
        context.quest_ui.quests_collapsed = !context.quest_ui.quests_collapsed;
    } else if let Some(id) = parse_suffix::<u32>(action, tracker::OPEN_QUEST_PREFIX) {
        context.quest_ui.log_selected = Some(id);
        context.manager.open(WindowId::QuestLog);
    }
}

fn log_action(action: &str, context: &mut QuestUiContext) -> Vec<NpcInteractionRequest> {
    let selected = super::view::selected_quest(&context.runtime, &context.quest_ui);
    if action == log::CLOSE_ACTION {
        context.manager.close(WindowId::QuestLog);
    } else if let Some(id) = parse_suffix::<u32>(action, log::SELECT_PREFIX) {
        context.quest_ui.log_selected = Some(id);
    } else if let Some(sort_id) = parse_suffix::<i32>(action, log::HEADER_PREFIX) {
        let headers = &mut context.quest_ui.collapsed_headers;
        if !headers.remove(&sort_id) {
            headers.insert(sort_id);
        }
    } else if action == log::TRACK_ACTION
        && let Some(quest_id) = selected
    {
        let watched = !context.runtime.is_watched(quest_id);
        return vec![NpcInteractionRequest::SetWatched { quest_id, watched }];
    } else if action == log::ABANDON_ACTION
        && let Some(entry) = selected.and_then(|id| context.runtime.entry(id))
    {
        context.pending_abandon.0 = Some(entry.quest_id);
        context.popups.push(abandon_popup(&entry.title));
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

pub(super) fn handle_abandon_popup(
    mut results: MessageReader<PopupResult>,
    mut pending: ResMut<PendingAbandon>,
    mut requests: MessageWriter<NpcInteractionRequest>,
) {
    for result in results.read() {
        if result.key != ABANDON_QUEST_POPUP {
            continue;
        }
        let quest_id = pending.0.take();
        if let (PopupOutcome::Accepted, Some(quest_id)) = (result.outcome, quest_id) {
            requests.write(NpcInteractionRequest::Abandon { quest_id });
        }
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

#[cfg(test)]
#[path = "actions_tests.rs"]
mod tests;
