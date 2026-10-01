//! Quest UI: the objective tracker, the quest log window (L, Panel class) and the
//! quest giver frame (npc-driven Panel), all driven by [`QuestRuntime`].

mod actions;

use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::character_models::{class_name, race_name};
use game_engine::input_bindings::InputAction;
use game_engine::quest_runtime::{QuestDialogPage, QuestRuntime, QuestTextTokens, QuestUiState};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::popup::PopupStack;
use game_engine::ui::screens::objective_tracker_component::{
    ObjectiveTrackerState, objective_tracker_screen,
};
use game_engine::ui::screens::quest_frame_component::{QuestFrameState, quest_frame_screen};
use game_engine::ui::screens::quest_log_frame_component::{
    QuestLogFrameState, quest_log_frame_screen,
};
use shared::components::Player as NetPlayer;
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::networking::LocalPlayer;
use crate::networking_quests::NpcInteractionRequest;
use crate::scenes::static_popup::StaticPopupSystems;
use crate::window_manager::{WindowId, WindowManager};

use game_engine::quest_view::{self as view, QuestDetailsCache};

struct QuestScreens {
    tracker: Screen,
    log: Screen,
    frame: Screen,
    shared: SharedContext,
}

// SAFETY: screens are only touched from main-schedule systems, like every other
// Screen resource in this crate.
unsafe impl Send for QuestScreens {}
unsafe impl Sync for QuestScreens {}

#[derive(Resource)]
struct QuestScreensRes(QuestScreens);

/// Last states pushed into the screens' shared context.
#[derive(Resource, Default, PartialEq)]
struct QuestScreenModels {
    tracker: ObjectiveTrackerState,
    log: QuestLogFrameState,
    frame: QuestFrameState,
}

#[derive(Resource, Default)]
struct QuestLogData {
    details: QuestDetailsCache,
    zone_names: HashMap<i32, String>,
}

pub struct QuestUiPlugin;

impl Plugin for QuestUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QuestUiState>()
            .init_resource::<QuestLogData>()
            .init_resource::<actions::PendingAbandon>()
            .init_resource::<PopupStack>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_quest_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_quest_ui);
        app.add_systems(
            Update,
            (
                toggle_quest_log,
                actions::handle_quest_ui_clicks,
                actions::handle_abandon_popup,
                sync_quest_giver_window,
                remember_quest_details,
                sync_quest_screens,
            )
                .chain()
                .after(StaticPopupSystems)
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

fn build_quest_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let models = QuestScreenModels::default();
    let mut shared = SharedContext::new();
    shared.insert(models.tracker.clone());
    shared.insert(models.log.clone());
    shared.insert(models.frame.clone());
    let mut screens = QuestScreens {
        tracker: Screen::new(objective_tracker_screen),
        log: Screen::new(quest_log_frame_screen),
        frame: Screen::new(quest_frame_screen),
        shared,
    };
    sync_all(&mut screens, &mut ui);
    commands.insert_resource(QuestScreensRes(screens));
    commands.insert_resource(models);
}

fn sync_all(screens: &mut QuestScreens, ui: &mut UiState) {
    screens.tracker.sync(&screens.shared, &mut ui.registry);
    screens.log.sync(&screens.shared, &mut ui.registry);
    screens.frame.sync(&screens.shared, &mut ui.registry);
}

fn teardown_quest_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut screens: Option<ResMut<QuestScreensRes>>,
    mut quest_ui: ResMut<QuestUiState>,
    mut data: ResMut<QuestLogData>,
) {
    if let Some(res) = screens.as_mut() {
        res.0.tracker.teardown(&mut ui.registry);
        res.0.log.teardown(&mut ui.registry);
        res.0.frame.teardown(&mut ui.registry);
    }
    *quest_ui = QuestUiState::default();
    *data = QuestLogData::default();
    commands.remove_resource::<QuestScreensRes>();
    commands.remove_resource::<QuestScreenModels>();
}

fn toggle_quest_log(
    keybinds: crate::ui_input_mode::WorldKeybinds,
    mut window_manager: ResMut<WindowManager>,
) {
    if keybinds.just_pressed(InputAction::ToggleQuestLog) {
        window_manager.toggle(WindowId::QuestLog);
    }
}

/// An open quest giver dialog opens the QuestFrame window; the window manager closing
/// it (Escape, eviction) ends the interaction; a dialog the server closed closes it.
fn sync_quest_giver_window(
    mut manager: ResMut<WindowManager>,
    mut runtime: ResMut<QuestRuntime>,
    mut requests: MessageWriter<NpcInteractionRequest>,
    mut open_npc: Local<Option<u64>>,
) {
    let dialog_npc = runtime.dialog.as_ref().map(|dialog| dialog.npc);
    let window_open = manager.is_open(WindowId::QuestGiver);
    if dialog_npc.is_some() && *open_npc != dialog_npc {
        manager.open(WindowId::QuestGiver);
    } else if let Some(npc) = dialog_npc.filter(|_| !window_open) {
        runtime.dialog = None;
        requests.write(NpcInteractionRequest::Close { npc });
    } else if dialog_npc.is_none() && window_open {
        manager.close(WindowId::QuestGiver);
    }
    *open_npc = runtime.dialog.as_ref().map(|dialog| dialog.npc);
}

/// Keeps the story text and rewards of every quest detail page shown, for the log.
fn remember_quest_details(runtime: Res<QuestRuntime>, mut data: ResMut<QuestLogData>) {
    if !runtime.is_changed() {
        return;
    }
    if let Some(QuestDialogPage::Detail(details)) = runtime.dialog.as_ref().map(|d| &d.page)
        && data.details.get(&details.quest_id) != Some(details)
    {
        data.details.insert(details.quest_id, details.clone());
    }
}

#[derive(SystemParam)]
struct QuestViewInputs<'w, 's> {
    runtime: Res<'w, QuestRuntime>,
    quest_ui: Res<'w, QuestUiState>,
    manager: Res<'w, WindowManager>,
    data: ResMut<'w, QuestLogData>,
    players: Query<'w, 's, &'static NetPlayer, With<LocalPlayer>>,
}

fn sync_quest_screens(
    mut ui: ResMut<UiState>,
    screens: Option<ResMut<QuestScreensRes>>,
    models: Option<ResMut<QuestScreenModels>>,
    mut inputs: QuestViewInputs,
) {
    let (Some(mut screens), Some(mut models)) = (screens, models) else {
        return;
    };
    let changed = inputs.runtime.is_changed()
        || inputs.quest_ui.is_changed()
        || inputs.manager.is_changed()
        || inputs.data.is_changed();
    if !changed {
        return;
    }
    let next = build_models(&mut inputs);
    if *models == next {
        return;
    }
    let shared = &mut screens.0.shared;
    if models.tracker != next.tracker {
        shared.insert(next.tracker.clone());
    }
    if models.log != next.log {
        shared.insert(next.log.clone());
    }
    if models.frame != next.frame {
        shared.insert(next.frame.clone());
    }
    *models = next;
    sync_all(&mut screens.0, &mut ui);
}

fn build_models(inputs: &mut QuestViewInputs) -> QuestScreenModels {
    let tokens = text_tokens(inputs.players.iter().next());
    // The zone-name memo is a cache, not a model change.
    let QuestLogData {
        details,
        zone_names,
    } = inputs.data.bypass_change_detection();
    let mut header_name = |sort_id: i32| {
        zone_names
            .entry(sort_id)
            .or_insert_with(|| quest_header_name(sort_id))
            .clone()
    };
    QuestScreenModels {
        tracker: ObjectiveTrackerState::from_runtime(
            &inputs.runtime,
            inputs.quest_ui.tracker_collapsed,
            inputs.quest_ui.quests_collapsed,
        ),
        log: view::quest_log_state(
            &inputs.runtime,
            &inputs.quest_ui,
            details,
            &tokens,
            &mut header_name,
            inputs.manager.is_open(WindowId::QuestLog),
        ),
        frame: view::quest_frame_state(inputs.runtime.dialog.as_ref(), &tokens),
    }
}

/// Quest log header: `QuestSortID` > 0 is an `AreaTable` zone.
fn quest_header_name(sort_id: i32) -> String {
    u32::try_from(sort_id)
        .map(crate::zone_names::zone_id_to_name)
        .unwrap_or_else(|_| "Unknown".into())
}

fn text_tokens(player: Option<&NetPlayer>) -> QuestTextTokens {
    let Some(player) = player else {
        return QuestTextTokens::default();
    };
    QuestTextTokens {
        name: player.name.clone(),
        class: class_name(player.class).into(),
        race: race_name(player.race).into(),
        female: player.appearance.sex == 1,
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
