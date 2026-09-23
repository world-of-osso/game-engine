//! Death and resurrection UI: Retail `DEATH` / `RECOVER_CORPSE` StaticPopups, the ghost
//! world tint and the corpse-run hint, all driven by the live [`DeathStatusSnapshot`].
//!
//! The server pushes `DeathStateUpdate` only on death, release, resurrection and queries,
//! so its `can_resurrect_at_corpse` goes stale while the ghost runs. Corpse range is
//! therefore checked client-side against the local player transform with the server's
//! [`CORPSE_RESURRECT_RANGE`]; the server re-validates the request.

use bevy::prelude::*;
use bevy::render::view::ColorGrading;
use bevy::window::PrimaryWindow;
use game_engine::camera_control::WowCamera;
use game_engine::death::{DeathRuntimeState, reset_runtime};
use game_engine::status::{DeathPositionEntry, DeathStateEntry, DeathStatusSnapshot};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::popup::{PopupOutcome, PopupResult, PopupSpec, PopupStack};
use game_engine::ui::screens::ghost_hint_component::{GhostHintState, ghost_hint_screen};
use game_engine::ui::ui_errors::UiErrors;
use shared::death::CORPSE_RESURRECT_RANGE;
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::networking::LocalPlayer;

pub const DEATH_POPUP: &str = "DEATH";
pub const RECOVER_CORPSE_POPUP: &str = "RECOVER_CORPSE";
const DEATH_POPUP_KEYS: [&str; 2] = [DEATH_POPUP, RECOVER_CORPSE_POPUP];

/// Ghost world saturation. Retail greys the world while the player is a ghost.
pub const GHOST_POST_SATURATION: f32 = 0.2;

/// Popup keys the player already answered; not re-shown until the death state changes
/// or the popup's condition lapses.
#[derive(Resource, Default)]
struct DeathPopupState {
    answered: Vec<&'static str>,
}

struct GhostHintRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for GhostHintRes {}
unsafe impl Sync for GhostHintRes {}

#[derive(Resource)]
struct GhostHintWrap(GhostHintRes);

#[derive(Resource, PartialEq)]
struct GhostHintModel(GhostHintState);

pub struct DeathUiPlugin;

impl Plugin for DeathUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DeathPopupState>();
        app.init_resource::<UiErrors>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_ghost_hint_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), leave_world);
        app.add_systems(
            Update,
            (
                handle_death_popup_results,
                report_death_errors,
                sync_death_popups,
                sync_ghost_hint_ui,
            )
                .chain()
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(Update, sync_ghost_color_grading);
    }
}

fn death_popup_spec() -> PopupSpec {
    PopupSpec {
        key: DEATH_POPUP.to_string(),
        text: "You have died. Release spirit to the nearest graveyard?".to_string(),
        // DEATH_RELEASE
        accept_label: "Release Spirit".to_string(),
        cancel_label: None,
        timeout: None,
    }
}

/// Server resurrection has no delay, so Retail's "Resurrect in %d sec" timer never shows.
fn recover_corpse_popup_spec() -> PopupSpec {
    PopupSpec {
        key: RECOVER_CORPSE_POPUP.to_string(),
        // RECOVER_CORPSE
        text: "Resurrect now?".to_string(),
        accept_label: "Accept".to_string(),
        cancel_label: None,
        timeout: None,
    }
}

fn corpse_distance(corpse: &DeathPositionEntry, player: Vec3) -> f32 {
    player.distance(Vec3::new(corpse.x, corpse.y, corpse.z))
}

/// Distance to the corpse while a ghost; `None` otherwise.
fn ghost_corpse_distance(snapshot: &DeathStatusSnapshot, player: Option<Vec3>) -> Option<f32> {
    if snapshot.state != Some(DeathStateEntry::Ghost) {
        return None;
    }
    Some(corpse_distance(snapshot.corpse.as_ref()?, player?))
}

fn desired_death_popup(snapshot: &DeathStatusSnapshot, player: Option<Vec3>) -> Option<PopupSpec> {
    match snapshot.state {
        Some(DeathStateEntry::Dead) => Some(death_popup_spec()),
        Some(DeathStateEntry::Ghost) => ghost_corpse_distance(snapshot, player)
            .filter(|distance| *distance <= CORPSE_RESURRECT_RANGE)
            .map(|_| recover_corpse_popup_spec()),
        _ => None,
    }
}

fn local_player_position(players: &Query<&Transform, With<LocalPlayer>>) -> Option<Vec3> {
    players.iter().next().map(|transform| transform.translation)
}

fn handle_death_popup_results(
    mut results: MessageReader<PopupResult>,
    mut popups: ResMut<DeathPopupState>,
    mut runtime: ResMut<DeathRuntimeState>,
) {
    for result in results.read() {
        let Some(key) = DEATH_POPUP_KEYS.into_iter().find(|key| *key == result.key) else {
            continue;
        };
        match (key, result.outcome) {
            (DEATH_POPUP, PopupOutcome::Accepted) => runtime.request_release_spirit(),
            (RECOVER_CORPSE_POPUP, PopupOutcome::Accepted) => runtime.request_resurrect_at_corpse(),
            // Retail DEATH cannot be dismissed: it comes back until released.
            (DEATH_POPUP, _) => continue,
            _ => {}
        }
        popups.answered.push(key);
    }
}

fn report_death_errors(snapshot: Res<DeathStatusSnapshot>, mut errors: ResMut<UiErrors>) {
    if snapshot.is_changed()
        && let Some(error) = &snapshot.last_error
    {
        errors.add(error.clone());
    }
}

fn sync_death_popups(
    snapshot: Res<DeathStatusSnapshot>,
    players: Query<&Transform, With<LocalPlayer>>,
    mut popups: ResMut<DeathPopupState>,
    mut stack: ResMut<PopupStack>,
) {
    if snapshot.is_changed() {
        popups.answered.clear();
    }
    let desired = desired_death_popup(&snapshot, local_player_position(&players));
    for key in DEATH_POPUP_KEYS {
        if desired.as_ref().is_none_or(|spec| spec.key != key) {
            stack.hide(key);
            popups.answered.retain(|answered| *answered != key);
        }
    }
    let Some(spec) = desired else { return };
    if !popups.answered.iter().any(|key| *key == spec.key) && !stack.contains(&spec.key) {
        stack.push(spec);
    }
}

fn ghost_hint_state(
    snapshot: &DeathStatusSnapshot,
    players: &Query<&Transform, With<LocalPlayer>>,
) -> GhostHintState {
    let corpse_yards = ghost_corpse_distance(snapshot, local_player_position(players))
        .filter(|distance| *distance > CORPSE_RESURRECT_RANGE)
        .map(|distance| distance.round() as u32);
    GhostHintState { corpse_yards }
}

fn build_ghost_hint_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    snapshot: Res<DeathStatusSnapshot>,
    players: Query<&Transform, With<LocalPlayer>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = ghost_hint_state(&snapshot, &players);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(ghost_hint_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(GhostHintWrap(GhostHintRes { screen, shared }));
    commands.insert_resource(GhostHintModel(state));
}

fn sync_ghost_hint_ui(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<GhostHintWrap>>,
    mut last_model: Option<ResMut<GhostHintModel>>,
    snapshot: Res<DeathStatusSnapshot>,
    players: Query<&Transform, With<LocalPlayer>>,
) {
    let (Some(wrap), Some(last_model)) = (wrap.as_mut(), last_model.as_mut()) else {
        return;
    };
    let state = ghost_hint_state(&snapshot, &players);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

/// Greys the world through the main camera's post-tonemapping saturation while a ghost.
fn sync_ghost_color_grading(
    mut commands: Commands,
    snapshot: Res<DeathStatusSnapshot>,
    mut cameras: Query<(Entity, Option<&mut ColorGrading>), With<WowCamera>>,
) {
    let ghost = snapshot.state == Some(DeathStateEntry::Ghost);
    let saturation = if ghost { GHOST_POST_SATURATION } else { 1.0 };
    for (entity, grading) in &mut cameras {
        match grading {
            Some(mut grading) if grading.global.post_saturation != saturation => {
                grading.global.post_saturation = saturation;
            }
            None if ghost => {
                let mut grading = ColorGrading::default();
                grading.global.post_saturation = saturation;
                commands.entity(entity).insert(grading);
            }
            _ => {}
        }
    }
}

/// Death state belongs to the world session; the server resends it on the next entry.
fn leave_world(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<GhostHintWrap>>,
    mut popups: ResMut<DeathPopupState>,
    mut stack: ResMut<PopupStack>,
    mut runtime: ResMut<DeathRuntimeState>,
    mut snapshot: ResMut<DeathStatusSnapshot>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    for key in DEATH_POPUP_KEYS {
        stack.hide(key);
    }
    popups.answered.clear();
    reset_runtime(&mut runtime, &mut snapshot);
    commands.remove_resource::<GhostHintWrap>();
    commands.remove_resource::<GhostHintModel>();
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
