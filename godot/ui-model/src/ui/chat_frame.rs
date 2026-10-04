//! In-world chat frame model (`ChatFrame1`, Chattynator look): tabs, Retail line
//! formatting, the client slash-command parser, combat log lines, timestamps, tab flashing,
//! scrolling and word wrapping. Chattynator references are to its Lua source.

use chrono::TimeZone;
use shared::protocol::{ChatType, CombatLogEvent, CombatLogKind, EmoteKind};

use crate::character_frame::break_up_large_numbers;
use crate::chat_data::{ChatChannelType, ChatMessage, ChatState, now_timestamp};
use crate::group_state::GroupCommand;

pub const MAX_COMBAT_LINES: usize = 200;
/// Retail `ChatEdit` history depth.
pub const MAX_SENT_HISTORY: usize = 32;
/// Retail `HELP_TEXT_SIMPLE`.
pub const UNKNOWN_COMMAND_TEXT: &str = "Type '/help' for a listing of a few commands.";
pub const UNKNOWN_NAME: &str = "Unknown";
/// Retail hyperlink colour `|cff71d5ff` (`DEATH_RECAP_LINK`, GlobalStrings 25485).
pub const LINK_COLOR: [f32; 4] = [0.443, 0.835, 1.0, 1.0];
/// Chattynator copies at most 200 lines (Display/CopyChat.lua:96).
pub const MAX_COPY_LINES: usize = 200;

pub const HELP_LINES: [&str; 6] = [
    "Chat: /s /say, /y /yell, /p /party, /g /guild, /e /emote",
    "Whisper: /w /whisper <name> <message>, /r /reply <message>",
    "Emotes: /dance /wave /sit /sleep /kneel",
    "Social: /who <query>; Group: /invite /uninvite /promote <name>, /leave, /readycheck",
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

    /// Tab colour: GENERAL `06a1ff` (Core/Config.lua:43), COMBAT_LOG `c97c48`
    /// (Core/Initialize.lua:50); whisper tabs take the WHISPER chat colour
    /// (Display/Tabs.lua:588).
    pub fn color(self) -> [f32; 3] {
        match self {
            Self::General => rgb(0x06a1ff),
            Self::CombatLog => rgb(0xc97c48),
            Self::Whispers => {
                let [r, g, b, _] = ChatChannelType::Whisper.color();
                [r, g, b]
            }
        }
    }

    /// Background colour: `1a1a1a` (Core/Config.lua:11,43), COMBAT_LOG `262626`
    /// (Core/Initialize.lua:49).
    pub fn background(self) -> [f32; 3] {
        match self {
            Self::CombatLog => rgb(0x262626),
            _ => rgb(0x1a1a1a),
        }
    }

    /// Chattynator's combat log tab embeds Blizzard's ChatFrame2 (API/CustomTab.lua:6-54),
    /// which has no Chattynator timestamps or message spacing.
    pub fn is_combat_log(self) -> bool {
        self == Self::CombatLog
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

#[derive(Clone, Debug, PartialEq)]
pub enum ChatSpan {
    /// Text in the line's colour.
    Text(String),
    /// Text in its own colour (`|cffrrggbb...|r`).
    Colored([f32; 4], String),
    /// A spell hyperlink shown as the bare spell name (`TEXT_MODE_A_STRING_SPELL`, no
    /// braces: `spellBraces = false`); the name is resolved when the line is laid out.
    SpellLink { id: u32, color: [f32; 4] },
}

fn text(value: impl Into<String>) -> ChatSpan {
    ChatSpan::Text(value.into())
}

fn colored(color: [f32; 4], value: impl Into<String>) -> ChatSpan {
    ChatSpan::Colored(color, value.into())
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

    /// The line without colours, spell links as their names.
    pub fn plain_text(&self, spell_name: impl Fn(u32) -> String) -> String {
        self.spans
            .iter()
            .map(|span| match span {
                ChatSpan::Text(value) | ChatSpan::Colored(_, value) => value.clone(),
                ChatSpan::SpellLink { id, .. } => spell_name(*id),
            })
            .collect()
    }
}

fn rgb(hex: u32) -> [f32; 3] {
    [hex >> 16, hex >> 8, hex].map(|channel| (channel & 0xff) as f32 / 255.0)
}

/// A shown line and when it arrived (Unix seconds).
#[derive(Clone, Debug, PartialEq)]
pub struct ChatEntry {
    pub timestamp: f64,
    pub line: ChatLine,
}

/// Combat log tab lines. Names are resolved when the event arrives because the
/// entities may be gone by the time the line is shown.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CombatLogChat {
    pub lines: Vec<ChatEntry>,
    /// Lines ever pushed, so newly arrived lines are known after old ones are dropped.
    pub received: u64,
}

impl CombatLogChat {
    pub fn push(&mut self, timestamp: f64, line: ChatLine) {
        self.lines.push(ChatEntry { timestamp, line });
        self.received += 1;
        if self.lines.len() > MAX_COMBAT_LINES {
            let overflow = self.lines.len() - MAX_COMBAT_LINES;
            self.lines.drain(0..overflow);
        }
    }
}

/// Selected tab, flashing tabs, scroll positions, edit box visibility and sent-line history.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChatFrameState {
    pub tab: ChatTab,
    /// Unselected tabs with unseen messages.
    pub flashing: Vec<ChatTab>,
    /// Per tab ([`ChatTab::index`]), messages scrolled up from the newest; 0 is at the
    /// bottom. Retail's tabs are separate scrolling frames (the combat log is ChatFrame2,
    /// Blizzard_CombatLog.lua:22), so each keeps its own position.
    pub scrolls: [usize; ChatTab::ALL.len()],
    pub input_open: bool,
    /// Sent lines, oldest first.
    pub sent: Vec<String>,
    /// Position in `sent` while cycling with Up/Down.
    pub history_index: Option<usize>,
}

impl ChatFrameState {
    /// Clicking a tab deselects, and so stops flashing, every tab (Display/Tabs.lua:170-173,
    /// 299-302). Each tab keeps its scroll position.
    pub fn select_tab(&mut self, tab: ChatTab) {
        self.tab = tab;
        self.flashing.clear();
    }

    /// The selected tab's scroll position.
    pub fn scroll(&self) -> usize {
        self.scrolls[self.tab.index()]
    }

    /// The selected tab returns to its newest message.
    pub fn scroll_to_bottom(&mut self) {
        self.scrolls[self.tab.index()] = 0;
    }

    /// Scroll the selected tab `amount` messages up (negative: down), between the newest
    /// message and the oldest of `total`.
    pub fn scroll_by(&mut self, amount: isize, total: usize) {
        self.scroll_tab_by(self.tab, amount, total);
    }

    fn scroll_tab_by(&mut self, tab: ChatTab, amount: isize, total: usize) {
        let scroll = &mut self.scrolls[tab.index()];
        *scroll = scroll
            .saturating_add_signed(amount)
            .min(total.saturating_sub(1));
    }

    pub fn start_flashing(&mut self, tabs: impl IntoIterator<Item = ChatTab>) {
        for tab in tabs {
            if !self.flashing.contains(&tab) {
                self.flashing.push(tab);
            }
        }
    }
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

/// Tabs that start flashing for newly received messages, with Chattynator's default
/// `tab_flash_on = "all"` (Core/Config.lua:107, Display/Tabs.lua:188-230): outgoing
/// whispers never flash, and nothing flashes when the selected tab lists any of them. The
/// combat log tab lists no chat messages, so it never flashes.
pub fn tabs_to_flash(selected: ChatTab, new: &[ChatMessage]) -> Vec<ChatTab> {
    let incoming: Vec<&ChatMessage> = new.iter().filter(|msg| !is_outgoing(msg)).collect();
    let matching: Vec<ChatTab> = ChatTab::ALL
        .into_iter()
        .filter(|tab| incoming.iter().any(|msg| tab.shows(msg)))
        .collect();
    if matching.contains(&selected) {
        return Vec::new();
    }
    matching
}

/// Flash alpha `elapsed` seconds in: a looping BOUNCE of 0 to 1 over 0.5 s
/// (Skins/Dark.lua:329-348).
pub fn flash_alpha(elapsed: f32) -> f32 {
    let phase = (elapsed / 0.5).rem_euclid(2.0);
    if phase < 1.0 { phase } else { 2.0 - phase }
}

fn is_outgoing(msg: &ChatMessage) -> bool {
    msg.channel_type == ChatChannelType::Whisper && !msg.channel_name.is_empty()
}

/// Chattynator's default timestamp format `%X` (Core/Config.lua:97), `HH:MM:SS`.
pub fn format_timestamp<Tz: TimeZone>(unix_seconds: f64, zone: &Tz) -> String
where
    Tz::Offset: std::fmt::Display,
{
    zone.timestamp_opt(unix_seconds.floor() as i64, 0)
        .earliest()
        .expect("chat timestamps are within chrono's range")
        .format("%H:%M:%S")
        .to_string()
}

/// [`format_timestamp`] in the local time zone, as Lua `date` does.
pub fn local_timestamp(unix_seconds: f64) -> String {
    format_timestamp(unix_seconds, &chrono::Local)
}

/// How many messages, newest first, fit in `available` height with `spacing` between
/// them. A message that does not fit whole is not shown.
pub fn messages_that_fit(heights_newest_first: &[f32], available: f32, spacing: f32) -> usize {
    let mut used = 0.0;
    heights_newest_first
        .iter()
        .take_while(|height| {
            let next = if used > 0.0 { used + spacing } else { 0.0 } + **height;
            let fits = next <= available;
            if fits {
                used = next;
            }
            fits
        })
        .count()
}

/// Copy Chat text: the newest [`MAX_COPY_LINES`] lines, oldest first, each prefixed with
/// its `[timestamp] ` (Core/Config.lua:122 `copy_timestamps`, Display/CopyChat.lua:96-121).
pub fn copy_chat_text<Tz: TimeZone>(
    entries: &[ChatEntry],
    spell_name: impl Fn(u32) -> String,
    zone: &Tz,
) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let start = entries.len().saturating_sub(MAX_COPY_LINES);
    entries[start..]
        .iter()
        .map(|entry| {
            format!(
                "[{}] {}",
                format_timestamp(entry.timestamp, zone),
                entry.line.plain_text(&spell_name)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A tab's lines, oldest first: the combat log's own, else the chat messages it lists.
pub fn tab_entries(tab: ChatTab, chat: &ChatState, combat: &CombatLogChat) -> Vec<ChatEntry> {
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

pub fn tab_len(tab: ChatTab, chat: &ChatState, combat: &CombatLogChat) -> usize {
    match tab {
        ChatTab::CombatLog => combat.lines.len(),
        tab => chat.messages.iter().filter(|msg| tab.shows(msg)).count(),
    }
}

/// Messages added since `ChatState::received` was `seen`.
pub fn new_chat_messages(chat: &ChatState, seen: u64) -> &[ChatMessage] {
    let new = (chat.received - seen).min(chat.messages.len() as u64) as usize;
    &chat.messages[chat.messages.len() - new..]
}

/// New lines do not move the view of a tab that is scrolled up, selected or not.
pub fn hold_scroll_position(
    state: &mut ChatFrameState,
    new_chat: &[ChatMessage],
    new_combat: usize,
    chat: &ChatState,
    combat: &CombatLogChat,
) {
    for tab in ChatTab::ALL {
        if state.scrolls[tab.index()] == 0 {
            continue;
        }
        let arrived = match tab {
            ChatTab::CombatLog => new_combat,
            tab => new_chat.iter().filter(|msg| tab.shows(msg)).count(),
        };
        state.scroll_tab_by(tab, arrived as isize, tab_len(tab, chat, combat));
    }
}

/// A local system line, shown only to this client.
pub fn add_system_line(chat: &mut ChatState, text: &str) {
    chat.add_message(ChatMessage {
        channel_type: ChatChannelType::System,
        channel_name: String::new(),
        sender: String::new(),
        text: text.to_string(),
        timestamp: now_timestamp(),
    });
}

/// [`copy_chat_text`] with local timestamps, as [`local_timestamp`] shows them.
pub fn local_copy_chat_text(entries: &[ChatEntry], spell_name: impl Fn(u32) -> String) -> String {
    copy_chat_text(entries, spell_name, &chrono::Local)
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
        // CHAT_MONSTER_SAY_GET / CHAT_MONSTER_YELL_GET: the creature's name, no link.
        ChatChannelType::MonsterSay => format!("{sender} says: {body}"),
        ChatChannelType::MonsterYell => format!("{sender} yells: {body}"),
        ChatChannelType::MonsterEmote | ChatChannelType::RaidBossEmote => {
            crate::chat_data::monster_emote_text(body, sender)
        }
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
    Group(GroupCommand),
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
        // Retail SLASH_INVITE/UNINVITE/PROMOTE/READYCHECK aliases.
        "invite" | "inv" => named_group_command(rest, GroupCommand::Invite),
        "uninvite" | "un" | "u" | "kick" => named_group_command(rest, GroupCommand::Uninvite),
        "promote" | "pr" => named_group_command(rest, GroupCommand::Promote),
        "readycheck" | "rc" => ChatCommand::Group(GroupCommand::StartReadyCheck),
        // Bare `/leave` leaves the party or raid (`C_PartyInfo.LeaveParty`); `/leave <n>`
        // (chat channels) stays unsupported.
        "leave" if rest.is_empty() => ChatCommand::Group(GroupCommand::Leave),
        "help" | "h" | "?" => ChatCommand::System(HELP_LINES.map(String::from).to_vec()),
        other => match emote_command(other) {
            Some(emote) => ChatCommand::Emote(emote),
            None => ChatCommand::System(vec![UNKNOWN_COMMAND_TEXT.to_string()]),
        },
    }
}

fn named_group_command(rest: &str, command: fn(String) -> GroupCommand) -> ChatCommand {
    match split_word(rest).0 {
        "" => ChatCommand::None,
        name => ChatCommand::Group(command(name.to_string())),
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

/// What a combat log unit is to the local player (`COMBATLOG_FILTER_*`,
/// Blizzard_CombatLogBase/Shared/CombatLogFilters.lua).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatLogUnit {
    /// The local player (`COMBATLOG_FILTER_MINE`).
    Mine,
    Friendly,
    /// Hostile and neutral units share a colour.
    Hostile,
    /// Not replicated (`COMBATLOG_FILTER_UNKNOWN_UNITS`).
    Unknown,
}

impl CombatLogUnit {
    /// `COMBATLOG_DEFAULT_COLORS.unitColoring` (Mainline/CombatLogColors.lua:3-11).
    pub fn color(self) -> [f32; 4] {
        match self {
            Self::Mine => [0.70, 0.70, 0.70, 1.0],
            Self::Friendly => [0.34, 0.64, 1.00, 1.0],
            Self::Hostile => [0.75, 0.05, 0.05, 1.0],
            Self::Unknown => [0.75, 0.75, 0.75, 1.0],
        }
    }
}

/// A combat log event's source or target: its display name and what it is to the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatLogActor<'a> {
    pub name: &'a str,
    pub unit: CombatLogUnit,
}

/// The action word of events other than damage (`missColoring`,
/// Blizzard_CombatLogProcessor.lua:1185-1200; `GetColorByEventType`'s default,
/// CombatLogUtil.lua:14-18).
const COMBAT_ACTION_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 1.0];

/// `CombatLogUtil.HighlightColor` (CombatLogUtil.lua:39-51): spell name, amount and school
/// (`abilityHighlighting`, `amountHighlighting`, `schoolNameHighlighting`).
fn highlight([r, g, b, a]: [f32; 4]) -> [f32; 4] {
    [
        (r * 1.5).min(1.0),
        (g * 1.5).min(1.0),
        (b * 1.5).min(1.0),
        a,
    ]
}

/// Retail's combat log line for an event, or None when the default filters do not list
/// it. `CombatLogProcessor:GenerateMessage` (Blizzard_CombatLogProcessor.lua:196-1417)
/// with `COMBATLOG_DEFAULT_SETTINGS` (Blizzard_CombatLog.lua:27-66): not full text, so
/// `TEXT_MODE_A_STRING_1` "source spell action dest value. result" with the player as
/// You/Your, no timestamp, the line in its source's unit colour.
///
/// Listed, from the two default quick filters (Blizzard_CombatLog.lua:229-410; there is
/// no filter bar, so both apply): "My actions" - the player's damage, heals and kills;
/// "What happened to me?" - damage and heals on the player and the player's death.
/// Neither lists misses, energizes, interrupts, dispels or casts, and auras are dropped by
/// `hideBuffs`/`hideDebuffs` (Processor.lua:689-698).
pub fn combat_log_line(
    event: &CombatLogEvent,
    source: CombatLogActor,
    target: CombatLogActor,
) -> Option<ChatLine> {
    let by_me = source.unit == CombatLogUnit::Mine;
    let to_me = target.unit == CombatLogUnit::Mine;
    match event.kind {
        CombatLogKind::Damage | CombatLogKind::Heal if by_me || to_me => {
            Some(amount_line(event, source, target))
        }
        CombatLogKind::Death if to_me => Some(death_recap_line()),
        CombatLogKind::Death if by_me => Some(kill_line(target.name)),
        _ => None,
    }
}

/// SWING_DAMAGE, SPELL_DAMAGE, SPELL_PERIODIC_DAMAGE, SPELL_HEAL and SPELL_PERIODIC_HEAL:
/// "Your Melee hit Kobold Vermin 12 Physical. (Critical)".
fn amount_line(event: &CombatLogEvent, source: CombatLogActor, target: CombatLogActor) -> ChatLine {
    let color = source.unit.color();
    let bright = highlight(color);
    let heal = event.kind == CombatLogKind::Heal;
    // `ACTION_SWING` "Melee" (GlobalStrings 17851) stands in for a swing's spell.
    let spell = match event.spell_id {
        Some(id) => Some(ChatSpan::SpellLink { id, color: bright }),
        None if heal => None,
        None => Some(colored(bright, "Melee")),
    };
    // `UNIT_YOU_SOURCE` "You" / `UNIT_YOU_SOURCE_POSSESSIVE` "Your" (17848, 18492): the
    // possessive needs a spell name; other units keep their bare name (Processor.lua:1011-1024).
    let source_name = match (source.unit, &spell) {
        (CombatLogUnit::Mine, Some(_)) => "Your",
        (CombatLogUnit::Mine, None) => "You",
        _ => source.name,
    };
    // `UNIT_YOU_DEST` "You" (17849).
    let target_name = match target.unit {
        CombatLogUnit::Mine => "You",
        _ => target.name,
    };
    let mut spans = vec![text(format!("{source_name} "))];
    if let Some(spell) = spell {
        spans.extend([spell, text(" ")]);
    }
    // `ACTION_SPELL_HEAL` / `ACTION_SPELL_PERIODIC_HEAL` "healed" (17872, 17889),
    // `ACTION_SPELL_PERIODIC_DAMAGE` "damaged" (17879), `ACTION_SWING_DAMAGE` /
    // `ACTION_SPELL_DAMAGE` "hit" (17853, 17862).
    spans.push(match (heal, event.periodic) {
        (true, _) => colored(COMBAT_ACTION_COLOR, "healed"),
        (false, true) => text("damaged"),
        (false, false) => text("hit"),
    });
    // The shown amount leaves out overkill and overhealing (Processor.lua:300-302, 389);
    // `TEXT_MODE_A_STRING_VALUE_SCHOOL` "%s %s" (17836).
    let amount = i64::from(event.amount - event.overflow.max(0));
    spans.extend([
        text(format!(" {target_name} ")),
        colored(bright, break_up_large_numbers(amount)),
        text(" "),
        colored(bright, school_name(event.school_mask)),
        text(format!(".{}", amount_results(event))),
    ]);
    ChatLine { color, spans }
}

/// `CombatLogUtil.GenerateDamageResultString` (CombatLogUtil.lua:176-226), in its order:
/// `TEXT_MODE_A_STRING_RESULT_RESIST`, `_BLOCK`, `_ABSORB`, `_GLANCING`, `_OVERHEALING`,
/// `_OVERKILLING`, `_CRITICAL` (GlobalStrings 17840-17844, 18618, 19426).
fn amount_results(event: &CombatLogEvent) -> String {
    let overflow = match event.kind {
        CombatLogKind::Heal => "Overhealed",
        _ => "Overkill",
    };
    let amounts = [
        (event.resisted, "Resisted"),
        (event.blocked, "Blocked"),
        (event.absorbed, "Absorbed"),
    ];
    let mut results: Vec<String> = amounts
        .into_iter()
        .filter(|(value, _)| *value > 0)
        .map(|(value, label)| format!("({} {label})", break_up_large_numbers(value.into())))
        .collect();
    if event.glancing {
        results.push("(Glancing)".into());
    }
    if event.overflow > 0 {
        let value = break_up_large_numbers(event.overflow.into());
        results.push(format!("({value} {overflow})"));
    }
    if event.crit {
        results.push("(Critical)".into());
    }
    results.iter().map(|result| format!(" {result}")).collect()
}

/// `STRING_SCHOOL_*` (GlobalStrings 18106-18113, 18330) of a single-school mask;
/// `STRING_SCHOOL_UNKNOWN` otherwise (CombatLogUtil.lua:127-134).
fn school_name(mask: u32) -> &'static str {
    match mask {
        1 => "Physical",
        2 => "Holy",
        4 => "Fire",
        8 => "Nature",
        16 => "Frost",
        32 => "Shadow",
        64 => "Arcane",
        _ => "Unknown",
    }
}

/// PARTY_KILL: "You killed Kobold Vermin." (`ACTION_PARTY_KILL` "killed", GlobalStrings
/// 18080, not possessive 18274), a highlighted event (CombatLogColors.lua:33-35).
fn kill_line(target: &str) -> ChatLine {
    ChatLine {
        color: highlight(CombatLogUnit::Mine.color()),
        spans: vec![
            text("You "),
            colored(COMBAT_ACTION_COLOR, "killed"),
            text(format!(" {target}.")),
        ],
    }
}

/// The player's UNIT_DIED is the death recap link `DEATH_RECAP_LINK` "[You died.]"
/// (GlobalStrings 25485; Processor.lua:847-854).
fn death_recap_line() -> ChatLine {
    ChatLine {
        color: CombatLogUnit::Mine.color(),
        spans: vec![colored(LINK_COLOR, "[You died.]")],
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
    for (token, color, spell_id) in line_tokens(line, &spell_name) {
        let width = measure(&token);
        if x > 0.0 && x + width > max_width && !token.trim().is_empty() {
            rows.push(ChatRow::default());
            x = 0.0;
        }
        let row = rows.last_mut().expect("rows start non-empty");
        match row.runs.last_mut() {
            Some(run) if run.spell_id.is_none() && spell_id.is_none() && run.color == color => {
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

/// `(text, colour, spell id)` pieces: words keep their trailing whitespace; a link is one
/// token.
fn line_tokens(
    line: &ChatLine,
    spell_name: &impl Fn(u32) -> String,
) -> Vec<(String, [f32; 4], Option<u32>)> {
    let words = |value: &str, color: [f32; 4]| {
        value
            .split_inclusive(char::is_whitespace)
            .map(|word| (word.to_string(), color, None))
            .collect::<Vec<_>>()
    };
    let mut tokens = Vec::new();
    for span in &line.spans {
        match span {
            ChatSpan::SpellLink { id, color } => tokens.push((spell_name(*id), *color, Some(*id))),
            ChatSpan::Text(value) => tokens.extend(words(value, line.color)),
            ChatSpan::Colored(color, value) => tokens.extend(words(value, *color)),
        }
    }
    tokens
}

#[cfg(test)]
#[path = "chat_frame_tests.rs"]
mod tests;
