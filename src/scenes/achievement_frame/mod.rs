use bevy::prelude::*;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::achievement_frame_component::{
    AchievementCategory, AchievementFrameState, AchievementRow, AchievementTab,
    achievement_frame_screen,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game_state::GameState;
use game_engine::achievements::{
    AchievementCompletionState, achievements_for_category, build_category_tree, categories_for_tab,
};

/// Tracks whether the Achievement panel is open.
#[derive(Resource, Default)]
pub struct AchievementFrameOpen(pub bool);

struct AchievementFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for AchievementFrameRes {}
unsafe impl Sync for AchievementFrameRes {}

#[derive(Resource)]
struct AchievementFrameWrap(AchievementFrameRes);

#[derive(Resource, Clone, PartialEq)]
struct AchievementFrameModel(AchievementFrameState);

pub struct AchievementFramePlugin;

impl Plugin for AchievementFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AchievementFrameOpen>();
        app.init_resource::<AchievementCompletionState>();
        app.add_systems(OnEnter(GameState::InWorld), build_achievement_frame_ui);
        app.add_systems(OnExit(GameState::InWorld), teardown_achievement_frame_ui);
        app.add_systems(
            Update,
            (toggle_achievement_frame, sync_achievement_frame_state)
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

fn build_achievement_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    completion: Res<AchievementCompletionState>,
    open: Res<AchievementFrameOpen>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(&completion, &open);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(achievement_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(AchievementFrameWrap(AchievementFrameRes { screen, shared }));
    commands.insert_resource(AchievementFrameModel(state));
}

fn teardown_achievement_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<AchievementFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<AchievementFrameWrap>();
    commands.remove_resource::<AchievementFrameModel>();
}

fn toggle_achievement_frame(
    keys: Res<ButtonInput<KeyCode>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    mut open: ResMut<AchievementFrameOpen>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    if keys.just_pressed(KeyCode::KeyY) {
        open.0 = !open.0;
    }
}

fn sync_achievement_frame_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<AchievementFrameWrap>>,
    mut last_model: Option<ResMut<AchievementFrameModel>>,
    completion: Res<AchievementCompletionState>,
    open: Res<AchievementFrameOpen>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let state = build_state(&completion, &open);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn build_state(
    completion: &AchievementCompletionState,
    open: &AchievementFrameOpen,
) -> AchievementFrameState {
    let tab_cats = categories_for_tab(0);
    let first_cat_id = tab_cats.first().map(|c| c.id);
    AchievementFrameState {
        visible: open.0,
        tabs: default_tabs(),
        categories: build_sidebar_categories(first_cat_id),
        achievements: build_achievement_rows(first_cat_id, completion),
        total_points: sum_earned_points(completion),
    }
}

fn default_tabs() -> Vec<AchievementTab> {
    vec![
        AchievementTab {
            name: "Achievements".to_string(),
            active: true,
        },
        AchievementTab {
            name: "Statistics".to_string(),
            active: false,
        },
    ]
}

fn build_sidebar_categories(selected_id: Option<i32>) -> Vec<AchievementCategory> {
    build_category_tree()
        .iter()
        .map(|(id, name, is_child)| AchievementCategory {
            name: name.to_string(),
            is_child: *is_child,
            selected: Some(*id) == selected_id,
        })
        .collect()
}

fn build_achievement_rows(
    category_id: Option<i32>,
    completion: &AchievementCompletionState,
) -> Vec<AchievementRow> {
    let Some(cat_id) = category_id else {
        return vec![];
    };
    achievements_for_category(cat_id)
        .iter()
        .map(|def| def_to_row(def, completion))
        .collect()
}

fn def_to_row(
    def: &game_engine::achievements::AchievementDef,
    completion: &AchievementCompletionState,
) -> AchievementRow {
    let completed = completion.earned.contains(&def.id);
    let (current, required) = completion
        .progress
        .get(&def.id)
        .copied()
        .unwrap_or((if completed { def.points } else { 0 }, def.points));
    let progress = if required == 0 {
        if completed { 1.0 } else { 0.0 }
    } else {
        current as f32 / required as f32
    };
    AchievementRow {
        name: def.name.to_string(),
        description: def.description.to_string(),
        points: def.points,
        icon_fdid: def.icon_fdid,
        completed,
        progress,
        progress_text: format!("{current} / {required}"),
    }
}

fn sum_earned_points(completion: &AchievementCompletionState) -> u32 {
    completion
        .earned
        .iter()
        .filter_map(|id| {
            game_engine::achievements::ACHIEVEMENTS
                .iter()
                .find(|a| a.id == *id)
                .map(|a| a.points)
        })
        .sum()
}
