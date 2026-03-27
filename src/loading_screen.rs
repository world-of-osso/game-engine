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

use crate::game_state::{GameState, evaluate_world_loading};
use crate::networking::{LocalPlayer, SelectedCharacterId};
use crate::terrain::AdtManager;

const DEFAULT_ZONE_TEXT: &str = "Entering Elwynn Forest";
const DEFAULT_TIP_TEXT: &str =
    "Tip: The first zone load streams terrain and replicated actors before gameplay begins.";

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
    selected: Res<SelectedCharacterId>,
    local_player_q: Query<(), With<LocalPlayer>>,
    adt_manager: Res<AdtManager>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_loading_state(&selected, !local_player_q.is_empty(), &adt_manager);
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
    selected: Res<SelectedCharacterId>,
    local_player_q: Query<(), With<LocalPlayer>>,
    adt_manager: Res<AdtManager>,
) {
    let (Some(mut screen_wrap), Some(mut last_state), Some(mut last_layout)) =
        (screen_wrap.take(), last_state.take(), last_layout.take())
    else {
        return;
    };

    let state = build_loading_state(&selected, !local_player_q.is_empty(), &adt_manager);
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
    selected: &SelectedCharacterId,
    local_player_ready: bool,
    adt_manager: &AdtManager,
) -> LoadingScreenState {
    let readiness = evaluate_world_loading(local_player_ready, adt_manager);
    let zone_text = selected
        .character_name
        .as_ref()
        .map(|name| format!("Entering {name}"))
        .unwrap_or_else(|| DEFAULT_ZONE_TEXT.to_string());

    LoadingScreenState {
        status_text: readiness.status_text.to_string(),
        zone_text,
        tip_text: DEFAULT_TIP_TEXT.to_string(),
        progress_percent: readiness.progress_percent,
    }
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

    #[test]
    fn loading_screen_builds_expected_frames() {
        let mut shared = ui_toolkit::screen::SharedContext::new();
        shared.insert(LoadingScreenState {
            status_text: "Loading terrain...".to_string(),
            zone_text: "Entering Elwynn Forest".to_string(),
            tip_text: DEFAULT_TIP_TEXT.to_string(),
            progress_percent: 86,
        });
        shared.insert(LoadingScreenLayout::default());

        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(loading_screen);
        screen.sync(&shared, &mut reg);

        assert!(reg.get_by_name("LoadingRoot").is_some());
        assert!(reg.get_by_name("LoadingBarFill").is_some());
        assert!(reg.get_by_name("LoadingStatusText").is_some());
    }

    #[test]
    fn loading_bar_fill_clip_starts_at_shell_inner_left_edge() {
        let mut shared = ui_toolkit::screen::SharedContext::new();
        shared.insert(LoadingScreenState {
            status_text: "Loading terrain...".to_string(),
            zone_text: "Entering Elwynn Forest".to_string(),
            tip_text: DEFAULT_TIP_TEXT.to_string(),
            progress_percent: 50,
        });
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

        assert_eq!(bar_clip.x, bar_bg.x + layout.bar_cap_width);
        assert_eq!(bar_clip.width, layout.bar_width - (layout.bar_cap_width * 2.0));
    }
}
