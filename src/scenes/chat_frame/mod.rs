//! In-world chat frame: edit box input, slash command dispatch, tabs, tab flashing,
//! scrolling and Copy Chat.

use std::sync::{Arc, Mutex};

use bevy::ecs::system::SystemParam;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::chat_data::{
    ChatChannelType, ChatMessage as RuntimeChatMessage, ChatState, WhisperState, now_timestamp,
};
use game_engine::group_state::GroupCommand;
use game_engine::spell_catalog::SpellCatalog;
use game_engine::ui::chat_frame::{
    ChatCommand, ChatEntry, ChatFrameState, ChatTab, CombatLogChat, chat_message_line,
    copy_chat_text, flash_alpha, local_timestamp, messages_that_fit, parse_chat_input,
    tabs_to_flash, wrap_chat_line,
};
use game_engine::ui::frame::WidgetData;
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::popup::PopupStack;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::chat_frame_component::{
    CHAT_EDITBOX, CHAT_FONT, CHAT_FONT_SIZE, CHAT_LINE_H, CHAT_MESSAGES, CHAT_MESSAGES_AVAILABLE_H,
    COPY_CHAT_ACTION, ChatFrameView, ChatMessageView, SCROLL_TO_BOTTOM_ACTION, chat_frame_screen,
    chat_text_area, tab_flash_name,
};
use game_engine::who::{WhoRuntimeState, queue_query};
use shared::protocol::{ChatMessage, EmoteIntent};
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::text_measure::measure_text;

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::networking::{ChatInput, EmoteInput, ReconnectState};
use crate::ui_input::walk_up_for_onclick;
use crate::ui_input_mode::{UiInputMode, focused_editbox};

#[cfg(test)]
#[path = "../../ui/screens/menu_character_layout_test_support.rs"]
mod native_layout_support;

const ENTER_KEYS: [KeyCode; 2] = [KeyCode::Enter, KeyCode::NumpadEnter];

struct ChatFrameRes {
    screen: Screen,
    shared: SharedContext,
    view: ChatFrameView,
    /// `ChatState::received` / `CombatLogChat::received` already accounted for.
    seen_chat: u64,
    seen_combat: u64,
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
        app.init_resource::<ChatClipboard>();
        app.add_message::<GroupCommand>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_chat_frame_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_chat_frame_ui);
        app.add_systems(
            Update,
            (
                handle_chat_keyboard,
                handle_chat_clicks,
                handle_chat_wheel,
                sync_chat_frame_ui,
                animate_tab_flashes,
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
    state.scroll = 0;
    state.flashing.clear();
    let mut res = ChatFrameRes {
        screen: Screen::new(chat_frame_screen),
        shared: SharedContext::new(),
        view: chat_view(&state, &chat, &combat, catalog.as_deref()),
        seen_chat: chat.received,
        seen_combat: combat.received,
    };
    sync_screen(&mut res, &mut ui.registry);
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
struct ChatOutputs<'w> {
    chat_input: ResMut<'w, ChatInput>,
    emote_input: ResMut<'w, EmoteInput>,
    who: Option<ResMut<'w, WhoRuntimeState>>,
    group: MessageWriter<'w, GroupCommand>,
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
        ChatCommand::Group(command) => {
            out.group.write(command);
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
        timestamp: now_timestamp(),
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

/// Writes Copy Chat text to the system clipboard.
#[derive(Resource, Clone)]
pub(crate) struct ChatClipboard(Arc<dyn Fn(&str) -> Result<(), String> + Send + Sync>);

impl Default for ChatClipboard {
    /// One clipboard for the process lifetime: on Linux the copied text is served only while
    /// the owning `arboard::Clipboard` lives.
    fn default() -> Self {
        let clipboard: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);
        Self(Arc::new(move |text| {
            let mut clipboard = clipboard
                .lock()
                .map_err(|_| "clipboard lock poisoned".to_string())?;
            if clipboard.is_none() {
                *clipboard = Some(
                    arboard::Clipboard::new().map_err(|err| format!("clipboard init: {err}"))?,
                );
            }
            clipboard
                .as_mut()
                .expect("clipboard was just created")
                .set_text(text)
                .map_err(|err| format!("clipboard write: {err}"))
        }))
    }
}

#[derive(SystemParam)]
struct CopyChat<'w> {
    chat: ResMut<'w, ChatState>,
    combat: Res<'w, CombatLogChat>,
    catalog: Option<Res<'w, SpellCatalog>>,
    clipboard: Res<'w, ChatClipboard>,
}

impl CopyChat<'_> {
    fn copy(&mut self, tab: ChatTab) {
        let entries = tab_entries(tab, &self.chat, &self.combat);
        let text = copy_chat_text(
            &entries,
            spell_namer(self.catalog.as_deref()),
            &chrono::Local,
        );
        if let Err(err) = (self.clipboard.0)(&text) {
            add_system_line(&mut self.chat, &format!("Copy Chat failed: {err}"));
        }
    }
}

/// Tab, Copy Chat and Scroll to bottom clicks.
fn handle_chat_clicks(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    ui: Res<UiState>,
    mut state: ResMut<ChatFrameState>,
    mut copy: CopyChat,
) {
    if !mouse.is_some_and(|mouse| mouse.just_pressed(MouseButton::Left)) {
        return;
    }
    let Some(action) = windows
        .single()
        .ok()
        .and_then(|window| ui_cursor_position(&ui.registry, window))
        .and_then(|cursor| find_frame_at(&ui.registry, cursor.x, cursor.y))
        .and_then(|frame| walk_up_for_onclick(&ui.registry, frame))
    else {
        return;
    };
    if let Some(tab) = ChatTab::from_action(&action) {
        state.select_tab(tab);
    } else if action == SCROLL_TO_BOTTOM_ACTION {
        state.scroll = 0;
    } else if action == COPY_CHAT_ACTION {
        copy.copy(state.tab);
    }
}

/// The wheel over the messages scrolls one message per notch, 5 with Ctrl and to the end
/// with Shift (Display/ScrollingMessages.lua:25-38).
fn handle_chat_wheel(
    windows: Query<&Window, With<PrimaryWindow>>,
    wheel: Option<Res<AccumulatedMouseScroll>>,
    keys: Res<ButtonInput<KeyCode>>,
    ui: Res<UiState>,
    mut state: ResMut<ChatFrameState>,
    chat: Res<ChatState>,
    combat: Res<CombatLogChat>,
) {
    let Some(wheel) = wheel.filter(|wheel| wheel.delta.y != 0.0) else {
        return;
    };
    let notches = match wheel.unit {
        MouseScrollUnit::Line => wheel.delta.y,
        MouseScrollUnit::Pixel => wheel.delta.y / CHAT_LINE_H,
    }
    .round() as isize;
    let over_messages = windows
        .single()
        .ok()
        .and_then(|window| ui_cursor_position(&ui.registry, window))
        .zip(messages_rect(&ui.registry))
        .is_some_and(|(cursor, rect)| {
            (rect.x..rect.x + rect.width).contains(&cursor.x)
                && (rect.y..rect.y + rect.height).contains(&cursor.y)
        });
    if notches == 0 || !over_messages {
        return;
    }
    let multiplier = if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
        1000
    } else if keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
        5
    } else {
        1
    };
    let total = tab_len(state.tab, &chat, &combat);
    state.scroll_by(notches * multiplier, total);
}

fn messages_rect(registry: &FrameRegistry) -> Option<game_engine::ui::layout::LayoutRect> {
    let frame = registry.get(registry.get_by_name(CHAT_MESSAGES)?)?;
    frame.layout_rect.clone()
}

fn sync_chat_frame_ui(
    mut ui: ResMut<UiState>,
    wrap: Option<ResMut<ChatFrameWrap>>,
    mut state: ResMut<ChatFrameState>,
    chat: Res<ChatState>,
    combat: Res<CombatLogChat>,
    catalog: Option<Res<SpellCatalog>>,
) {
    let Some(mut wrap) = wrap else { return };
    let res = &mut wrap.0;
    let catalog_changed = catalog.as_ref().is_some_and(|catalog| catalog.is_changed());
    if !(state.is_changed() || chat.is_changed() || combat.is_changed() || catalog_changed) {
        return;
    }
    let new_chat = new_chat_messages(&chat, res.seen_chat);
    let new_combat = (combat.received - res.seen_combat).min(combat.lines.len() as u64) as usize;
    res.seen_chat = chat.received;
    res.seen_combat = combat.received;
    let flash = tabs_to_flash(state.tab, new_chat);
    if flash.iter().any(|tab| !state.flashing.contains(tab)) {
        state.start_flashing(flash);
    }
    hold_scroll_position(&mut state, new_chat, new_combat, &chat, &combat);
    let view = chat_view(&state, &chat, &combat, catalog.as_deref());
    if view != res.view {
        res.view = view;
        sync_screen(res, &mut ui.registry);
    }
}

fn new_chat_messages(chat: &ChatState, seen: u64) -> &[RuntimeChatMessage] {
    let new = (chat.received - seen).min(chat.messages.len() as u64) as usize;
    &chat.messages[chat.messages.len() - new..]
}

/// While scrolled up, new lines in the shown tab do not move the view.
fn hold_scroll_position(
    state: &mut ChatFrameState,
    new_chat: &[RuntimeChatMessage],
    new_combat: usize,
    chat: &ChatState,
    combat: &CombatLogChat,
) {
    if state.scroll == 0 {
        return;
    }
    let tab = state.tab;
    let arrived = match tab {
        ChatTab::CombatLog => new_combat,
        tab => new_chat.iter().filter(|msg| tab.shows(msg)).count(),
    };
    if arrived > 0 {
        state.scroll_by(arrived as isize, tab_len(tab, chat, combat));
    }
}

/// Pulse the flashing tabs' flash art (Skins/Dark.lua:329-348).
fn animate_tab_flashes(time: Res<Time>, state: Res<ChatFrameState>, mut ui: ResMut<UiState>) {
    if state.flashing.is_empty() {
        return;
    }
    let alpha = flash_alpha(time.elapsed_secs());
    for tab in &state.flashing {
        if let Some(id) = ui.registry.get_by_name(&tab_flash_name(tab.index())) {
            ui.registry.set_alpha(id, alpha);
        }
    }
}

fn sync_screen(res: &mut ChatFrameRes, registry: &mut FrameRegistry) {
    res.shared.insert(res.view.clone());
    res.screen.sync(&res.shared, registry);
}

fn tab_entries(tab: ChatTab, chat: &ChatState, combat: &CombatLogChat) -> Vec<ChatEntry> {
    match tab {
        ChatTab::CombatLog => combat.lines.clone(),
        tab => chat
            .messages
            .iter()
            .filter(|msg| tab.shows(msg))
            .map(|msg| ChatEntry {
                timestamp: msg.timestamp,
                line: chat_message_line(msg),
            })
            .collect(),
    }
}

fn tab_len(tab: ChatTab, chat: &ChatState, combat: &CombatLogChat) -> usize {
    match tab {
        ChatTab::CombatLog => combat.lines.len(),
        tab => chat.messages.iter().filter(|msg| tab.shows(msg)).count(),
    }
}

/// The messages that fit, counting up from the newest past the scrolled-over ones.
fn chat_view(
    state: &ChatFrameState,
    chat: &ChatState,
    combat: &CombatLogChat,
    catalog: Option<&SpellCatalog>,
) -> ChatFrameView {
    let area = chat_text_area(state.tab);
    let spell_name = spell_namer(catalog);
    let entries = tab_entries(state.tab, chat, combat);
    let mut messages = Vec::new();
    let mut heights = Vec::new();
    for entry in entries.iter().rev().skip(state.scroll) {
        let rows = wrap_chat_line(&entry.line, &spell_name, area.width, measure_chat_text);
        heights.push(rows.len() as f32 * CHAT_LINE_H);
        if messages_that_fit(&heights, CHAT_MESSAGES_AVAILABLE_H, area.spacing) < heights.len() {
            break;
        }
        messages.push(ChatMessageView {
            timestamp: (!state.tab.is_combat_log()).then(|| local_timestamp(entry.timestamp)),
            rows,
        });
    }
    messages.reverse();
    ChatFrameView {
        tab: state.tab,
        messages,
        input_open: state.input_open,
        flashing: state.flashing.clone(),
        scrolled_up: state.scroll > 0,
    }
}

fn spell_namer(catalog: Option<&SpellCatalog>) -> impl Fn(u32) -> String + '_ {
    move |id| match catalog.and_then(|catalog| catalog.get(id)) {
        Some(spell) => spell.name.to_string(),
        None => format!("Spell #{id}"),
    }
}

fn measure_chat_text(value: &str) -> f32 {
    measure_text(value, CHAT_FONT, CHAT_FONT_SIZE).map_or(0.0, |(width, _)| width)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
