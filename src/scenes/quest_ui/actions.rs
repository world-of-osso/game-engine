//! Click actions of the quest screens and the abandon confirmation popup.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::quest_actions::{ABANDON_QUEST_POPUP, QuestUiEffect, quest_ui_action};
use game_engine::quest_runtime::{QuestRuntime, QuestUiState};
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::UiState;
use game_engine::ui::popup::{PopupOutcome, PopupResult, PopupStack};
use game_engine::ui::ui_errors::UiErrors;

use crate::networking_quests::NpcInteractionRequest;
use crate::ui_input::walk_up_for_onclick;
use crate::window_manager::{WindowId, WindowManager};

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
    errors: ResMut<'w, UiErrors>,
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
    let mut requests = Vec::new();
    let effects = quest_ui_action(action, &mut context.runtime, &mut context.quest_ui);
    for effect in effects {
        match effect {
            QuestUiEffect::Send(request) => requests.push(request),
            QuestUiEffect::OpenLog => context.manager.open(WindowId::QuestLog),
            QuestUiEffect::CloseLog => {
                context.manager.close(WindowId::QuestLog);
            }
            QuestUiEffect::ConfirmAbandon { quest_id, popup } => {
                context.pending_abandon.0 = Some(quest_id);
                context.popups.push(popup);
            }
            QuestUiEffect::Error(text) => context.errors.add(text),
        }
    }
    requests
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

#[cfg(test)]
#[path = "actions_tests.rs"]
mod tests;
