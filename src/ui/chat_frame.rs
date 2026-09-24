//! In-world chat frame model (Retail `ChatFrame1`): tabs, Retail line formatting,
//! the client slash-command parser, combat log lines and word wrapping.

use bevy::prelude::*;
use shared::protocol::{ChatType, CombatLogEvent, CombatLogKind, EmoteKind, MissKind};

use crate::chat_data::{ChatChannelType, ChatMessage};

pub const MAX_COMBAT_LINES: usize = 200;
/// Retail `ChatEdit` history depth.
pub const MAX_SENT_HISTORY: usize = 32;
/// Retail `HELP_TEXT_SIMPLE`.
pub const UNKNOWN_COMMAND_TEXT: &str = "Type '/help' for a listing of a few commands.";
pub const UNKNOWN_NAME: &str = "Unknown";
/// Retail spell link colour `|cff71d5ff`.
pub const SPELL_LINK_COLOR: [f32; 4] = [0.443, 0.835, 1.0, 1.0];
pub const COMBAT_LOG_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub const HELP_LINES: [&str; 6] = [
    "Chat: /s /say, /y /yell, /p /party, /g /guild, /e /emote",
    "Whisper: /w /whisper <name> <message>, /r /reply <message>",
    "Emotes: /dance /wave /sit /sleep /kneel",
    "Social: /who <query>, /invite <name>",
    "Keys: Enter opens chat, / starts a command, R replies to the last whisper",
    "Up/Down recall sent lines; Escape closes the chat box",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChatTab {
    #[default]
    General,
    CombatLog,
    Whispers,
}

impl ChatTab {
    pub const ALL: [Self; 3] = [Self::General, Self::CombatLog, Self::Whispers];

    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::CombatLog => "Combat Log",
            Self::Whispers => "Whispers",
        }
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|tab| *tab == self).unwrap_or(0)
    }

    pub fn action(self) -> &'static str {
        match self {
            Self::General => "chat_tab:general",
            Self::CombatLog => "chat_tab:combat_log",
            Self::Whispers => "chat_tab:whispers",
        }
    }

    pub fn from_action(action: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tab| tab.action() == action)
    }

    /// Whether a received chat message is listed in this tab.
    pub fn shows(self, msg: &ChatMessage) -> bool {
        match self {
            Self::General => true,
            Self::CombatLog => false,
            Self::Whispers => msg.channel_type == ChatChannelType::Whisper,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatSpan {
    Text(String),
    /// Rendered as `[Spell Name]`; the name is resolved when the line is laid out.
    SpellLink(u32),
}

fn text(value: impl Into<String>) -> ChatSpan {
    ChatSpan::Text(value.into())
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChatLine {
    pub color: [f32; 4],
    pub spans: Vec<ChatSpan>,
}

impl ChatLine {
    pub fn plain(color: [f32; 4], value: impl Into<String>) -> Self {
        Self {
            color,
            spans: vec![text(value)],
        }
    }

    /// Text with spell links shown as `[id]`; for tests and dumps.
    pub fn plain_text(&self, spell_name: impl Fn(u32) -> String) -> String {
        self.spans
            .iter()
            .map(|span| match span {
                ChatSpan::Text(value) => value.clone(),
                ChatSpan::SpellLink(id) => format!("[{}]", spell_name(*id)),
            })
            .collect()
    }
}

/// Combat log tab lines. Names are resolved when the event arrives because the
/// entities may be gone by the time the line is shown.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct CombatLogChat {
    pub lines: Vec<ChatLine>,
}

impl CombatLogChat {
    pub fn push(&mut self, line: ChatLine) {
        self.lines.push(line);
        if self.lines.len() > MAX_COMBAT_LINES {
            let overflow = self.lines.len() - MAX_COMBAT_LINES;
            self.lines.drain(0..overflow);
        }
    }
}

/// Selected tab, edit box visibility, sent-line history and idle time.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct ChatFrameState {
    pub tab: ChatTab,
    pub input_open: bool,
    /// Sent lines, oldest first.
    pub sent: Vec<String>,
    /// Position in `sent` while cycling with Up/Down.
    pub history_index: Option<usize>,
}

impl ChatFrameState {
    pub fn remember_sent(&mut self, line: &str) {
        self.history_index = None;
        if line.is_empty() || self.sent.last().is_some_and(|last| last == line) {
            return;
        }
        self.sent.push(line.to_string());
        if self.sent.len() > MAX_SENT_HISTORY {
            self.sent.remove(0);
        }
    }

    /// Older sent line, staying on the oldest.
    pub fn history_prev(&mut self) -> Option<&str> {
        if self.sent.is_empty() {
            return None;
        }
        let index = match self.history_index {
            None => self.sent.len() - 1,
            Some(index) => index.saturating_sub(1),
        };
        self.history_index = Some(index);
        Some(&self.sent[index])
    }

    /// Newer sent line; past the newest the edit box is empty again.
    pub fn history_next(&mut self) -> Option<&str> {
        let index = self.history_index? + 1;
        if index >= self.sent.len() {
            self.history_index = None;
            return Some("");
        }
        self.history_index = Some(index);
        Some(&self.sent[index])
    }
}

/// Retail chat line for a received message.
pub fn chat_message_line(msg: &ChatMessage) -> ChatLine {
    let sender = &msg.sender;
    let body = &msg.text;
    let value = match msg.channel_type {
        ChatChannelType::Say => format!("[{sender}] says: {body}"),
        ChatChannelType::Yell => format!("[{sender}] yells: {body}"),
        ChatChannelType::Whisper if msg.channel_name.is_empty() => {
            format!("[{sender}] whispers: {body}")
        }
        ChatChannelType::Whisper => format!("To [{}]: {body}", msg.channel_name),
        ChatChannelType::Emote => format!("{sender} {body}"),
        ChatChannelType::System => body.clone(),
        ChatChannelType::Custom => format!("[{}] [{sender}]: {body}", msg.channel_name),
        ChatChannelType::Party
        | ChatChannelType::Raid
        | ChatChannelType::Guild
        | ChatChannelType::Officer => {
            format!("{} [{sender}]: {body}", msg.channel_type.prefix())
        }
    };
    ChatLine::plain(msg.channel_type.color(), value)
}

/// What a submitted chat edit box line does.
#[derive(Clone, Debug, PartialEq)]
pub enum ChatCommand {
    Send {
        channel: ChatType,
        text: String,
    },
    Emote(EmoteKind),
    Who(String),
    Invite(String),
    /// Local system lines, shown only to this client.
    System(Vec<String>),
    None,
}

/// Parse an edit box line. `reply_target` is the last player who whispered us.
pub fn parse_chat_input(line: &str, reply_target: Option<&str>) -> ChatCommand {
    let line = line.trim();
    let Some(command_line) = line.strip_prefix('/') else {
        return send(ChatType::Say, line);
    };
    let (command, rest) = split_word(command_line);
    match command.to_ascii_lowercase().as_str() {
        "s" | "say" => send(ChatType::Say, rest),
        "y" | "yell" => send(ChatType::Yell, rest),
        "p" | "party" => send(ChatType::Party, rest),
        "g" | "guild" => send(ChatType::Guild, rest),
        "e" | "em" | "me" | "emote" => send(ChatType::Emote, rest),
        "w" | "whisper" | "t" | "tell" => parse_whisper(rest),
        "r" | "reply" => match reply_target {
            Some(target) => send(ChatType::Whisper(target.to_string()), rest),
            None => ChatCommand::None,
        },
        "who" => ChatCommand::Who(rest.to_string()),
        "invite" | "inv" => match split_word(rest).0 {
            "" => ChatCommand::None,
            name => ChatCommand::Invite(name.to_string()),
        },
        "help" | "h" | "?" => ChatCommand::System(HELP_LINES.map(String::from).to_vec()),
        other => match emote_command(other) {
            Some(emote) => ChatCommand::Emote(emote),
            None => ChatCommand::System(vec![UNKNOWN_COMMAND_TEXT.to_string()]),
        },
    }
}

fn send(channel: ChatType, body: &str) -> ChatCommand {
    if body.is_empty() {
        return ChatCommand::None;
    }
    ChatCommand::Send {
        channel,
        text: body.to_string(),
    }
}

fn parse_whisper(rest: &str) -> ChatCommand {
    match split_word(rest) {
        ("", _) => ChatCommand::None,
        (name, body) => send(ChatType::Whisper(name.to_string()), body),
    }
}

fn split_word(value: &str) -> (&str, &str) {
    let value = value.trim_start();
    match value.split_once(char::is_whitespace) {
        Some((word, rest)) => (word, rest.trim()),
        None => (value, ""),
    }
}

fn emote_command(command: &str) -> Option<EmoteKind> {
    match command {
        "dance" => Some(EmoteKind::Dance),
        "wave" => Some(EmoteKind::Wave),
        "sit" => Some(EmoteKind::Sit),
        "sleep" => Some(EmoteKind::Sleep),
        "kneel" => Some(EmoteKind::Kneel),
        _ => None,
    }
}

/// Retail-style combat log line. `source`/`target` are resolved display names.
pub fn combat_log_line(event: &CombatLogEvent, source: &str, target: &str) -> ChatLine {
    let spans = match event.kind {
        CombatLogKind::Damage => damage_spans(event, source, target),
        CombatLogKind::Heal => heal_spans(event, source, target),
        CombatLogKind::Energize => {
            let mut spans = vec![text(format!("{target} gains {} from ", event.amount))];
            spans.extend(actor(source, event.spell_id));
            spans.push(text("."));
            spans
        }
        CombatLogKind::Miss(miss) => miss_spans(event, miss, source, target),
        CombatLogKind::AuraApplied => spell_sentence(event, &format!("{target} gains "), "."),
        CombatLogKind::AuraRemoved => spell_sentence(event, "", &format!(" fades from {target}.")),
        CombatLogKind::AuraRefreshed => {
            spell_sentence(event, &format!("{target}'s "), " is refreshed.")
        }
        CombatLogKind::Interrupt => {
            spell_sentence(event, &format!("{source} interrupts {target}'s "), ".")
        }
        CombatLogKind::Dispel => {
            spell_sentence(event, &format!("{source} dispels {target}'s "), ".")
        }
        CombatLogKind::CastStart => {
            spell_sentence(event, &format!("{source} begins to cast "), ".")
        }
        CombatLogKind::CastSuccess => {
            let suffix = match event.target {
                Some(_) => format!(" on {target}."),
                None => ".".to_string(),
            };
            spell_sentence(event, &format!("{source} casts "), &suffix)
        }
        CombatLogKind::Death => vec![text(format!("{target} dies."))],
    };
    ChatLine {
        color: COMBAT_LOG_COLOR,
        spans,
    }
}

/// "Bob" for melee, "Bob's [Fireball]" for spells.
fn actor(source: &str, spell_id: Option<u32>) -> Vec<ChatSpan> {
    match spell_id {
        Some(id) => vec![text(format!("{source}'s ")), ChatSpan::SpellLink(id)],
        None => vec![text(source)],
    }
}

fn spell_sentence(event: &CombatLogEvent, before: &str, after: &str) -> Vec<ChatSpan> {
    let mut spans = vec![text(before)];
    match event.spell_id {
        Some(id) => spans.push(ChatSpan::SpellLink(id)),
        None => spans.push(text("an ability")),
    }
    spans.push(text(after));
    spans
}

fn damage_spans(event: &CombatLogEvent, source: &str, target: &str) -> Vec<ChatSpan> {
    let amount = event.amount;
    let mut spans = if event.periodic {
        let mut spans = vec![text(format!("{target} suffers {amount} damage from "))];
        spans.extend(actor(source, event.spell_id));
        spans
    } else {
        let verb = if event.crit { "crits" } else { "hits" };
        let mut spans = actor(source, event.spell_id);
        spans.push(text(format!(" {verb} {target} for {amount}")));
        spans
    };
    spans.push(text(format!("{}.", damage_details(event))));
    spans
}

fn damage_details(event: &CombatLogEvent) -> String {
    [
        (event.absorbed, "Absorbed"),
        (event.resisted, "Resisted"),
        (event.blocked, "Blocked"),
        (event.overflow, "Overkill"),
    ]
    .into_iter()
    .filter(|(value, _)| *value > 0)
    .map(|(value, label)| format!(" ({value} {label})"))
    .collect()
}

fn heal_spans(event: &CombatLogEvent, source: &str, target: &str) -> Vec<ChatSpan> {
    let amount = event.amount;
    let mut spans = if event.periodic {
        let mut spans = vec![text(format!("{target} gains {amount} health from "))];
        spans.extend(actor(source, event.spell_id));
        spans
    } else {
        let verb = if event.crit {
            "critically heals"
        } else {
            "heals"
        };
        let mut spans = actor(source, event.spell_id);
        spans.push(text(format!(" {verb} {target} for {amount}")));
        spans
    };
    let overheal = match event.overflow {
        0 => String::new(),
        value => format!(" ({value} Overhealed)"),
    };
    spans.push(text(format!("{overheal}.")));
    spans
}

fn miss_spans(event: &CombatLogEvent, miss: MissKind, source: &str, target: &str) -> Vec<ChatSpan> {
    if event.spell_id.is_none() {
        return vec![text(melee_miss_text(miss, source, target))];
    }
    let mut spans = actor(source, event.spell_id);
    spans.push(text(match miss {
        MissKind::Miss => format!(" missed {target}."),
        MissKind::Immune => format!(" failed. {target} is immune."),
        _ => format!(" was {} by {target}.", miss_past_tense(miss)),
    }));
    spans
}

fn melee_miss_text(miss: MissKind, source: &str, target: &str) -> String {
    match miss {
        MissKind::Miss => format!("{source} misses {target}."),
        MissKind::Immune => format!("{source} attacks but {target} is immune."),
        _ => format!("{source} attacks. {target} {}.", miss_present_tense(miss)),
    }
}

fn miss_past_tense(miss: MissKind) -> &'static str {
    match miss {
        MissKind::Miss => "missed",
        MissKind::Dodge => "dodged",
        MissKind::Parry => "parried",
        MissKind::Block => "blocked",
        MissKind::Resist => "resisted",
        MissKind::Immune => "ignored",
        MissKind::Evade => "evaded",
        MissKind::Absorb => "absorbed",
        MissKind::Deflect => "deflected",
        MissKind::Reflect => "reflected",
    }
}

fn miss_present_tense(miss: MissKind) -> &'static str {
    match miss {
        MissKind::Miss => "is missed",
        MissKind::Dodge => "dodges",
        MissKind::Parry => "parries",
        MissKind::Block => "blocks",
        MissKind::Resist => "resists",
        MissKind::Immune => "is immune",
        MissKind::Evade => "evades",
        MissKind::Absorb => "absorbs",
        MissKind::Deflect => "deflects",
        MissKind::Reflect => "reflects",
    }
}

/// One single-colour piece of a wrapped row, `x` from the row start.
#[derive(Clone, Debug, PartialEq)]
pub struct ChatRun {
    pub text: String,
    pub color: [f32; 4],
    pub x: f32,
    pub width: f32,
    pub spell_id: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChatRow {
    pub runs: Vec<ChatRun>,
}

/// Word-wrap a line into rows no wider than `max_width`. Links never split.
pub fn wrap_chat_line(
    line: &ChatLine,
    spell_name: impl Fn(u32) -> String,
    max_width: f32,
    measure: impl Fn(&str) -> f32,
) -> Vec<ChatRow> {
    let mut rows = vec![ChatRow::default()];
    let mut x = 0.0;
    for (token, spell_id) in line_tokens(line, &spell_name) {
        let width = measure(&token);
        if x > 0.0 && x + width > max_width && !token.trim().is_empty() {
            rows.push(ChatRow::default());
            x = 0.0;
        }
        let row = rows.last_mut().expect("rows start non-empty");
        let color = if spell_id.is_some() {
            SPELL_LINK_COLOR
        } else {
            line.color
        };
        match row.runs.last_mut() {
            Some(run) if run.spell_id.is_none() && spell_id.is_none() => {
                run.text.push_str(&token);
                run.width += width;
            }
            _ => row.runs.push(ChatRun {
                text: token,
                color,
                x,
                width,
                spell_id,
            }),
        }
        x += width;
    }
    rows
}

/// Words keep their trailing whitespace; a link is one token.
fn line_tokens(line: &ChatLine, spell_name: &impl Fn(u32) -> String) -> Vec<(String, Option<u32>)> {
    let mut tokens = Vec::new();
    for span in &line.spans {
        match span {
            ChatSpan::SpellLink(id) => tokens.push((format!("[{}]", spell_name(*id)), Some(*id))),
            ChatSpan::Text(value) => tokens.extend(
                value
                    .split_inclusive(char::is_whitespace)
                    .map(|word| (word.to_string(), None)),
            ),
        }
    }
    tokens
}

#[cfg(test)]
#[path = "chat_frame_tests.rs"]
mod tests;
