use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use lightyear::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

use game_engine::ui::automation::{UiAutomationAction, UiAutomationQueue, UiAutomationRunner};
use game_engine::ui::frame::{Dimension, NineSlice, WidgetData};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::char_create_component::{
    AppearanceField, BACK_BUTTON, CHAR_CREATE_ROOT, CREATE_BUTTON, CREATE_NAME_INPUT,
    CharCreateAction, CharCreateMode, CharCreateUiState, ERROR_TEXT, NEXT_BUTTON, RANDOMIZE_BUTTON,
    SEX_TOGGLE_BUTTON, char_create_screen,
};
use game_engine::ui::widgets::font_string::GameFont;
use game_engine::ui_resource;
use shared::components::CharacterAppearance;
use shared::protocol::{AuthChannel, CreateCharacter, CreateCharacterResponse};
use ui_toolkit::screen::Screen;

use crate::game_state::GameState;
use crate::scenes::login::helpers;
use game_engine::char_create_data::{CLASSES, first_available_class, race_can_be_class};
use game_engine::customization_data::{CustomizationDb, OptionType};
use helpers::{
    editbox_backspace, editbox_cursor_end, editbox_cursor_home, editbox_delete,
    editbox_move_cursor, get_editbox_text, hit_frame, insert_char_into_editbox, set_button_hovered,
};

mod appearance;
mod input;
pub mod scene;

use appearance as appearance_logic;
use input::{
    char_create_keyboard_input, char_create_mouse_input, char_create_run_automation,
    clamp_appearance_field, hit_active_frame,
};
pub use scene::CharCreateScenePlugin;

const EDITBOX_BG: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const EDITBOX_BORDER: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const EDITBOX_FOCUSED_BG: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const EDITBOX_FOCUSED_BORDER: [f32; 4] = [1.0, 0.92, 0.72, 1.0];
const GLUE_NORMAL_FONT_COLOR: [f32; 4] = [1.0, 0.82, 0.0, 1.0];

ui_resource! {
    pub(crate) CharCreateUi {
        root: CHAR_CREATE_ROOT,
        back_button: BACK_BUTTON,
        next_button ?: NEXT_BUTTON,
        sex_toggle ?: SEX_TOGGLE_BUTTON,
        randomize_button ?: RANDOMIZE_BUTTON,
        create_button ?: CREATE_BUTTON,
        name_input ?: CREATE_NAME_INPUT,
        error_text ?: ERROR_TEXT,
    }
}

#[derive(Resource)]
pub(crate) struct CharCreateState {
    pub(crate) selected_race: u8,
    pub(crate) selected_class: u8,
    pub(crate) selected_sex: u8,
    pub(crate) appearance: CharacterAppearance,
    pub(crate) mode: CharCreateMode,
    pub(crate) error_text: Option<String>,
    pub(crate) open_dropdown: Option<AppearanceField>,
}

impl Default for CharCreateState {
    fn default() -> Self {
        Self {
            selected_race: 1,
            selected_class: 1,
            selected_sex: 0,
            appearance: CharacterAppearance::default(),
            mode: CharCreateMode::RaceClass,
            error_text: None,
            open_dropdown: None,
        }
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
        app.add_systems(OnEnter(GameState::CharCreate), build_char_create_ui);
        app.add_systems(OnExit(GameState::CharCreate), teardown_char_create_ui);
        app.add_systems(
            Update,
            (
                char_create_mouse_input,
                char_create_keyboard_input,
                char_create_run_automation,
                char_create_hover_visuals,
                char_create_update_visuals,
                handle_create_response,
            )
                .into_configs()
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
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let initial_state = initial_char_create_state(startup_mode.as_deref().copied(), &cust_db);
    let ui_state = CharCreateUiState {
        mode: initial_state.mode,
        selected_race: initial_state.selected_race,
        selected_class: initial_state.selected_class,
        selected_sex: initial_state.selected_sex,
        skin_color: initial_state.appearance.skin_color,
        face: initial_state.appearance.face,
        hair_style: initial_state.appearance.hair_style,
        hair_color: initial_state.appearance.hair_color,
        facial_style: initial_state.appearance.facial_style,
        ..CharCreateUiState::default()
    };
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(ui_state);
    let mut screen = Screen::new(char_create_screen);
    screen.sync(&shared, &mut ui.registry);

    let cc = CharCreateUi::resolve(&ui.registry);
    apply_post_setup(&mut ui.registry, &cc);

    commands.insert_resource(initial_state);
    commands.init_resource::<CharCreateFocus>();
    commands.insert_resource(CharCreateScreenWrap(CharCreateScreenRes { screen, shared }));
    commands.insert_resource(cc);
    commands.remove_resource::<crate::game_state::StartupScreenTarget>();
    commands.remove_resource::<StartupCharCreateMode>();
}

fn initial_char_create_state(
    startup_mode: Option<StartupCharCreateMode>,
    db: &CustomizationDb,
) -> CharCreateState {
    let mut state = CharCreateState::default();
    if let Some(mode) = startup_mode {
        state.mode = mode.0;
    }
    randomize_appearance(&mut state, db);
    state
}

const CHAR_CREATE_RANDOM_SEED_MIX: u64 = 0x9e37_79b9_7f4a_7c15;

fn fresh_random_seed() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos() as u64,
        Err(_) => CHAR_CREATE_RANDOM_SEED_MIX,
    }
}

fn mix_seed(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(CHAR_CREATE_RANDOM_SEED_MIX);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn pick_random_choice(seed: &mut u64, count: u8) -> u8 {
    if count == 0 {
        return 0;
    }
    *seed = mix_seed(*seed);
    (*seed % count as u64) as u8
}

fn randomize_appearance(state: &mut CharCreateState, db: &CustomizationDb) {
    randomize_appearance_with_seed(state, db, fresh_random_seed());
}

fn randomize_appearance_with_seed(state: &mut CharCreateState, db: &CustomizationDb, seed: u64) {
    appearance_logic::randomize_appearance_with_seed(state, db, seed);
}

fn apply_post_setup(reg: &mut FrameRegistry, cc: &CharCreateUi) {
    let (sw, sh) = (reg.screen_width, reg.screen_height);
    if let Some(frame) = reg.get_mut(cc.root) {
        frame.width = Dimension::Fixed(sw);
        frame.height = Dimension::Fixed(sh);
    }
    if let Some(id) = cc.name_input {
        set_editbox_backdrop(reg, id);
    }
}

fn set_editbox_backdrop(reg: &mut FrameRegistry, id: u64) {
    if let Some(frame) = reg.get_mut(id) {
        frame.nine_slice = Some(NineSlice {
            edge_size: 8.0,
            part_textures: Some(common_input_border_part_textures()),
            bg_color: EDITBOX_BG,
            border_color: EDITBOX_BORDER,
            ..Default::default()
        });
        if let Some(WidgetData::EditBox(eb)) = &mut frame.widget_data {
            eb.text_insets = [12.0, 5.0, 8.0, 8.0];
            eb.font = GameFont::ArialNarrow;
            eb.font_size = 16.0;
            eb.text_color = GLUE_NORMAL_FONT_COLOR;
        }
    }
}

fn common_input_border_part_textures() -> [game_engine::ui::widgets::texture::TextureSource; 9] {
    use game_engine::ui::widgets::texture::TextureSource;
    let base = "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-";
    [
        TextureSource::File(format!("{base}TL.blp")),
        TextureSource::File(format!("{base}T.blp")),
        TextureSource::File(format!("{base}TR.blp")),
        TextureSource::File(format!("{base}L.blp")),
        TextureSource::File(format!("{base}M.blp")),
        TextureSource::File(format!("{base}R.blp")),
        TextureSource::File(format!("{base}BL.blp")),
        TextureSource::File(format!("{base}B.blp")),
        TextureSource::File(format!("{base}BR.blp")),
    ]
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
    commands.remove_resource::<CharCreateState>();
    commands.remove_resource::<CharCreateFocus>();
    ui.focused_frame = None;
}

// --- Create response handler ---

fn handle_create_response(
    mut receivers: Query<&mut MessageReceiver<CreateCharacterResponse>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut state: ResMut<CharCreateState>,
) {
    for mut receiver in receivers.iter_mut() {
        for resp in receiver.receive() {
            if resp.success {
                info!("Character created, returning to CharSelect");
                next_state.set(GameState::CharSelect);
            } else {
                let err = resp.error.unwrap_or_else(|| "Creation failed".to_string());
                error!("Create character failed: {err}");
                state.error_text = Some(err);
            }
        }
    }
}

// --- Hover ---

fn char_create_hover_visuals(
    windows: Query<&Window>,
    mut ui: ResMut<UiState>,
    cc_ui: Option<Res<CharCreateUi>>,
) {
    let Some(cc) = cc_ui.as_ref() else { return };
    let cursor = windows.iter().next().and_then(|w| w.cursor_position());
    let button_ids: Vec<u64> = [
        Some(cc.back_button),
        cc.next_button,
        cc.sex_toggle,
        cc.randomize_button,
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
    state: Option<Res<CharCreateState>>,
    focus: Res<CharCreateFocus>,
    mut screen_res: Option<ResMut<CharCreateScreenWrap>>,
    cust_db: Res<CustomizationDb>,
) {
    let Some(cc) = cc_ui.as_ref() else { return };
    let Some(state) = state.as_ref() else { return };
    sync_screen_state(&mut screen_res, &mut ui.registry, state, &cust_db);
    if let Some(id) = cc.name_input {
        sync_editbox_focus(
            &mut ui.registry,
            id,
            focus.0 == Some(id) && state.mode == CharCreateMode::Customize,
        );
    }
    ui.focused_frame = focus.0.filter(|_| state.mode == CharCreateMode::Customize);
}

fn sync_screen_state(
    screen_res: &mut Option<ResMut<CharCreateScreenWrap>>,
    reg: &mut FrameRegistry,
    state: &CharCreateState,
    cust_db: &CustomizationDb,
) {
    let Some(res) = screen_res.as_mut() else {
        return;
    };
    let inner = &mut res.0;
    let new_state = build_ui_state(state, cust_db);
    inner.shared.insert(new_state);
    inner.screen.sync(&inner.shared, reg);
}

fn build_class_availability(race: u8) -> Vec<(u8, &'static str, &'static str, bool)> {
    CLASSES
        .iter()
        .map(|c| (c.id, c.name, c.icon_file, race_can_be_class(race, c.id)))
        .collect()
}

struct AppearanceLabels {
    face: String,
    hair_style: String,
    facial_style: String,
}

struct AppearanceSwatches {
    skin_colors: Vec<Option<[u8; 3]>>,
    hair_colors: Vec<Option<[u8; 3]>>,
}

fn build_appearance_labels(
    state: &CharCreateState,
    cust_db: &CustomizationDb,
    race: u8,
    sex: u8,
) -> AppearanceLabels {
    AppearanceLabels {
        face: choice_label(
            cust_db,
            race,
            sex,
            state.selected_class,
            OptionType::Face,
            state.appearance.face,
        ),
        hair_style: choice_label(
            cust_db,
            race,
            sex,
            state.selected_class,
            OptionType::HairStyle,
            state.appearance.hair_style,
        ),
        facial_style: choice_label(
            cust_db,
            race,
            sex,
            state.selected_class,
            OptionType::FacialHair,
            state.appearance.facial_style,
        ),
    }
}

fn choice_label(
    cust_db: &CustomizationDb,
    race: u8,
    sex: u8,
    class: u8,
    option: OptionType,
    selected: u8,
) -> String {
    cust_db
        .choice_name_for_class(race, sex, class, option, selected)
        .unwrap_or_default()
        .to_string()
}

fn build_appearance_swatches(cust_db: &CustomizationDb, race: u8, sex: u8) -> AppearanceSwatches {
    AppearanceSwatches {
        skin_colors: cust_db.all_swatch_colors(race, sex, OptionType::SkinColor),
        hair_colors: cust_db.all_swatch_colors(race, sex, OptionType::HairColor),
    }
}

fn build_ui_state(state: &CharCreateState, cust_db: &CustomizationDb) -> CharCreateUiState {
    let (race, sex) = (state.selected_race, state.selected_sex);
    let labels = build_appearance_labels(state, cust_db, race, sex);
    let swatches = build_appearance_swatches(cust_db, race, sex);
    CharCreateUiState {
        mode: state.mode,
        selected_race: race,
        selected_class: state.selected_class,
        selected_sex: sex,
        skin_color: state.appearance.skin_color,
        face: state.appearance.face,
        hair_style: state.appearance.hair_style,
        hair_color: state.appearance.hair_color,
        facial_style: state.appearance.facial_style,
        face_label: labels.face,
        hair_style_label: labels.hair_style,
        facial_style_label: labels.facial_style,
        skin_color_swatches: swatches.skin_colors,
        hair_color_swatches: swatches.hair_colors,
        open_dropdown: state.open_dropdown,
        name: String::new(),
        error_text: state.error_text.clone(),
        class_availability: build_class_availability(race),
    }
}

fn sync_editbox_focus(reg: &mut FrameRegistry, id: u64, focused: bool) {
    let Some(frame) = reg.get_mut(id) else { return };
    let Some(nine_slice) = frame.nine_slice.as_mut() else {
        return;
    };
    if focused {
        nine_slice.bg_color = EDITBOX_FOCUSED_BG;
        nine_slice.border_color = EDITBOX_FOCUSED_BORDER;
    } else {
        nine_slice.bg_color = EDITBOX_BG;
        nine_slice.border_color = EDITBOX_BORDER;
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/char_create_tests.rs"]
mod tests;
