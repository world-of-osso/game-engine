use bevy::prelude::*;

use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::trash_button_component::{TRASH_BUTTON_ROOT, trash_button_screen};
use game_engine::ui_resource;
use ui_toolkit::screen::Screen;

use crate::game_state::GameState;

ui_resource! {
    pub(crate) TrashButtonUi {
        root: TRASH_BUTTON_ROOT,
    }
}

struct TrashButtonScreenRes {
    screen: Screen,
}

unsafe impl Send for TrashButtonScreenRes {}
unsafe impl Sync for TrashButtonScreenRes {}

#[derive(Resource)]
struct TrashButtonScreenWrap(TrashButtonScreenRes);

pub struct TrashButtonScreenPlugin;

impl Plugin for TrashButtonScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::TrashButton), build_trash_button_ui);
        app.add_systems(OnExit(GameState::TrashButton), teardown_trash_button_ui);
    }
}

fn build_trash_button_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);

    let shared = ui_toolkit::screen::SharedContext::new();
    let mut screen = Screen::new(trash_button_screen);
    screen.sync(&shared, &mut ui.registry);

    let tb = TrashButtonUi::resolve(&ui.registry);
    commands.insert_resource(TrashButtonScreenWrap(TrashButtonScreenRes { screen }));
    commands.insert_resource(tb);
}

fn teardown_trash_button_ui(
    mut ui: ResMut<UiState>,
    mut screen: Option<ResMut<TrashButtonScreenWrap>>,
    mut commands: Commands,
) {
    if let Some(res) = screen.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<TrashButtonScreenWrap>();
    commands.remove_resource::<TrashButtonUi>();
    ui.focused_frame = None;
}
