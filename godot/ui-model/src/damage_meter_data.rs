//! Retail damage meter window (`Blizzard_DamageMeter`): which session the primary window
//! shows and the rows it draws from the server's `DamageMeterSnapshot`. Spec:
//! docs/specs/damage-meter.md.
//!
//! The server owns the numbers (`C_DamageMeter`); the window picks the `Current` or
//! `Overall` session, ranks its sources and formats each bar the way
//! `DamageMeterSourceEntryMixin` does with the Edit Mode default `Numbers` = Compact.
//!
//! Every category and death recap comes from the server snapshot, including remote
//! group members. Combat log delivery never contributes to these totals.

use shared::protocol::{
    CombatLogEvent, CombatLogKind, DamageMeterDeathRecap, DamageMeterSession, DamageMeterSnapshot,
    DamageMeterSource,
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
/// Recap bar colours: damage red, healing green.
const RECAP_DAMAGE_COLOR: [f32; 3] = [0.85, 0.15, 0.15];
const RECAP_HEAL_COLOR: [f32; 3] = [0.2, 0.8, 0.2];

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

    /// Death rows open their server recap; recap rows close it.
    pub fn rows_clickable(&self) -> bool {
        self.recap_open || self.meter_type == MeterType::Deaths
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
    pub threat_in_combat: bool,
    /// Names resolved by the host for recap actors outside the snapshot roster.
    pub recap_unit_names: std::collections::BTreeMap<u64, String>,
    pub recap_spell_names: std::collections::BTreeMap<u32, String>,
    pub menu_open: bool,
    pub type_menu_open: bool,
    /// The selected server recap: victim unit and death timestamp.
    pub recap: Option<(u64, u64)>,
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
        if self.threat_in_combat && !in_combat {
            self.threat_tables.clear();
        }
        self.threat_in_combat = in_combat;
    }

    fn threat_rows(&self) -> Vec<DamageMeterRow> {
        if !self.threat_in_combat {
            return Vec::new();
        }
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

    /// Replace every category with the server's newest session data.
    pub fn set_snapshot(&mut self, snapshot: Option<DamageMeterSnapshot>) {
        self.snapshot = snapshot;
        if self.selected_recap().is_none() {
            self.recap = None;
        }
    }

    fn category_sources(&self) -> Vec<(&DamageMeterSource, u64)> {
        let Some(session) = self.session_data() else {
            return Vec::new();
        };
        let mut totals: Vec<_> = session
            .sources
            .iter()
            .map(|source| {
                let amount = match self.meter_type {
                    MeterType::HealingDone => source.healing_done,
                    MeterType::Interrupts => source.interrupts,
                    MeterType::Dispels => source.dispels,
                    MeterType::Deaths => source.deaths,
                    _ => source.total_amount,
                };
                (source, amount)
            })
            .collect();
        totals.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.name.cmp(&b.0.name)));
        totals
    }

    /// Damage retains the server's order and DPS; other categories rank their totals.
    pub fn rows(&self) -> Vec<DamageMeterRow> {
        if let Some(recap) = self.selected_recap() {
            return self.recap_rows(recap);
        }
        match self.meter_type {
            MeterType::DamageDone => self.damage_rows(),
            MeterType::Threat => self.threat_rows(),
            _ => {
                let duration = self.session_data().map_or(0.0, |s| s.duration_secs);
                category_rows(&self.category_sources(), self.meter_type, duration)
            }
        }
    }

    fn selected_recap(&self) -> Option<&DamageMeterDeathRecap> {
        let (unit, timestamp) = self.recap?;
        let source = self
            .session_data()?
            .sources
            .iter()
            .find(|s| s.unit == unit)?;
        source
            .death_recaps
            .iter()
            .find(|r| r.timestamp_unix_ms == timestamp)
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

    /// Server-capped damage and healing events, newest first, using server timestamps.
    fn recap_rows(&self, recap: &DamageMeterDeathRecap) -> Vec<DamageMeterRow> {
        let events: Vec<_> = recap
            .events
            .iter()
            .rev()
            .filter(|event| {
                matches!(
                    event.kind,
                    CombatLogKind::Damage | CombatLogKind::Heal | CombatLogKind::Environmental(_)
                )
            })
            .collect();
        let max = events.iter().map(|e| recap_amount(e)).max().unwrap_or(0);
        events
            .into_iter()
            .map(|event| {
                let name = format!(
                    "{} by {}",
                    self.recap_spell_name(event),
                    self.recap_source_name(event.source)
                );
                recap_row(event, recap.timestamp_unix_ms, max, name)
            })
            .collect()
    }

    fn recap_spell_name(&self, event: &CombatLogEvent) -> String {
        match (event.kind, event.spell_id) {
            (CombatLogKind::Environmental(kind), _) => environmental_name(kind).to_owned(),
            (_, Some(id)) => self
                .recap_spell_names
                .get(&id)
                .cloned()
                .unwrap_or_else(|| format!("Spell {id}")),
            (_, None) => MELEE_LABEL.to_owned(),
        }
    }

    fn recap_source_name(&self, unit: Option<u64>) -> String {
        let Some(unit) = unit else {
            return "Environment".into();
        };
        self.session_data()
            .and_then(|s| s.sources.iter().find(|s| s.unit == unit))
            .map(|s| s.name.clone())
            .or_else(|| self.recap_unit_names.get(&unit).cloned())
            .unwrap_or_else(|| format!("Unit {unit}"))
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
    }

    /// Open a member's latest exported recap; a recap row closes it.
    fn click_row(&mut self, index: usize) -> Result<(), String> {
        if self.recap.take().is_some() {
            return Ok(());
        }
        if self.meter_type != MeterType::Deaths {
            return Err(format!("{:?} rows are not clickable", self.meter_type));
        }
        let sources = self.category_sources();
        let (source, _) = sources
            .get(index)
            .ok_or_else(|| format!("No death row {index}"))?;
        self.recap = source
            .death_recaps
            .last()
            .map(|r| (source.unit, r.timestamp_unix_ms));
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

/// Effective healing excludes overhealing and consumed shields; count types omit rates.
fn category_rows(
    totals: &[(&DamageMeterSource, u64)],
    kind: MeterType,
    duration: f32,
) -> Vec<DamageMeterRow> {
    let max = totals.first().map_or(0, |(_, total)| *total);
    totals
        .iter()
        .enumerate()
        .map(|(index, (source, total))| {
            let value_text = category_value_text(*total, kind, duration);
            DamageMeterRow {
                name_text: if kind == MeterType::Deaths {
                    source.name.clone()
                } else {
                    format!("{}. {}", index + 1, source.name)
                },
                value_text,
                fraction: if kind == MeterType::Deaths {
                    1.0
                } else if max > 0 {
                    *total as f32 / max as f32
                } else {
                    0.0
                },
                color: class_color(source.class_id),
                class_id: source.class_id,
                is_local_player: source.is_local_player,
            }
        })
        .collect()
}

fn category_value_text(total: u64, kind: MeterType, duration: f32) -> String {
    if kind != MeterType::HealingDone {
        return abbreviate_large_number(total);
    }
    let rate = if duration > 0.0 {
        (total as f32 / duration) as u64
    } else {
        0
    };
    format!(
        "{} ({})",
        abbreviate_large_number(total),
        abbreviate_large_number(rate)
    )
}

fn recap_row(
    event: &CombatLogEvent,
    death_timestamp: u64,
    max: u64,
    name_text: String,
) -> DamageMeterRow {
    let amount = recap_amount(event);
    let (sign, color) = if event.kind == CombatLogKind::Heal {
        ('+', RECAP_HEAL_COLOR)
    } else {
        ('-', RECAP_DAMAGE_COLOR)
    };
    let seconds = death_timestamp.saturating_sub(event.timestamp_unix_ms) as f64 / 1000.0;
    DamageMeterRow {
        name_text,
        value_text: format!("{sign}{} ({seconds:.1}s)", break_up_large_number(amount)),
        fraction: if max > 0 {
            amount as f32 / max as f32
        } else {
            0.0
        },
        color,
        class_id: 0,
        is_local_player: false,
    }
}

fn recap_amount(event: &CombatLogEvent) -> u64 {
    let amount = event.amount.max(0);
    let effective = if event.kind == CombatLogKind::Heal {
        amount - event.overflow.clamp(0, amount)
    } else {
        amount
    };
    effective as u64
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
            healing_done: 0,
            overhealing: 0,
            absorbs: 0,
            interrupts: 0,
            dispels: 0,
            deaths: 0,
            death_recaps: vec![],
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
