use bevy::prelude::*;

use game_engine::ui::input::find_frame_at;
use game_engine::ui::plugin::UiState;
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

/// When present, the game menu overlay is visible.
/// Insert this resource to show the menu, remove to hide.
#[derive(Resource)]
pub struct GameMenuOverlay {
    wrap: GameMenuScreenRes,
}

pub struct GameMenuScreenPlugin;

impl Plugin for GameMenuScreenPlugin {
    fn build(&self, app: &mut App) {
        // Keep the standalone --screen menu state for testing
        app.add_systems(OnEnter(GameState::GameMenu), open_menu_overlay);
        app.add_systems(OnExit(GameState::GameMenu), close_menu_overlay);
        // Overlay input runs on ANY state when the overlay is present
        app.add_systems(
            Update,
            handle_overlay_input.run_if(resource_exists::<GameMenuOverlay>),
        );
    }
}

/// Open the overlay (callable from any screen).
pub fn open_game_menu(ui: &mut UiState, commands: &mut Commands, game_state: GameState) {
    // Check if already open by looking for the root frame
    if ui.registry.get_by_name("GameMenuRoot").is_some() {
        return;
    }
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(game_state);
    let mut screen = Screen::new(game_menu_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(GameMenuOverlay {
        wrap: GameMenuScreenRes { screen },
    });
}

/// Close the overlay (callable from any screen).
pub fn close_game_menu(commands: &mut Commands) {
    commands.queue(CloseMenuCommand);
}

struct CloseMenuCommand;

impl Command for CloseMenuCommand {
    fn apply(self, world: &mut World) {
        let Some(mut overlay) = world.remove_resource::<GameMenuOverlay>() else {
            return;
        };
        let Some(mut ui) = world.get_resource_mut::<UiState>() else {
            return;
        };
        overlay.wrap.screen.teardown(&mut ui.registry);
    }
}

// --- Standalone test screen (--screen menu) ---

fn open_menu_overlay(mut ui: ResMut<UiState>, mut commands: Commands) {
    open_game_menu(&mut ui, &mut commands, GameState::GameMenu);
}

fn close_menu_overlay(mut commands: Commands) {
    close_game_menu(&mut commands);
}

// --- Overlay input handling ---

fn handle_overlay_input(
    mut ui: ResMut<UiState>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut exit: MessageWriter<AppExit>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    mut commands: Commands,
    state: Res<State<GameState>>,
) {
    if let Some(ref kb) = keyboard {
        if kb.just_pressed(KeyCode::Escape) {
            close_game_menu(&mut commands);
            return;
        }
    }
    let Some(mouse) = mouse else { return };
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(pos) = window.cursor_position() else {
        return;
    };
    let Some(frame_id) = find_frame_at(&ui.registry, pos.x, pos.y) else {
        return;
    };
    let action = walk_up_for_onclick(&ui.registry, frame_id);
    if let Some(action) = action {
        dispatch_overlay_action(&action, &mut exit, &mut commands, &state);
    }
    ui.registry.click_frame(frame_id);
}

fn walk_up_for_onclick(
    reg: &game_engine::ui::registry::FrameRegistry,
    mut id: u64,
) -> Option<String> {
    loop {
        let frame = reg.get(id)?;
        if let Some(ref onclick) = frame.onclick {
            return Some(onclick.clone());
        }
        id = frame.parent_id?;
    }
}

fn dispatch_overlay_action(
    action: &str,
    exit: &mut MessageWriter<AppExit>,
    commands: &mut Commands,
    state: &State<GameState>,
) {
    match action {
        ACTION_EXIT => {
            exit.write(AppExit::Success);
        }
        ACTION_LOGOUT => {
            close_game_menu(commands);
            commands.queue(SetStateCommand(GameState::Login));
        }
        ACTION_RESUME => close_game_menu(commands),
        ACTION_OPTIONS => info!("Options: not implemented yet"),
        ACTION_SUPPORT => info!("Support: not implemented yet"),
        ACTION_ADDONS => info!("AddOns: not implemented yet"),
        _ => {
            // For the standalone test screen, handle state-based resume
            if *state.get() == GameState::GameMenu {
                warn!("Unknown menu action in test mode: {action}");
            }
        }
    }
}

struct SetStateCommand(GameState);

impl Command for SetStateCommand {
    fn apply(self, world: &mut World) {
        if let Some(mut next) = world.get_resource_mut::<NextState<GameState>>() {
            next.set(self.0);
        }
    }
}
