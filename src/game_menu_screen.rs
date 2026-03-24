use bevy::prelude::*;

use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::game_menu_component::{GAME_MENU_ROOT, game_menu_screen};
use game_engine::ui_resource;
use ui_toolkit::screen::Screen;

use crate::game_state::GameState;

ui_resource! {
    pub(crate) GameMenuUi {
        root: GAME_MENU_ROOT,
    }
}

struct GameMenuScreenRes {
    screen: Screen,
}

unsafe impl Send for GameMenuScreenRes {}
unsafe impl Sync for GameMenuScreenRes {}

#[derive(Resource)]
struct GameMenuScreenWrap(GameMenuScreenRes);

pub struct GameMenuScreenPlugin;

impl Plugin for GameMenuScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::GameMenu), build_game_menu_ui);
        app.add_systems(OnExit(GameState::GameMenu), teardown_game_menu_ui);
    }
}

fn build_game_menu_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);

    let shared = ui_toolkit::screen::SharedContext::new();
    let mut screen = Screen::new(game_menu_screen);
    screen.sync(&shared, &mut ui.registry);

    let menu = GameMenuUi::resolve(&ui.registry);
    commands.insert_resource(GameMenuScreenWrap(GameMenuScreenRes { screen }));
    commands.insert_resource(menu);
}

fn teardown_game_menu_ui(
    mut ui: ResMut<UiState>,
    mut screen: Option<ResMut<GameMenuScreenWrap>>,
    mut commands: Commands,
) {
    if let Some(res) = screen.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<GameMenuScreenWrap>();
    commands.remove_resource::<GameMenuUi>();
    ui.focused_frame = None;
}
