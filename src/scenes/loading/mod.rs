use bevy::prelude::*;

use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screen::Screen;
use game_engine::ui::screens::loading_component::{
    DEFAULT_TIP_TEXT, DEFAULT_ZONE_TEXT, LOADING_BAR_FILL_RATE_PERCENT_PER_SEC, LOADING_ROOT,
    LoadingScreenLayout, LoadingScreenState, LoadingViewportHeight, advance_displayed_progress,
    debug_loading_layout_from_source, loading_screen,
};
use game_engine::ui_resource;

use crate::game_state::{GameState, InitialGameState, evaluate_world_loading};
use crate::networking::{CurrentZone, LocalPlayer};
use crate::terrain::AdtManager;
use crate::zone_names::zone_id_to_name;

#[cfg(test)]
#[path = "../../ui/screens/menu_character_layout_test_support.rs"]
mod layout_support;

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
struct LoadingLayoutState(LoadingScreenLayout, LoadingViewportHeight);

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
            loading_update_visuals.run_if(in_state(GameState::Loading)),
        );
    }
}

fn build_loading_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    current_zone: Res<CurrentZone>,
    local_player_q: Query<(), With<LocalPlayer>>,
    player_q: Query<&Transform, With<crate::camera::Player>>,
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
        player_q.single().ok(),
        &mut progress_animation,
        0.0,
    );
    let layout = debug_loading_layout_from_source();
    let viewport = LoadingViewportHeight(ui.registry.screen_height);
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(state.clone());
    shared.insert(layout.clone());
    shared.insert(viewport);
    let mut screen = Screen::new(loading_screen);
    screen.sync(&shared, &mut ui.registry);

    let loading_ui = LoadingUi::resolve(&ui.registry);

    commands.insert_resource(LoadingUiState(state));
    commands.insert_resource(LoadingLayoutState(layout, viewport));
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

fn loading_update_visuals(
    mut ui: ResMut<UiState>,
    mut screen_wrap: Option<ResMut<LoadingScreenWrap>>,
    mut last_state: Option<ResMut<LoadingUiState>>,
    mut last_layout: Option<ResMut<LoadingLayoutState>>,
    mut progress_animation: Option<ResMut<LoadingProgressAnimation>>,
    current_zone: Res<CurrentZone>,
    local_player_q: Query<(), With<LocalPlayer>>,
    player_q: Query<&Transform, With<crate::camera::Player>>,
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
        player_q.single().ok(),
        &mut progress_animation,
        time.delta_secs(),
    );
    let layout = LoadingLayoutState(
        debug_loading_layout_from_source(),
        LoadingViewportHeight(ui.registry.screen_height),
    );
    if last_state.0 == state && *last_layout == layout {
        return;
    }

    last_state.0 = state.clone();
    *last_layout = layout.clone();
    let res = &mut screen_wrap.0;
    res.shared.insert(state);
    res.shared.insert(layout.0);
    res.shared.insert(layout.1);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn build_loading_state(
    zone_id: u32,
    local_player_ready: bool,
    adt_manager: &AdtManager,
    player: Option<&Transform>,
    progress_animation: &mut LoadingProgressAnimation,
    delta_secs: f32,
) -> LoadingScreenState {
    let readiness = evaluate_world_loading(local_player_ready, adt_manager, player);
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

#[cfg(test)]
mod tests {
    use super::layout_support::compute_layout as recompute_layouts;
    use super::*;
    use game_engine::ui::registry::FrameRegistry;

    fn sample_loading_state(progress_percent: u8) -> LoadingScreenState {
        LoadingScreenState {
            status_text: "Loading terrain...".to_string(),
            zone_text: "Entering Elwynn Forest".to_string(),
            tip_text: DEFAULT_TIP_TEXT.to_string(),
            progress_percent,
        }
    }

    fn laid_out_loading_screen(width: f32, height: f32, progress_percent: u8) -> FrameRegistry {
        let mut shared = ui_toolkit::screen::SharedContext::new();
        shared.insert(sample_loading_state(progress_percent));
        shared.insert(LoadingScreenLayout::default());
        shared.insert(LoadingViewportHeight(height));
        let mut reg = FrameRegistry::new(width, height);
        reg.register_three_slice_style(
            "loading_bar_shell",
            game_engine::ui::screens::loading_component::loading_bar_shell(),
        );
        Screen::new(loading_screen).sync(&shared, &mut reg);
        recompute_layouts(&mut reg);
        reg
    }

    fn rect(reg: &FrameRegistry, name: &str) -> game_engine::ui::layout::LayoutRect {
        reg.get_by_name(name)
            .and_then(|id| reg.get(id))
            .and_then(|frame| frame.layout_rect.clone())
            .unwrap_or_else(|| panic!("{name} rect"))
    }

    const LOADING_ELEMENTS: [&str; 6] = [
        "LoadingLogo",
        "LoadingZoneText",
        "LoadingBarBackground",
        "LoadingStatusText",
        "LoadingProgressText",
        "LoadingTipText",
    ];

    fn assert_loading_screen_fits(width: f32, height: f32) {
        let reg = laid_out_loading_screen(width, height, 40);
        for name in LOADING_ELEMENTS {
            let r = rect(&reg, name);
            assert!(
                r.x >= 0.0 && r.y >= 0.0 && r.x + r.width <= width && r.y + r.height <= height,
                "{name} {r:?} escapes {width}x{height}"
            );
        }
        let art = rect(&reg, "LoadingArtwork");
        assert_eq!((art.y, art.height), (0.0, height), "artwork fills height");
        assert!((art.x + art.width / 2.0 - width / 2.0).abs() < 0.5);

        let (logo, zone, bar, tip) = (
            rect(&reg, "LoadingLogo"),
            rect(&reg, "LoadingZoneText"),
            rect(&reg, "LoadingBarBackground"),
            rect(&reg, "LoadingTipText"),
        );
        assert!(logo.y + logo.height <= zone.y, "logo overlaps zone text");
        assert!(zone.y + zone.height <= bar.y, "zone text overlaps bar");
        assert!(bar.y + bar.height <= tip.y, "bar overlaps tip text");
        for inside in ["LoadingStatusText", "LoadingProgressText"] {
            let r = rect(&reg, inside);
            assert!(r.y >= bar.y && r.y + r.height <= bar.y + bar.height + 1.0);
        }
    }

    #[test]
    fn loading_screen_fits_1280x720() {
        assert_loading_screen_fits(1280.0, 720.0);
    }

    #[test]
    fn loading_screen_fits_1920x1080() {
        assert_loading_screen_fits(1920.0, 1080.0);
    }

    #[test]
    fn loading_screen_builds_expected_frames() {
        let mut shared = ui_toolkit::screen::SharedContext::new();
        shared.insert(sample_loading_state(86));
        shared.insert(LoadingScreenLayout::default());
        shared.insert(LoadingViewportHeight(1080.0));

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
        shared.insert(LoadingViewportHeight(1080.0));

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
    fn loading_bar_fill_width_scales_with_progress_percent() {
        let mut shared = ui_toolkit::screen::SharedContext::new();
        shared.insert(sample_loading_state(50));
        let layout = LoadingScreenLayout::default();
        shared.insert(layout.clone());
        shared.insert(LoadingViewportHeight(1080.0));

        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(loading_screen);
        screen.sync(&shared, &mut reg);
        recompute_layouts(&mut reg);

        let fill = reg
            .get_by_name("LoadingBarFill")
            .and_then(|id| reg.get(id))
            .and_then(|frame| frame.layout_rect.as_ref())
            .expect("LoadingBarFill rect");

        assert_eq!(fill.width, layout.bar_fill_max_width * 0.5);
        assert_eq!(fill.height, layout.bar_fill_height);
    }

    #[test]
    fn loading_progress_text_shows_current_percent() {
        let mut shared = ui_toolkit::screen::SharedContext::new();
        shared.insert(sample_loading_state(86));
        shared.insert(LoadingScreenLayout::default());
        shared.insert(LoadingViewportHeight(1080.0));

        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(loading_screen);
        screen.sync(&shared, &mut reg);

        let progress = reg
            .get_by_name("LoadingProgressText")
            .and_then(|id| reg.get(id))
            .expect("LoadingProgressText frame");

        let Some(game_engine::ui::frame::WidgetData::FontString(text)) =
            progress.widget_data.as_ref()
        else {
            panic!("LoadingProgressText should be a FontString");
        };

        assert_eq!(text.text, "86%");
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
        let state =
            build_loading_state(12, false, &AdtManager::default(), None, &mut progress, 0.0);
        assert_eq!(state.zone_text, "Entering Elwynn Forest");
    }

    #[test]
    fn loading_zone_text_falls_back_when_zone_unknown() {
        let mut progress = LoadingProgressAnimation {
            displayed_percent: 0.0,
            elapsed_secs: 0.0,
            preview_mode: false,
        };
        let state = build_loading_state(0, false, &AdtManager::default(), None, &mut progress, 0.0);
        assert_eq!(state.zone_text, DEFAULT_ZONE_TEXT);
    }
}
