use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use game_engine::ui::automation::{UiAutomationAction, UiAutomationQueue, UiAutomationRunner};
use game_engine::ui::frame::Dimension;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::char_create_component::{
    BACK_BUTTON, CHAR_CREATE_ROOT, CREATE_BUTTON, CREATE_NAME_INPUT, CharCreateAction,
    CharCreateMode, CharCreateUiState, ERROR_TEXT, NEXT_BUTTON, RANDOM_NAME_BUTTON,
    RANDOMIZE_BUTTON, apply_character_create_styles, char_create_screen,
};
use game_engine::ui_resource;
use shared::protocol::{AuthChannel, CreateCharacter};
use ui_toolkit::screen::Screen;

use crate::game_state::GameState;
use crate::scenes::login::helpers;
use game_engine::customization_data::CustomizationDb;
use helpers::{
    editbox_backspace, editbox_cursor_end, editbox_cursor_home, editbox_delete,
    editbox_move_cursor, get_editbox_text, hit_frame, insert_char_into_editbox, set_button_hovered,
    set_editbox_text,
};

/// Paths the shared creation logic resolves through `super::deps`.
mod deps {
    pub(crate) use game_engine::appearance_options;
    pub(crate) use game_engine::char_create_data;
    pub(crate) use game_engine::customization_data::{
        CustomizationCatalog as CustomizationDb, CustomizationChoice, CustomizationOption,
        ModelPresentation, OptionType, RequiredChoices,
    };
    pub(crate) use game_engine::ui::screens::char_create_component;
    #[cfg(test)]
    pub(crate) const NAME_GEN_CSV: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/data/NameGen.csv");
}
mod camera_orbit;
mod icon_masks;
mod input;
mod logic;
pub(crate) use logic::*;
pub mod scene;

use input::{
    char_create_keyboard_input, char_create_mouse_input, char_create_run_automation,
    hit_active_frame,
};
pub use scene::CharCreateScenePlugin;

ui_resource! {
    pub(crate) CharCreateUi {
        root: CHAR_CREATE_ROOT,
        back_button: BACK_BUTTON,
        next_button ?: NEXT_BUTTON,
        randomize_button ?: RANDOMIZE_BUTTON,
        create_button ?: CREATE_BUTTON,
        name_input ?: CREATE_NAME_INPUT,
        error_text ?: ERROR_TEXT,
    }
}

/// Bevy resource holding the shared creation state.
#[derive(Resource, Default, Deref, DerefMut)]
pub(crate) struct CharCreateStateRes(pub(crate) CharCreateState);

#[derive(Resource)]
pub(crate) struct NameCatalogResource(pub(crate) Result<NameCatalog, String>);

impl NameCatalogResource {
    fn catalog(&self) -> Result<&NameCatalog, &str> {
        self.0.as_ref().map_err(String::as_str)
    }
}

#[derive(Resource, Default)]
struct CharCreateFocus(Option<u64>);

#[derive(Resource, Clone, Copy)]
pub(crate) struct StartupCharCreateMode(pub(crate) CharCreateMode);

struct CharCreateScreenRes {
    screen: Screen,
    shared: ui_toolkit::screen::SharedContext,
}
unsafe impl Send for CharCreateScreenRes {}
unsafe impl Sync for CharCreateScreenRes {}

#[derive(Resource)]
struct CharCreateScreenWrap(CharCreateScreenRes);

pub struct CharCreatePlugin;

impl Plugin for CharCreatePlugin {
    fn build(&self, app: &mut App) {
        let name_catalog = NameCatalog::load(std::path::Path::new("data/NameGen.csv"));
        if let Err(err) = &name_catalog {
            error!("Random name control unavailable: {err}");
        }
        app.insert_resource(NameCatalogResource(name_catalog));
        app.init_resource::<game_engine::ui::character_creation_icons::CharacterCreationIconMasks>(
        );
        app.add_systems(OnEnter(GameState::CharCreate), build_char_create_ui);
        app.add_systems(OnExit(GameState::CharCreate), teardown_char_create_ui);
        app.add_observer(handle_create_response);
        app.add_systems(
            Update,
            (
                char_create_mouse_input,
                char_create_keyboard_input,
                char_create_run_automation,
                char_create_hover_visuals,
                char_create_update_visuals,
                icon_masks::mask_character_create_icons,
            )
                .chain()
                .run_if(in_state(GameState::CharCreate)),
        );
    }
}

// --- UI Building ---

fn build_char_create_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    startup_mode: Option<Res<StartupCharCreateMode>>,
    cust_db: Res<CustomizationDb>,
    name_catalog: Res<NameCatalogResource>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let initial_state = initial_state(startup_mode.as_deref().map(|mode| mode.0), &cust_db);
    let ui_state = ui_state(
        &initial_state,
        &cust_db,
        name_catalog.catalog(),
        false,
        (
            ui.registry.screen_width as u32,
            ui.registry.screen_height as u32,
        ),
    );
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(ui_state);
    let mut screen = Screen::new(char_create_screen);
    screen.sync(&shared, &mut ui.registry);
    apply_character_create_styles(&mut ui.registry, initial_state.open_dropdown);

    let cc = CharCreateUi::resolve(&ui.registry);
    apply_post_setup(&mut ui.registry, &cc);

    commands.insert_resource(CharCreateStateRes(initial_state));
    commands.init_resource::<CharCreateFocus>();
    commands.insert_resource(CharCreateScreenWrap(CharCreateScreenRes { screen, shared }));
    commands.insert_resource(cc);
    commands.remove_resource::<crate::game_state::StartupScreenTarget>();
    commands.remove_resource::<StartupCharCreateMode>();
}

fn apply_post_setup(reg: &mut FrameRegistry, cc: &CharCreateUi) {
    let (sw, sh) = (reg.screen_width, reg.screen_height);
    if let Some(frame) = reg.get_mut(cc.root) {
        frame.width = Dimension::Fixed(sw);
        frame.height = Dimension::Fixed(sh);
    }
}

fn teardown_char_create_ui(
    mut ui: ResMut<UiState>,
    mut screen: Option<ResMut<CharCreateScreenWrap>>,
    mut commands: Commands,
) {
    if let Some(res) = screen.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<CharCreateScreenWrap>();
    commands.remove_resource::<CharCreateUi>();
    commands.remove_resource::<CharCreateStateRes>();
    commands.remove_resource::<CharCreateFocus>();
    ui.focused_frame = None;
}

// --- Create response handler ---

fn handle_create_response(
    result: On<crate::networking_auth::CharacterCreationResult>,
    game_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    state: Option<ResMut<CharCreateStateRes>>,
) {
    if *game_state.get() != GameState::CharCreate {
        return;
    }
    let Some(mut state) = state else { return };
    if receive_create_result(&mut state, result.success, result.error.clone()).is_some() {
        info!("Character created, returning to CharSelect");
        next_state.set(GameState::CharSelect);
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/char_create_response_tests.rs"]
mod response_tests;

// --- Hover ---

fn char_create_hover_visuals(
    windows: Query<&Window>,
    mut ui: ResMut<UiState>,
    cc_ui: Option<Res<CharCreateUi>>,
) {
    let Some(cc) = cc_ui.as_ref() else { return };
    let cursor = windows
        .iter()
        .next()
        .and_then(|w| ui_toolkit::input::ui_cursor_position(&ui.registry, w));
    let button_ids: Vec<u64> = [
        Some(cc.back_button),
        cc.next_button,
        cc.randomize_button,
        ui.registry.get_by_name(RANDOM_NAME_BUTTON.0),
        cc.create_button,
    ]
    .into_iter()
    .flatten()
    .collect();
    for id in button_ids {
        let hovered = cursor.is_some_and(|c| hit_active_frame(&ui, id, c.x, c.y));
        set_button_hovered(&mut ui.registry, id, hovered);
    }
}

// --- Visual Updates ---

fn char_create_update_visuals(
    mut ui: ResMut<UiState>,
    cc_ui: Option<Res<CharCreateUi>>,
    mut state: Option<ResMut<CharCreateStateRes>>,
    focus: Res<CharCreateFocus>,
    mut screen_res: Option<ResMut<CharCreateScreenWrap>>,
    cust_db: Res<CustomizationDb>,
    name_catalog: Res<NameCatalogResource>,
) {
    let Some(_cc) = cc_ui.as_ref() else { return };
    let Some(state) = state.as_mut() else { return };
    // Look up by name each frame — the editbox only exists in Customize mode
    // so the ID resolved at startup may be None/stale.
    let name_input_id = ui.registry.get_by_name(CREATE_NAME_INPUT.0);
    let name_focused = name_input_id
        .is_some_and(|id| focus.0 == Some(id) && state.mode == CharCreateMode::Customize);
    sync_screen_state(
        &mut screen_res,
        &mut ui.registry,
        state,
        &cust_db,
        &name_catalog,
        name_focused,
    );
    ui.focused_frame = focus.0.filter(|_| state.mode == CharCreateMode::Customize);
}

fn sync_screen_state(
    screen_res: &mut Option<ResMut<CharCreateScreenWrap>>,
    reg: &mut FrameRegistry,
    state: &mut CharCreateState,
    cust_db: &CustomizationDb,
    name_catalog: &NameCatalogResource,
    name_input_focused: bool,
) {
    let Some(res) = screen_res.as_mut() else {
        return;
    };
    let inner = &mut res.0;
    if let Some(id) = reg.get_by_name(CREATE_NAME_INPUT.0) {
        state.name = get_editbox_text(reg, id);
    }
    let new_state = ui_state(
        state,
        cust_db,
        name_catalog.catalog(),
        name_input_focused,
        (reg.screen_width as u32, reg.screen_height as u32),
    );
    if inner.shared.get::<CharCreateUiState>() != Some(&new_state) {
        inner.shared.insert(new_state);
    }
    inner.screen.sync(&inner.shared, reg);
    apply_character_create_styles(reg, state.open_dropdown);
}

#[cfg(test)]
#[path = "../../../tests/unit/char_create_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../tests/unit/char_create_shared_tests.rs"]
mod shared_tests;

#[cfg(test)]
#[path = "name_action_tests.rs"]
mod name_action_tests;
