use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use lightyear::prelude::*;

use game_engine::ui::automation::{UiAutomationAction, UiAutomationQueue, UiAutomationRunner};
use game_engine::ui::frame::{Dimension, NineSlice, WidgetData};
use game_engine::ui::plugin::{sync_registry_to_primary_window, UiState};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::char_create_component::{
    char_create_screen, AppearanceField, CharCreateAction, CharCreateMode, CharCreateUiState,
    BACK_BUTTON, CHAR_CREATE_ROOT, CREATE_BUTTON, CREATE_NAME_INPUT, ERROR_TEXT, NEXT_BUTTON,
    SEX_TOGGLE_BUTTON,
};
use game_engine::ui::widgets::font_string::GameFont;
use game_engine::ui_resource;
use shared::components::CharacterAppearance;
use shared::protocol::{AuthChannel, CreateCharacter, CreateCharacterResponse};
use ui_toolkit::screen::Screen;

use crate::game_state::GameState;
use crate::login_screen_helpers as helpers;
use game_engine::char_create_data::{first_available_class, race_can_be_class, CLASSES};
use game_engine::customization_data::{CustomizationDb, OptionType};
use helpers::{
    editbox_backspace, editbox_cursor_end, editbox_cursor_home, editbox_delete,
    editbox_move_cursor, get_editbox_text, hit_frame, insert_char_into_editbox, set_button_hovered,
};

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
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let initial_state = initial_char_create_state(startup_mode.as_deref().copied());
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
    commands.remove_resource::<StartupCharCreateMode>();
}

fn initial_char_create_state(startup_mode: Option<StartupCharCreateMode>) -> CharCreateState {
    let mut state = CharCreateState::default();
    if let Some(mode) = startup_mode {
        state.mode = mode.0;
    }
    state
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

// --- Input Handling ---

#[allow(clippy::too_many_arguments)]
fn char_create_mouse_input(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    ui: Res<UiState>,
    cc_ui: Option<Res<CharCreateUi>>,
    mut state: ResMut<CharCreateState>,
    mut focus: ResMut<CharCreateFocus>,
    mut create_senders: Query<&mut MessageSender<CreateCharacter>>,
    mut next_state: ResMut<NextState<GameState>>,
    cust_db: Res<CustomizationDb>,
) {
    let Some(cc) = cc_ui.as_ref() else { return };
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = windows.iter().next().and_then(|w| w.cursor_position()) else {
        return;
    };
    let (mx, my) = (cursor.x, cursor.y);

    if let Some(id) = cc
        .name_input
        .filter(|&id| hit_active_frame(&ui, id, mx, my))
    {
        focus.0 = Some(id);
        return;
    }

    let action = find_clicked_action(&ui, mx, my);
    if let Some(action) = action {
        dispatch_action(
            &action,
            &mut state,
            &mut focus,
            &mut create_senders,
            &mut next_state,
            &ui.registry,
            cc,
            &cust_db,
        );
    } else {
        focus.0 = None;
        state.open_dropdown = None;
    }
}

fn find_clicked_action(ui: &UiState, mx: f32, my: f32) -> Option<String> {
    let hit_id = ui_toolkit::input::find_frame_at(&ui.registry, mx, my)?;
    walk_up_for_onclick(&ui.registry, hit_id)
}

fn walk_up_for_onclick(reg: &FrameRegistry, mut id: u64) -> Option<String> {
    loop {
        if let Some(frame) = reg.get(id) {
            if let Some(ref action) = frame.onclick {
                return Some(action.clone());
            }
            if let Some(parent) = frame.parent_id {
                id = parent;
                continue;
            }
        }
        return None;
    }
}

#[allow(clippy::too_many_arguments)]
fn dispatch_action(
    action_str: &str,
    state: &mut CharCreateState,
    focus: &mut CharCreateFocus,
    create_senders: &mut Query<&mut MessageSender<CreateCharacter>>,
    next_state: &mut NextState<GameState>,
    reg: &FrameRegistry,
    cc: &CharCreateUi,
    cust_db: &CustomizationDb,
) {
    let Some(action) = CharCreateAction::parse(action_str) else {
        focus.0 = None;
        return;
    };
    match action {
        CharCreateAction::SelectRace(id) => apply_race_change(state, id),
        CharCreateAction::SelectClass(id) => apply_class_change(state, id),
        CharCreateAction::ToggleSex => apply_sex_toggle(state),
        CharCreateAction::NextMode => state.mode = CharCreateMode::Customize,
        CharCreateAction::Back => handle_back(state, next_state),
        CharCreateAction::AppearanceInc(f) => {
            adjust_appearance(state, f, 1, cust_db);
            state.open_dropdown = None;
        }
        CharCreateAction::AppearanceDec(f) => {
            adjust_appearance(state, f, -1, cust_db);
            state.open_dropdown = None;
        }
        CharCreateAction::ToggleDropdown(f) => toggle_dropdown(state, f),
        CharCreateAction::SelectChoice(f, idx) => select_choice(state, f, idx),
        CharCreateAction::CreateConfirm => {
            send_create_request(state, reg, cc, create_senders);
            if let Some(id) = cc.name_input {
                focus.0 = Some(id);
            }
        }
    }
}

fn apply_race_change(state: &mut CharCreateState, race_id: u8) {
    state.selected_race = race_id;
    if !race_can_be_class(race_id, state.selected_class) {
        state.selected_class = first_available_class(race_id);
    }
}

fn apply_class_change(state: &mut CharCreateState, class_id: u8) {
    if race_can_be_class(state.selected_race, class_id) {
        state.selected_class = class_id;
    }
}

fn apply_sex_toggle(state: &mut CharCreateState) {
    state.selected_sex = if state.selected_sex == 0 { 1 } else { 0 };
    state.appearance.sex = state.selected_sex;
}

fn handle_back(state: &mut CharCreateState, next_state: &mut NextState<GameState>) {
    if state.mode == CharCreateMode::Customize {
        state.mode = CharCreateMode::RaceClass;
    } else {
        next_state.set(GameState::CharSelect);
    }
}

fn adjust_appearance(
    state: &mut CharCreateState,
    field: AppearanceField,
    delta: i8,
    db: &CustomizationDb,
) {
    let (race, sex) = (state.selected_race, state.selected_sex);
    let opt_type = match field {
        AppearanceField::SkinColor => OptionType::SkinColor,
        AppearanceField::Face => OptionType::Face,
        AppearanceField::HairStyle => OptionType::HairStyle,
        AppearanceField::HairColor => OptionType::HairColor,
        AppearanceField::FacialStyle => OptionType::FacialHair,
    };
    let max = db.choice_count(race, sex, opt_type);
    let val = match field {
        AppearanceField::SkinColor => &mut state.appearance.skin_color,
        AppearanceField::Face => &mut state.appearance.face,
        AppearanceField::HairStyle => &mut state.appearance.hair_style,
        AppearanceField::HairColor => &mut state.appearance.hair_color,
        AppearanceField::FacialStyle => &mut state.appearance.facial_style,
    };
    if max == 0 {
        return;
    }
    *val = if delta > 0 {
        if *val + 1 >= max {
            0
        } else {
            *val + 1
        }
    } else if *val == 0 {
        max - 1
    } else {
        *val - 1
    };
}

fn toggle_dropdown(state: &mut CharCreateState, field: AppearanceField) {
    if state.open_dropdown == Some(field) {
        state.open_dropdown = None;
    } else {
        state.open_dropdown = Some(field);
    }
}

fn select_choice(state: &mut CharCreateState, field: AppearanceField, idx: u8) {
    let val = match field {
        AppearanceField::SkinColor => &mut state.appearance.skin_color,
        AppearanceField::Face => &mut state.appearance.face,
        AppearanceField::HairStyle => &mut state.appearance.hair_style,
        AppearanceField::HairColor => &mut state.appearance.hair_color,
        AppearanceField::FacialStyle => &mut state.appearance.facial_style,
    };
    *val = idx;
    state.open_dropdown = None;
}

fn send_create_request(
    state: &mut CharCreateState,
    reg: &FrameRegistry,
    cc: &CharCreateUi,
    senders: &mut Query<&mut MessageSender<CreateCharacter>>,
) {
    let name = cc
        .name_input
        .map(|id| get_editbox_text(reg, id))
        .unwrap_or_default();
    if name.is_empty() {
        state.error_text = Some("Please enter a name".to_string());
        return;
    }
    let msg = CreateCharacter {
        name: name.clone(),
        race: state.selected_race,
        class: state.selected_class,
        appearance: state.appearance,
    };
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(msg.clone());
    }
    state.error_text = None;
    info!("Requested create character '{name}'");
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

// --- Keyboard ---

fn char_create_keyboard_input(
    mut key_events: MessageReader<KeyboardInput>,
    mut ui: ResMut<UiState>,
    focus: Res<CharCreateFocus>,
    cc_ui: Option<Res<CharCreateUi>>,
) {
    let Some(_cc) = cc_ui.as_ref() else { return };
    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        let Some(focused_id) = focus.0 else { continue };
        if let Key::Character(ch) = &event.logical_key {
            insert_char_into_editbox(&mut ui.registry, focused_id, ch.as_str());
        } else {
            handle_char_create_key(event.key_code, focused_id, &mut ui);
        }
    }
}

fn handle_char_create_key(key: KeyCode, focused_id: u64, ui: &mut UiState) {
    match key {
        KeyCode::Backspace => editbox_backspace(&mut ui.registry, focused_id),
        KeyCode::Delete => editbox_delete(&mut ui.registry, focused_id),
        KeyCode::ArrowLeft => editbox_move_cursor(&mut ui.registry, focused_id, -1),
        KeyCode::ArrowRight => editbox_move_cursor(&mut ui.registry, focused_id, 1),
        KeyCode::Home => editbox_cursor_home(&mut ui.registry, focused_id),
        KeyCode::End => editbox_cursor_end(&mut ui.registry, focused_id),
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn char_create_run_automation(
    mut ui: ResMut<UiState>,
    cc_ui: Option<Res<CharCreateUi>>,
    mut state: ResMut<CharCreateState>,
    mut focus: ResMut<CharCreateFocus>,
    mut create_senders: Query<&mut MessageSender<CreateCharacter>>,
    mut next_state: ResMut<NextState<GameState>>,
    cust_db: Res<CustomizationDb>,
    mut queue: ResMut<UiAutomationQueue>,
    mut runner: ResMut<UiAutomationRunner>,
) {
    let Some(cc) = cc_ui.as_ref() else { return };
    let Some(action) = queue.peek().cloned() else {
        return;
    };
    if !action.is_input_action() {
        return;
    }
    let result = run_char_create_automation_action(
        &mut ui,
        cc,
        &mut state,
        &mut focus,
        &mut create_senders,
        &mut next_state,
        &cust_db,
        &action,
    );
    queue.pop();
    if let Err(err) = result {
        runner.last_error = Some(err.clone());
        error!("UI automation failed in CharCreate: {err}");
    }
}

#[allow(clippy::too_many_arguments)]
fn run_char_create_automation_action(
    ui: &mut UiState,
    cc: &CharCreateUi,
    state: &mut CharCreateState,
    focus: &mut CharCreateFocus,
    create_senders: &mut Query<&mut MessageSender<CreateCharacter>>,
    next_state: &mut NextState<GameState>,
    cust_db: &CustomizationDb,
    action: &UiAutomationAction,
) -> Result<(), String> {
    match action {
        UiAutomationAction::ClickFrame(name) => click_char_create_frame(
            ui,
            cc,
            state,
            focus,
            create_senders,
            next_state,
            cust_db,
            name,
        )?,
        UiAutomationAction::TypeText(text) => {
            let focused_id = focus
                .0
                .ok_or("automation type requires a focused edit box")?;
            for ch in text.chars() {
                insert_char_into_editbox(&mut ui.registry, focused_id, &ch.to_string());
            }
        }
        UiAutomationAction::PressKey(key) => {
            let focused_id = focus
                .0
                .ok_or("automation key press requires a focused frame")?;
            handle_char_create_key(*key, focused_id, ui);
        }
        UiAutomationAction::WaitForState(_, _)
        | UiAutomationAction::WaitForFrame(_, _)
        | UiAutomationAction::DumpTree
        | UiAutomationAction::DumpUiTree => {}
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn click_char_create_frame(
    ui: &mut UiState,
    cc: &CharCreateUi,
    state: &mut CharCreateState,
    focus: &mut CharCreateFocus,
    create_senders: &mut Query<&mut MessageSender<CreateCharacter>>,
    next_state: &mut NextState<GameState>,
    cust_db: &CustomizationDb,
    frame_name: &str,
) -> Result<(), String> {
    let frame_id = ui
        .registry
        .get_by_name(frame_name)
        .ok_or_else(|| format!("unknown char create frame '{frame_name}'"))?;
    if cc.name_input == Some(frame_id) {
        focus.0 = Some(frame_id);
        return Ok(());
    }
    let action = walk_up_for_onclick(&ui.registry, frame_id)
        .ok_or_else(|| format!("char create frame '{frame_name}' has no onclick action"))?;
    dispatch_action(
        &action,
        state,
        focus,
        create_senders,
        next_state,
        &ui.registry,
        cc,
        cust_db,
    );
    Ok(())
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

fn build_ui_state(state: &CharCreateState, cust_db: &CustomizationDb) -> CharCreateUiState {
    let (race, sex) = (state.selected_race, state.selected_sex);
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
        skin_color_swatches: cust_db.all_swatch_colors(race, sex, OptionType::SkinColor),
        hair_color_swatches: cust_db.all_swatch_colors(race, sex, OptionType::HairColor),
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

fn hit_active_frame(ui: &UiState, frame_id: u64, mx: f32, my: f32) -> bool {
    ui.registry
        .get(frame_id)
        .is_some_and(|frame| frame.visible && !frame.hidden)
        && hit_frame(ui, frame_id, mx, my)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_mode_can_open_customize_directly() {
        let state =
            initial_char_create_state(Some(StartupCharCreateMode(CharCreateMode::Customize)));
        assert_eq!(state.mode, CharCreateMode::Customize);
    }

    #[test]
    fn default_startup_mode_stays_on_race_class() {
        let state = initial_char_create_state(None);
        assert_eq!(state.mode, CharCreateMode::RaceClass);
    }
}
