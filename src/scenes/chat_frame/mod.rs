//! In-world chat frame: edit box input, slash command dispatch, tabs and fading.

use bevy::ecs::system::SystemParam;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::chat_data::{
    ChatChannelType, ChatMessage as RuntimeChatMessage, ChatState, WhisperState,
};
use game_engine::network_runtime::messages::MessageSenders;
use game_engine::spell_catalog::SpellCatalog;
use game_engine::ui::chat_frame::{
    ChatCommand, ChatFrameState, ChatLine, ChatRow, ChatTab, CombatLogChat, chat_message_line,
    parse_chat_input, wrap_chat_line,
};
use game_engine::ui::frame::WidgetData;
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::popup::PopupStack;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::chat_frame_component::{
    CHAT_EDITBOX, CHAT_FONT, CHAT_FONT_SIZE, CHAT_MESSAGES, CHAT_TEXT_W, ChatFrameView,
    chat_frame_screen,
};
use game_engine::who::{WhoRuntimeState, queue_query};
use shared::protocol::{ChatMessage, CombatChannel, EmoteIntent, GroupInviteIntent};
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::text_measure::measure_text;

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::networking::{ChatInput, EmoteInput, ReconnectState};
use crate::ui_input::walk_up_for_onclick;
use crate::ui_input_mode::{UiInputMode, focused_editbox};

const ENTER_KEYS: [KeyCode; 2] = [KeyCode::Enter, KeyCode::NumpadEnter];

struct ChatFrameRes {
    screen: Screen,
    shared: SharedContext,
    view: ChatFrameView,
    scroll_generation: u64,
}

unsafe impl Send for ChatFrameRes {}
unsafe impl Sync for ChatFrameRes {}

#[derive(Resource)]
struct ChatFrameWrap(ChatFrameRes);

pub struct ChatFramePlugin;

impl Plugin for ChatFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChatFrameState>();
        app.init_resource::<CombatLogChat>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_chat_frame_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_chat_frame_ui);
        app.add_systems(
            Update,
            (
                handle_chat_keyboard,
                handle_chat_tab_clicks,
                sync_chat_frame_ui,
            )
                .chain()
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

fn build_chat_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<ChatFrameState>,
    chat: Res<ChatState>,
    combat: Res<CombatLogChat>,
    catalog: Option<Res<SpellCatalog>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    state.input_open = false;
    let view = ChatFrameView {
        tab: state.tab,
        rows: chat_rows(state.tab, &chat, &combat, catalog.as_deref()),
        input_open: false,
    };
    let mut res = ChatFrameRes {
        screen: Screen::new(chat_frame_screen),
        shared: SharedContext::new(),
        view,
        scroll_generation: 0,
    };
    sync_screen(&mut res, &mut ui.registry, true);
    commands.insert_resource(ChatFrameWrap(res));
}

fn teardown_chat_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<ChatFrameWrap>>,
    mut state: ResMut<ChatFrameState>,
    mut combat: ResMut<CombatLogChat>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    state.input_open = false;
    *combat = CombatLogChat::default();
    commands.remove_resource::<ChatFrameWrap>();
}

/// Where a submitted line goes.
#[derive(SystemParam)]
struct ChatOutputs<'w, 's> {
    chat_input: ResMut<'w, ChatInput>,
    emote_input: ResMut<'w, EmoteInput>,
    who: Option<ResMut<'w, WhoRuntimeState>>,
    invites: MessageSenders<'w, 's, GroupInviteIntent>,
    chat: ResMut<'w, ChatState>,
}

#[derive(SystemParam)]
struct ChatOpenGate<'w> {
    mode: Res<'w, UiInputMode>,
    reconnect: Option<Res<'w, ReconnectState>>,
    popups: Option<Res<'w, PopupStack>>,
    whispers: Res<'w, WhisperState>,
}

/// Enter / `/` / R open the edit box in World mode; while it has focus, keys edit it,
/// Enter sends and Up/Down recall sent lines. Escape belongs to the in-world Escape
/// chain, which clears focus; losing focus closes the box without sending.
fn handle_chat_keyboard(
    mut key_events: MessageReader<KeyboardInput>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    gate: ChatOpenGate,
    mut ui: ResMut<UiState>,
    mut state: ResMut<ChatFrameState>,
    mut out: ChatOutputs,
) {
    let Some(editbox) = ui.registry.get_by_name(CHAT_EDITBOX.0) else {
        key_events.read().for_each(drop);
        return;
    };
    let focused = focused_editbox(&ui) == Some(editbox);
    if state.input_open && !focused {
        close_input(&mut ui, editbox, &mut state);
    }
    if !focused {
        // Drain every event: none of them may reach the edit box once it opens.
        let slash = key_events
            .read()
            .filter(|event| event.state == ButtonState::Pressed && is_slash(event))
            .count()
            > 0;
        if let Some(prefill) = open_prefill(&keys, slash, &gate) {
            keys.clear_just_pressed(KeyCode::Enter);
            keys.clear_just_pressed(KeyCode::NumpadEnter);
            open_input(&mut ui, editbox, &mut state, &prefill);
        }
        return;
    }
    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        match event.key_code {
            KeyCode::Enter | KeyCode::NumpadEnter => {
                let line = editbox_text(&ui.registry, editbox);
                close_input(&mut ui, editbox, &mut state);
                state.remember_sent(&line);
                dispatch(
                    parse_chat_input(&line, gate.whispers.reply_target.as_deref()),
                    &mut out,
                );
                // One action per press: the popup Enter handler must not also see it.
                for key in ENTER_KEYS {
                    keys.clear_just_pressed(key);
                }
                break;
            }
            KeyCode::ArrowUp => {
                if let Some(line) = state.history_prev().map(str::to_owned) {
                    set_editbox_text(&mut ui.registry, editbox, &line);
                }
            }
            KeyCode::ArrowDown => {
                if let Some(line) = state.history_next().map(str::to_owned) {
                    set_editbox_text(&mut ui.registry, editbox, &line);
                }
            }
            KeyCode::Escape => {}
            _ => edit_from_key(&mut ui.registry, editbox, event),
        }
    }
}

fn is_slash(event: &KeyboardInput) -> bool {
    matches!(&event.logical_key, Key::Character(ch) if ch.as_str() == "/")
}

/// Edit box prefill for an open request, or `None` when nothing opens it this frame.
/// Enter never opens chat while a popup owns it.
fn open_prefill(keys: &ButtonInput<KeyCode>, slash: bool, gate: &ChatOpenGate) -> Option<String> {
    let reconnecting = gate.reconnect.as_ref().is_some_and(|r| r.is_active());
    if !gate.mode.is_world() || reconnecting {
        return None;
    }
    if keys.any_just_pressed(ENTER_KEYS) {
        let popup_open = gate.popups.as_ref().is_some_and(|p| p.top().is_some());
        return (!popup_open).then(String::new);
    }
    if slash {
        return Some("/".to_string());
    }
    if keys.just_pressed(KeyCode::KeyR) {
        return gate
            .whispers
            .reply_target
            .as_ref()
            .map(|target| format!("/w {target} "));
    }
    None
}

fn open_input(ui: &mut UiState, editbox: u64, state: &mut ChatFrameState, prefill: &str) {
    set_editbox_text(&mut ui.registry, editbox, prefill);
    ui.registry.focused_frame = Some(editbox);
    ui.focused_frame = Some(editbox);
    state.input_open = true;
    state.history_index = None;
}

fn close_input(ui: &mut UiState, editbox: u64, state: &mut ChatFrameState) {
    set_editbox_text(&mut ui.registry, editbox, "");
    if focused_editbox(ui) == Some(editbox) {
        ui.registry.focused_frame = None;
        ui.focused_frame = None;
    }
    state.input_open = false;
    state.history_index = None;
}

fn dispatch(command: ChatCommand, out: &mut ChatOutputs) {
    match command {
        ChatCommand::Send { channel, text } => {
            out.chat_input.0 = Some(ChatMessage {
                sender: String::new(),
                content: text,
                channel,
            });
        }
        ChatCommand::Emote(emote) => out.emote_input.0 = Some(EmoteIntent { emote }),
        ChatCommand::Who(query) => match out.who.as_mut() {
            Some(who) => queue_query(who, query),
            None => add_system_line(&mut out.chat, "Who is unavailable."),
        },
        ChatCommand::Invite(name) => {
            let mut sent = false;
            for mut sender in out.invites.iter_mut() {
                sender.send::<CombatChannel>(GroupInviteIntent { name: name.clone() });
                sent = true;
            }
            if !sent {
                add_system_line(&mut out.chat, "You are not connected.");
            }
        }
        ChatCommand::System(lines) => {
            for line in lines {
                add_system_line(&mut out.chat, &line);
            }
        }
        ChatCommand::None => {}
    }
}

fn add_system_line(chat: &mut ChatState, text: &str) {
    chat.add_message(RuntimeChatMessage {
        channel_type: ChatChannelType::System,
        channel_name: String::new(),
        sender: String::new(),
        text: text.to_string(),
        timestamp: 0.0,
    });
}

fn editbox_text(registry: &FrameRegistry, editbox: u64) -> String {
    match registry
        .get(editbox)
        .and_then(|frame| frame.widget_data.as_ref())
    {
        Some(WidgetData::EditBox(data)) => data.text.clone(),
        _ => String::new(),
    }
}

fn set_editbox_text(registry: &mut FrameRegistry, editbox: u64, value: &str) {
    if let Some(WidgetData::EditBox(data)) = registry
        .get_mut(editbox)
        .and_then(|frame| frame.widget_data.as_mut())
    {
        data.replace_range(0, data.text.len(), value);
        data.cursor_end();
    }
}

fn edit_from_key(registry: &mut FrameRegistry, editbox: u64, event: &KeyboardInput) {
    let Some(WidgetData::EditBox(data)) = registry
        .get_mut(editbox)
        .and_then(|frame| frame.widget_data.as_mut())
    else {
        return;
    };
    match event.key_code {
        KeyCode::Backspace => data.backspace(),
        KeyCode::Delete => data.delete_forward(),
        KeyCode::ArrowLeft => data.cursor_left(),
        KeyCode::ArrowRight => data.cursor_right(),
        KeyCode::Home => data.cursor_home(),
        KeyCode::End => data.cursor_end(),
        _ => {
            if let Some(typed) = event.text.as_deref()
                && !typed.chars().any(char::is_control)
            {
                data.insert_at_cursor(typed);
            }
        }
    }
}

fn handle_chat_tab_clicks(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    ui: Res<UiState>,
    mut state: ResMut<ChatFrameState>,
) {
    if !mouse.is_some_and(|mouse| mouse.just_pressed(MouseButton::Left)) {
        return;
    }
    let Some(cursor) = windows
        .single()
        .ok()
        .and_then(|window| ui_cursor_position(&ui.registry, window))
    else {
        return;
    };
    let Some(tab) = find_frame_at(&ui.registry, cursor.x, cursor.y)
        .and_then(|frame| walk_up_for_onclick(&ui.registry, frame))
        .and_then(|action| ChatTab::from_action(&action))
    else {
        return;
    };
    if state.tab != tab {
        state.tab = tab;
    }
}

fn sync_chat_frame_ui(
    mut ui: ResMut<UiState>,
    wrap: Option<ResMut<ChatFrameWrap>>,
    state: Res<ChatFrameState>,
    chat: Res<ChatState>,
    combat: Res<CombatLogChat>,
    catalog: Option<Res<SpellCatalog>>,
) {
    let Some(mut wrap) = wrap else { return };
    let res = &mut wrap.0;
    let tab_changed = res.view.tab != state.tab;
    let rows_dirty = tab_changed
        || chat.is_changed()
        || combat.is_changed()
        || catalog.as_ref().is_some_and(|catalog| catalog.is_changed());
    let view_changed = rows_dirty || res.view.input_open != state.input_open;
    let scrolled = ui.registry.scroll_lists.generation(CHAT_MESSAGES) != res.scroll_generation;
    if !view_changed && !scrolled {
        return;
    }
    if rows_dirty {
        res.view.rows = chat_rows(state.tab, &chat, &combat, catalog.as_deref());
    }
    res.view.tab = state.tab;
    res.view.input_open = state.input_open;
    sync_screen(res, &mut ui.registry, tab_changed);
}

/// Rebuild the screen; a list showing its newest row keeps following new rows.
fn sync_screen(res: &mut ChatFrameRes, registry: &mut FrameRegistry, force_bottom: bool) {
    let at_bottom = registry
        .scroll_lists
        .get(CHAT_MESSAGES)
        .is_none_or(|list| list.first_row >= list.geometry.max_first_row());
    res.shared.insert(res.view.clone());
    res.screen.sync(&res.shared, registry);
    if (force_bottom || at_bottom) && registry.scroll_lists.scroll_to(CHAT_MESSAGES, usize::MAX) {
        res.screen.sync(&res.shared, registry);
    }
    res.scroll_generation = registry.scroll_lists.generation(CHAT_MESSAGES);
}

fn chat_rows(
    tab: ChatTab,
    chat: &ChatState,
    combat: &CombatLogChat,
    catalog: Option<&SpellCatalog>,
) -> Vec<ChatRow> {
    let lines: Vec<ChatLine> = match tab {
        ChatTab::CombatLog => combat.lines.clone(),
        tab => chat
            .messages
            .iter()
            .filter(|msg| tab.shows(msg))
            .map(chat_message_line)
            .collect(),
    };
    let spell_name = |id: u32| match catalog.and_then(|catalog| catalog.get(id)) {
        Some(spell) => spell.name.to_string(),
        None => format!("Spell #{id}"),
    };
    lines
        .iter()
        .flat_map(|line| wrap_chat_line(line, spell_name, CHAT_TEXT_W, measure_chat_text))
        .collect()
}

fn measure_chat_text(value: &str) -> f32 {
    measure_text(value, CHAT_FONT, CHAT_FONT_SIZE).map_or(0.0, |(width, _)| width)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
