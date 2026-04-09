use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use lightyear::prelude::*;

use game_engine::ui::automation::{UiAutomationAction, UiAutomationQueue, UiAutomationRunner};
use game_engine::ui::plugin::UiState;
use game_engine::ui::screens::char_select_component::{CharSelectAction, DELETE_CONFIRM_INPUT};
use shared::protocol::{AuthChannel, DeleteCharacter, SelectCharacter};

use crate::game_state::GameState;
use crate::networking::CharacterList;
use crate::scenes::char_select::{
    CampsitePanelVisible, CharSelectFocus, CharSelectUi, DeleteCharacterConfirmationState,
    DeleteCharacterTarget, SelectedCharIndex, delete_confirm_ready,
};
use crate::scenes::login::helpers::{
    editbox_backspace, editbox_cursor_end, editbox_cursor_home, editbox_delete,
    editbox_move_cursor, hit_frame, insert_char_into_editbox,
};
use crate::ui_input::walk_up_for_onclick;

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
    if let Some(id) = ui
        .registry
        .get_by_name(DELETE_CONFIRM_INPUT.0)
        .filter(|&id| hit_frame(&ui, id, cursor.x, cursor.y))
    {
        focus.0 = Some(id);
        return;
    }
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
    mut focus: ResMut<CharSelectFocus>,
    mut delete_confirm: ResMut<DeleteCharacterConfirmationState>,
    cs_ui: Option<Res<CharSelectUi>>,
    mut del_senders: Query<&mut MessageSender<DeleteCharacter>>,
) {
    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        if event.key_code == KeyCode::Escape {
            if delete_confirm.target.is_some() {
                delete_confirm.target = None;
                delete_confirm.typed_text.clear();
                delete_confirm.elapsed_secs = 0.0;
                focus.0 = None;
                return;
            }
            crate::scenes::game_menu::open_game_menu(
                &mut ui,
                &mut commands,
                crate::game_state::GameState::CharSelect,
            );
            return;
        }
        if handle_delete_confirm_key(
            event,
            &mut ui,
            &mut focus,
            cs_ui.as_deref(),
            &mut delete_confirm,
            &mut del_senders,
        ) {
            continue;
        }
        if delete_confirm.target.is_some() {
            continue;
        }
        let _ = handle_selection_key(event.key_code, &mut selected, &char_list, &mut senders);
    }
}

fn handle_delete_confirm_key(
    event: &KeyboardInput,
    ui: &mut UiState,
    focus: &mut CharSelectFocus,
    cs_ui: Option<&CharSelectUi>,
    delete_confirm: &mut DeleteCharacterConfirmationState,
    del_senders: &mut Query<&mut MessageSender<DeleteCharacter>>,
) -> bool {
    if delete_confirm.target.is_none() {
        return false;
    }
    if event.key_code == KeyCode::Enter && delete_confirm_ready(delete_confirm) {
        try_delete_character(delete_confirm, del_senders);
        focus.0 = None;
        return true;
    }
    let Some(focused_id) = focus.0 else {
        return false;
    };
    if !delete_confirm_input_has_focus(ui, cs_ui, focused_id) {
        return false;
    }
    if let Key::Character(ch) = &event.logical_key {
        insert_delete_confirm_text(ui, delete_confirm, focused_id, ch.as_str());
        return true;
    }
    if !apply_delete_confirm_edit_key(ui, focused_id, event.key_code) {
        return false;
    }
    sync_delete_confirm_text(ui, delete_confirm, focused_id);
    true
}

fn delete_confirm_input_has_focus(
    ui: &UiState,
    cs_ui: Option<&CharSelectUi>,
    focused_id: u64,
) -> bool {
    cs_ui.and_then(|ui| ui.delete_confirm_input) == Some(focused_id)
        || ui.registry.get_by_name(DELETE_CONFIRM_INPUT.0) == Some(focused_id)
}

fn insert_delete_confirm_text(
    ui: &mut UiState,
    delete_confirm: &mut DeleteCharacterConfirmationState,
    focused_id: u64,
    text: &str,
) {
    insert_char_into_editbox(&mut ui.registry, focused_id, &text.to_ascii_uppercase());
    sync_delete_confirm_text(ui, delete_confirm, focused_id);
}

fn sync_delete_confirm_text(
    ui: &UiState,
    delete_confirm: &mut DeleteCharacterConfirmationState,
    focused_id: u64,
) {
    delete_confirm.typed_text =
        crate::scenes::login::helpers::get_editbox_text(&ui.registry, focused_id);
}

fn apply_delete_confirm_edit_key(ui: &mut UiState, focused_id: u64, key: KeyCode) -> bool {
    match key {
        KeyCode::Backspace => editbox_backspace(&mut ui.registry, focused_id),
        KeyCode::Delete => editbox_delete(&mut ui.registry, focused_id),
        KeyCode::ArrowLeft => editbox_move_cursor(&mut ui.registry, focused_id, -1),
        KeyCode::ArrowRight => editbox_move_cursor(&mut ui.registry, focused_id, 1),
        KeyCode::Home => editbox_cursor_home(&mut ui.registry, focused_id),
        KeyCode::End => editbox_cursor_end(&mut ui.registry, focused_id),
        _ => return false,
    }
    true
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
    mut delete_confirm: ResMut<DeleteCharacterConfirmationState>,
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
        &mut delete_confirm,
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
    delete_confirm: &mut DeleteCharacterConfirmationState,
    action: &UiAutomationAction,
) -> Result<(), String> {
    match action {
        UiAutomationAction::ClickFrame(name) => {
            click_char_select_frame(ui, events, name)?;
        }
        UiAutomationAction::TypeText(text) => {
            type_delete_confirm_text(ui, delete_confirm, text)?;
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

fn click_char_select_frame(
    ui: &UiState,
    events: &mut MessageWriter<CharSelectClickEvent>,
    name: &str,
) -> Result<(), String> {
    let frame_id = ui
        .registry
        .get_by_name(name)
        .ok_or_else(|| format!("unknown char select frame '{name}'"))?;
    if name == DELETE_CONFIRM_INPUT.0 {
        return Ok(());
    }
    let onclick = walk_up_for_onclick(&ui.registry, frame_id)
        .ok_or_else(|| format!("char select frame '{name}' has no onclick action"))?;
    events.write(CharSelectClickEvent(onclick));
    Ok(())
}

fn type_delete_confirm_text(
    ui: &mut UiState,
    delete_confirm: &mut DeleteCharacterConfirmationState,
    text: &str,
) -> Result<(), String> {
    let frame_id = ui
        .registry
        .get_by_name(DELETE_CONFIRM_INPUT.0)
        .ok_or("delete confirmation input is not available")?;
    for ch in text.chars() {
        insert_delete_confirm_text(ui, delete_confirm, frame_id, &ch.to_string());
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
    mut selected_scene: Option<ResMut<crate::scenes::char_select::warband::SelectedWarbandScene>>,
    mut ui: ResMut<UiState>,
    mut delete_confirm: ResMut<DeleteCharacterConfirmationState>,
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
                open_delete_confirmation(
                    &selected,
                    &char_list,
                    &mut focus,
                    &mut delete_confirm,
                    &ui,
                );
            }
            Some(CharSelectAction::ConfirmDeleteChar) => {
                if delete_confirm_ready(&delete_confirm) {
                    try_delete_character(&mut delete_confirm, &mut del_senders);
                    focus.0 = None;
                }
            }
            Some(CharSelectAction::CancelDeleteChar) => {
                clear_delete_confirmation(&mut focus, &mut delete_confirm);
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

fn open_delete_confirmation(
    selected: &SelectedCharIndex,
    char_list: &CharacterList,
    focus: &mut CharSelectFocus,
    delete_confirm: &mut DeleteCharacterConfirmationState,
    ui: &UiState,
) {
    let Some(idx) = selected.0 else { return };
    let Some(ch) = char_list.0.get(idx) else {
        return;
    };
    delete_confirm.target = Some(DeleteCharacterTarget {
        character_id: ch.character_id,
        name: ch.name.clone(),
    });
    delete_confirm.typed_text.clear();
    delete_confirm.elapsed_secs = 0.0;
    focus.0 = ui.registry.get_by_name(DELETE_CONFIRM_INPUT.0);
}

fn clear_delete_confirmation(
    focus: &mut CharSelectFocus,
    delete_confirm: &mut DeleteCharacterConfirmationState,
) {
    delete_confirm.target = None;
    delete_confirm.typed_text.clear();
    delete_confirm.elapsed_secs = 0.0;
    focus.0 = None;
}

fn try_delete_character(
    delete_confirm: &mut DeleteCharacterConfirmationState,
    senders: &mut Query<&mut MessageSender<DeleteCharacter>>,
) {
    let Some(ch) = delete_confirm.target.as_ref() else {
        return;
    };
    let msg = DeleteCharacter {
        character_id: ch.character_id,
    };
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(msg.clone());
    }
    info!("Requested delete character '{}'", ch.name);
    delete_confirm.target = None;
    delete_confirm.typed_text.clear();
    delete_confirm.elapsed_secs = 0.0;
}
