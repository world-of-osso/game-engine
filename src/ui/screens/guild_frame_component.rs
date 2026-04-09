use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::strata::FrameStrata;

pub const ACTION_GUILD_TOGGLE: &str = "guild_toggle";
pub const ACTION_TAB_ROSTER: &str = "guild_tab_roster";
pub const ACTION_TAB_INFO: &str = "guild_tab_info";

const FRAME_W: f32 = 620.0;
const FRAME_H: f32 = 430.0;
const HEADER_H: f32 = 28.0;
const TAB_H: f32 = 28.0;
const TAB_GAP: f32 = 4.0;
const INSET: f32 = 8.0;
const ROW_H: f32 = 20.0;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const TAB_BG_ACTIVE: &str = "0.2,0.15,0.05,0.95";
const TAB_BG_INACTIVE: &str = "0.08,0.07,0.06,0.88";
const TAB_TEXT_ACTIVE: &str = "1.0,0.82,0.0,1.0";
const TAB_TEXT_INACTIVE: &str = "0.6,0.6,0.6,1.0";
const PANEL_BG: &str = "0.0,0.0,0.0,0.35";
const HEADER_BG: &str = "0.12,0.1,0.08,0.9";
const HEADER_TEXT: &str = "0.8,0.8,0.8,1.0";
const TEXT: &str = "1.0,1.0,1.0,1.0";
const SUBTLE: &str = "0.8,0.8,0.8,1.0";
const ROW_EVEN: &str = "0.04,0.04,0.04,0.6";
const ROW_ODD: &str = "0.06,0.06,0.06,0.6";

#[derive(Clone, Debug, PartialEq, Eq)]
struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GuildTabKind {
    #[default]
    Roster,
    Info,
}

impl GuildTabKind {
    pub fn action(self) -> &'static str {
        match self {
            Self::Roster => ACTION_TAB_ROSTER,
            Self::Info => ACTION_TAB_INFO,
        }
    }

    pub fn from_action(action: &str) -> Option<Self> {
        match action {
            ACTION_TAB_ROSTER => Some(Self::Roster),
            ACTION_TAB_INFO => Some(Self::Info),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuildTab {
    pub name: String,
    pub active: bool,
    pub action: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuildMemberRow {
    pub name: String,
    pub level: u16,
    pub class_name: String,
    pub rank_name: String,
    pub status: String,
    pub officer_note: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuildFrameState {
    pub visible: bool,
    pub guild_name: String,
    pub motd: String,
    pub info_text: String,
    pub status_text: String,
    pub active_tab: GuildTabKind,
    pub tabs: Vec<GuildTab>,
    pub members: Vec<GuildMemberRow>,
}

impl Default for GuildFrameState {
    fn default() -> Self {
        Self {
            visible: false,
            guild_name: String::new(),
            motd: String::new(),
            info_text: String::new(),
            status_text: String::new(),
            active_tab: GuildTabKind::Roster,
            tabs: vec![
                GuildTab {
                    name: "Roster".into(),
                    active: true,
                    action: ACTION_TAB_ROSTER.into(),
                },
                GuildTab {
                    name: "Info".into(),
                    active: false,
                    action: ACTION_TAB_INFO.into(),
                },
            ],
            members: Vec::new(),
        }
    }
}

pub fn guild_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<GuildFrameState>()
        .expect("GuildFrameState must be in SharedContext");
    let hide = !state.visible;
    let title = if state.guild_name.is_empty() {
        "Guild".to_string()
    } else {
        format!("Guild - {}", state.guild_name)
    };
    rsx! {
        r#frame {
            name: "GuildFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            hidden: hide,
            strata: FrameStrata::Dialog,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "180",
                y: "-90",
            }
            {title_bar(&title)}
            {tab_row(&state.tabs)}
            {status_line(&state.status_text)}
            {roster_panel(state)}
            {info_panel(state)}
        }
    }
}

fn title_bar(title: &str) -> Element {
    rsx! {
        fontstring {
            name: "GuildFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: title,
            font_size: 16.0,
            font_color: TITLE_COLOR,
            justify_h: "CENTER",
            anchor { point: AnchorPoint::Top, relative_point: AnchorPoint::Top, x: "0", y: "0" }
        }
    }
}

fn tab_row(tabs: &[GuildTab]) -> Element {
    let count = tabs.len().max(1) as f32;
    let available_w = FRAME_W - INSET * 2.0;
    let tab_w = (available_w - (count - 1.0) * TAB_GAP) / count;
    tabs.iter()
        .enumerate()
        .flat_map(|(i, tab)| {
            let x = INSET + i as f32 * (tab_w + TAB_GAP);
            guild_tab_frame(i, tab, tab_w, x)
        })
        .collect()
}

fn guild_tab_frame(index: usize, tab: &GuildTab, tab_w: f32, x: f32) -> Element {
    let (bg, color) = guild_tab_style(tab.active);
    rsx! {
        r#frame {
            name: DynName(format!("GuildTab{index}")),
            width: {tab_w},
            height: {TAB_H},
            background_color: bg,
            onclick: {tab.action.as_str()},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {-(HEADER_H + TAB_GAP)},
            }
            {guild_tab_label(index, &tab.name, tab_w, color)}
        }
    }
}

fn guild_tab_style(active: bool) -> (&'static str, &'static str) {
    if active {
        (TAB_BG_ACTIVE, TAB_TEXT_ACTIVE)
    } else {
        (TAB_BG_INACTIVE, TAB_TEXT_INACTIVE)
    }
}

fn guild_tab_label(index: usize, name: &str, tab_w: f32, color: &str) -> Element {
    rsx! {
        fontstring {
            name: DynName(format!("GuildTab{index}Label")),
            width: {tab_w},
            height: {TAB_H},
            text: name,
            font_size: 11.0,
            font_color: color,
            justify_h: "CENTER",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
        }
    }
}

fn status_line(text: &str) -> Element {
    rsx! {
        fontstring {
            name: "GuildFrameStatus",
            width: {FRAME_W - INSET * 2.0},
            height: 16.0,
            text: text,
            font_size: 10.0,
            font_color: SUBTLE,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INSET},
                y: {-(HEADER_H + TAB_GAP + TAB_H + 4.0)},
            }
        }
    }
}

fn roster_panel(state: &GuildFrameState) -> Element {
    let hidden = state.active_tab != GuildTabKind::Roster;
    let panel_y = -(HEADER_H + TAB_GAP + TAB_H + 24.0);
    let panel_h = FRAME_H - HEADER_H - TAB_H - 36.0;
    let row_w = FRAME_W - INSET * 2.0;
    rsx! {
        r#frame {
            name: "GuildRosterPanel",
            width: {row_w},
            height: {panel_h},
            hidden,
            background_color: PANEL_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INSET},
                y: {panel_y},
            }
            {roster_header(row_w)}
            {roster_rows(&state.members, row_w)}
        }
    }
}

fn roster_header(row_w: f32) -> Element {
    let mut x = 4.0;
    let cells: Element = GUILD_ROSTER_HEADER_COLUMNS
        .into_iter()
        .enumerate()
        .flat_map(|(i, (label, frac))| {
            let w = row_w * frac;
            let cell = roster_header_cell(i, label, w, x);
            x += w;
            cell
        })
        .collect();
    rsx! {
        r#frame {
            name: "GuildRosterHeaderRow",
            width: {row_w},
            height: {ROW_H},
            background_color: HEADER_BG,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            {cells}
        }
    }
}

const GUILD_ROSTER_HEADER_COLUMNS: [(&str, f32); 6] = [
    ("Name", 0.18),
    ("Lvl", 0.08),
    ("Class", 0.15),
    ("Rank", 0.14),
    ("Status", 0.13),
    ("Officer Note", 0.32),
];

fn roster_header_cell(index: usize, label: &str, width: f32, x: f32) -> Element {
    rsx! {
        fontstring {
            name: DynName(format!("GuildRosterHeader{index}")),
            width: {width},
            height: {ROW_H},
            text: label,
            font_size: 9.0,
            font_color: HEADER_TEXT,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: "0",
            }
        }
    }
}

fn roster_rows(rows: &[GuildMemberRow], row_w: f32) -> Element {
    rows.iter()
        .enumerate()
        .flat_map(|(index, row)| roster_row(index, row, row_w))
        .collect()
}

fn roster_row(index: usize, row: &GuildMemberRow, row_w: f32) -> Element {
    let bg = if index.is_multiple_of(2) {
        ROW_EVEN
    } else {
        ROW_ODD
    };
    let top = -((index + 1) as f32 * ROW_H);
    let values = [
        row.name.clone(),
        row.level.to_string(),
        row.class_name.clone(),
        row.rank_name.clone(),
        row.status.clone(),
        row.officer_note.clone(),
    ];
    let widths = [0.18, 0.08, 0.15, 0.14, 0.13, 0.32];
    let mut x = 4.0;
    let cells: Element = values
        .into_iter()
        .zip(widths)
        .enumerate()
        .flat_map(|(col, (value, frac))| {
            let w = row_w * frac;
            let cell = rsx! {
                fontstring {
                    name: DynName(format!("GuildRosterRow{index}Col{col}")),
                    width: {w},
                    height: {ROW_H},
                    text: {value.as_str()},
                    font_size: 9.0,
                    font_color: TEXT,
                    justify_h: "LEFT",
                    anchor {
                        point: AnchorPoint::TopLeft,
                        relative_point: AnchorPoint::TopLeft,
                        x: {x},
                        y: "0",
                    }
                }
            };
            x += w;
            cell
        })
        .collect();
    rsx! {
        r#frame {
            name: DynName(format!("GuildRosterRow{index}")),
            width: {row_w},
            height: {ROW_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: {top},
            }
            {cells}
        }
    }
}

fn info_panel(state: &GuildFrameState) -> Element {
    let hidden = state.active_tab != GuildTabKind::Info;
    let panel_y = -(HEADER_H + TAB_GAP + TAB_H + 24.0);
    let panel_h = FRAME_H - HEADER_H - TAB_H - 36.0;
    let panel_w = FRAME_W - INSET * 2.0;
    let info_text = if state.info_text.is_empty() {
        "No guild info set.".to_string()
    } else {
        state.info_text.clone()
    };
    let motd = if state.motd.is_empty() {
        "No guild message of the day.".to_string()
    } else {
        state.motd.clone()
    };
    rsx! {
        r#frame {
            name: "GuildInfoPanel",
            width: {panel_w},
            height: {panel_h},
            hidden,
            background_color: PANEL_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INSET},
                y: {panel_y},
            }
            fontstring {
                name: "GuildInfoMotdLabel",
                width: {panel_w - 8.0},
                height: 18.0,
                text: "Message of the Day",
                font_size: 11.0,
                font_color: TITLE_COLOR,
                justify_h: "LEFT",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "4", y: "-6" }
            }
            fontstring {
                name: "GuildInfoMotdText",
                width: {panel_w - 8.0},
                height: 36.0,
                text: {motd.as_str()},
                font_size: 10.0,
                font_color: TEXT,
                justify_h: "LEFT",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "4", y: "-26" }
            }
            fontstring {
                name: "GuildInfoInfoLabel",
                width: {panel_w - 8.0},
                height: 18.0,
                text: "Guild Info",
                font_size: 11.0,
                font_color: TITLE_COLOR,
                justify_h: "LEFT",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "4", y: "-78" }
            }
            fontstring {
                name: "GuildInfoText",
                width: {panel_w - 8.0},
                height: {panel_h - 96.0},
                text: {info_text.as_str()},
                font_size: 10.0,
                font_color: TEXT,
                justify_h: "LEFT",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "4", y: "-98" }
            }
        }
    }
}

#[cfg(test)]
#[path = "guild_frame_component_tests.rs"]
mod tests;
