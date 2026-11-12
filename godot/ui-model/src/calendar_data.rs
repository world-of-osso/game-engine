//! Calendar month state and dates shown by the native `CalendarFrame`.
use chrono::{DateTime, Datelike, Duration, Local, Months, NaiveDate, Utc};
use shared::protocol::{CalendarEventSnapshot, CalendarSnapshot};

pub const ACTION_TOGGLE: &str = "calendar:toggle";
pub const ACTION_CLOSE: &str = "calendar:close";
pub const ACTION_PREVIOUS_MONTH: &str = "calendar:previous_month";
pub const ACTION_NEXT_MONTH: &str = "calendar:next_month";
pub const ACTION_TOGGLE_FILTER: &str = "calendar:toggle_filter";
pub const ACTION_TOGGLE_SERVER_EVENTS: &str = "calendar:toggle_server_events";

const GRID_DAYS: usize = 42;
const SUNDAY_FIRST_WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarEvent {
    pub event_id: u64,
    pub title: String,
    pub date: NaiveDate,
}

impl CalendarEvent {
    fn from_snapshot(event: &CalendarEventSnapshot) -> Result<Self, String> {
        let timestamp = i64::try_from(event.starts_at_unix_secs).map_err(|error| {
            format!(
                "calendar event {} has an invalid start timestamp: {error}",
                event.event_id
            )
        })?;
        let starts_at = DateTime::<Utc>::from_timestamp(timestamp, 0).ok_or_else(|| {
            format!(
                "calendar event {} has an out-of-range start timestamp",
                event.event_id
            )
        })?;
        Ok(Self {
            event_id: event.event_id,
            title: event.title.clone(),
            date: starts_at.with_timezone(&Local).date_naive(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarDay {
    pub date: NaiveDate,
    pub in_viewed_month: bool,
    pub is_today: bool,
    pub events: Vec<CalendarEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarView {
    pub month_name: String,
    pub year: i32,
    pub weekdays: [&'static str; 7],
    pub days: Vec<CalendarDay>,
    pub filter_open: bool,
    pub show_server_events: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarModel {
    open: bool,
    viewed_month: Option<NaiveDate>,
    today: Option<NaiveDate>,
    events: Vec<CalendarEvent>,
    filter_open: bool,
    show_server_events: bool,
}

impl Default for CalendarModel {
    fn default() -> Self {
        Self {
            open: false,
            viewed_month: None,
            today: None,
            events: Vec::new(),
            filter_open: false,
            show_server_events: true,
        }
    }
}

impl CalendarModel {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn apply_action(&mut self, action: &str, today: NaiveDate) -> Result<(), String> {
        match action {
            ACTION_TOGGLE => self.toggle(today),
            ACTION_CLOSE => self.close(),
            ACTION_PREVIOUS_MONTH => self.change_month(false)?,
            ACTION_NEXT_MONTH => self.change_month(true)?,
            ACTION_TOGGLE_FILTER => self.filter_open = !self.filter_open,
            ACTION_TOGGLE_SERVER_EVENTS => {
                self.show_server_events = !self.show_server_events;
                self.filter_open = false;
            }
            "" => {}
            other => return Err(format!("unknown calendar action: {other}")),
        }
        Ok(())
    }

    pub fn replace_events(&mut self, snapshot: &CalendarSnapshot) -> Result<(), String> {
        let events = snapshot
            .events
            .iter()
            .map(CalendarEvent::from_snapshot)
            .collect::<Result<Vec<_>, _>>()?;
        self.events = events;
        Ok(())
    }

    pub fn view(&self) -> Result<Option<CalendarView>, String> {
        if !self.open {
            return Ok(None);
        }
        let month = self
            .viewed_month
            .ok_or("open calendar has no viewed month")?;
        let today = self.today.ok_or("open calendar has no current date")?;
        let first_day_offset = i64::from(month.weekday().num_days_from_sunday());
        let first_cell = month
            .checked_sub_signed(Duration::days(first_day_offset))
            .ok_or("calendar grid starts outside the supported date range")?;
        let days = (0..GRID_DAYS)
            .map(|index| {
                let date = first_cell
                    .checked_add_signed(Duration::days(index as i64))
                    .ok_or("calendar grid ends outside the supported date range")?;
                Ok(CalendarDay {
                    date,
                    in_viewed_month: date.year() == month.year() && date.month() == month.month(),
                    is_today: date == today,
                    events: self
                        .events
                        .iter()
                        .filter(|event| self.show_server_events && event.date == date)
                        .cloned()
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Some(CalendarView {
            month_name: month.format("%B").to_string(),
            year: month.year(),
            weekdays: SUNDAY_FIRST_WEEKDAYS,
            days,
            filter_open: self.filter_open,
            show_server_events: self.show_server_events,
        }))
    }

    fn toggle(&mut self, today: NaiveDate) {
        if self.open {
            self.close();
        } else {
            self.open_on(today);
        }
    }

    fn open_on(&mut self, today: NaiveDate) {
        self.open = true;
        self.today = Some(today);
        self.viewed_month = Some(first_of_month(today));
        self.filter_open = false;
    }

    fn close(&mut self) {
        self.open = false;
        self.filter_open = false;
    }

    fn change_month(&mut self, next: bool) -> Result<(), String> {
        if !self.open {
            return Err("calendar month navigation requires an open calendar".into());
        }
        let month = self
            .viewed_month
            .ok_or("open calendar has no viewed month")?;
        let next_month = if next {
            month.checked_add_months(Months::new(1))
        } else {
            month.checked_sub_months(Months::new(1))
        }
        .ok_or("calendar month is outside the supported date range")?;
        self.viewed_month = Some(next_month);
        self.filter_open = false;
        Ok(())
    }
}

fn first_of_month(date: NaiveDate) -> NaiveDate {
    date.with_day(1)
        .expect("valid calendar date has a first day")
}
