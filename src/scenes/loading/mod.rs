use bevy::prelude::*;
use game_engine::ui::plugin::UiState;
use game_engine::ui::screens::loading_component::{
    LoadingScreenLayout, LoadingScreenState, debug_loading_layout_from_source,
};
use ui_toolkit::render::UiCamera;

use crate::game_state::{GameState, InitialGameState, evaluate_world_loading};
use crate::networking::{CurrentZone, LocalPlayer};
use crate::terrain::AdtManager;
use crate::zone_names::zone_id_to_name;

mod native_view;
use native_view::{LoadingView, LoadingViewAssets, spawn_loading_view, sync_loading_view};

const DEFAULT_ZONE_TEXT: &str = "Entering Elwynn Forest";
const DEFAULT_TIP_TEXT: &str =
    "Tip: The first zone load streams terrain and replicated actors before gameplay begins.";
const LOADING_BAR_FILL_RATE_PERCENT_PER_SEC: f32 = 6.0;
const PREVIEW_MODE_HOLD_PERCENT: f32 = 100.0;

#[derive(Resource)]
struct LoadingSnapshot {
    state: LoadingScreenState,
    layout: LoadingScreenLayout,
}

#[derive(Resource, Default)]
struct LoadingCameraOrders(Vec<(Entity, isize)>);

#[derive(Resource)]
struct LoadingProgressAnimation {
    displayed_percent: f32,
    elapsed_secs: f32,
    preview_mode: bool,
}

pub struct LoadingScreenPlugin;

impl Plugin for LoadingScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Loading), build_loading_ui)
            .add_systems(OnExit(GameState::Loading), teardown_loading_ui)
            .add_systems(PostStartup, raise_startup_ui_cameras)
            .add_systems(
                Update,
                loading_update_visuals.run_if(in_state(GameState::Loading)),
            );
    }
}

fn build_loading_ui(
    mut commands: Commands,
    mut assets: LoadingViewAssets,
    current_zone: Res<CurrentZone>,
    local_player_q: Query<(), With<LocalPlayer>>,
    adt_manager: Res<AdtManager>,
    initial_state: Option<Res<InitialGameState>>,
    mut cameras: Query<(Entity, &mut Camera), With<UiCamera>>,
) {
    let mut progress = LoadingProgressAnimation {
        displayed_percent: 0.0,
        elapsed_secs: 0.0,
        preview_mode: initial_state
            .as_ref()
            .is_some_and(|state| state.0 == GameState::Loading),
    };
    let state = build_loading_state(
        current_zone.zone_id,
        !local_player_q.is_empty(),
        &adt_manager,
        &mut progress,
        0.0,
    );
    let layout = debug_loading_layout_from_source();
    let view = match spawn_loading_view(&mut commands, &mut assets, &state, &layout, 1) {
        Ok(view) => view,
        Err(error) => {
            error!("Cannot construct native loading screen: {error}");
            return;
        }
    };
    let mut orders = LoadingCameraOrders::default();
    for (entity, mut camera) in &mut cameras {
        orders.0.push((entity, camera.order));
        camera.order = 2;
    }
    commands.insert_resource(orders);
    commands.insert_resource(LoadingSnapshot { state, layout });
    commands.insert_resource(progress);
    commands.insert_resource(view);
}

// Initial state entry precedes the toolkit camera's Startup system.
fn raise_startup_ui_cameras(
    orders: Option<ResMut<LoadingCameraOrders>>,
    mut cameras: Query<(Entity, &mut Camera), With<UiCamera>>,
) {
    let Some(mut orders) = orders else {
        return;
    };
    for (entity, mut camera) in &mut cameras {
        if !orders.0.iter().any(|(saved, _)| *saved == entity) {
            orders.0.push((entity, camera.order));
        }
        camera.order = 2;
    }
}

fn teardown_loading_ui(world: &mut World) {
    if let Some(view) = world.remove_resource::<LoadingView>() {
        world.despawn(view.root);
        world.despawn(view.camera);
    }
    if let Some(orders) = world.remove_resource::<LoadingCameraOrders>() {
        for (entity, order) in orders.0 {
            if let Some(mut camera) = world.get_mut::<Camera>(entity) {
                camera.order = order;
            }
        }
    }
    world.remove_resource::<LoadingSnapshot>();
    world.remove_resource::<LoadingProgressAnimation>();
    if let Some(mut ui) = world.get_resource_mut::<UiState>() {
        ui.focused_frame = None;
    }
}

fn loading_update_visuals(world: &mut World) {
    let Some(view) = world.get_resource::<LoadingView>().cloned() else {
        return;
    };
    let zone_id = world.resource::<CurrentZone>().zone_id;
    let local_player_ready = !world
        .query_filtered::<(), With<LocalPlayer>>()
        .is_empty(world);
    let delta = world.resource::<Time>().delta_secs();
    let state = world.resource_scope(|world, mut progress: Mut<LoadingProgressAnimation>| {
        build_loading_state(
            zone_id,
            local_player_ready,
            world.resource::<AdtManager>(),
            &mut progress,
            delta,
        )
    });
    let layout = debug_loading_layout_from_source();
    let previous = world.resource::<LoadingSnapshot>();
    if previous.state == state && previous.layout == layout {
        return;
    }
    sync_loading_view(world, &view, &state, &layout);
    *world.resource_mut::<LoadingSnapshot>() = LoadingSnapshot { state, layout };
}

fn build_loading_state(
    zone_id: u32,
    local_player_ready: bool,
    adt_manager: &AdtManager,
    progress_animation: &mut LoadingProgressAnimation,
    delta_secs: f32,
) -> LoadingScreenState {
    let readiness = evaluate_world_loading(local_player_ready, adt_manager);
    let zone_text = if zone_id == 0 {
        DEFAULT_ZONE_TEXT.to_string()
    } else {
        format!("Entering {}", zone_id_to_name(zone_id))
    };
    progress_animation.elapsed_secs += delta_secs.max(0.0);
    progress_animation.displayed_percent = advance_displayed_progress(
        progress_animation.displayed_percent,
        target_progress_percent(&readiness, progress_animation),
        delta_secs,
    );
    LoadingScreenState {
        status_text: readiness.status_text.to_string(),
        zone_text,
        tip_text: DEFAULT_TIP_TEXT.to_string(),
        progress_percent: progress_animation.displayed_percent.round() as u8,
    }
}

fn target_progress_percent(
    readiness: &crate::game_state::LoadingReadiness,
    progress_animation: &LoadingProgressAnimation,
) -> f32 {
    if progress_animation.preview_mode {
        preview_target_progress(progress_animation.elapsed_secs)
    } else {
        f32::from(readiness.progress_percent)
    }
}

fn preview_target_progress(elapsed_secs: f32) -> f32 {
    (elapsed_secs.max(0.0) * LOADING_BAR_FILL_RATE_PERCENT_PER_SEC).min(PREVIEW_MODE_HOLD_PERCENT)
}

fn advance_displayed_progress(current: f32, target: f32, delta_secs: f32) -> f32 {
    if delta_secs <= 0.0 {
        return current.min(target);
    }
    if current >= target {
        return target;
    }
    let step = delta_secs * LOADING_BAR_FILL_RATE_PERCENT_PER_SEC;
    (current + step).min(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_mode_progress_fills_slowly_over_time() {
        assert_eq!(preview_target_progress(0.0), 0.0);
        assert_eq!(preview_target_progress(2.0), 12.0);
        assert_eq!(preview_target_progress(20.0), PREVIEW_MODE_HOLD_PERCENT);
    }

    #[test]
    fn displayed_progress_advances_toward_target_without_overshoot() {
        assert_eq!(advance_displayed_progress(0.0, 86.0, 1.0), 6.0);
        assert_eq!(advance_displayed_progress(80.0, 86.0, 1.0), 86.0);
        assert_eq!(advance_displayed_progress(90.0, 86.0, 1.0), 86.0);
    }

    #[test]
    fn loading_zone_text_uses_current_zone_name() {
        let mut progress = LoadingProgressAnimation {
            displayed_percent: 0.0,
            elapsed_secs: 0.0,
            preview_mode: false,
        };
        let state = build_loading_state(12, false, &AdtManager::default(), &mut progress, 0.0);
        assert_eq!(state.zone_text, "Entering Elwynn Forest");
    }

    #[test]
    fn loading_zone_text_falls_back_when_zone_unknown() {
        let mut progress = LoadingProgressAnimation {
            displayed_percent: 0.0,
            elapsed_secs: 0.0,
            preview_mode: false,
        };
        let state = build_loading_state(0, false, &AdtManager::default(), &mut progress, 0.0);
        assert_eq!(state.zone_text, DEFAULT_ZONE_TEXT);
    }
}
