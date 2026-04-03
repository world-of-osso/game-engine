use bevy::prelude::*;

use game_engine::ui::frame::Dimension;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screen::Screen;
use game_engine::ui::screens::loading_component::{
    LOADING_ROOT, LoadingScreenLayout, LoadingScreenState, debug_loading_layout_from_source,
    loading_screen,
};
use game_engine::ui_resource;

use crate::game_state::{GameState, InitialGameState, evaluate_world_loading};
use crate::networking::{CurrentZone, LocalPlayer};
use crate::terrain::AdtManager;
use crate::zone_names::zone_id_to_name;

const DEFAULT_ZONE_TEXT: &str = "Entering Elwynn Forest";
const DEFAULT_TIP_TEXT: &str =
    "Tip: The first zone load streams terrain and replicated actors before gameplay begins.";
const LOADING_BAR_FILL_RATE_PERCENT_PER_SEC: f32 = 6.0;
const PREVIEW_MODE_HOLD_PERCENT: f32 = 100.0;

ui_resource! {
    LoadingUi {
        root: LOADING_ROOT,
        bar_fill: "LoadingBarFill",
        status_text: "LoadingStatusText",
        progress_text: "LoadingProgressText",
    }
}

struct LoadingScreenRes {
    screen: Screen,
    shared: ui_toolkit::screen::SharedContext,
}

unsafe impl Send for LoadingScreenRes {}
unsafe impl Sync for LoadingScreenRes {}

#[derive(Resource)]
struct LoadingScreenWrap(LoadingScreenRes);

#[derive(Resource, Clone, PartialEq, Eq)]
struct LoadingUiState(LoadingScreenState);

#[derive(Resource, Clone, PartialEq)]
struct LoadingLayoutState(LoadingScreenLayout);

#[derive(Resource)]
struct LoadingProgressAnimation {
    displayed_percent: f32,
    elapsed_secs: f32,
    preview_mode: bool,
}

pub struct LoadingScreenPlugin;

impl Plugin for LoadingScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Loading), build_loading_ui);
        app.add_systems(OnExit(GameState::Loading), teardown_loading_ui);
        app.add_systems(
            Update,
            (loading_sync_root_size, loading_update_visuals).run_if(in_state(GameState::Loading)),
        );
    }
}

fn build_loading_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    current_zone: Res<CurrentZone>,
    local_player_q: Query<(), With<LocalPlayer>>,
    adt_manager: Res<AdtManager>,
    initial_state: Option<Res<InitialGameState>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let mut progress_animation = LoadingProgressAnimation {
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
        &mut progress_animation,
        0.0,
    );
    let layout = debug_loading_layout_from_source();
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(state.clone());
    shared.insert(layout.clone());
    let mut screen = Screen::new(loading_screen);
    screen.sync(&shared, &mut ui.registry);

    let loading_ui = LoadingUi::resolve(&ui.registry);
    apply_post_setup(&mut ui.registry, loading_ui.root);

    commands.insert_resource(LoadingUiState(state));
    commands.insert_resource(LoadingLayoutState(layout));
    commands.insert_resource(progress_animation);
    commands.insert_resource(LoadingScreenWrap(LoadingScreenRes { screen, shared }));
    commands.insert_resource(loading_ui);
}

fn teardown_loading_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut screen: Option<ResMut<LoadingScreenWrap>>,
) {
    if let Some(res) = screen.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<LoadingScreenWrap>();
    commands.remove_resource::<LoadingUi>();
    commands.remove_resource::<LoadingUiState>();
    commands.remove_resource::<LoadingLayoutState>();
    commands.remove_resource::<LoadingProgressAnimation>();
    ui.focused_frame = None;
}

fn loading_sync_root_size(
    mut ui: ResMut<UiState>,
    loading_ui: Option<Res<LoadingUi>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    let Some(loading_ui) = loading_ui else {
        return;
    };
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    apply_post_setup(&mut ui.registry, loading_ui.root);
}

fn loading_update_visuals(
    mut ui: ResMut<UiState>,
    mut screen_wrap: Option<ResMut<LoadingScreenWrap>>,
    mut last_state: Option<ResMut<LoadingUiState>>,
    mut last_layout: Option<ResMut<LoadingLayoutState>>,
    mut progress_animation: Option<ResMut<LoadingProgressAnimation>>,
    current_zone: Res<CurrentZone>,
    local_player_q: Query<(), With<LocalPlayer>>,
    adt_manager: Res<AdtManager>,
    time: Res<Time>,
) {
    let (
        Some(mut screen_wrap),
        Some(mut last_state),
        Some(mut last_layout),
        Some(mut progress_animation),
    ) = (
        screen_wrap.take(),
        last_state.take(),
        last_layout.take(),
        progress_animation.take(),
    )
    else {
        return;
    };

    let state = build_loading_state(
        current_zone.zone_id,
        !local_player_q.is_empty(),
        &adt_manager,
        &mut progress_animation,
        time.delta_secs(),
    );
    let layout = debug_loading_layout_from_source();
    if last_state.0 == state && last_layout.0 == layout {
        return;
    }

    last_state.0 = state.clone();
    last_layout.0 = layout.clone();
    let res = &mut screen_wrap.0;
    res.shared.insert(state);
    res.shared.insert(layout.clone());
    res.screen.sync(&res.shared, &mut ui.registry);
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

fn apply_post_setup(reg: &mut FrameRegistry, root_id: u64) {
    let width = reg.screen_width;
    let height = reg.screen_height;
    if let Some(root) = reg.get_mut(root_id) {
        root.width = Dimension::Fixed(width);
        root.height = Dimension::Fixed(height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine::ui::layout::recompute_layouts;

    fn sample_loading_state(progress_percent: u8) -> LoadingScreenState {
        LoadingScreenState {
            status_text: "Loading terrain...".to_string(),
            zone_text: "Entering Elwynn Forest".to_string(),
            tip_text: DEFAULT_TIP_TEXT.to_string(),
            progress_percent,
        }
    }

    #[test]
    fn loading_screen_builds_expected_frames() {
        let mut shared = ui_toolkit::screen::SharedContext::new();
        shared.insert(sample_loading_state(86));
        shared.insert(LoadingScreenLayout::default());

        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(loading_screen);
        screen.sync(&shared, &mut reg);

        assert!(reg.get_by_name("LoadingRoot").is_some());
        assert!(reg.get_by_name("LoadingBarFill").is_some());
        assert!(reg.get_by_name("LoadingStatusText").is_some());
    }

    #[test]
    fn loading_bar_fill_clip_uses_configured_fill_offset() {
        let mut shared = ui_toolkit::screen::SharedContext::new();
        shared.insert(sample_loading_state(50));
        let layout = LoadingScreenLayout::default();
        shared.insert(layout.clone());

        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(loading_screen);
        screen.sync(&shared, &mut reg);
        recompute_layouts(&mut reg);

        let bar_bg = reg
            .get_by_name("LoadingBarBackground")
            .and_then(|id| reg.get(id))
            .and_then(|frame| frame.layout_rect.as_ref())
            .expect("LoadingBarBackground rect");
        let bar_clip = reg
            .get_by_name("LoadingBarFillClip")
            .and_then(|id| reg.get(id))
            .and_then(|frame| frame.layout_rect.as_ref())
            .expect("LoadingBarFillClip rect");

        assert_eq!(bar_clip.x, bar_bg.x + layout.bar_fill_start_x);
        assert_eq!(bar_clip.width, layout.bar_fill_max_width);
    }

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
