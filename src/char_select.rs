use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use lightyear::prelude::*;

use game_engine::ui::frame::{Dimension, NineSlice, WidgetData};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::char_select_component::{
    BACK_BUTTON, CHAR_LIST_PANEL, CHAR_SELECT_ROOT, CREATE_CHAR_BUTTON, CREATE_CONFIRM_BUTTON,
    CREATE_NAME_INPUT, CREATE_PANEL, CampsiteEntry, CampsiteState, CharDisplayEntry,
    CharSelectAction, CharSelectState, DELETE_CHAR_BUTTON, ENTER_WORLD_BUTTON,
    SELECTED_NAME_TEXT, STATUS_TEXT, char_select_screen,
};
use game_engine::ui::widgets::font_string::GameFont;
use game_engine::ui::widgets::texture::TextureSource;
use game_engine::ui_resource;
use shared::components::CharacterAppearance;
use shared::protocol::{AuthChannel, CreateCharacter, DeleteCharacter, SelectCharacter};
use ui_toolkit::screen::Screen;

use crate::game_state::GameState;
use crate::networking::CharacterList;

use crate::login_screen_helpers as helpers;
use helpers::{
    editbox_backspace, editbox_cursor_end, editbox_cursor_home, editbox_delete,
    editbox_move_cursor, get_editbox_text, hit_frame, insert_char_into_editbox, set_button_hovered,
};

const REALM_NAME: &str = "World of Osso";
const EDITBOX_BG: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const EDITBOX_BORDER: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const EDITBOX_FOCUSED_BG: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const EDITBOX_FOCUSED_BORDER: [f32; 4] = [1.0, 0.92, 0.72, 1.0];
const GLUE_NORMAL_FONT_COLOR: [f32; 4] = [1.0, 0.82, 0.0, 1.0];

ui_resource! {
    pub(crate) CharSelectUi {
        root: CHAR_SELECT_ROOT,
        enter_button: ENTER_WORLD_BUTTON,
        create_button: CREATE_CHAR_BUTTON,
        delete_button: DELETE_CHAR_BUTTON,
        back_button: BACK_BUTTON,
        create_panel: CREATE_PANEL,
        create_name_input: CREATE_NAME_INPUT,
        create_confirm_button: CREATE_CONFIRM_BUTTON,
        status_text: STATUS_TEXT,
        selected_name_text: SELECTED_NAME_TEXT,
        list_panel: CHAR_LIST_PANEL,
    }
}

#[derive(Resource, Default)]
pub(crate) struct SelectedCharIndex(pub(crate) Option<usize>);

/// CLI-provided character name to pre-select on the char select screen.
#[derive(Resource)]
pub(crate) struct PreselectedCharName(pub(crate) String);

#[derive(Resource, Default)]
struct CreatePanelVisible(bool);

#[derive(Resource, Default)]
struct CampsitePanelVisible(bool);

#[derive(Resource, Default)]
struct CharSelectFocus(Option<u64>);

struct CharSelectScreenRes {
    screen: Screen,
    shared: ui_toolkit::screen::SharedContext,
}
unsafe impl Send for CharSelectScreenRes {}
unsafe impl Sync for CharSelectScreenRes {}

#[derive(Resource)]
struct CharSelectScreenWrap(CharSelectScreenRes);

pub struct CharSelectPlugin;

impl Plugin for CharSelectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedCharIndex>();
        app.init_resource::<CreatePanelVisible>();
        app.init_resource::<CharSelectFocus>();
        app.add_systems(OnEnter(GameState::CharSelect), build_char_select_ui);
        app.add_systems(OnExit(GameState::CharSelect), teardown_char_select_ui);
        app.add_systems(
            Update,
            (
                char_select_mouse_input,
                char_select_keyboard_input,
                char_select_hover_visuals,
                char_select_update_visuals,
            )
                .into_configs()
                .run_if(in_state(GameState::CharSelect)),
        );
    }
}

// --- UI Building ---

fn build_char_select_state(char_list: &CharacterList, selected: Option<usize>) -> CharSelectState {
    let characters: Vec<CharDisplayEntry> = char_list
        .0
        .iter()
        .map(|ch| CharDisplayEntry {
            name: ch.name.clone(),
            info: format!("Level {}   Race {}   Class {}", ch.level, ch.race, ch.class),
            status: "Ready to enter world".to_string(),
        })
        .collect();
    let selected_name = selected
        .and_then(|i| char_list.0.get(i))
        .map(|ch| ch.name.clone())
        .unwrap_or_else(|| "Character Selection".to_string());
    CharSelectState {
        characters,
        selected_index: selected,
        create_panel_visible: false,
        selected_name,
        status_text: String::new(),
    }
}

fn build_char_select_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    char_list: Res<CharacterList>,
    preselected: Option<Res<PreselectedCharName>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let initial_selected = preselected
        .as_ref()
        .and_then(|p| {
            char_list
                .0
                .iter()
                .position(|ch| ch.name.eq_ignore_ascii_case(&p.0))
        })
        .or_else(|| char_list.0.first().map(|_| 0));
    let state = build_char_select_state(&char_list, initial_selected);

    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(state);
    shared.insert(build_campsite_state(false));
    let mut screen = Screen::new(char_select_screen);
    screen.sync(&shared, &mut ui.registry);

    let cs = CharSelectUi::resolve(&ui.registry);
    apply_post_setup(&mut ui.registry, &cs);

    commands.insert_resource(SelectedCharIndex(initial_selected));
    commands.insert_resource(CreatePanelVisible(false));
    commands.insert_resource(CampsitePanelVisible(false));
    commands.insert_resource(CharSelectFocus(None));
    commands.insert_resource(CharSelectScreenWrap(CharSelectScreenRes { screen, shared }));
    commands.insert_resource(cs);
}

fn apply_post_setup(reg: &mut FrameRegistry, cs: &CharSelectUi) {
    let (sw, sh) = (reg.screen_width, reg.screen_height);
    if let Some(frame) = reg.get_mut(cs.root) {
        frame.width = Dimension::Fixed(sw);
        frame.height = Dimension::Fixed(sh);
    }
    set_list_panel_backdrop(reg, cs.list_panel);
    set_editbox_backdrop(reg, cs.create_name_input);
}

fn set_list_panel_backdrop(reg: &mut FrameRegistry, id: u64) {
    if let Some(frame) = reg.get_mut(id) {
        frame.nine_slice = Some(NineSlice {
            edge_size: 12.0,
            uv_edge_size: Some(12.0),
            texture: Some(TextureSource::Atlas(
                "glues-characterselect-card-all-bg".to_string(),
            )),
            bg_color: [1.0, 1.0, 1.0, 1.0],
            border_color: [1.0, 1.0, 1.0, 1.0],
            ..Default::default()
        });
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

fn common_input_border_part_textures() -> [TextureSource; 9] {
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

fn teardown_char_select_ui(
    mut ui: ResMut<UiState>,
    mut screen: Option<ResMut<CharSelectScreenWrap>>,
    mut commands: Commands,
) {
    if let Some(res) = screen.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<CharSelectScreenWrap>();
    commands.remove_resource::<CharSelectUi>();
    ui.focused_frame = None;
}

// --- Input Handling ---

#[allow(clippy::too_many_arguments)]
fn char_select_mouse_input(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    ui: Res<UiState>,
    cs_ui: Option<Res<CharSelectUi>>,
    mut selected: ResMut<SelectedCharIndex>,
    mut focus: ResMut<CharSelectFocus>,
    mut create_visible: ResMut<CreatePanelVisible>,
    mut campsite_visible: ResMut<CampsitePanelVisible>,
    mut senders: Query<&mut MessageSender<SelectCharacter>>,
    mut del_senders: Query<&mut MessageSender<DeleteCharacter>>,
    mut create_senders: Query<&mut MessageSender<CreateCharacter>>,
    char_list: Res<CharacterList>,
    mut next_state: ResMut<NextState<GameState>>,
    selected_scene: Option<ResMut<crate::warband_scene::SelectedWarbandScene>>,
) {
    let Some(cs) = cs_ui.as_ref() else { return };
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = cursor_pos(&windows) else {
        return;
    };
    handle_cs_click(
        cs, &ui, cursor, &mut selected, &mut focus, &mut create_visible,
        &mut campsite_visible, &mut senders, &mut del_senders, &mut create_senders,
        &char_list, &mut next_state, selected_scene,
    );
}

fn cursor_pos(windows: &Query<&Window>) -> Option<Vec2> {
    windows.iter().next().and_then(|w| w.cursor_position())
}

#[allow(clippy::too_many_arguments)]
fn dispatch_onclick(
    action: &str,
    selected: &mut SelectedCharIndex,
    focus: &mut CharSelectFocus,
    _create_visible: &mut CreatePanelVisible,
    campsite_visible: &mut CampsitePanelVisible,
    senders: &mut Query<&mut MessageSender<SelectCharacter>>,
    del_senders: &mut Query<&mut MessageSender<DeleteCharacter>>,
    create_senders: &mut Query<&mut MessageSender<CreateCharacter>>,
    char_list: &CharacterList,
    next_state: &mut NextState<GameState>,
    reg: &FrameRegistry,
    cs: &CharSelectUi,
    mut selected_scene: Option<ResMut<crate::warband_scene::SelectedWarbandScene>>,
) {
    match CharSelectAction::parse(action) {
        Some(CharSelectAction::SelectChar(idx)) => {
            selected.0 = Some(idx);
            focus.0 = None;
        }
        Some(CharSelectAction::EnterWorld) => try_enter_world(selected, char_list, senders),
        Some(CharSelectAction::CreateToggle) => next_state.set(GameState::CharCreate),
        Some(CharSelectAction::DeleteChar) => {
            try_delete_character(selected, char_list, del_senders)
        }
        Some(CharSelectAction::Back) => next_state.set(GameState::Login),
        Some(CharSelectAction::CreateConfirm) => {
            try_create_character(reg, cs, create_senders);
            focus.0 = Some(cs.create_name_input);
        }
        Some(CharSelectAction::CampsiteToggle) => {
            campsite_visible.0 = !campsite_visible.0;
        }
        Some(CharSelectAction::SelectCampsite(id)) => {
            if let Some(ref mut sel) = selected_scene {
                sel.scene_id = id;
            }
            campsite_visible.0 = false;
        }
        None => { focus.0 = None; }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_cs_click(
    cs: &CharSelectUi,
    ui: &UiState,
    cursor: Vec2,
    selected: &mut SelectedCharIndex,
    focus: &mut CharSelectFocus,
    create_visible: &mut CreatePanelVisible,
    campsite_visible: &mut CampsitePanelVisible,
    senders: &mut Query<&mut MessageSender<SelectCharacter>>,
    del_senders: &mut Query<&mut MessageSender<DeleteCharacter>>,
    create_senders: &mut Query<&mut MessageSender<CreateCharacter>>,
    char_list: &CharacterList,
    next_state: &mut NextState<GameState>,
    selected_scene: Option<ResMut<crate::warband_scene::SelectedWarbandScene>>,
) {
    let (mx, my) = (cursor.x, cursor.y);
    if hit_active_frame(ui, cs.create_name_input, mx, my) {
        focus.0 = Some(cs.create_name_input);
        return;
    }
    let action = find_clicked_action(ui, mx, my);
    if let Some(action) = action {
        dispatch_onclick(
            &action, selected, focus, create_visible, campsite_visible,
            senders, del_senders, create_senders, char_list, next_state,
            &ui.registry, cs, selected_scene,
        );
    } else {
        focus.0 = None;
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

fn try_enter_world(
    selected: &SelectedCharIndex,
    char_list: &CharacterList,
    senders: &mut Query<&mut MessageSender<SelectCharacter>>,
) {
    let Some(idx) = selected.0 else { return };
    let Some(ch) = char_list.0.get(idx) else {
        return;
    };
    let msg = SelectCharacter {
        character_id: ch.character_id,
    };
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(msg.clone());
    }
    info!("Requested enter world for '{}'", ch.name);
}

fn try_delete_character(
    selected: &SelectedCharIndex,
    char_list: &CharacterList,
    senders: &mut Query<&mut MessageSender<DeleteCharacter>>,
) {
    let Some(idx) = selected.0 else { return };
    let Some(ch) = char_list.0.get(idx) else {
        return;
    };
    let msg = DeleteCharacter {
        character_id: ch.character_id,
    };
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(msg.clone());
    }
    info!("Requested delete character '{}'", ch.name);
}

fn try_create_character(
    reg: &FrameRegistry,
    cs: &CharSelectUi,
    senders: &mut Query<&mut MessageSender<CreateCharacter>>,
) {
    let name = get_editbox_text(reg, cs.create_name_input);
    if name.is_empty() {
        return;
    }
    let msg = CreateCharacter {
        name: name.clone(),
        race: 1,
        class: 1,
        appearance: default_create_character_appearance(),
    };
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(msg.clone());
    }
    info!("Requested create character '{name}'");
}

fn default_create_character_appearance() -> CharacterAppearance {
    CharacterAppearance::default()
}

// --- Keyboard ---

fn char_select_keyboard_input(
    mut key_events: MessageReader<KeyboardInput>,
    mut ui: ResMut<UiState>,
    focus: Res<CharSelectFocus>,
    cs_ui: Option<Res<CharSelectUi>>,
    mut selected: ResMut<SelectedCharIndex>,
    char_list: Res<CharacterList>,
    mut create_senders: Query<&mut MessageSender<CreateCharacter>>,
    mut senders: Query<&mut MessageSender<SelectCharacter>>,
) {
    let Some(cs) = cs_ui.as_ref() else { return };
    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        if handle_selection_key(event.key_code, &mut selected, &char_list, &mut senders) {
            continue;
        }
        let Some(focused_id) = focus.0 else { continue };
        if let Key::Character(ch) = &event.logical_key {
            insert_char_into_editbox(&mut ui.registry, focused_id, ch.as_str());
        } else {
            handle_cs_key(event.key_code, focused_id, &mut ui, cs, &mut create_senders);
        }
    }
}

fn handle_selection_key(
    key: KeyCode,
    selected: &mut SelectedCharIndex,
    char_list: &CharacterList,
    senders: &mut Query<&mut MessageSender<SelectCharacter>>,
) -> bool {
    let count = char_list.0.len();
    if count == 0 {
        return false;
    }
    match key {
        KeyCode::ArrowUp => {
            let idx = selected.0.unwrap_or(0);
            selected.0 = Some(if idx == 0 { count - 1 } else { idx - 1 });
            true
        }
        KeyCode::ArrowDown => {
            let idx = selected.0.unwrap_or(count.wrapping_sub(1));
            selected.0 = Some(if idx + 1 >= count { 0 } else { idx + 1 });
            true
        }
        KeyCode::Enter if selected.0.is_some() => {
            try_enter_world(selected, char_list, senders);
            true
        }
        _ => false,
    }
}

fn handle_cs_key(
    key: KeyCode,
    focused_id: u64,
    ui: &mut UiState,
    cs: &CharSelectUi,
    create_senders: &mut Query<&mut MessageSender<CreateCharacter>>,
) {
    match key {
        KeyCode::Backspace => editbox_backspace(&mut ui.registry, focused_id),
        KeyCode::Delete => editbox_delete(&mut ui.registry, focused_id),
        KeyCode::ArrowLeft => editbox_move_cursor(&mut ui.registry, focused_id, -1),
        KeyCode::ArrowRight => editbox_move_cursor(&mut ui.registry, focused_id, 1),
        KeyCode::Home => editbox_cursor_home(&mut ui.registry, focused_id),
        KeyCode::End => editbox_cursor_end(&mut ui.registry, focused_id),
        KeyCode::Enter => try_create_character(&ui.registry, cs, create_senders),
        _ => {}
    }
}

// --- Hover ---

fn char_select_hover_visuals(
    windows: Query<&Window>,
    mut ui: ResMut<UiState>,
    cs_ui: Option<Res<CharSelectUi>>,
) {
    let Some(cs) = cs_ui.as_ref() else { return };
    let cursor = cursor_pos(&windows);
    let button_ids = [
        cs.enter_button,
        cs.create_button,
        cs.delete_button,
        cs.back_button,
        cs.create_confirm_button,
    ];
    for id in button_ids {
        let hovered = cursor.is_some_and(|c| hit_active_frame(&ui, id, c.x, c.y));
        set_button_hovered(&mut ui.registry, id, hovered);
    }
}

// --- Visual Updates ---

fn char_select_update_visuals(
    mut ui: ResMut<UiState>,
    cs_ui: Option<Res<CharSelectUi>>,
    selected: Res<SelectedCharIndex>,
    create_visible: Res<CreatePanelVisible>,
    campsite_visible: Res<CampsitePanelVisible>,
    focus: Res<CharSelectFocus>,
    char_list: Res<CharacterList>,
    mut screen_res: Option<ResMut<CharSelectScreenWrap>>,
) {
    let Some(cs) = cs_ui.as_ref() else { return };
    sync_screen_state(
        &mut screen_res,
        &mut ui.registry,
        &char_list,
        &selected,
        &create_visible,
        &campsite_visible,
    );
    sync_editbox_focus_visual(
        &mut ui.registry,
        cs.create_name_input,
        focus.0 == Some(cs.create_name_input) && create_visible.0,
    );
    ui.focused_frame = focus.0.filter(|_| create_visible.0);
}

fn sync_screen_state(
    screen_res: &mut Option<ResMut<CharSelectScreenWrap>>,
    reg: &mut FrameRegistry,
    char_list: &CharacterList,
    selected: &SelectedCharIndex,
    create_visible: &CreatePanelVisible,
    campsite_visible: &CampsitePanelVisible,
) {
    let Some(res) = screen_res.as_mut() else {
        return;
    };
    let inner = &mut res.0;
    let new_state = build_char_select_state_full(char_list, selected.0, create_visible.0);
    inner.shared.insert(new_state);
    inner.shared.insert(build_campsite_state(campsite_visible.0));
    inner.screen.sync(&inner.shared, reg);
}

fn build_char_select_state_full(
    char_list: &CharacterList,
    selected: Option<usize>,
    create_visible: bool,
) -> CharSelectState {
    let characters: Vec<CharDisplayEntry> = char_list
        .0
        .iter()
        .map(|ch| CharDisplayEntry {
            name: ch.name.clone(),
            info: format!("Level {}   Race {}   Class {}", ch.level, ch.race, ch.class),
            status: "Ready to enter world".to_string(),
        })
        .collect();
    let selected_name = selected
        .and_then(|i| char_list.0.get(i))
        .map(|ch| ch.name.clone())
        .unwrap_or_else(|| "Character Selection".to_string());
    let status_text = compute_status_text(&char_list.0, selected, create_visible);
    CharSelectState {
        characters,
        selected_index: selected,
        create_panel_visible: create_visible,
        selected_name,
        status_text,
    }
}

fn compute_status_text(
    chars: &[shared::protocol::CharacterListEntry],
    selected: Option<usize>,
    create_visible: bool,
) -> String {
    if create_visible {
        "Choose a name and create a new character".to_string()
    } else if let Some(ch) = selected.and_then(|idx| chars.get(idx)) {
        format!(
            "Realm: {}    Level {}    Race {}    Class {}",
            REALM_NAME, ch.level, ch.race, ch.class
        )
    } else if chars.is_empty() {
        "No characters available on this realm".to_string()
    } else {
        "Select a character to enter the world".to_string()
    }
}

fn sync_editbox_focus_visual(reg: &mut FrameRegistry, id: u64, focused: bool) {
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

fn build_campsite_state(panel_visible: bool) -> CampsiteState {
    let warband = crate::warband_scene::WarbandScenes::load();
    let selected_id = warband.scenes.first().map(|s| s.id);
    CampsiteState {
        scenes: warband.scenes.iter().map(|s| CampsiteEntry {
            id: s.id, name: s.name.clone(),
        }).collect(),
        panel_visible,
        selected_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine::ui::registry::FrameRegistry;

    fn test_registry() -> FrameRegistry {
        FrameRegistry::new(1920.0, 1080.0)
    }

    #[test]
    fn screen_builds_with_empty_char_list() {
        let mut reg = test_registry();
        let state = CharSelectState::default();
        let mut shared = ui_toolkit::screen::SharedContext::new();
        shared.insert(state);
        let mut screen = Screen::new(char_select_screen);
        screen.sync(&shared, &mut reg);
        assert!(reg.get_by_name("CharSelectRoot").is_some());
        assert!(reg.get_by_name("EnterWorld").is_some());
        assert!(reg.get_by_name("BackToLogin").is_some());
    }

    #[test]
    fn screen_builds_with_characters() {
        let mut reg = test_registry();
        let state = CharSelectState {
            characters: vec![CharDisplayEntry {
                name: "TestChar".to_string(),
                info: "Level 60   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            }],
            selected_index: Some(0),
            ..Default::default()
        };
        let mut shared = ui_toolkit::screen::SharedContext::new();
        shared.insert(state);
        let mut screen = Screen::new(char_select_screen);
        screen.sync(&shared, &mut reg);
        assert!(reg.get_by_name("CharCard_0").is_some());
        assert!(reg.get_by_name("CharCard_0Name").is_some());
    }

    #[test]
    fn create_panel_hidden_by_default() {
        let mut reg = test_registry();
        let state = CharSelectState::default();
        let mut shared = ui_toolkit::screen::SharedContext::new();
        shared.insert(state);
        let mut screen = Screen::new(char_select_screen);
        screen.sync(&shared, &mut reg);
        let panel_id = reg.get_by_name("CreatePanel").unwrap();
        let panel = reg.get(panel_id).unwrap();
        assert!(panel.hidden);
    }
}
