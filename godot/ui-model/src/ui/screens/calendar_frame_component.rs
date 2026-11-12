//! Retail `CalendarFrame` from Blizzard_Calendar/Mainline/Blizzard_Calendar.xml and
//! `CalendarDayButtonTemplate` from Blizzard_CalendarTemplates.xml. Layout is the authored
//! 659×624 panel with its seven 91px columns and six 91px day rows.
use chrono::Datelike;
use ui_toolkit::{rsx, widget_def::Element, widgets::font_string::GameFont};

use crate::calendar_data::{
    ACTION_CLOSE, ACTION_NEXT_MONTH, ACTION_PREVIOUS_MONTH, ACTION_TOGGLE_FILTER,
    ACTION_TOGGLE_SERVER_EVENTS, CalendarDay, CalendarView,
};
use crate::menu_primitives::{
    ContextMenu, ContextMenuItem, DropdownButton, DropdownState, DropdownStyle, MenuElementKind,
    context_menu_with_rows, dropdown_button_with_state, menu_element, menu_row_rect_with_heading,
};
use crate::quest_art::{DynName, close_button};
use crate::ui::strata::FrameStrata;

const FRAME_WIDTH: f32 = 659.0;
const FRAME_HEIGHT: f32 = 624.0;
const DAY_WIDTH: f32 = 91.0;
const DAY_HEIGHT: f32 = 91.0;
const DAY_COLUMNS: usize = 7;
const DAY_ROWS: usize = 6;
const DAY_COUNT: usize = DAY_COLUMNS * DAY_ROWS;
const WEEKDAY_LEFT: f32 = 12.0;
const WEEKDAY_TOP: f32 = 46.0;
const DAY_TOP: f32 = 74.0;
const CALENDAR_BACKGROUND: u32 = 235_428;
const CALENDAR_EVENT_BACKGROUND: u32 = 235_429;
const CALENDAR_FRAME_SIDES: u32 = 235_430;
const CALENDAR_FRAME_TOP_BOTTOM: u32 = 235_431;
const CALENDAR_CURRENT_DAY: u32 = 235_433;
const CALENDAR_HIGHLIGHTS: u32 = 235_438;
const CALENDAR_MORE_ARROW: u32 = 235_487;
const PREVIOUS_PAGE_UP: u32 = 130_869;
const PREVIOUS_PAGE_DOWN: u32 = 130_868;
const PREVIOUS_PAGE_DISABLED: u32 = 130_867;
const NEXT_PAGE_UP: u32 = 130_866;
const NEXT_PAGE_DOWN: u32 = 130_865;
const NEXT_PAGE_DISABLED: u32 = 130_864;
const MOUSE_HIGHLIGHT: u32 = 130_757;
const NORMAL: &str = "1.0,1.0,1.0,1.0";
const MUTED: &str = "0.55,0.55,0.55,1.0";
const WEEKDAY_GOLD: &str = "1.0,1.0,0.60392157,1.0";
const CALENDAR_BORDER_TEXTURES: [CalendarTexture; 12] = [
    CalendarTexture::new(
        CALENDAR_FRAME_TOP_BOTTOM,
        "0,1,0.00390625,0.16015625",
        [0.0, 0.0, 256.0, 46.0],
    ),
    CalendarTexture::new(
        CALENDAR_FRAME_TOP_BOTTOM,
        "0,1,0.16796875,0.32421875",
        [256.0, 0.0, 256.0, 46.0],
    ),
    CalendarTexture::new(
        CALENDAR_FRAME_TOP_BOTTOM,
        "0,0.57421875,0.328125,0.4296875",
        [512.0, 16.0, 147.0, 30.0],
    ),
    CalendarTexture::new(
        CALENDAR_FRAME_SIDES,
        "0,0.09375,0,1",
        [0.0, 46.0, 12.0, 256.0],
    ),
    CalendarTexture::new(
        CALENDAR_FRAME_SIDES,
        "0.09375,0.1875,0,1",
        [0.0, 302.0, 12.0, 256.0],
    ),
    CalendarTexture::new(
        CALENDAR_FRAME_SIDES,
        "0.1953125,0.28125,0,0.2421875",
        [2.0, 558.0, 11.0, 62.0],
    ),
    CalendarTexture::new(
        CALENDAR_FRAME_SIDES,
        "0.296875,0.3984375,0,1",
        [646.0, 46.0, 13.0, 256.0],
    ),
    CalendarTexture::new(
        CALENDAR_FRAME_SIDES,
        "0.3984375,0.484375,0,1",
        [648.0, 302.0, 11.0, 256.0],
    ),
    CalendarTexture::new(
        CALENDAR_FRAME_SIDES,
        "0.484375,0.5703125,0,0.2421875",
        [648.0, 558.0, 11.0, 62.0],
    ),
    CalendarTexture::new(
        CALENDAR_FRAME_TOP_BOTTOM,
        "0,1,0.43359375,0.46875",
        [2.0, 620.0, 256.0, 9.0],
    ),
    CalendarTexture::new(
        CALENDAR_FRAME_TOP_BOTTOM,
        "0,1,0.46875,0.50390625",
        [258.0, 620.0, 256.0, 9.0],
    ),
    CalendarTexture::new(
        CALENDAR_FRAME_TOP_BOTTOM,
        "0,0.56640625,0.50390625,0.5390625",
        [514.0, 620.0, 145.0, 9.0],
    ),
];

#[derive(Clone, Copy)]
struct CalendarTexture {
    file_data_id: u32,
    tex_coords: &'static str,
    rect: [f32; 4],
}

impl CalendarTexture {
    const fn new(file_data_id: u32, tex_coords: &'static str, rect: [f32; 4]) -> Self {
        Self {
            file_data_id,
            tex_coords,
            rect,
        }
    }
}

/// FileDataIDs read from the Retail Calendar XML and its two button templates.
pub fn calendar_frame_texture_fdids(view: &CalendarView) -> Vec<u32> {
    let mut fdids = vec![
        CALENDAR_BACKGROUND,
        CALENDAR_EVENT_BACKGROUND,
        CALENDAR_FRAME_SIDES,
        CALENDAR_FRAME_TOP_BOTTOM,
        CALENDAR_CURRENT_DAY,
        CALENDAR_HIGHLIGHTS,
        PREVIOUS_PAGE_UP,
        PREVIOUS_PAGE_DOWN,
        PREVIOUS_PAGE_DISABLED,
        NEXT_PAGE_UP,
        NEXT_PAGE_DOWN,
        NEXT_PAGE_DISABLED,
        MOUSE_HIGHLIGHT,
    ];
    if view.days.iter().any(|day| day.events.len() > 4) {
        fdids.push(CALENDAR_MORE_ARROW);
    }
    fdids.sort_unstable();
    fdids.dedup();
    fdids
}

pub fn calendar_frame_screen(ctx: &ui_toolkit::screen::SharedContext) -> Element {
    let view = ctx
        .get::<CalendarView>()
        .expect("CalendarView must be in SharedContext");
    let mut children = calendar_border();
    children.extend(calendar_month_header(view));
    children.extend(calendar_weekdays(view));
    children.extend(calendar_days(view));
    children.extend(calendar_month_buttons());
    children.extend(calendar_filter(view));
    children.extend(calendar_close_button());
    rsx! {
        r#frame {
            name: "CalendarFrame",
            width: FRAME_WIDTH,
            height: FRAME_HEIGHT,
            strata: FrameStrata::High,
            toplevel: true,
            mouse_enabled: true,
            pos_type: "absolute",
            left: 15.0,
            top: 80.0,
            {children}
        }
    }
}

fn calendar_border() -> Element {
    CALENDAR_BORDER_TEXTURES
        .iter()
        .enumerate()
        .flat_map(|(index, art)| {
            calendar_texture(
                format!("CalendarFrameBorder{index}"),
                art.file_data_id,
                art.tex_coords,
                art.rect,
                "BORDER",
            )
        })
        .collect()
}

fn calendar_month_header(view: &CalendarView) -> Element {
    let month_rect = [255.5, 1.0, 142.0, 36.0];
    let year_rect = [301.0, 50.0, 47.0, 24.0];
    let mut children = calendar_texture(
        "CalendarMonthBackground".into(),
        CALENDAR_BACKGROUND,
        "0,0.5546875,0.828125,0.96875",
        month_rect,
        "OVERLAY",
    );
    children.extend(calendar_texture(
        "CalendarYearBackground".into(),
        CALENDAR_BACKGROUND,
        "0,0.5546875,0.828125,0.96875",
        year_rect,
        "ARTWORK",
    ));
    children.extend(calendar_font_string(
        "CalendarMonthName",
        &view.month_name,
        month_rect,
        16.0,
        NORMAL,
        "CENTER",
        "OVERLAY",
    ));
    children.extend(calendar_font_string(
        "CalendarYearName",
        &view.year.to_string(),
        [year_rect[0], year_rect[1] + 1.0, year_rect[2], year_rect[3]],
        11.0,
        WEEKDAY_GOLD,
        "CENTER",
        "ARTWORK",
    ));
    children
}

fn calendar_weekdays(view: &CalendarView) -> Element {
    view.weekdays
        .iter()
        .enumerate()
        .flat_map(|(index, weekday)| {
            let left = WEEKDAY_LEFT + index as f32 * DAY_WIDTH;
            let x_region = if index % 2 == 0 { 90.0 / 256.0 } else { 0.0 };
            let tex_coords = format!(
                "{x_region},{},{},{}",
                x_region + 90.0 / 256.0 - 0.001,
                180.0 / 256.0,
                180.0 / 256.0 + 28.0 / 256.0 - 0.001
            );
            let mut elements = calendar_texture(
                format!("CalendarWeekday{}Background", index + 1),
                CALENDAR_BACKGROUND,
                &tex_coords,
                [left, WEEKDAY_TOP, DAY_WIDTH, 28.0],
                "BACKGROUND",
            );
            elements.extend(calendar_font_string(
                format!("CalendarWeekday{}Name", index + 1),
                weekday,
                [left, WEEKDAY_TOP, DAY_WIDTH, 28.0],
                12.0,
                WEEKDAY_GOLD,
                "CENTER",
                "ARTWORK",
            ));
            elements
        })
        .collect()
}

fn calendar_days(view: &CalendarView) -> Element {
    let mut elements = Vec::new();
    for (index, day) in view.days.iter().take(DAY_COUNT).enumerate() {
        let column = index % DAY_COLUMNS;
        let row = index / DAY_COLUMNS;
        let left = WEEKDAY_LEFT + column as f32 * DAY_WIDTH;
        let top = DAY_TOP + row as f32 * DAY_HEIGHT;
        elements.extend(calendar_day_button(index, day, left, top));
        elements.extend(calendar_day_date(index, day, left, top));
        elements.extend(calendar_day_events(index, day, left, top));
        if day.is_today {
            elements.extend(calendar_today_art(left, top));
        }
    }
    elements
}

fn calendar_day_button(index: usize, day: &CalendarDay, left: f32, top: f32) -> Element {
    let cell_x = if index % 2 == 0 { 90.0 / 256.0 } else { 0.0 };
    let cell_y = if index % 4 < 2 { 0.0 } else { 90.0 / 256.0 };
    let tex_coords = format!(
        "{cell_x},{},{cell_y},{}",
        cell_x + 90.0 / 256.0 - 0.001,
        cell_y + 90.0 / 256.0 - 0.001
    );
    let vertex_color = if day.in_viewed_month { NORMAL } else { MUTED };
    rsx! {
        button {
            name: {DynName(format!("CalendarDayButton{}", index + 1))},
            width: DAY_WIDTH,
            height: DAY_HEIGHT,
            pos_type: "absolute",
            left,
            top,
            mouse_enabled: true,
            button_default_skin: false,
            button_fdid_up: "235428",
            button_up_tex_coords: {tex_coords.as_str()},
            button_up_vertex_color: vertex_color,
            button_fdid_highlight: "235438",
            button_highlight_tex_coords: "0,0.34375,0,0.6875",
            button_highlight_size: "88,88",
            button_highlight_offset: "1.5,1.5",
            button_highlight_blend_mode: "ADD",
            button_highlight_alpha: 0.5,
            draw_layer: "BACKGROUND",
        }
    }
}

fn calendar_day_date(index: usize, day: &CalendarDay, left: f32, top: f32) -> Element {
    let text = day.date.day().to_string();
    let color = if day.in_viewed_month { NORMAL } else { MUTED };
    calendar_font_string(
        format!("CalendarDayButton{}Date", index + 1),
        &text,
        [left + 5.0, top + 5.0, 23.0, 18.0],
        12.0,
        color,
        "LEFT",
        "ARTWORK",
    )
}

fn calendar_day_events(index: usize, day: &CalendarDay, left: f32, top: f32) -> Element {
    if day.events.is_empty() {
        return Vec::new();
    }
    let mut elements = calendar_texture(
        format!("CalendarDayButton{}EventBackground", index + 1),
        CALENDAR_EVENT_BACKGROUND,
        "0,0.671875,0,0.671875",
        [left, top + 5.0, DAY_WIDTH, 86.0],
        "ARTWORK",
    );
    let labels = event_labels(day);
    for (event_index, label) in labels.iter().enumerate() {
        let event_top =
            top + DAY_HEIGHT - 3.0 - (event_index + 1) as f32 * 12.0 - event_index as f32 * 3.0;
        elements.extend(calendar_font_string(
            format!("CalendarDayButton{}Event{}Text", index + 1, event_index + 1),
            label,
            [left + 4.0, event_top, 83.0, 12.0],
            10.0,
            NORMAL,
            "LEFT",
            "OVERLAY",
        ));
    }
    if day.events.len() > 4 {
        elements.extend(calendar_texture(
            format!("CalendarDayButton{}MoreEventsArrow", index + 1),
            CALENDAR_MORE_ARROW,
            "0,0.9375,0,0.6875",
            [left + 71.0, top + 7.0, 15.0, 11.0],
            "OVERLAY",
        ));
    }
    elements
}

fn event_labels(day: &CalendarDay) -> Vec<String> {
    if day.events.len() <= 4 {
        return day.events.iter().map(|event| event.title.clone()).collect();
    }
    let mut labels: Vec<_> = day
        .events
        .iter()
        .take(3)
        .map(|event| event.title.clone())
        .collect();
    labels.push(format!("+{} more", day.events.len() - 3));
    labels
}

fn calendar_today_art(left: f32, top: f32) -> Element {
    let mut elements = calendar_texture(
        "CalendarTodayTexture".into(),
        CALENDAR_CURRENT_DAY,
        "0.0078125,0.57421875,0.0390625,0.5859375",
        [left - 27.0, top - 24.5, 145.0, 140.0],
        "BORDER",
    );
    elements.extend(calendar_texture(
        "CalendarTodayTextureGlow".into(),
        CALENDAR_CURRENT_DAY,
        "0.5625,1,0,0.5078125",
        [left - 8.5, top - 34.5, 112.0, 130.0],
        "OVERLAY",
    ));
    elements
}

fn calendar_month_buttons() -> Element {
    let mut elements = calendar_month_button(
        "CalendarPrevMonthButton",
        [223.5, 2.0, 32.0, 32.0],
        ACTION_PREVIOUS_MONTH,
        PREVIOUS_PAGE_UP,
        PREVIOUS_PAGE_DOWN,
        PREVIOUS_PAGE_DISABLED,
    );
    elements.extend(calendar_month_button(
        "CalendarNextMonthButton",
        [397.5, 2.0, 32.0, 32.0],
        ACTION_NEXT_MONTH,
        NEXT_PAGE_UP,
        NEXT_PAGE_DOWN,
        NEXT_PAGE_DISABLED,
    ));
    elements
}

fn calendar_month_button(
    name: &str,
    [left, top, width, height]: [f32; 4],
    action: &str,
    up_fdid: u32,
    down_fdid: u32,
    disabled_fdid: u32,
) -> Element {
    let up = up_fdid.to_string();
    let down = down_fdid.to_string();
    let disabled = disabled_fdid.to_string();
    let highlight = MOUSE_HIGHLIGHT.to_string();
    rsx! {
        button {
            name: {DynName(name.into())},
            width,
            height,
            pos_type: "absolute",
            left,
            top,
            onclick: action,
            button_default_skin: false,
            button_fdid_up: {up.as_str()},
            button_fdid_pushed: {down.as_str()},
            button_fdid_disabled: {disabled.as_str()},
            button_fdid_highlight: {highlight.as_str()},
            button_highlight_blend_mode: "ADD",
        }
    }
}

fn calendar_filter(view: &CalendarView) -> Element {
    let dropdown = dropdown_button_with_state(
        DropdownButton {
            frame_name: "CalendarFilterButton",
            label_name: "CalendarFilterButtonText",
            arrow_name: "CalendarFilterButtonArrow",
            text: "Filters",
            width: 106.0,
            height: 24.0,
            x: 526.0,
            y: -20.0,
            background_color: "0,0,0,0",
            text_color: NORMAL,
            arrow_color: NORMAL,
            onclick: Some(ACTION_TOGGLE_FILTER),
        },
        DropdownState {
            style: DropdownStyle::Filter,
            resize_to_text: false,
            open: view.filter_open,
            enabled: true,
            reset_action: None,
        },
    );
    let item = ContextMenuItem {
        name: "CalendarFilterServerEvents",
        label: "Server events",
        action: ACTION_TOGGLE_SERVER_EVENTS,
    };
    let rows = menu_element(
        item,
        MenuElementKind::Checkbox {
            checked: view.show_server_events,
        },
        true,
        menu_row_rect_with_heading(184.0, 0, false),
    );
    let popup = context_menu_with_rows(
        ContextMenu {
            frame_name: "CalendarFilterMenu",
            title_name: "CalendarFilterMenuTitle",
            divider_name: "CalendarFilterMenuDivider",
            hidden: !view.filter_open,
            title: "",
            width: 184.0,
            x: 526.0,
            y: 44.0,
            items: &[item],
        },
        rows,
    );
    let mut elements = dropdown;
    elements.extend(popup);
    elements
}

fn calendar_close_button() -> Element {
    // The shared UIPanelCloseButton helper uses Forever's default (-2,-1) anchor. The
    // XML's explicit TOPRIGHT(+1,-17) anchor is therefore a (3,18) host-frame offset.
    rsx! {
        r#frame {
            name: "CalendarCloseButtonHost",
            width: FRAME_WIDTH,
            height: FRAME_HEIGHT,
            pos_type: "absolute",
            left: 3.0,
            top: 18.0,
            {close_button("Calendar", FRAME_WIDTH, ACTION_CLOSE)}
        }
    }
}

fn calendar_font_string(
    name: impl Into<String>,
    text: &str,
    [left, top, width, height]: [f32; 4],
    font_size: f32,
    color: &str,
    justify: &str,
    draw_layer: &str,
) -> Element {
    let name = DynName(name.into());
    rsx! {
        fontstring {
            name,
            width,
            height,
            text,
            font: GameFont::FrizQuadrata,
            font_size,
            font_color: color,
            justify_h: justify,
            word_wrap: false,
            max_lines: 1,
            pos_type: "absolute",
            left,
            top,
            draw_layer,
        }
    }
}

fn calendar_texture(
    name: String,
    file_data_id: u32,
    tex_coords: &str,
    [left, top, width, height]: [f32; 4],
    draw_layer: &str,
) -> Element {
    let name = DynName(name);
    rsx! {
        texture {
            name,
            width,
            height,
            texture_fdid: file_data_id,
            tex_coords,
            pos_type: "absolute",
            left,
            top,
            draw_layer,
        }
    }
}
