//! Retail damage meter window (`Blizzard_DamageMeter`): which session the primary window
//! shows and the rows it draws from the server's `DamageMeterSnapshot`. Spec:
//! docs/specs/damage-meter.md.
//!
//! The server owns the numbers (`C_DamageMeter`); the window picks the `Current` or
//! `Overall` session, ranks its sources and formats each bar the way
//! `DamageMeterSourceEntryMixin` does with the Edit Mode default `Numbers` = Compact.

use shared::protocol::{DamageMeterSession, DamageMeterSnapshot};

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

/// `DAMAGE_METER_TYPE_DAMAGE_DONE`: the default window type (`GetDefaultWindowData`,
/// DamageMeter.lua:92-97).
pub const DAMAGE_DONE_LABEL: &str = "Damage Done";

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
    pub is_local_player: bool,
}

/// What the window draws.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DamageMeterView {
    pub session: MeterSessionType,
    /// `"[%s] "` of the session's duration while the player is in combat, else empty
    /// (`ShouldDisplaySessionTimer`, `SetSessionDuration`).
    pub timer_text: String,
    /// Width the host measured for `timer_text`; the type dropdown follows it.
    pub timer_width: f32,
    pub rows: Vec<DamageMeterRow>,
    /// The session menu is open.
    pub menu_open: bool,
}

/// The primary window's state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DamageMeterWindow {
    pub session: MeterSessionType,
    pub snapshot: Option<DamageMeterSnapshot>,
    pub menu_open: bool,
}

impl DamageMeterWindow {
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

    /// Rows in the server's rank order (`BuildDataProvider`).
    pub fn rows(&self) -> Vec<DamageMeterRow> {
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
            .map(|(index, source)| DamageMeterRow {
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
                is_local_player: source.is_local_player,
            })
            .collect()
    }

    pub fn view(&self, in_combat: bool, timer_width: f32) -> DamageMeterView {
        DamageMeterView {
            session: self.session,
            timer_text: self.timer_text(in_combat),
            timer_width,
            rows: self.rows(),
            menu_open: self.menu_open,
        }
    }
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
