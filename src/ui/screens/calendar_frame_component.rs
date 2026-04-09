use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::strata::FrameStrata;

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub const FRAME_W: f32 = 420.0;
pub const FRAME_H: f32 = 470.0;
const HEADER_H: f32 = 28.0;
const INSET: f32 = 8.0;
const BUTTON_H: f32 = 24.0;
const BUTTON_GAP: f32 = 6.0;
const EVENT_LIST_H: f32 = 180.0;
const EVENT_ROW_H: f32 = 36.0;
const EVENT_ROW_GAP: f32 = 2.0;
const DETAIL_TOP: f32 = HEADER_H + INSET + BUTTON_H + BUTTON_GAP + EVENT_LIST_H + BUTTON_GAP * 2.0;
const CONTENT_BG: &str = "0.0,0.0,0.0,0.3";
const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const BTN_BG: &str = "0.15,0.12,0.05,0.95";
const BTN_TEXT: &str = "1.0,0.82,0.0,1.0";
const ROW_BG_ACTIVE: &str = "0.2,0.15,0.05,0.95";
const ROW_BG: &str = "0.08,0.07,0.06,0.88";
const PRIMARY_TEXT: &str = "1.0,1.0,1.0,1.0";
const SECONDARY_TEXT: &str = "0.75,0.75,0.75,1.0";

pub const ACTION_CALENDAR_TOGGLE: &str = "calendar_toggle";
pub const ACTION_CALENDAR_CLOSE: &str = "calendar_close";
pub const ACTION_CALENDAR_REFRESH: &str = "calendar_refresh";
pub const ACTION_CALENDAR_SCHEDULE_RAID: &str = "calendar_schedule_raid";
pub const ACTION_CALENDAR_SCHEDULE_PARTY: &str = "calendar_schedule_party";
pub const ACTION_CALENDAR_SELECT_PREFIX: &str = "calendar_select:";
pub const ACTION_CALENDAR_SIGNUP_PREFIX: &str = "calendar_signup:";

#[derive(Clone, Debug, PartialEq)]
pub struct CalendarEventRow {
    pub title: String,
    pub schedule_text: String,
    pub counts_text: String,
    pub active: bool,
    pub action: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalendarSignupRow {
    pub name: String,
    pub status_text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalendarDetailState {
    pub title: String,
    pub organizer: String,
    pub schedule_text: String,
    pub type_text: String,
    pub signup_text: String,
    pub player_status_text: String,
    pub signups: Vec<CalendarSignupRow>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct CalendarFrameState {
    pub visible: bool,
    pub events: Vec<CalendarEventRow>,
    pub detail: Option<CalendarDetailState>,
    pub status_text: String,
    pub empty_text: Option<String>,
}

pub fn calendar_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<CalendarFrameState>()
        .expect("CalendarFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "CalendarFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "760",
                y: "-80",
            }
            {title_bar()}
            {button_row()}
            {event_list(state)}
            {detail_panel(state)}
            {status_line(&state.status_text)}
        }
    }
}

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "CalendarFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Calendar",
            font_size: 16.0,
            font_color: TITLE_COLOR,
            justify_h: "CENTER",
            anchor { point: AnchorPoint::Top, relative_point: AnchorPoint::Top }
        }
    }
}

fn button_row() -> Element {
    let buttons = [
        (
            "CalendarRefreshButton",
            "Refresh",
            ACTION_CALENDAR_REFRESH,
            INSET,
        ),
        (
            "CalendarScheduleRaidButton",
            "Raid +60m",
            ACTION_CALENDAR_SCHEDULE_RAID,
            INSET + 92.0,
        ),
        (
            "CalendarSchedulePartyButton",
            "Party +30m",
            ACTION_CALENDAR_SCHEDULE_PARTY,
            INSET + 196.0,
        ),
        (
            "CalendarCloseButton",
            "Close",
            ACTION_CALENDAR_CLOSE,
            FRAME_W - INSET - 64.0,
        ),
    ];
    buttons
        .into_iter()
        .flat_map(|(name, text, action, x)| calendar_button(name, text, action, x))
        .collect()
}

fn calendar_button(name: &str, text: &str, action: &str, x: f32) -> Element {
    let frame_name = DynName(name.to_string());
    rsx! {
        r#frame {
            name: frame_name,
            width: "88.0",
            height: {BUTTON_H},
            background_color: BTN_BG,
            onclick: action,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {-(HEADER_H + INSET)},
            }
            fontstring {
                name: {DynName(format!("{name}Text"))},
                width: "88.0",
                height: {BUTTON_H},
                text,
                font_size: 11.0,
                font_color: BTN_TEXT,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
    }
}

fn event_list(state: &CalendarFrameState) -> Element {
    let content: Element = if state.events.is_empty() {
        state
            .empty_text
            .as_deref()
            .and_then(empty_text)
            .unwrap_or_default()
    } else {
        state
            .events
            .iter()
            .enumerate()
            .flat_map(|(index, event)| event_row(index, event))
            .collect()
    };
    rsx! {
        r#frame {
            name: "CalendarEventList",
            width: {FRAME_W - 2.0 * INSET},
            height: {EVENT_LIST_H},
            background_color: CONTENT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INSET},
                y: {-(HEADER_H + INSET + BUTTON_H + BUTTON_GAP)},
            }
            {content}
        }
    }
}

fn event_row(index: usize, event: &CalendarEventRow) -> Element {
    let row_name = DynName(format!("CalendarEventRow{index}"));
    let y = -(index as f32 * (EVENT_ROW_H + EVENT_ROW_GAP));
    let bg = if event.active { ROW_BG_ACTIVE } else { ROW_BG };
    rsx! {
        r#frame {
            name: row_name,
            width: {FRAME_W - 2.0 * INSET},
            height: {EVENT_ROW_H},
            background_color: bg,
            onclick: {event.action.as_str()},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: {y},
            }
            {event_row_title(index, &event.title)}
            {event_row_time(index, &event.schedule_text)}
            {event_row_counts(index, &event.counts_text)}
        }
    }
}

fn event_row_title(index: usize, title: &str) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("CalendarEventRow{index}Title"))},
            width: "200",
            height: "16",
            text: title,
            font_size: 12.0,
            font_color: PRIMARY_TEXT,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "8",
                y: "-5",
            }
        }
    }
}

fn event_row_time(index: usize, schedule_text: &str) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("CalendarEventRow{index}Time"))},
            width: "120",
            height: "14",
            text: schedule_text,
            font_size: 11.0,
            font_color: SECONDARY_TEXT,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "8",
                y: "-20",
            }
        }
    }
}

fn event_row_counts(index: usize, counts_text: &str) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("CalendarEventRow{index}Counts"))},
            width: "120",
            height: "14",
            text: counts_text,
            font_size: 11.0,
            font_color: SECONDARY_TEXT,
            justify_h: "RIGHT",
            anchor {
                point: AnchorPoint::TopRight,
                relative_point: AnchorPoint::TopRight,
                x: "-8",
                y: "-12",
            }
        }
    }
}

fn detail_panel(state: &CalendarFrameState) -> Element {
    let content: Element = state
        .detail
        .as_ref()
        .map(detail_content)
        .into_iter()
        .flatten()
        .collect();
    rsx! {
        r#frame {
            name: "CalendarDetailPanel",
            width: {FRAME_W - 2.0 * INSET},
            height: {FRAME_H - DETAIL_TOP - 28.0},
            background_color: CONTENT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INSET},
                y: {-DETAIL_TOP},
            }
            {content}
        }
    }
}

fn detail_content(detail: &CalendarDetailState) -> Element {
    let signup_buttons = [
        ("CalendarConfirmButton", "Confirm", "confirmed", 8.0),
        ("CalendarTentativeButton", "Tentative", "tentative", 96.0),
        ("CalendarDeclineButton", "Decline", "declined", 196.0),
    ];
    let buttons: Element = signup_buttons
        .into_iter()
        .flat_map(|(name, label, token, x)| {
            detail_signup_button(
                name,
                label,
                &format!("{ACTION_CALENDAR_SIGNUP_PREFIX}{token}"),
                x,
            )
        })
        .collect();
    let signup_rows: Element = detail
        .signups
        .iter()
        .enumerate()
        .flat_map(|(index, signup)| signup_row(index, signup))
        .collect();
    rsx! {
        fontstring {
            name: "CalendarDetailTitle",
            width: "300",
            height: "18",
            text: {detail.title.as_str()},
            font_size: 14.0,
            font_color: TITLE_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "8",
                y: "-8",
            }
        }
        fontstring {
            name: "CalendarDetailOrganizer",
            width: "320",
            height: "14",
            text: {detail.organizer.as_str()},
            font_size: 11.0,
            font_color: SECONDARY_TEXT,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "8",
                y: "-30",
            }
        }
        fontstring {
            name: "CalendarDetailSchedule",
            width: "320",
            height: "14",
            text: {detail.schedule_text.as_str()},
            font_size: 11.0,
            font_color: SECONDARY_TEXT,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "8",
                y: "-46",
            }
        }
        fontstring {
            name: "CalendarDetailType",
            width: "320",
            height: "14",
            text: {detail.type_text.as_str()},
            font_size: 11.0,
            font_color: SECONDARY_TEXT,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "8",
                y: "-62",
            }
        }
        fontstring {
            name: "CalendarDetailSignupSummary",
            width: "320",
            height: "14",
            text: {detail.signup_text.as_str()},
            font_size: 11.0,
            font_color: SECONDARY_TEXT,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "8",
                y: "-78",
            }
        }
        fontstring {
            name: "CalendarDetailPlayerStatus",
            width: "320",
            height: "14",
            text: {detail.player_status_text.as_str()},
            font_size: 11.0,
            font_color: SECONDARY_TEXT,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "8",
                y: "-94",
            }
        }
        {buttons}
        {signup_rows}
    }
}

fn detail_signup_button(name: &str, text: &str, action: &str, x: f32) -> Element {
    let frame_name = DynName(name.to_string());
    rsx! {
        r#frame {
            name: frame_name,
            width: "88.0",
            height: {BUTTON_H},
            background_color: BTN_BG,
            onclick: action,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: "-112",
            }
            fontstring {
                name: {DynName(format!("{name}Text"))},
                width: "88.0",
                height: {BUTTON_H},
                text,
                font_size: 11.0,
                font_color: BTN_TEXT,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
    }
}

fn signup_row(index: usize, signup: &CalendarSignupRow) -> Element {
    let y = -(140.0 + index as f32 * 18.0);
    rsx! {
        fontstring {
            name: {DynName(format!("CalendarSignupRow{index}"))},
            width: "360",
            height: "14",
            text: {format!("{} - {}", signup.name, signup.status_text)},
            font_size: 11.0,
            font_color: PRIMARY_TEXT,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "8",
                y: {y},
            }
        }
    }
}

fn empty_text(text: &str) -> Option<Element> {
    Some(rsx! {
        fontstring {
            name: "CalendarEmptyText",
            width: {FRAME_W - 2.0 * INSET},
            height: "16",
            text,
            font_size: 12.0,
            font_color: SECONDARY_TEXT,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                x: "0",
                y: "-16",
            }
        }
    })
}

fn status_line(text: &str) -> Element {
    rsx! {
        fontstring {
            name: "CalendarStatusText",
            width: {FRAME_W - 2.0 * INSET},
            height: "14",
            text,
            font_size: 11.0,
            font_color: SECONDARY_TEXT,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::BottomLeft,
                relative_point: AnchorPoint::BottomLeft,
                x: {INSET},
                y: "8",
            }
        }
    }
}

#[cfg(test)]
#[path = "calendar_frame_component_tests.rs"]
mod tests;
