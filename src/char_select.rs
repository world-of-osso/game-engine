use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use lightyear::prelude::*;

use game_engine::ui::atlas;
use game_engine::ui::automation::{UiAutomationAction, UiAutomationQueue, UiAutomationRunner};
use game_engine::ui::frame::{Dimension, NineSlice};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::char_select_component::{
    BACK_BUTTON, CHAR_LIST_PANEL, CHAR_SELECT_ROOT, CREATE_CHAR_BUTTON, CampsiteEntry,
    CampsiteState, CharDisplayEntry, CharSelectAction, CharSelectState, DELETE_CHAR_BUTTON,
    ENTER_WORLD_BUTTON, SELECTED_NAME_TEXT, STATUS_TEXT, char_select_screen,
};
use game_engine::ui::widgets::texture::TextureSource;
use game_engine::ui_resource;
use shared::protocol::{AuthChannel, DeleteCharacter, SelectCharacter};
use ui_toolkit::screen::Screen;

use crate::game_state::GameState;
use crate::networking::CharacterList;

use crate::login_screen_helpers as helpers;
use helpers::{hit_frame, set_button_hovered};

const REALM_NAME: &str = "World of Osso";
ui_resource! {
    pub(crate) CharSelectUi {
        root: CHAR_SELECT_ROOT,
        enter_button: ENTER_WORLD_BUTTON,
        create_button: CREATE_CHAR_BUTTON,
        delete_button: DELETE_CHAR_BUTTON,
        back_button: BACK_BUTTON,
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

/// Marker resource: skip CharSelect UI and enter world immediately with first/preselected char.
#[derive(Resource)]
pub struct AutoEnterWorld;

pub struct CharSelectPlugin;

impl Plugin for CharSelectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedCharIndex>();
        app.init_resource::<CharSelectFocus>();
        app.add_systems(OnEnter(GameState::CharSelect), build_char_select_ui);
        app.add_systems(OnExit(GameState::CharSelect), teardown_char_select_ui);
        app.add_systems(
            Update,
            (
                char_select_mouse_input,
                char_select_keyboard_input,
                char_select_run_automation,
                char_select_hover_visuals,
                char_select_update_visuals,
                auto_enter_world,
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
}

fn set_list_panel_backdrop(reg: &mut FrameRegistry, id: u64) {
    if let Some(frame) = reg.get_mut(id) {
        frame.nine_slice = atlas_nine_slice(
            "glues-characterselect-card-all-bg",
            frame.resolved_width(),
            frame.resolved_height(),
        );
    }
}

fn atlas_nine_slice(name: &str, frame_w: f32, frame_h: f32) -> Option<NineSlice> {
    let uv_edges = atlas::nine_slice_margins(name)?;
    let _ = (frame_w, frame_h);
    let edge_sizes = uv_edges;
    Some(NineSlice {
        edge_size: edge_sizes[0],
        edge_size_v: Some(edge_sizes[1]),
        edge_sizes: Some(edge_sizes),
        uv_edge_size: Some(uv_edges[0]),
        uv_edge_sizes: Some(uv_edges),
        texture: Some(TextureSource::Atlas(name.to_string())),
        bg_color: [1.0, 1.0, 1.0, 1.0],
        border_color: [1.0, 1.0, 1.0, 1.0],
        ..Default::default()
    })
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
    mut campsite_visible: ResMut<CampsitePanelVisible>,
    mut senders: Query<&mut MessageSender<SelectCharacter>>,
    mut del_senders: Query<&mut MessageSender<DeleteCharacter>>,
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
        cs,
        &ui,
        cursor,
        &mut selected,
        &mut focus,
        &mut campsite_visible,
        &mut senders,
        &mut del_senders,
        &char_list,
        &mut next_state,
        selected_scene,
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
    campsite_visible: &mut CampsitePanelVisible,
    senders: &mut Query<&mut MessageSender<SelectCharacter>>,
    del_senders: &mut Query<&mut MessageSender<DeleteCharacter>>,
    char_list: &CharacterList,
    next_state: &mut NextState<GameState>,
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
        Some(CharSelectAction::CampsiteToggle) => {
            campsite_visible.0 = !campsite_visible.0;
        }
        Some(CharSelectAction::SelectCampsite(id)) => {
            if let Some(ref mut sel) = selected_scene {
                sel.scene_id = id;
            }
            campsite_visible.0 = false;
        }
        None => {
            focus.0 = None;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_cs_click(
    _cs: &CharSelectUi,
    ui: &UiState,
    cursor: Vec2,
    selected: &mut SelectedCharIndex,
    focus: &mut CharSelectFocus,
    campsite_visible: &mut CampsitePanelVisible,
    senders: &mut Query<&mut MessageSender<SelectCharacter>>,
    del_senders: &mut Query<&mut MessageSender<DeleteCharacter>>,
    char_list: &CharacterList,
    next_state: &mut NextState<GameState>,
    selected_scene: Option<ResMut<crate::warband_scene::SelectedWarbandScene>>,
) {
    let (mx, my) = (cursor.x, cursor.y);
    let action = find_clicked_action(ui, mx, my);
    if let Some(action) = action {
        dispatch_onclick(
            &action,
            selected,
            focus,
            campsite_visible,
            senders,
            del_senders,
            char_list,
            next_state,
            selected_scene,
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

// --- Keyboard ---

fn char_select_keyboard_input(
    mut key_events: MessageReader<KeyboardInput>,
    mut selected: ResMut<SelectedCharIndex>,
    char_list: Res<CharacterList>,
    mut senders: Query<&mut MessageSender<SelectCharacter>>,
) {
    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        let _ = handle_selection_key(event.key_code, &mut selected, &char_list, &mut senders);
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

#[allow(clippy::too_many_arguments)]
fn char_select_run_automation(
    mut ui: ResMut<UiState>,
    cs_ui: Option<Res<CharSelectUi>>,
    mut selected: ResMut<SelectedCharIndex>,
    mut focus: ResMut<CharSelectFocus>,
    mut campsite_visible: ResMut<CampsitePanelVisible>,
    mut senders: Query<&mut MessageSender<SelectCharacter>>,
    mut del_senders: Query<&mut MessageSender<DeleteCharacter>>,
    char_list: Res<CharacterList>,
    mut next_state: ResMut<NextState<GameState>>,
    selected_scene: Option<ResMut<crate::warband_scene::SelectedWarbandScene>>,
    mut queue: ResMut<UiAutomationQueue>,
    mut runner: ResMut<UiAutomationRunner>,
) {
    let Some(cs) = cs_ui.as_ref() else { return };
    let Some(action) = queue.peek().cloned() else {
        return;
    };
    if !action.is_input_action() {
        return;
    }
    let result = run_char_select_automation_action(
        &mut ui,
        cs,
        &mut selected,
        &mut focus,
        &mut campsite_visible,
        &mut senders,
        &mut del_senders,
        &char_list,
        &mut next_state,
        selected_scene,
        &action,
    );
    queue.pop();
    if let Err(err) = result {
        runner.last_error = Some(err.clone());
        error!("UI automation failed in CharSelect: {err}");
    }
}

#[allow(clippy::too_many_arguments)]
fn run_char_select_automation_action(
    ui: &mut UiState,
    cs: &CharSelectUi,
    selected: &mut SelectedCharIndex,
    focus: &mut CharSelectFocus,
    campsite_visible: &mut CampsitePanelVisible,
    senders: &mut Query<&mut MessageSender<SelectCharacter>>,
    del_senders: &mut Query<&mut MessageSender<DeleteCharacter>>,
    char_list: &CharacterList,
    next_state: &mut NextState<GameState>,
    selected_scene: Option<ResMut<crate::warband_scene::SelectedWarbandScene>>,
    action: &UiAutomationAction,
) -> Result<(), String> {
    match action {
        UiAutomationAction::ClickFrame(name) => click_char_select_frame(
            ui,
            cs,
            selected,
            focus,
            campsite_visible,
            senders,
            del_senders,
            char_list,
            next_state,
            selected_scene,
            name,
        )?,
        UiAutomationAction::TypeText(_) => {
            return Err("char select does not support text entry automation".to_string());
        }
        UiAutomationAction::PressKey(key) => {
            if handle_selection_key(*key, selected, char_list, senders) {
                return Ok(());
            }
            return Err(format!("unsupported char select key press: {key:?}"));
        }
        UiAutomationAction::WaitForState(_, _)
        | UiAutomationAction::WaitForFrame(_, _)
        | UiAutomationAction::DumpTree
        | UiAutomationAction::DumpUiTree => {}
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn click_char_select_frame(
    ui: &mut UiState,
    _cs: &CharSelectUi,
    selected: &mut SelectedCharIndex,
    focus: &mut CharSelectFocus,
    campsite_visible: &mut CampsitePanelVisible,
    senders: &mut Query<&mut MessageSender<SelectCharacter>>,
    del_senders: &mut Query<&mut MessageSender<DeleteCharacter>>,
    char_list: &CharacterList,
    next_state: &mut NextState<GameState>,
    selected_scene: Option<ResMut<crate::warband_scene::SelectedWarbandScene>>,
    frame_name: &str,
) -> Result<(), String> {
    let frame_id = ui
        .registry
        .get_by_name(frame_name)
        .ok_or_else(|| format!("unknown char select frame '{frame_name}'"))?;
    let action = walk_up_for_onclick(&ui.registry, frame_id)
        .ok_or_else(|| format!("char select frame '{frame_name}' has no onclick action"))?;
    dispatch_onclick(
        &action,
        selected,
        focus,
        campsite_visible,
        senders,
        del_senders,
        char_list,
        next_state,
        selected_scene,
    );
    Ok(())
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
    ];
    for id in button_ids {
        let hovered = cursor.is_some_and(|c| hit_active_frame(&ui, id, c.x, c.y));
        set_button_hovered(&mut ui.registry, id, hovered);
    }
}

// --- Visual Updates ---

#[allow(clippy::too_many_arguments)]
fn char_select_update_visuals(
    mut ui: ResMut<UiState>,
    cs_ui: Option<Res<CharSelectUi>>,
    selected: Res<SelectedCharIndex>,
    campsite_visible: Res<CampsitePanelVisible>,
    char_list: Res<CharacterList>,
    mut screen_res: Option<ResMut<CharSelectScreenWrap>>,
) {
    sync_screen_state(
        &mut screen_res,
        &mut ui.registry,
        cs_ui.as_deref(),
        &char_list,
        &selected,
        &campsite_visible,
    );
    ui.focused_frame = None;
}

fn sync_screen_state(
    screen_res: &mut Option<ResMut<CharSelectScreenWrap>>,
    reg: &mut FrameRegistry,
    cs_ui: Option<&CharSelectUi>,
    char_list: &CharacterList,
    selected: &SelectedCharIndex,
    campsite_visible: &CampsitePanelVisible,
) {
    let Some(res) = screen_res.as_mut() else {
        return;
    };
    let inner = &mut res.0;
    let new_state = build_char_select_state_full(char_list, selected.0);
    inner.shared.insert(new_state);
    inner
        .shared
        .insert(build_campsite_state(campsite_visible.0));
    inner.screen.sync(&inner.shared, reg);
    if let Some(cs) = cs_ui {
        apply_post_setup(reg, cs);
    }
}

fn build_char_select_state_full(
    char_list: &CharacterList,
    selected: Option<usize>,
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
    let status_text = compute_status_text(&char_list.0, selected);
    CharSelectState {
        characters,
        selected_index: selected,
        selected_name,
        status_text,
    }
}

fn compute_status_text(
    chars: &[shared::protocol::CharacterListEntry],
    selected: Option<usize>,
) -> String {
    if let Some(ch) = selected.and_then(|idx| chars.get(idx)) {
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
        scenes: warband
            .scenes
            .iter()
            .map(|s| CampsiteEntry {
                id: s.id,
                name: s.name.clone(),
            })
            .collect(),
        panel_visible,
        selected_id,
    }
}

fn auto_enter_world(
    auto: Option<Res<AutoEnterWorld>>,
    selected: Res<SelectedCharIndex>,
    char_list: Res<crate::networking_auth::CharacterList>,
    mut senders: Query<&mut MessageSender<SelectCharacter>>,
    mut commands: Commands,
) {
    if auto.is_none() {
        return;
    }
    try_enter_world(&selected, &char_list, &mut senders);
    commands.remove_resource::<AutoEnterWorld>();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    use bevy::input::ButtonInput;
    use bevy::input::keyboard::KeyboardInput;
    use bevy::window::PrimaryWindow;
    use shared::protocol::CharacterListEntry;

    use game_engine::ui::automation::{UiAutomationAction, UiAutomationPlugin, UiAutomationQueue};
    use game_engine::ui::event::EventBus;
    use game_engine::ui::frame::WidgetData;
    use game_engine::ui::registry::FrameRegistry;

    fn test_registry() -> FrameRegistry {
        FrameRegistry::new(1920.0, 1080.0)
    }

    fn build_screen(state: CharSelectState) -> FrameRegistry {
        let mut reg = test_registry();
        let mut shared = ui_toolkit::screen::SharedContext::new();
        shared.insert(state);
        Screen::new(char_select_screen).sync(&shared, &mut reg);
        reg
    }

    fn build_screen_with_campsites(
        state: CharSelectState,
        campsite: CampsiteState,
    ) -> FrameRegistry {
        let mut reg = test_registry();
        let mut shared = ui_toolkit::screen::SharedContext::new();
        shared.insert(state);
        shared.insert(campsite);
        Screen::new(char_select_screen).sync(&shared, &mut reg);
        reg
    }
    #[test]
    fn screen_builds_with_empty_char_list() {
        let reg = build_screen(CharSelectState::default());
        assert!(reg.get_by_name("CharSelectRoot").is_some());
        assert!(reg.get_by_name("EnterWorld").is_some());
        assert!(reg.get_by_name("BackToLogin").is_some());
    }

    #[test]
    fn screen_builds_with_characters() {
        let reg = build_screen(CharSelectState {
            characters: vec![CharDisplayEntry {
                name: "TestChar".to_string(),
                info: "Level 60   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            }],
            selected_index: Some(0),
            ..Default::default()
        });
        assert!(reg.get_by_name("CharCard_0").is_some());
        assert!(reg.get_by_name("CharCard_0Name").is_some());
    }

    #[test]
    fn character_cards_use_tinted_atlas_textures_without_css_border() {
        let reg = build_screen(CharSelectState {
            characters: vec![CharDisplayEntry {
                name: "TestChar".to_string(),
                info: "Level 60   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            }],
            selected_index: Some(0),
            ..Default::default()
        });

        let card_id = reg.get_by_name("CharCard_0").expect("CharCard_0");
        let card = reg.get(card_id).expect("card frame");
        assert!(
            card.border.is_none(),
            "card should rely on atlas art, not CSS border"
        );

        let backdrop_id = reg
            .get_by_name("CharCard_0Backdrop")
            .expect("CharCard_0Backdrop");
        let backdrop = reg.get(backdrop_id).expect("backdrop frame");
        let Some(WidgetData::Texture(backdrop_tex)) = backdrop.widget_data.as_ref() else {
            panic!("backdrop should be a texture");
        };
        assert_eq!(backdrop_tex.vertex_color, [0.76, 0.70, 0.57, 0.96]);

        let selected_id = reg
            .get_by_name("CharCard_0Selected")
            .expect("CharCard_0Selected");
        let selected = reg.get(selected_id).expect("selected frame");
        let Some(WidgetData::Texture(selected_tex)) = selected.widget_data.as_ref() else {
            panic!("selected highlight should be a texture");
        };
        assert_eq!(selected_tex.vertex_color, [0.82, 0.74, 0.46, 0.9]);
    }

    #[test]
    fn screen_does_not_include_inline_create_panel() {
        let reg = build_screen(CharSelectState::default());
        assert!(reg.get_by_name("CreatePanel").is_none());
    }

    #[test]
    fn character_list_backdrop_uses_atlas_slice_metadata() {
        let ns = atlas_nine_slice("glues-characterselect-card-all-bg", 386.0, 520.0)
            .expect("atlas-backed nine-slice");
        assert_eq!(ns.uv_edge_sizes, Some([14.0, 11.0, 14.0, 17.0]));
        let display = ns.edge_sizes.expect("display edge sizes");
        assert_eq!(display, [14.0, 11.0, 14.0, 17.0]);
    }

    #[test]
    fn campsite_tab_is_anchored_to_top_center_without_offsets() {
        let reg = build_screen_with_campsites(
            CharSelectState::default(),
            CampsiteState {
                scenes: vec![CampsiteEntry {
                    id: 1,
                    name: "Forest".to_string(),
                }],
                panel_visible: true,
                selected_id: Some(1),
            },
        );

        let root_id = reg.get_by_name("CharSelectRoot").expect("CharSelectRoot");
        let tab_id = reg.get_by_name("CampsiteTab").expect("CampsiteTab");
        let tab = reg.get(tab_id).expect("tab frame");

        assert_eq!(tab.anchors.len(), 1);
        assert_eq!(
            tab.anchors[0].point,
            game_engine::ui::anchor::AnchorPoint::Top
        );
        assert_eq!(
            tab.anchors[0].relative_point,
            game_engine::ui::anchor::AnchorPoint::Top
        );
        assert_eq!(tab.anchors[0].relative_to, Some(root_id));
        assert_eq!(tab.anchors[0].x_offset, 0.0);
        assert_eq!(tab.anchors[0].y_offset, 0.0);
    }

    #[test]
    fn campsite_panel_is_anchored_to_top_center_without_offsets() {
        let reg = build_screen_with_campsites(
            CharSelectState::default(),
            CampsiteState {
                scenes: vec![CampsiteEntry {
                    id: 1,
                    name: "Forest".to_string(),
                }],
                panel_visible: true,
                selected_id: Some(1),
            },
        );

        let root_id = reg.get_by_name("CharSelectRoot").expect("CharSelectRoot");
        let panel_id = reg.get_by_name("CampsitePanel").expect("CampsitePanel");
        let panel = reg.get(panel_id).expect("panel frame");

        assert_eq!(panel.anchors.len(), 1);
        assert_eq!(
            panel.anchors[0].point,
            game_engine::ui::anchor::AnchorPoint::Top
        );
        assert_eq!(
            panel.anchors[0].relative_point,
            game_engine::ui::anchor::AnchorPoint::Top
        );
        assert_eq!(panel.anchors[0].relative_to, Some(root_id));
        assert_eq!(panel.anchors[0].x_offset, 0.0);
        assert_eq!(panel.anchors[0].y_offset, -12.0);
    }

    #[test]
    fn automation_click_create_char_transitions_to_char_create() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.add_plugins(UiAutomationPlugin);
        app.add_plugins(CharSelectPlugin);
        app.add_message::<KeyboardInput>();
        app.insert_resource(UiState {
            registry: FrameRegistry::new(0.0, 0.0),
            event_bus: EventBus::new(),
            focused_frame: None,
        });
        app.insert_resource(ButtonInput::<MouseButton>::default());
        app.insert_resource(CharacterList(vec![CharacterListEntry {
            character_id: 1,
            name: "Elara".to_string(),
            level: 1,
            race: 1,
            class: 1,
            appearance: shared::components::CharacterAppearance::default(),
        }]));
        app.insert_state(GameState::CharSelect);
        app.insert_resource(UiAutomationQueue(VecDeque::from([
            UiAutomationAction::ClickFrame("CreateChar".to_string()),
        ])));

        let mut window = Window::default();
        window.resolution.set(1280.0, 720.0);
        app.world_mut().spawn((window, PrimaryWindow));

        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<GameState>>().get(),
            GameState::CharCreate
        );
        assert!(
            app.world().resource::<UiAutomationQueue>().is_empty(),
            "expected CreateChar click to be consumed by CharSelect automation"
        );
    }
}
