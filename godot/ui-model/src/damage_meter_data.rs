//! Retail damage meter window (`Blizzard_DamageMeter`): which session the primary window
//! shows and the rows it draws from the server's `DamageMeterSnapshot`. Spec:
//! docs/specs/damage-meter.md.
//!
//! The server owns the numbers (`C_DamageMeter`); the window picks the `Current` or
//! `Overall` session, ranks its sources and formats each bar the way
//! `DamageMeterSourceEntryMixin` does with the Edit Mode default `Numbers` = Compact.
//!
//! The snapshot carries damage only. Healing, interrupts, dispels and deaths are counted
//! here from the `CombatLogEvent`s this client receives ([`MeterLog`]): the lines whose
//! source or target is the local player. Lines between two other units never arrive, so a
//! group member's healing of others, interrupts, dispels and deaths are not listed.

use shared::protocol::{
    CombatLogEvent, CombatLogKind, DamageMeterSession, DamageMeterSnapshot, DamageMeterSource,
};

use crate::ui::chat_frame::environmental_name;

/// Session dropdown click: open or close the session menu.
pub const ACTION_DAMAGE_METER_MENU: &str = "damage_meter:menu";
/// Session menu radio clicks.
pub const ACTION_DAMAGE_METER_CURRENT: &str = "damage_meter:current";
pub const ACTION_DAMAGE_METER_OVERALL: &str = "damage_meter:overall";
/// Type dropdown click: open or close the type menu.
pub const ACTION_DAMAGE_METER_TYPE_MENU: &str = "damage_meter:type_menu";
/// Row click, followed by the row's index: a death opens its recap, a recap row closes it.
pub const ACTION_DAMAGE_METER_ROW: &str = "damage_meter:row:";

/// The `Enum.DamageMeterType` values the window offers
/// (DamageMeterConstantsDocumentation.lua:102-112). `Dps`/`Hps` (the same sources with
/// the per-second value first), `Absorbs` and the damage-taken types are not offered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MeterType {
    /// The default window type (`GetDefaultWindowData`, DamageMeter.lua:92-97).
    #[default]
    DamageDone,
    HealingDone,
    Interrupts,
    Dispels,
    Deaths,
    /// Details-like current-target threat; not an Enum.DamageMeterType value.
    Threat,
}

impl MeterType {
    /// In the type dropdown's order (`DAMAGE_METER_CATEGORIES`,
    /// DamageMeterSessionWindow.lua:1-5).
    pub const ALL: [Self; 6] = [
        Self::DamageDone,
        Self::HealingDone,
        Self::Interrupts,
        Self::Dispels,
        Self::Deaths,
        Self::Threat,
    ];

    /// `DAMAGE_METER_TYPE_DAMAGE_DONE`, `_HEALING_DONE`, `_INTERRUPTS`, `_DISPELS`,
    /// `_DEATHS`.
    pub fn label(self) -> &'static str {
        match self {
            Self::DamageDone => "Damage Done",
            Self::HealingDone => "Healing Done",
            Self::Interrupts => "Interrupts",
            Self::Dispels => "Dispels",
            Self::Deaths => "Deaths",
            Self::Threat => "Threat",
        }
    }

    /// The type menu radio's click action.
    pub fn action(self) -> &'static str {
        match self {
            Self::DamageDone => "damage_meter:type:damage",
            Self::HealingDone => "damage_meter:type:healing",
            Self::Interrupts => "damage_meter:type:interrupts",
            Self::Dispels => "damage_meter:type:dispels",
            Self::Deaths => "damage_meter:type:deaths",
            Self::Threat => "damage_meter:threat",
        }
    }
}

/// `DEATH_RECAP_TITLE`.
pub const DEATH_RECAP_LABEL: &str = "Death Recap";
/// `ACTION_SWING`: the recap's name for a melee swing (Blizzard_DeathRecap.lua:107-109).
pub const MELEE_LABEL: &str = "Melee";
/// Recap rows: Retail's recap lists the last damage events (`C_DeathRecap.GetRecapEvents`;
/// the count is the engine's). Ours lists damage and healing, as asked, this many at most
/// from this long before the death.
pub const RECAP_EVENTS: usize = 5;
pub const RECAP_WINDOW_SECS: f64 = 10.0;
/// Recap bar colours: damage red, healing green.
const RECAP_DAMAGE_COLOR: [f32; 3] = [0.85, 0.15, 0.15];
const RECAP_HEAL_COLOR: [f32; 3] = [0.2, 0.8, 0.2];
/// A combat's first line reaches the client just before the snapshot that announces its
/// session; lines this long before the session's computed start still belong to it.
const SESSION_START_SLACK_SECS: f64 = 0.5;

/// `Enum.DamageMeterSessionType` values a window can select without a session id.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MeterSessionType {
    /// The primary window starts on `Overall` (`DamageMeterMixin:OnLoad`,
    /// DamageMeter.lua:80).
    #[default]
    Overall,
    Current,
}

impl MeterSessionType {
    /// `DAMAGE_METER_OVERALL_SESSION_SHORT` "O", `DAMAGE_METER_CURRENT_SESSION_SHORT` "C".
    pub fn short_name(self) -> &'static str {
        match self {
            Self::Overall => "O",
            Self::Current => "C",
        }
    }

    /// `DAMAGE_METER_OVERALL_SESSION` "Overall", `DAMAGE_METER_CURRENT_SESSION`
    /// "Current Segment": the session menu's radio labels.
    pub fn label(self) -> &'static str {
        match self {
            Self::Overall => "Overall",
            Self::Current => "Current Segment",
        }
    }
}

/// `RAID_CLASS_COLORS` by `ChrClasses` id; white for unknown classes.
pub fn class_color(class_id: u8) -> [f32; 3] {
    match class_id {
        1 => [0.78, 0.61, 0.43],
        2 => [0.96, 0.55, 0.73],
        3 => [0.67, 0.83, 0.45],
        4 => [1.00, 0.96, 0.41],
        6 => [0.77, 0.12, 0.23],
        7 => [0.00, 0.44, 0.87],
        8 => [0.25, 0.78, 0.92],
        9 => [0.53, 0.53, 0.93],
        10 => [0.00, 1.00, 0.60],
        11 => [1.00, 0.49, 0.04],
        12 => [0.64, 0.19, 0.79],
        13 => [0.20, 0.58, 0.50],
        _ => [1.00, 1.00, 1.00],
    }
}

/// `AbbreviateLargeNumbers` (UIParent.lua:774-785 as last shipped in Lua): more than 8
/// digits keep the millions with `SECOND_NUMBER_CAP` "M", more than 5 the thousands with
/// `FIRST_NUMBER_CAP` "K", 4-5 digits get `BreakUpLargeNumbers` commas.
pub fn abbreviate_large_number(value: u64) -> String {
    let digits = value.to_string();
    match digits.len() {
        9.. => format!("{}M", &digits[..digits.len() - 6]),
        6.. => format!("{}K", &digits[..digits.len() - 3]),
        4.. => {
            let split = digits.len() - 3;
            format!("{},{}", &digits[..split], &digits[split..])
        }
        _ => digits,
    }
}

/// `SecondsToClock`: `MINUTES_SECONDS` "%.2d:%.2d", or `HOURS_MINUTES_SECONDS` from an hour.
pub fn seconds_to_clock(seconds: f32) -> String {
    let total = seconds.max(0.0) as u64;
    let (hours, minutes, seconds) = (total / 3600, total / 60 % 60, total % 60);
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

/// `BreakUpLargeNumbers`: thousands separators.
pub fn break_up_large_number(value: u64) -> String {
    let digits = value.to_string();
    let mut text = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            text.push(',');
        }
        text.push(digit);
    }
    text
}

/// `deathTimeFormatter` (DamageMeterEntry.lua:587-591): one-letter units, the two
/// largest, down to seconds: "3m 22s".
pub fn death_time_text(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    let (hours, minutes, seconds) = (total / 3600, total / 60 % 60, total % 60);
    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else {
        format!("{seconds}s")
    }
}

/// A unit of a combat log line, as the meter lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct MeterUnit {
    /// Server entity bits.
    pub unit: u64,
    pub name: String,
    /// `ChrClasses` id of a player; 0 for creatures.
    pub class_id: u8,
    pub is_local_player: bool,
}

impl MeterUnit {
    fn is_player(&self) -> bool {
        self.class_id != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeterEventKind {
    Damage,
    Heal,
    Interrupt,
    Dispel,
    Death,
}

/// A combat log line the meter counts, at the client time it arrived.
#[derive(Clone, Debug, PartialEq)]
pub struct MeterEvent {
    pub time: f64,
    /// When the server logged it, Unix seconds (CLEU `timestamp`).
    pub logged_at: f64,
    pub kind: MeterEventKind,
    pub source: MeterUnit,
    pub target: MeterUnit,
    /// The damaging or healing spell's name, [`MELEE_LABEL`] for a swing, the type's name
    /// for environmental damage (`ACTION_ENVIRONMENTAL_DAMAGE_*`).
    pub spell_name: String,
    /// CLEU `extraSpellName`: the interrupted spell, the dispelled aura.
    pub extra_spell_name: Option<String>,
    /// Damage dealt, healing without overheal, or 1 for an interrupt, a dispelled aura
    /// or a death.
    pub amount: u64,
}

impl MeterEvent {
    /// The line as the meter counts it; none for lines it ignores: other kinds, no
    /// amount, and damage to or deaths of creatures (only players get a recap).
    pub fn from_combat_log(
        time: f64,
        event: &CombatLogEvent,
        source: MeterUnit,
        target: MeterUnit,
        spell_name: impl Fn(u32) -> String,
    ) -> Option<Self> {
        let (kind, amount) = match event.kind {
            CombatLogKind::Damage | CombatLogKind::Environmental(_) if target.is_player() => {
                (MeterEventKind::Damage, event.amount)
            }
            CombatLogKind::Heal => (MeterEventKind::Heal, event.amount - event.overflow),
            CombatLogKind::Interrupt => (MeterEventKind::Interrupt, 1),
            CombatLogKind::Dispel => (MeterEventKind::Dispel, 1),
            CombatLogKind::Death if target.is_player() => (MeterEventKind::Death, 1),
            _ => return None,
        };
        let name = match (event.kind, event.spell_id) {
            (CombatLogKind::Environmental(kind), _) => environmental_name(kind).to_owned(),
            (_, Some(id)) => spell_name(id),
            (_, None) => MELEE_LABEL.to_owned(),
        };
        (amount > 0).then(|| Self {
            time,
            logged_at: event.timestamp_unix_ms as f64 / 1000.0,
            kind,
            source,
            target,
            spell_name: name,
            extra_spell_name: event.extra_spell_id.map(&spell_name),
            amount: amount as u64,
        })
    }
}

/// When a server session ran, in client time.
#[derive(Clone, Debug, PartialEq)]
struct SessionSpan {
    id: u32,
    start: f64,
    /// None while the session's combat runs.
    end: Option<f64>,
}

impl SessionSpan {
    fn contains(&self, time: f64) -> bool {
        time >= self.start - SESSION_START_SLACK_SECS && self.end.is_none_or(|end| time <= end)
    }
}

/// The counted combat log lines since world entry, which is what `Overall` shows (the
/// server's `Overall` likewise counts every damage line), and when the server's `Current`
/// session ran.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeterLog {
    events: Vec<MeterEvent>,
    current: Option<SessionSpan>,
}

impl MeterLog {
    pub fn push(&mut self, event: MeterEvent) {
        self.events.push(event);
    }

    /// Note when the snapshot's `Current` session started (now less its duration) and
    /// when it stopped being active.
    fn observe(&mut self, now: f64, current: &DamageMeterSession) {
        match self
            .current
            .as_mut()
            .filter(|span| span.id == current.session_id)
        {
            Some(span) if current.active => span.end = None,
            Some(span) => span.end = span.end.or(Some(now)),
            None => {
                self.current = Some(SessionSpan {
                    id: current.session_id,
                    start: now - f64::from(current.duration_secs),
                    end: (!current.active).then_some(now),
                })
            }
        }
    }
}

/// One source bar.
#[derive(Clone, Debug, PartialEq)]
pub struct DamageMeterRow {
    /// `DAMAGE_METER_SOURCE_NAME` "%d. %s".
    pub name_text: String,
    /// `DAMAGE_METER_ENTRY_FORMAT_COMPACT` "%s (%s)": damage, then damage per second.
    pub value_text: String,
    /// StatusBar fill: the source's damage over the session's highest.
    pub fraction: f32,
    pub color: [f32; 3],
    /// Source class identity for the Forever row icon; no specialization is supplied.
    pub class_id: u8,
    pub is_local_player: bool,
}

/// What the window draws.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DamageMeterView {
    pub session: MeterSessionType,
    pub meter_type: MeterType,
    /// `"[%s] "` of the session's duration while the player is in combat, else empty
    /// (`ShouldDisplaySessionTimer`, `SetSessionDuration`).
    pub timer_text: String,
    /// Width the host measured for `timer_text`; the type dropdown follows it.
    pub timer_width: f32,
    pub rows: Vec<DamageMeterRow>,
    /// The session menu is open.
    pub menu_open: bool,
    /// The type menu is open.
    pub type_menu_open: bool,
    /// The rows are a death's recap.
    pub recap_open: bool,
    /// The rows are an Interrupts or Dispels source's spell breakdown.
    pub breakdown_open: bool,
}

impl DamageMeterView {
    /// The header's type name.
    pub fn type_label(&self) -> &'static str {
        if self.recap_open {
            DEATH_RECAP_LABEL
        } else {
            self.meter_type.label()
        }
    }

    /// Death rows open their recap, Interrupts and Dispels rows their breakdown; recap and
    /// breakdown rows close them.
    pub fn rows_clickable(&self) -> bool {
        self.recap_open
            || self.breakdown_open
            || matches!(
                self.meter_type,
                MeterType::Deaths | MeterType::Interrupts | MeterType::Dispels
            )
    }
}

/// The primary window's state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DamageMeterWindow {
    pub session: MeterSessionType,
    pub meter_type: MeterType,
    pub snapshot: Option<DamageMeterSnapshot>,
    pub threat_tables: std::collections::BTreeMap<u64, shared::protocol::ThreatUpdate>,
    pub threat_target: Option<u64>,
    pub local_unit: Option<u64>,
    pub log: MeterLog,
    pub menu_open: bool,
    pub type_menu_open: bool,
    /// The death whose recap is shown: its index in the log.
    pub recap: Option<usize>,
    /// The Interrupts or Dispels source whose spell breakdown is shown: its unit.
    pub breakdown: Option<u64>,
}

impl DamageMeterWindow {
    pub fn receive_threat(&mut self, update: shared::protocol::ThreatUpdate) {
        if update.entries.is_empty() {
            self.threat_tables.remove(&update.creature);
        } else {
            self.threat_tables.insert(update.creature, update);
        }
    }

    pub fn select_threat_target(
        &mut self,
        target: Option<u64>,
        local: Option<u64>,
        in_combat: bool,
    ) {
        self.local_unit = local;
        self.threat_target = target;
        if !in_combat {
            self.threat_tables.clear();
        }
    }

    fn threat_rows(&self) -> Vec<DamageMeterRow> {
        let Some(table) = self
            .threat_target
            .and_then(|target| self.threat_tables.get(&target))
        else {
            return Vec::new();
        };
        let max = table
            .entries
            .iter()
            .map(|entry| entry.raw_threat)
            .fold(0.0_f32, f32::max);
        table
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| DamageMeterRow {
                name_text: format!("{}. {}", index + 1, entry.name),
                value_text: format!("{:.1}%", entry.raw_percent),
                fraction: if max > 0.0 {
                    entry.raw_threat / max
                } else {
                    0.0
                },
                color: class_color(entry.class_id),
                class_id: entry.class_id,
                is_local_player: self.local_unit == Some(entry.unit),
            })
            .collect()
    }

    /// The selected session: none before the first combat for `Current`.
    pub fn session_data(&self) -> Option<&DamageMeterSession> {
        let snapshot = self.snapshot.as_ref()?;
        match self.session {
            MeterSessionType::Overall => Some(&snapshot.overall),
            MeterSessionType::Current => snapshot.current.as_ref(),
        }
    }

    pub fn timer_text(&self, in_combat: bool) -> String {
        let duration = self
            .session_data()
            .map_or(0.0, |session| session.duration_secs);
        if in_combat && duration > 0.0 {
            format!("[{}] ", seconds_to_clock(duration))
        } else {
            String::new()
        }
    }

    /// The server's newest snapshot, received by client time `now`.
    pub fn set_snapshot(&mut self, now: f64, snapshot: Option<DamageMeterSnapshot>) {
        if let Some(current) = snapshot.as_ref().and_then(|s| s.current.as_ref()) {
            self.log.observe(now, current);
        }
        self.snapshot = snapshot;
    }

    /// The log's lines in the selected session, with their indices: all of them for
    /// `Overall`, those that arrived while the `Current` session ran for `Current`.
    fn session_events(&self) -> impl Iterator<Item = (usize, &MeterEvent)> {
        let current = self.log.current.as_ref();
        let session = self.session;
        self.log
            .events
            .iter()
            .enumerate()
            .filter(move |(_, event)| match session {
                MeterSessionType::Overall => true,
                MeterSessionType::Current => current.is_some_and(|span| span.contains(event.time)),
            })
    }

    /// What each player did of `kind` in the selected session, most first.
    fn source_totals(&self, kind: MeterEventKind) -> Vec<(&MeterUnit, u64)> {
        let mut totals: Vec<(&MeterUnit, u64)> = Vec::new();
        for (_, event) in self.session_events() {
            if event.kind != kind || !event.source.is_player() {
                continue;
            }
            match totals
                .iter_mut()
                .find(|(source, _)| source.unit == event.source.unit)
            {
                Some((_, total)) => *total += event.amount,
                None => totals.push((&event.source, event.amount)),
            }
        }
        totals.sort_by(|a, b| b.1.cmp(&a.1));
        totals
    }

    /// The selected session's player deaths, newest first, as log indices. (Retail's
    /// order is `C_DamageMeter`'s and not visible in its files.)
    fn deaths(&self) -> Vec<usize> {
        let mut deaths: Vec<usize> = self
            .session_events()
            .filter(|(_, event)| event.kind == MeterEventKind::Death)
            .map(|(index, _)| index)
            .collect();
        deaths.reverse();
        deaths
    }

    /// Rows in the server's rank order (`BuildDataProvider`); a death's recap while one
    /// is open.
    pub fn rows(&self) -> Vec<DamageMeterRow> {
        if let Some(death) = self.recap {
            return self.recap_rows(death);
        }
        if let (Some(unit), Some(kind)) = (self.breakdown, self.counted_kind()) {
            return self.breakdown_rows(unit, kind);
        }
        match self.meter_type {
            MeterType::DamageDone => self.damage_rows(),
            MeterType::HealingDone => {
                let duration = self.session_data().map_or(0.0, |s| s.duration_secs);
                counted_rows(&self.source_totals(MeterEventKind::Heal), Some(duration))
            }
            MeterType::Interrupts | MeterType::Dispels => {
                let kind = self.counted_kind().expect("a counted type");
                counted_rows(&self.source_totals(kind), None)
            }
            MeterType::Deaths => self.death_rows(),
            MeterType::Threat => self.threat_rows(),
        }
    }

    /// The count each Interrupts or Dispels row ranks by.
    fn counted_kind(&self) -> Option<MeterEventKind> {
        match self.meter_type {
            MeterType::Interrupts => Some(MeterEventKind::Interrupt),
            MeterType::Dispels => Some(MeterEventKind::Dispel),
            _ => None,
        }
    }

    /// `DamageMeterSourceWindow`'s spell rows for one source (DamageMeterSourceWindow.lua
    /// 175-194, `DamageMeterSpellEntryMixin:GetNameText` DamageMeterEntry.lua:706-711): the
    /// interrupted spells or dispelled auras by count, most first, in the source's class
    /// colour, the bare count as the type suppresses per second.
    fn breakdown_rows(&self, unit: u64, kind: MeterEventKind) -> Vec<DamageMeterRow> {
        let mut spells: Vec<(&MeterEvent, u64)> = Vec::new();
        for (_, event) in self.session_events() {
            if event.kind != kind || event.source.unit != unit {
                continue;
            }
            match spells
                .iter_mut()
                .find(|(seen, _)| seen.extra_spell_name == event.extra_spell_name)
            {
                Some((_, count)) => *count += event.amount,
                None => spells.push((event, event.amount)),
            }
        }
        spells.sort_by(|a, b| b.1.cmp(&a.1));
        let max = spells.first().map_or(0, |(_, count)| *count);
        spells
            .into_iter()
            .map(|(event, count)| DamageMeterRow {
                // No spell id, no name text (`GetNameText` returns nil).
                name_text: event.extra_spell_name.clone().unwrap_or_default(),
                value_text: abbreviate_large_number(count),
                fraction: count as f32 / max as f32,
                color: class_color(event.source.class_id),
                class_id: event.source.class_id,
                is_local_player: event.source.is_local_player,
            })
            .collect()
    }

    fn damage_rows(&self) -> Vec<DamageMeterRow> {
        let Some(session) = self.session_data() else {
            return Vec::new();
        };
        let max = session
            .sources
            .iter()
            .map(|source| source.total_amount)
            .max()
            .unwrap_or(0);
        session
            .sources
            .iter()
            .enumerate()
            .map(|(index, source)| damage_row(index, source, max))
            .collect()
    }

    /// `DamageMeterSourceEntryMixin` for a death (DamageMeterEntry.lua:562-605): the name
    /// without a rank, a full bar and the time into the session, which `Overall` lacks.
    fn death_rows(&self) -> Vec<DamageMeterRow> {
        let start = match self.session {
            MeterSessionType::Current => self.log.current.as_ref().map(|span| span.start),
            MeterSessionType::Overall => None,
        };
        self.deaths()
            .into_iter()
            .map(|index| {
                let death = &self.log.events[index];
                DamageMeterRow {
                    name_text: death.target.name.clone(),
                    value_text: start
                        .map_or_else(String::new, |start| death_time_text(death.time - start)),
                    fraction: 1.0,
                    color: class_color(death.target.class_id),
                    class_id: death.target.class_id,
                    is_local_player: death.target.is_local_player,
                }
            })
            .collect()
    }

    /// The last damage and healing the dead unit took, newest first: "spell by source"
    /// (`DEATH_RECAP_CAST_BY_TT`), the amount (damage negative as in Retail's recap,
    /// Blizzard_DeathRecap.lua:162) and the seconds before the death ("%.1F", :72-74).
    fn recap_rows(&self, death_index: usize) -> Vec<DamageMeterRow> {
        let death = &self.log.events[death_index];
        let events: Vec<&MeterEvent> = self.log.events[..death_index]
            .iter()
            .rev()
            .take_while(|event| death.logged_at - event.logged_at <= RECAP_WINDOW_SECS)
            .filter(|event| {
                event.target.unit == death.target.unit
                    && matches!(event.kind, MeterEventKind::Damage | MeterEventKind::Heal)
            })
            .take(RECAP_EVENTS)
            .collect();
        let max = events.iter().map(|event| event.amount).max().unwrap_or(0);
        events
            .into_iter()
            .map(|event| {
                let (sign, color) = match event.kind {
                    MeterEventKind::Heal => ('+', RECAP_HEAL_COLOR),
                    _ => ('-', RECAP_DAMAGE_COLOR),
                };
                DamageMeterRow {
                    name_text: format!("{} by {}", event.spell_name, event.source.name),
                    value_text: format!(
                        "{sign}{} ({:.1}s)",
                        break_up_large_number(event.amount),
                        death.logged_at - event.logged_at
                    ),
                    fraction: event.amount as f32 / max as f32,
                    color,
                    class_id: 0,
                    is_local_player: false,
                }
            })
            .collect()
    }

    /// A header, menu or row click.
    pub fn click(&mut self, action: &str) -> Result<(), String> {
        if let Some(index) = action.strip_prefix(ACTION_DAMAGE_METER_ROW) {
            let index: usize = index
                .parse()
                .map_err(|_| format!("Damage meter row action without an index: {action}"))?;
            return self.click_row(index);
        }
        if let Some(meter_type) = MeterType::ALL.into_iter().find(|t| t.action() == action) {
            self.meter_type = meter_type;
            self.close_menus_and_recap();
            return Ok(());
        }
        match action {
            ACTION_DAMAGE_METER_MENU => {
                self.menu_open = !self.menu_open;
                self.type_menu_open = false;
            }
            ACTION_DAMAGE_METER_TYPE_MENU => {
                self.type_menu_open = !self.type_menu_open;
                self.menu_open = false;
            }
            ACTION_DAMAGE_METER_CURRENT => {
                self.session = MeterSessionType::Current;
                self.close_menus_and_recap();
            }
            ACTION_DAMAGE_METER_OVERALL => {
                self.session = MeterSessionType::Overall;
                self.close_menus_and_recap();
            }
            other => return Err(format!("Unknown damage meter action: {other}")),
        }
        Ok(())
    }

    fn close_menus_and_recap(&mut self) {
        self.menu_open = false;
        self.type_menu_open = false;
        self.recap = None;
        self.breakdown = None;
    }

    /// `ShowSourceWindow` (DamageMeterSessionWindow.lua:967-979): a death row opens its
    /// recap, an Interrupts or Dispels row its source's spell breakdown; a recap or
    /// breakdown row closes it.
    fn click_row(&mut self, index: usize) -> Result<(), String> {
        if self.recap.take().is_some() || self.breakdown.take().is_some() {
            return Ok(());
        }
        if let Some(kind) = self.counted_kind() {
            let source = self
                .source_totals(kind)
                .get(index)
                .map(|(unit, _)| unit.unit);
            self.breakdown = Some(source.ok_or_else(|| format!("No source row {index}"))?);
            return Ok(());
        }
        if self.meter_type != MeterType::Deaths {
            return Err(format!("{:?} rows are not clickable", self.meter_type));
        }
        let death = self.deaths().get(index).copied();
        self.recap = Some(death.ok_or_else(|| format!("No death row {index}"))?);
        Ok(())
    }

    pub fn view(&self, in_combat: bool, timer_width: f32) -> DamageMeterView {
        DamageMeterView {
            session: self.session,
            meter_type: self.meter_type,
            timer_text: self.timer_text(in_combat),
            timer_width,
            rows: self.rows(),
            menu_open: self.menu_open,
            type_menu_open: self.type_menu_open,
            recap_open: self.recap.is_some(),
            breakdown_open: self.breakdown.is_some(),
        }
    }
}

/// A damage source of the server's session: "N. Name" and "damage (dps)".
fn damage_row(index: usize, source: &DamageMeterSource, max: u64) -> DamageMeterRow {
    DamageMeterRow {
        name_text: format!("{}. {}", index + 1, source.name),
        value_text: format!(
            "{} ({})",
            abbreviate_large_number(source.total_amount),
            abbreviate_large_number(source.amount_per_second.max(0.0) as u64)
        ),
        fraction: if max > 0 {
            source.total_amount as f32 / max as f32
        } else {
            0.0
        },
        color: class_color(source.class_id),
        class_id: source.class_id,
        is_local_player: source.is_local_player,
    }
}

/// Ranked rows of client-counted totals: "amount (per second)" over the session's
/// `duration`, or the bare count for the types that suppress the per-second value
/// (`DAMAGE_METER_TYPE_SUPPRESS_VALUE_PER_SECOND`, DamageMeterSessionWindow.lua:29-32).
fn counted_rows(totals: &[(&MeterUnit, u64)], duration: Option<f32>) -> Vec<DamageMeterRow> {
    let max = totals.first().map_or(0, |(_, total)| *total);
    totals
        .iter()
        .enumerate()
        .map(|(index, (source, total))| DamageMeterRow {
            name_text: format!("{}. {}", index + 1, source.name),
            value_text: match duration {
                Some(duration) => {
                    let per_second = if duration > 0.0 {
                        (*total as f32 / duration) as u64
                    } else {
                        0
                    };
                    format!(
                        "{} ({})",
                        abbreviate_large_number(*total),
                        abbreviate_large_number(per_second)
                    )
                }
                None => abbreviate_large_number(*total),
            },
            fraction: *total as f32 / max as f32,
            color: class_color(source.class_id),
            class_id: source.class_id,
            is_local_player: source.is_local_player,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::{DamageMeterSource, DamageMeterSpell};

    fn source(name: &str, class_id: u8, total: u64, dps: f32, local: bool) -> DamageMeterSource {
        DamageMeterSource {
            unit: u64::from(class_id),
            name: name.into(),
            class_id,
            is_local_player: local,
            total_amount: total,
            amount_per_second: dps,
            spells: vec![DamageMeterSpell {
                spell_id: 228_597,
                total_amount: total,
                amount_per_second: dps,
            }],
        }
    }

    fn session(id: u32, duration: f32, sources: Vec<DamageMeterSource>) -> DamageMeterSession {
        DamageMeterSession {
            session_id: id,
            duration_secs: duration,
            active: false,
            total_amount: sources.iter().map(|s| s.total_amount).sum(),
            sources,
        }
    }

    fn two_combats() -> DamageMeterWindow {
        DamageMeterWindow {
            snapshot: Some(DamageMeterSnapshot {
                current: Some(session(2, 6.2, vec![source("Fbmage", 8, 93, 15.0, true)])),
                overall: session(
                    0,
                    75.0,
                    vec![
                        source("Fbwarrior", 1, 1_234_567, 16_460.9, false),
                        source("Fbmage", 8, 186, 2.48, true),
                    ],
                ),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn large_numbers_abbreviate_like_the_retail_lua() {
        let shown: Vec<_> = [7, 999, 1_234, 12_345, 123_456, 12_345_678, 123_456_789]
            .into_iter()
            .map(abbreviate_large_number)
            .collect();
        assert_eq!(
            shown,
            ["7", "999", "1,234", "12,345", "123K", "12345K", "123M"]
        );
        assert_eq!(seconds_to_clock(75.9), "01:15");
        assert_eq!(seconds_to_clock(3_725.0), "01:02:05");
    }

    #[test]
    fn overall_is_the_default_session_with_ranked_class_coloured_compact_rows() {
        let window = two_combats();
        let view = window.view(false, 0.0);
        assert_eq!(view.session.short_name(), "O");
        let rows: Vec<_> = view
            .rows
            .iter()
            .map(|row| {
                (
                    row.name_text.as_str(),
                    row.value_text.as_str(),
                    row.is_local_player,
                )
            })
            .collect();
        assert_eq!(
            rows,
            [
                ("1. Fbwarrior", "1234K (16,460)", false),
                ("2. Fbmage", "186 (2)", true),
            ]
        );
        assert_eq!(view.rows[0].fraction, 1.0);
        assert!((view.rows[1].fraction - 186.0 / 1_234_567.0).abs() < 1e-9);
        assert_eq!(view.rows[0].color, [0.78, 0.61, 0.43]);
        assert_eq!(view.rows[1].color, [0.25, 0.78, 0.92]);
        // Out of combat the timer is hidden.
        assert_eq!(view.timer_text, "");
    }

    #[test]
    fn current_shows_the_latest_combat_and_its_timer_in_combat() {
        let mut window = two_combats();
        window.session = MeterSessionType::Current;
        let view = window.view(true, 48.0);
        assert_eq!(view.session.label(), "Current Segment");
        assert_eq!(view.timer_text, "[00:06] ");
        assert_eq!(view.rows.len(), 1);
        assert_eq!(view.rows[0].name_text, "1. Fbmage");
        assert_eq!(view.rows[0].value_text, "93 (15)");

        // Before the first combat Current is empty.
        window.snapshot.as_mut().unwrap().current = None;
        assert!(window.view(true, 0.0).rows.is_empty());
        assert_eq!(window.view(true, 0.0).timer_text, "");
    }
}
