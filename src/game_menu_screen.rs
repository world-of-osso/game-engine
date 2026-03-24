use bevy::prelude::*;

use game_engine::ui::input::find_frame_at;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::game_menu_component::{
    ACTION_ADDONS, ACTION_EXIT, ACTION_LOGOUT, ACTION_OPTIONS, ACTION_RESUME, ACTION_SUPPORT,
    GAME_MENU_ROOT, game_menu_screen,
};
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
        app.add_systems(
            Update,
            handle_menu_input.run_if(in_state(GameState::GameMenu)),
        );
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

fn handle_menu_input(
    mut ui: ResMut<UiState>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<AppExit>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
) {
    if let Some(ref kb) = keyboard {
        if kb.just_pressed(KeyCode::Escape) {
            next_state.set(GameState::InWorld);
            return;
        }
    }
    let Some(mouse) = mouse else { return };
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(pos) = window.cursor_position() else { return };
    let Some(frame_id) = find_frame_at(&ui.registry, pos.x, pos.y) else { return };
    let action = walk_up_for_onclick(&ui.registry, frame_id);
    if let Some(action) = action {
        dispatch_action(&action, &mut next_state, &mut exit);
    }
    ui.registry.click_frame(frame_id);
}

fn walk_up_for_onclick(
    reg: &game_engine::ui::registry::FrameRegistry,
    mut id: u64,
) -> Option<String> {
    loop {
        if let Some(frame) = reg.get(id) {
            if let Some(ref onclick) = frame.onclick {
                return Some(onclick.clone());
            }
            match frame.parent_id {
                Some(pid) => id = pid,
                None => return None,
            }
        } else {
            return None;
        }
    }
}

fn dispatch_action(
    action: &str,
    next_state: &mut NextState<GameState>,
    exit: &mut MessageWriter<AppExit>,
) {
    match action {
        ACTION_EXIT => { exit.write(AppExit::Success); }
        ACTION_LOGOUT => { next_state.set(GameState::Login); }
        ACTION_RESUME => { next_state.set(GameState::InWorld); }
        ACTION_OPTIONS => info!("Options: not implemented yet"),
        ACTION_SUPPORT => info!("Support: not implemented yet"),
        ACTION_ADDONS => info!("AddOns: not implemented yet"),
        _ => warn!("Unknown menu action: {action}"),
    }
}
