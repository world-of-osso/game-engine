use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use lightyear::prelude::*;

use game_engine::ui::automation::{UiAutomationAction, UiAutomationQueue, UiAutomationRunner};
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::char_select_component::CharSelectAction;
use shared::protocol::{AuthChannel, DeleteCharacter, SelectCharacter};

use crate::game_state::GameState;
use crate::networking::CharacterList;
use crate::scenes::char_select::{
    CampsitePanelVisible, CharSelectFocus, CharSelectUi, SelectedCharIndex,
};

#[derive(Message)]
pub(crate) struct CharSelectClickEvent(pub String);

pub(crate) fn char_select_mouse_input(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    ui: Res<UiState>,
    cs_ui: Option<Res<CharSelectUi>>,
    mut events: MessageWriter<CharSelectClickEvent>,
    mut focus: ResMut<CharSelectFocus>,
) {
    let Some(_cs) = cs_ui.as_ref() else { return };
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = cursor_pos(&windows) else {
        return;
    };
    if let Some(action) = find_clicked_action(&ui, cursor.x, cursor.y) {
        events.write(CharSelectClickEvent(action));
    } else {
        focus.0 = None;
    }
}

pub(crate) fn char_select_keyboard_input(
    mut key_events: MessageReader<KeyboardInput>,
    mut selected: ResMut<SelectedCharIndex>,
    char_list: Res<CharacterList>,
    mut senders: Query<&mut MessageSender<SelectCharacter>>,
    mut ui: ResMut<UiState>,
    mut commands: Commands,
) {
    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        if event.key_code == KeyCode::Escape {
            crate::scenes::game_menu::open_game_menu(
                &mut ui,
                &mut commands,
                crate::game_state::GameState::CharSelect,
            );
            return;
        }
        let _ = handle_selection_key(event.key_code, &mut selected, &char_list, &mut senders);
    }
}

pub(crate) fn handle_selection_key(
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

pub(crate) fn char_select_run_automation(
    mut ui: ResMut<UiState>,
    cs_ui: Option<Res<CharSelectUi>>,
    mut selected: ResMut<SelectedCharIndex>,
    char_list: Res<CharacterList>,
    mut senders: Query<&mut MessageSender<SelectCharacter>>,
    mut events: MessageWriter<CharSelectClickEvent>,
    mut queue: ResMut<UiAutomationQueue>,
    mut runner: ResMut<UiAutomationRunner>,
) {
    let Some(_cs) = cs_ui.as_ref() else { return };
    let Some(action) = queue.peek().cloned() else {
        return;
    };
    if !action.is_input_action() {
        return;
    }
    let result = run_automation_action(
        &mut ui,
        &mut selected,
        &char_list,
        &mut senders,
        &mut events,
        &action,
    );
    queue.pop();
    if let Err(err) = result {
        runner.last_error = Some(err.clone());
        error!("UI automation failed in CharSelect: {err}");
    }
}

fn run_automation_action(
    ui: &mut UiState,
    selected: &mut SelectedCharIndex,
    char_list: &CharacterList,
    senders: &mut Query<&mut MessageSender<SelectCharacter>>,
    events: &mut MessageWriter<CharSelectClickEvent>,
    action: &UiAutomationAction,
) -> Result<(), String> {
    match action {
        UiAutomationAction::ClickFrame(name) => {
            let frame_id = ui
                .registry
                .get_by_name(name)
                .ok_or_else(|| format!("unknown char select frame '{name}'"))?;
            let onclick = walk_up_for_onclick(&ui.registry, frame_id)
                .ok_or_else(|| format!("char select frame '{name}' has no onclick action"))?;
            events.write(CharSelectClickEvent(onclick));
        }
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

pub(crate) fn dispatch_char_select_action(
    mut events: MessageReader<CharSelectClickEvent>,
    mut selected: ResMut<SelectedCharIndex>,
    mut focus: ResMut<CharSelectFocus>,
    mut campsite_visible: ResMut<CampsitePanelVisible>,
    mut senders: Query<&mut MessageSender<SelectCharacter>>,
    mut del_senders: Query<&mut MessageSender<DeleteCharacter>>,
    char_list: Res<CharacterList>,
    mut next_state: ResMut<NextState<GameState>>,
    mut selected_scene: Option<ResMut<crate::warband_scene::SelectedWarbandScene>>,
    mut ui: ResMut<UiState>,
    mut _commands: Commands,
) {
    for event in events.read() {
        match CharSelectAction::parse(&event.0) {
            Some(CharSelectAction::SelectChar(idx)) => {
                selected.0 = Some(idx);
                focus.0 = None;
            }
            Some(CharSelectAction::EnterWorld) => {
                try_enter_world(&selected, &char_list, &mut senders);
            }
            Some(CharSelectAction::CreateToggle) => next_state.set(GameState::CharCreate),
            Some(CharSelectAction::DeleteChar) => {
                try_delete_character(&selected, &char_list, &mut del_senders);
            }
            Some(CharSelectAction::Back) => next_state.set(GameState::Login),
            Some(CharSelectAction::Menu) => {
                crate::scenes::game_menu::open_game_menu(
                    &mut ui,
                    &mut _commands,
                    crate::game_state::GameState::CharSelect,
                );
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
            None => {
                focus.0 = None;
            }
        }
    }
}

pub(crate) fn cursor_pos(windows: &Query<&Window>) -> Option<Vec2> {
    windows.iter().next().and_then(|w| w.cursor_position())
}

pub(crate) fn find_clicked_action(ui: &UiState, mx: f32, my: f32) -> Option<String> {
    let hit_id = ui_toolkit::input::find_frame_at(&ui.registry, mx, my)?;
    walk_up_for_onclick(&ui.registry, hit_id)
}

pub(crate) fn walk_up_for_onclick(reg: &FrameRegistry, mut id: u64) -> Option<String> {
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

pub(crate) fn try_enter_world(
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
