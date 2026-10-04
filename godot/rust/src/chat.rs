//! In-world chat frame `ChatFrame1` (docs/specs/chat-frame.md): the root client's
//! Chattynator-look screen, tab routing, slash commands and combat log lines, hosted on a
//! dedicated RegistryUi. Enter, `/` and R open the edit box; while it has focus, gameplay
//! keys are off (a focused LineEdit disables them), Enter sends, Escape or focus loss closes.

use game_engine_core::spell_catalog::SpellCatalogData;
use game_engine_session::SessionScreen;
use game_engine_ui_model::chat_data::{
    ChatMessage as RuntimeChatMessage, ChatState, MAX_CHAT_MESSAGES, MAX_RECENT_WHISPER_TARGETS,
    WhisperState, now_timestamp, runtime_chat_channel,
};
use game_engine_ui_model::chat_frame::{
    ChatCommand, ChatFrameState, ChatTab, CombatLogActor, CombatLogChat, CombatLogUnit,
    UNKNOWN_NAME, add_system_line, combat_log_line, flash_alpha, hold_scroll_position,
    local_copy_chat_text, new_chat_messages, parse_chat_input, tab_entries, tab_len, tabs_to_flash,
};
use game_engine_ui_model::chat_frame_component::{
    CHAT_EDITBOX, CHAT_MESSAGES, COPY_CHAT_ACTION, ChatFrameView, FOREVER_CHAT_HEADER_FDIDS,
    SCROLL_TO_BOTTOM_ACTION, chat_frame_view, tab_flash_name,
};
use godot::classes::{DisplayServer, InputEvent, InputEventKey, InputEventMouseButton};
use godot::global::{Key, MouseButton};
use godot::prelude::*;
use shared::protocol::{ChatMessage, ChatType, CombatLogEvent, EmoteIntent, EmoteKind};

use crate::faction_reaction::Reaction;
use crate::frame_error::FrameError;
use crate::replicated::UnitFields;
use crate::ui::RegistryUi;
use game_engine_ui_model::group_state::GroupCommand;

/// The root client's line when `/who` has no who state.
const WHO_UNAVAILABLE_TEXT: &str = "Who is unavailable.";

/// A submitted line's network request.
#[derive(Debug, PartialEq)]
pub(crate) enum ChatRequest {
    Send {
        channel: ChatType,
        content: String,
    },
    Emote(EmoteKind),
    /// `/invite`, `/uninvite`, `/promote`, `/leave`, `/readycheck`.
    Group(GroupCommand),
    /// `/trade`: `InitiateTrade("target")` (SlashCommands.lua:911-913).
    TradeTarget,
}

/// Chat log, whisper partners and frame state. The log outlives the world, as the
/// root client's `ChatState` does; the combat log and edit box do not.
pub(crate) struct ChatModel {
    pub log: ChatState,
    pub whispers: WhisperState,
    pub combat: CombatLogChat,
    pub state: ChatFrameState,
    /// `ChatState::received` / `CombatLogChat::received` already shown.
    seen_chat: u64,
    seen_combat: u64,
}

impl Default for ChatModel {
    fn default() -> Self {
        Self {
            log: ChatState {
                max_messages: MAX_CHAT_MESSAGES,
                ..Default::default()
            },
            whispers: WhisperState {
                max_recent: MAX_RECENT_WHISPER_TARGETS,
                ..Default::default()
            },
            combat: CombatLogChat::default(),
            state: ChatFrameState::default(),
            seen_chat: 0,
            seen_combat: 0,
        }
    }
}

impl ChatModel {
    /// A server chat line (root `apply_incoming_chat_message`).
    pub fn receive(&mut self, msg: &ChatMessage, local_name: Option<&str>, timestamp: f64) {
        let (channel_type, channel_name) =
            runtime_chat_channel(&msg.channel, &msg.sender, local_name);
        self.whispers.record_message(msg, local_name);
        self.log.add_message(RuntimeChatMessage {
            channel_type,
            channel_name,
            sender: msg.sender.clone(),
            text: msg.content.clone(),
            timestamp,
        });
    }

    /// A combat event with its units already resolved: a Combat Log line when Retail's
    /// default filters list it.
    pub fn receive_combat(
        &mut self,
        event: &CombatLogEvent,
        source: CombatLogActor,
        target: CombatLogActor,
    ) {
        if let Some(line) = combat_log_line(event, source, target) {
            self.combat.push(now_timestamp(), line);
        }
    }

    /// Edit box prefill for an opening key, or None when the key does not open chat.
    /// R opens `/w <last whisperer> ` only after someone whispered.
    pub fn open_prefill(&self, key: ChatOpenKey) -> Option<String> {
        match key {
            ChatOpenKey::Enter => Some(String::new()),
            ChatOpenKey::Slash => Some("/".into()),
            ChatOpenKey::Reply => self
                .whispers
                .reply_target
                .as_ref()
                .map(|target| format!("/w {target} ")),
        }
    }

    pub fn open(&mut self) {
        self.state.input_open = true;
        self.state.history_index = None;
    }

    pub fn close(&mut self) {
        self.state.input_open = false;
        self.state.history_index = None;
    }

    /// Enter in the edit box: close it, remember the line and turn it into a request.
    /// Local commands (`/help`, unknown commands) print system lines instead.
    pub fn submit(&mut self, line: &str) -> Option<ChatRequest> {
        self.close();
        self.state.remember_sent(line);
        if is_trade_command(line) {
            return Some(ChatRequest::TradeTarget);
        }
        match parse_chat_input(line, self.whispers.reply_target.as_deref()) {
            ChatCommand::Send { channel, text } => {
                if let ChatType::Whisper(target) = &channel {
                    self.whispers.send_whisper(target);
                }
                Some(ChatRequest::Send {
                    channel,
                    content: text,
                })
            }
            ChatCommand::Emote(emote) => Some(ChatRequest::Emote(emote)),
            ChatCommand::Who(_) => self.system(WHO_UNAVAILABLE_TEXT),
            ChatCommand::Group(command) => Some(ChatRequest::Group(command)),
            ChatCommand::System(lines) => {
                for line in lines {
                    add_system_line(&mut self.log, &line);
                }
                None
            }
            ChatCommand::None => None,
        }
    }

    fn system(&mut self, text: &str) -> Option<ChatRequest> {
        add_system_line(&mut self.log, text);
        None
    }

    /// Clicking a chat tab, Copy Chat or Scroll to bottom. Returns the Copy Chat text.
    pub fn click(&mut self, action: &str, spell_name: impl Fn(u32) -> String) -> Option<String> {
        if let Some(tab) = ChatTab::from_action(action) {
            self.state.select_tab(tab);
        } else if action == SCROLL_TO_BOTTOM_ACTION {
            self.state.scroll_to_bottom();
        } else if action == COPY_CHAT_ACTION {
            let entries = tab_entries(self.state.tab, &self.log, &self.combat);
            return Some(local_copy_chat_text(&entries, spell_name));
        }
        None
    }

    /// Wheel notches over the messages: 1 message each, 5 with Ctrl, to the end with Shift.
    pub fn scroll(&mut self, notches: isize, shift: bool, ctrl: bool) {
        let multiplier = if shift {
            1000
        } else if ctrl {
            5
        } else {
            1
        };
        let total = tab_len(self.state.tab, &self.log, &self.combat);
        self.state.scroll_by(notches * multiplier, total);
    }

    /// Account for newly arrived lines: flash the tabs that list them and keep a
    /// scrolled-up view in place.
    pub fn absorb_new_lines(&mut self) {
        let new_chat = new_chat_messages(&self.log, self.seen_chat);
        let new_combat =
            (self.combat.received - self.seen_combat).min(self.combat.lines.len() as u64) as usize;
        let flash = tabs_to_flash(self.state.tab, new_chat);
        self.state.start_flashing(flash);
        hold_scroll_position(
            &mut self.state,
            new_chat,
            new_combat,
            &self.log,
            &self.combat,
        );
        self.seen_chat = self.log.received;
        self.seen_combat = self.combat.received;
    }

    pub fn view(&self, spell_name: impl Fn(u32) -> String) -> ChatFrameView {
        chat_frame_view(&self.state, &self.log, &self.combat, spell_name)
    }

    /// Leaving the world closes the edit box, clears the combat log and stops flashing.
    pub fn leave_world(&mut self) {
        self.close();
        self.state.scrolls = Default::default();
        self.state.flashing.clear();
        self.combat = CombatLogChat::default();
        self.seen_combat = 0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChatOpenKey {
    Enter,
    Slash,
    Reply,
}

/// The chat key a pressed, non-repeat key event is, if any.
/// `/trade`, whatever follows it: the slash command ignores its argument.
fn is_trade_command(line: &str) -> bool {
    line.trim()
        .split_whitespace()
        .next()
        .is_some_and(|word| word.eq_ignore_ascii_case("/trade"))
}

fn open_key(key: &Gd<InputEventKey>) -> Option<ChatOpenKey> {
    if !key.is_pressed() || key.is_echo() {
        return None;
    }
    match key.get_keycode() {
        Key::ENTER | Key::KP_ENTER => Some(ChatOpenKey::Enter),
        Key::R if !key.is_ctrl_pressed() && !key.is_alt_pressed() => Some(ChatOpenKey::Reply),
        _ if key.get_unicode() == u32::from('/') => Some(ChatOpenKey::Slash),
        _ => None,
    }
}

/// Native chat host: the model, its RegistryUi and the flash clock.
#[derive(Default)]
pub(crate) struct Chat {
    pub model: ChatModel,
    pub ui: Option<Gd<RegistryUi>>,
    /// `Account::combat_log_seq` already turned into combat log lines.
    combat_seen: u64,
    flash_elapsed: f32,
}

/// Spell link names from the catalog; `Spell #id` until it loads (root `spell_namer`).
fn spell_namer(catalog: Option<&SpellCatalogData>) -> impl Fn(u32) -> String + '_ {
    move |id| match catalog.and_then(|catalog| catalog.get(id)) {
        Some(spell) => spell.name.to_string(),
        None => format!("Spell #{id}"),
    }
}

impl crate::GameClient {
    fn attach_chat_ui(&mut self) -> Result<(), String> {
        if self.chat.ui.is_some() {
            return Ok(());
        }
        self.extract_art(&FOREVER_CHAT_HEADER_FDIDS);
        let view = self.chat.model.view(spell_namer(self.spells.catalog()));
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("ChatFrameUI");
        self.base_mut().add_child(&ui);
        let shown = ui.bind_mut().show_chat_frame(view);
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.chat.ui = Some(ui);
        Ok(())
    }

    /// A group result or notice from the server, as a system line.
    pub(crate) fn receive_group_notice(&mut self, text: &str) {
        add_system_line(&mut self.chat.model.log, text);
    }

    /// A server chat line.
    pub(crate) fn receive_chat(&mut self, msg: &ChatMessage) {
        let local = self.account.session.selected_character_name.clone();
        self.chat
            .model
            .receive(msg, local.as_deref(), now_timestamp());
    }

    /// Per frame: combat log lines, clicks, focus loss, the view and the tab flash pulse.
    pub(crate) fn update_chat(&mut self, delta: f32) -> Result<(), FrameError> {
        let in_world = self.account.session.screen == SessionScreen::InWorld;
        if !in_world {
            if let Some(ui) = self.chat.ui.as_mut()
                && ui.is_visible()
            {
                ui.set_visible(false);
                self.chat.model.leave_world();
                self.chat.combat_seen = self.account.combat_log_seq;
            }
            return Ok(());
        }
        self.attach_chat_ui()?;
        self.chat
            .ui
            .as_mut()
            .expect("chat UI attached")
            .set_visible(true);
        self.absorb_combat_log();
        self.apply_chat_clicks()?;
        let ui = self.chat.ui.as_ref().expect("chat UI attached");
        if self.chat.model.state.input_open && !ui.bind().is_frame_focused(CHAT_EDITBOX.0) {
            self.close_chat_input()?;
        }
        self.chat.model.absorb_new_lines();
        let view = self.chat.model.view(spell_namer(self.spells.catalog()));
        let mut ui = self.chat.ui.clone().expect("chat UI attached");
        ui.bind_mut().set_state(view)?;
        self.pulse_chat_flashes(delta)
    }

    fn absorb_combat_log(&mut self) {
        let seq = self.account.combat_log_seq;
        let fresh = (seq - self.chat.combat_seen).min(self.account.combat_log.len() as u64);
        self.chat.combat_seen = seq;
        let skip = self.account.combat_log.len() - fresh as usize;
        let events: Vec<_> = self.account.combat_log.iter().skip(skip).cloned().collect();
        for event in events {
            let source_name = self.unit_display_name(event.source);
            let target_name = self.unit_display_name(event.target);
            let source = CombatLogActor {
                name: &source_name,
                unit: self.combat_log_unit(event.source),
            };
            let target = CombatLogActor {
                name: &target_name,
                unit: self.combat_log_unit(event.target),
            };
            self.chat.model.receive_combat(&event, source, target);
        }
    }

    /// What a combat log unit is to the local player.
    fn combat_log_unit(&mut self, id: Option<u64>) -> CombatLogUnit {
        if id.is_some() && id == self.world.local_player_id() {
            return CombatLogUnit::Mine;
        }
        let Some(id) = id.filter(|id| self.replica.unit(*id).is_some()) else {
            return CombatLogUnit::Unknown;
        };
        match self.reaction_to(id) {
            Reaction::Friendly => CombatLogUnit::Friendly,
            Reaction::Neutral | Reaction::Hostile => CombatLogUnit::Hostile,
        }
    }

    /// Replicated NPC name, then player name, else `Unknown`.
    fn unit_display_name(&self, id: Option<u64>) -> String {
        id.and_then(|id| self.replica.unit(id))
            .and_then(|unit| Some(unit.name()?.to_owned()))
            .unwrap_or_else(|| UNKNOWN_NAME.to_string())
    }

    fn apply_chat_clicks(&mut self) -> Result<(), String> {
        let mut ui = self.chat.ui.clone().expect("chat UI attached");
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                return Ok(());
            }
            let copied = self
                .chat
                .model
                .click(&action, spell_namer(self.spells.catalog()));
            if let Some(text) = copied {
                DisplayServer::singleton().clipboard_set(&text);
            }
        }
    }

    /// Flashing tabs pulse 0→1→0 every second (Skins/Dark.lua:329-348).
    fn pulse_chat_flashes(&mut self, delta: f32) -> Result<(), FrameError> {
        if self.chat.model.state.flashing.is_empty() {
            self.chat.flash_elapsed = 0.0;
            return Ok(());
        }
        self.chat.flash_elapsed += delta;
        let alpha = flash_alpha(self.chat.flash_elapsed);
        let mut ui = self.chat.ui.clone().expect("chat UI attached");
        for tab in self.chat.model.state.flashing.clone() {
            ui.bind_mut()
                .set_frame_alpha(&tab_flash_name(tab.index()), alpha)?;
        }
        Ok(())
    }

    /// Unhandled Enter, `/` or R in the world opens the edit box. Returns whether it did.
    pub(crate) fn open_chat_from_key(&mut self, key: &Gd<InputEventKey>) -> Result<bool, String> {
        let open = self.chat.ui.is_some()
            && !self.chat.model.state.input_open
            && self.game_menu_ui.is_none()
            && self.account.session.screen == SessionScreen::InWorld
            && self.account.session.gameplay_input_allowed();
        let Some(prefill) = open_key(key)
            .filter(|_| open)
            .and_then(|key| self.chat.model.open_prefill(key))
        else {
            return Ok(false);
        };
        self.open_chat_input(&prefill)?;
        Ok(true)
    }

    pub(crate) fn open_chat_input(&mut self, prefill: &str) -> Result<(), String> {
        let mut ui = self.chat.ui.clone().ok_or("Chat frame is not shown")?;
        self.chat.model.open();
        let view = self.chat.model.view(spell_namer(self.spells.catalog()));
        let mut bound = ui.bind_mut();
        bound.set_state(view)?;
        bound.set_editbox_text(CHAT_EDITBOX.0, prefill)?;
        bound.focus_frame_named(CHAT_EDITBOX.0)
    }

    fn close_chat_input(&mut self) -> Result<(), String> {
        self.chat.model.close();
        let mut ui = self.chat.ui.clone().ok_or("Chat frame is not shown")?;
        let mut bound = ui.bind_mut();
        bound.set_editbox_text(CHAT_EDITBOX.0, "")?;
        bound.release_focus_named(CHAT_EDITBOX.0);
        let view = self.chat.model.view(spell_namer(self.spells.catalog()));
        bound.set_state(view)
    }

    /// Keys while the edit box has focus, before the LineEdit sees them: Enter sends,
    /// Escape closes, Up/Down recall sent lines. Returns whether the key was used.
    pub(crate) fn chat_edit_key(&mut self, event: &Gd<InputEvent>) -> Result<bool, String> {
        let Ok(key) = event.clone().try_cast::<InputEventKey>() else {
            return Ok(false);
        };
        if !key.is_pressed() || !self.chat_input_focused() {
            return Ok(false);
        }
        match key.get_keycode() {
            Key::ENTER | Key::KP_ENTER => {
                let line = self.chat_editbox_text();
                self.close_chat_input()?;
                if let Some(request) = self.chat.model.submit(&line) {
                    self.send_chat_request(request)?;
                }
            }
            Key::ESCAPE => self.close_chat_input()?,
            Key::UP | Key::DOWN => {
                let state = &mut self.chat.model.state;
                let recalled = if key.get_keycode() == Key::UP {
                    state.history_prev()
                } else {
                    state.history_next()
                }
                .map(str::to_owned);
                if let Some(line) = recalled {
                    let mut ui = self.chat.ui.clone().ok_or("Chat frame is not shown")?;
                    ui.bind_mut().set_editbox_text(CHAT_EDITBOX.0, &line)?;
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn chat_input_focused(&self) -> bool {
        self.chat.model.state.input_open
            && self.base().get_viewport().is_some_and(|viewport| {
                viewport
                    .gui_get_focus_owner()
                    .is_some_and(|focus| focus.get_name() == StringName::from(CHAT_EDITBOX.0))
            })
    }

    fn chat_editbox_text(&mut self) -> String {
        self.chat
            .ui
            .as_mut()
            .map(|ui| ui.bind_mut().frame_text(CHAT_EDITBOX.0.into()).to_string())
            .unwrap_or_default()
    }

    /// The emote's chat line, then the stand state a /sit, /sleep or /kneel asks for.
    fn send_emote(&self, emote: EmoteKind) -> Result<(), crate::frame_error::SessionError> {
        self.account.send_emote(EmoteIntent { emote })?;
        match crate::world::stand::emote_stand_state(emote) {
            Some(state) => self.account.send_stand_state(state),
            None => Ok(()),
        }
    }

    fn send_chat_request(&mut self, request: ChatRequest) -> Result<(), String> {
        match request {
            ChatRequest::Send { channel, content } => self.account.send_chat(ChatMessage {
                sender: String::new(),
                content,
                channel,
            }),
            ChatRequest::Emote(emote) => self.send_emote(emote),
            ChatRequest::Group(command) => self.account.send_group(command),
            ChatRequest::TradeTarget => return self.trade_with_target(),
        }
        .map_err(|error| error.0)
    }

    /// The wheel over the messages scrolls the chat instead of zooming the camera.
    pub(crate) fn chat_wheel(&mut self, event: &Gd<InputEvent>) -> bool {
        let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() else {
            return false;
        };
        let notches = match button.get_button_index() {
            MouseButton::WHEEL_UP => 1,
            MouseButton::WHEEL_DOWN => -1,
            _ => return false,
        };
        let Some(ui) = self.chat.ui.as_ref().filter(|ui| ui.is_visible()) else {
            return false;
        };
        let Some([x, y, width, height]) = ui.bind().frame_viewport_rect(CHAT_MESSAGES) else {
            return false;
        };
        let point = button.get_position();
        let over = (x..x + width).contains(&point.x) && (y..y + height).contains(&point.y);
        if !over || !button.is_pressed() {
            return over;
        }
        self.chat
            .model
            .scroll(notches, button.is_shift_pressed(), button.is_ctrl_pressed());
        true
    }
}

#[cfg(test)]
#[path = "chat_tests.rs"]
mod tests;
