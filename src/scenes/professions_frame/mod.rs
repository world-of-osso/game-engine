use bevy::prelude::*;
use game_engine::status::ProfessionStatusSnapshot;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::professions_frame_component::{
    ProfessionsFrameState, RecipeState, professions_frame_screen,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game_state::GameState;

/// Tracks whether the Professions panel is open.
#[derive(Resource, Default)]
pub struct ProfessionsFrameOpen(pub bool);

struct ProfessionsFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for ProfessionsFrameRes {}
unsafe impl Sync for ProfessionsFrameRes {}

#[derive(Resource)]
struct ProfessionsFrameWrap(ProfessionsFrameRes);

#[derive(Resource, Clone, PartialEq)]
struct ProfessionsFrameModel(ProfessionsFrameState);

pub struct ProfessionsFramePlugin;

impl Plugin for ProfessionsFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProfessionsFrameOpen>();
        app.add_systems(OnEnter(GameState::InWorld), build_professions_frame_ui);
        app.add_systems(OnExit(GameState::InWorld), teardown_professions_frame_ui);
        app.add_systems(
            Update,
            (toggle_professions_frame, sync_professions_frame_state)
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

fn build_professions_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    snapshot: Option<Res<ProfessionStatusSnapshot>>,
    open: Res<ProfessionsFrameOpen>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(snapshot.as_deref(), &open);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(professions_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(ProfessionsFrameWrap(ProfessionsFrameRes { screen, shared }));
    commands.insert_resource(ProfessionsFrameModel(state));
}

fn teardown_professions_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<ProfessionsFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<ProfessionsFrameWrap>();
    commands.remove_resource::<ProfessionsFrameModel>();
}

fn toggle_professions_frame(
    keys: Res<ButtonInput<KeyCode>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    mut open: ResMut<ProfessionsFrameOpen>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    if keys.just_pressed(KeyCode::KeyK) {
        open.0 = !open.0;
    }
}

fn sync_professions_frame_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<ProfessionsFrameWrap>>,
    mut last_model: Option<ResMut<ProfessionsFrameModel>>,
    snapshot: Option<Res<ProfessionStatusSnapshot>>,
    open: Res<ProfessionsFrameOpen>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let state = build_state(snapshot.as_deref(), &open);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn build_state(
    snapshot: Option<&ProfessionStatusSnapshot>,
    open: &ProfessionsFrameOpen,
) -> ProfessionsFrameState {
    let recipes = snapshot
        .map(|s| s.recipes.iter().map(recipe_entry_to_state).collect())
        .unwrap_or_default();
    ProfessionsFrameState {
        visible: open.0,
        recipes,
    }
}

fn recipe_entry_to_state(entry: &game_engine::status::ProfessionRecipeEntry) -> RecipeState {
    RecipeState {
        name: entry.name.clone(),
        profession: entry.profession.clone(),
        craftable: entry.craftable,
        cooldown: entry.cooldown.clone().unwrap_or_default(),
    }
}
