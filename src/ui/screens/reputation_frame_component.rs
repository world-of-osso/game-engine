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

// --- Layout constants ---

pub const FRAME_W: f32 = 500.0;
pub const FRAME_H: f32 = 480.0;
const HEADER_H: f32 = 28.0;
const INSET: f32 = 8.0;
const CONTENT_TOP: f32 = HEADER_H + 4.0;
const LIST_W: f32 = FRAME_W - 2.0 * INSET;

const CAT_HEADER_H: f32 = 22.0;
const FACTION_ROW_H: f32 = 24.0;
const ROW_GAP: f32 = 2.0;
const FACTION_INDENT: f32 = 16.0;

const BAR_H: f32 = 12.0;
const BAR_W: f32 = 200.0;
const BAR_X: f32 = LIST_W - BAR_W - 8.0;

const STANDING_LABEL_W: f32 = 80.0;

const PARAGON_ICON_SIZE: f32 = 16.0;
const PARAGON_ICON_X: f32 = 2.0;

const TOOLTIP_W: f32 = 220.0;
const TOOLTIP_LINE_H: f32 = 16.0;
const TOOLTIP_INSET: f32 = 8.0;
const TOOLTIP_HEADER_H: f32 = 18.0;

// --- Colors ---

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const LIST_BG: &str = "0.0,0.0,0.0,0.3";
const CAT_HEADER_BG: &str = "0.12,0.10,0.06,0.9";
const CAT_HEADER_COLOR: &str = "1.0,0.82,0.0,1.0";
const FACTION_NAME_COLOR: &str = "1.0,1.0,1.0,1.0";
const BAR_BG: &str = "0.1,0.1,0.1,0.9";
const BAR_TEXT_COLOR: &str = "1.0,1.0,1.0,1.0";
const COLLAPSE_ICON_COLOR: &str = "0.8,0.8,0.8,1.0";
const PARAGON_ICON_BG: &str = "0.6,0.3,0.9,0.9";
const PARAGON_ICON_TEXT: &str = "1.0,0.82,0.0,1.0";
const TOOLTIP_BG: &str = "0.08,0.06,0.04,0.95";
const TOOLTIP_BORDER: &str = "0.3,0.25,0.15,0.9";
const TOOLTIP_HEADER_COLOR: &str = "1.0,0.82,0.0,1.0";
const TOOLTIP_TEXT_COLOR: &str = "0.85,0.85,0.85,1.0";

const STANDING_HATED: &str = "0.8,0.2,0.2,0.95";
const STANDING_HOSTILE: &str = "0.8,0.3,0.2,0.95";
const STANDING_UNFRIENDLY: &str = "0.7,0.4,0.2,0.95";
const STANDING_NEUTRAL: &str = "0.7,0.7,0.2,0.95";
const STANDING_FRIENDLY: &str = "0.2,0.7,0.2,0.95";
const STANDING_HONORED: &str = "0.2,0.7,0.4,0.95";
const STANDING_REVERED: &str = "0.2,0.5,0.8,0.95";
const STANDING_EXALTED: &str = "0.6,0.3,0.9,0.95";

// --- Data types ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Standing {
    Hated,
    Hostile,
    Unfriendly,
    #[default]
    Neutral,
    Friendly,
    Honored,
    Revered,
    Exalted,
}

impl Standing {
    pub fn label(self) -> &'static str {
        match self {
            Self::Hated => "Hated",
            Self::Hostile => "Hostile",
            Self::Unfriendly => "Unfriendly",
            Self::Neutral => "Neutral",
            Self::Friendly => "Friendly",
            Self::Honored => "Honored",
            Self::Revered => "Revered",
            Self::Exalted => "Exalted",
        }
    }

    pub fn bar_color(self) -> &'static str {
        match self {
            Self::Hated => STANDING_HATED,
            Self::Hostile => STANDING_HOSTILE,
            Self::Unfriendly => STANDING_UNFRIENDLY,
            Self::Neutral => STANDING_NEUTRAL,
            Self::Friendly => STANDING_FRIENDLY,
            Self::Honored => STANDING_HONORED,
            Self::Revered => STANDING_REVERED,
            Self::Exalted => STANDING_EXALTED,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ParagonProgress {
    pub current: u32,
    pub max: u32,
    pub reward_pending: bool,
}

impl ParagonProgress {
    pub fn fraction(&self) -> f32 {
        if self.max == 0 {
            return 0.0;
        }
        (self.current as f32 / self.max as f32).min(1.0)
    }

    pub fn progress_text(&self) -> String {
        format!("{}/{}", self.current, self.max)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FactionEntry {
    pub name: String,
    pub standing: Standing,
    pub current: u32,
    pub max: u32,
    pub paragon: Option<ParagonProgress>,
}

impl FactionEntry {
    pub fn progress_fraction(&self) -> f32 {
        if self.max == 0 {
            return if self.standing == Standing::Exalted {
                1.0
            } else {
                0.0
            };
        }
        (self.current as f32 / self.max as f32).min(1.0)
    }

    pub fn progress_text(&self) -> String {
        format!("{}/{}", self.current, self.max)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FactionCategory {
    pub name: String,
    pub collapsed: bool,
    pub factions: Vec<FactionEntry>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ReputationFrameState {
    pub visible: bool,
    pub categories: Vec<FactionCategory>,
    /// Index of hovered faction for tooltip: (category_idx, faction_idx).
    pub hovered_faction: Option<(usize, usize)>,
}

// --- Screen entry ---

pub fn reputation_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<ReputationFrameState>()
        .expect("ReputationFrameState must be in SharedContext");
    let hide = !state.visible;
    let tooltip = build_tooltip(state);
    rsx! {
        r#frame {
            name: "ReputationFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
                x: "0",
                y: "0",
            }
            {title_bar()}
            {faction_list(&state.categories)}
            {tooltip}
        }
    }
}

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "ReputationFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Reputation",
            font_size: 16.0,
            font_color: TITLE_COLOR,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                x: "0",
                y: "0",
            }
        }
    }
}

// --- Faction list ---

fn faction_list(categories: &[FactionCategory]) -> Element {
    let list_h = FRAME_H - CONTENT_TOP - INSET;
    let positions = category_positions(categories);
    let rows: Element = positions
        .iter()
        .flat_map(|&(ci, header_y, ref faction_positions)| {
            let cat = &categories[ci];
            let mut elems = category_header(ci, &cat.name, cat.collapsed, header_y);
            if !cat.collapsed {
                for &(fi, fy) in faction_positions {
                    elems.extend(faction_row(ci, fi, &cat.factions[fi], fy));
                }
            }
            elems
        })
        .collect();
    rsx! {
        r#frame {
            name: "ReputationList",
            width: {LIST_W},
            height: {list_h},
            background_color: LIST_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INSET},
                y: {-CONTENT_TOP},
            }
            {rows}
        }
    }
}

fn category_positions(cats: &[FactionCategory]) -> Vec<(usize, f32, Vec<(usize, f32)>)> {
    let mut y: f32 = 0.0;
    cats.iter()
        .enumerate()
        .map(|(ci, cat)| {
            let header_y = y;
            y += CAT_HEADER_H + ROW_GAP;
            let faction_pos = if cat.collapsed {
                vec![]
            } else {
                cat.factions
                    .iter()
                    .enumerate()
                    .map(|(fi, _)| {
                        let fy = y;
                        y += FACTION_ROW_H + ROW_GAP;
                        (fi, fy)
                    })
                    .collect()
            };
            (ci, header_y, faction_pos)
        })
        .collect()
}

fn category_header(idx: usize, name: &str, collapsed: bool, y: f32) -> Element {
    let id = DynName(format!("RepCat{idx}"));
    let icon_text = if collapsed { "▶" } else { "▼" };
    rsx! {
        r#frame {
            name: id,
            width: {LIST_W - 4.0},
            height: {CAT_HEADER_H},
            background_color: CAT_HEADER_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "2",
                y: {-y},
            }
            {cat_collapse_icon(DynName(format!("RepCat{idx}Icon")), icon_text)}
            {cat_header_label(DynName(format!("RepCat{idx}Label")), name)}
        }
    }
}

fn cat_collapse_icon(id: DynName, text: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: 14.0,
            height: {CAT_HEADER_H},
            text: text,
            font_size: 10.0,
            font_color: COLLAPSE_ICON_COLOR,
            justify_h: "CENTER",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "4", y: "0" }
        }
    }
}

fn cat_header_label(id: DynName, text: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {LIST_W - 26.0},
            height: {CAT_HEADER_H},
            text: text,
            font_size: 11.0,
            font_color: CAT_HEADER_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "20", y: "0" }
        }
    }
}

fn faction_row(cat_idx: usize, fac_idx: usize, faction: &FactionEntry, y: f32) -> Element {
    let row_id = DynName(format!("RepFaction{cat_idx}_{fac_idx}"));
    let name_id = DynName(format!("RepFaction{cat_idx}_{fac_idx}Name"));
    rsx! {
        r#frame {
            name: row_id,
            width: {LIST_W - 4.0},
            height: {FACTION_ROW_H},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "2",
                y: {-y},
            }
            fontstring {
                name: name_id,
                width: {BAR_X - FACTION_INDENT - 4.0},
                height: {FACTION_ROW_H},
                text: {faction.name.as_str()},
                font_size: 10.0,
                font_color: FACTION_NAME_COLOR,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {FACTION_INDENT},
                    y: "0",
                }
            }
            {reputation_bar(cat_idx, fac_idx, faction)}
            {paragon_indicator(cat_idx, fac_idx, faction.paragon.as_ref())}
        }
    }
}

fn rep_bar_fill(id: DynName, w: f32, color: &str) -> Element {
    rsx! {
        r#frame {
            name: id,
            width: {w},
            height: {BAR_H},
            background_color: color,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "0", y: "0" }
        }
    }
}

fn rep_bar_text(id: DynName, text: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {BAR_W},
            height: {BAR_H},
            text: text,
            font_size: 8.0,
            font_color: BAR_TEXT_COLOR,
            justify_h: "CENTER",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "0", y: "0" }
        }
    }
}

fn rep_standing_label(id: DynName, standing: Standing) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {STANDING_LABEL_W},
            height: {FACTION_ROW_H},
            text: {standing.label()},
            font_size: 9.0,
            font_color: {standing.bar_color()},
            justify_h: "RIGHT",
            anchor { point: AnchorPoint::TopRight, relative_point: AnchorPoint::TopRight, x: "-4", y: "0" }
        }
    }
}

fn reputation_bar(cat_idx: usize, fac_idx: usize, faction: &FactionEntry) -> Element {
    let bar_id = DynName(format!("RepBar{cat_idx}_{fac_idx}"));
    let fill_w = faction.progress_fraction() * BAR_W;
    let bar_y = (FACTION_ROW_H - BAR_H) / 2.0;
    let progress = faction.progress_text();
    rsx! {
        r#frame {
            name: bar_id,
            width: {BAR_W},
            height: {BAR_H},
            background_color: BAR_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {BAR_X},
                y: {-bar_y},
            }
            {rep_bar_fill(DynName(format!("RepBar{cat_idx}_{fac_idx}Fill")), fill_w, faction.standing.bar_color())}
            {rep_bar_text(DynName(format!("RepBar{cat_idx}_{fac_idx}Text")), &progress)}
        }
        {rep_standing_label(DynName(format!("RepBar{cat_idx}_{fac_idx}Standing")), faction.standing)}
    }
}

// --- Paragon reward indicator ---

fn paragon_indicator(cat_idx: usize, fac_idx: usize, paragon: Option<&ParagonProgress>) -> Element {
    let id = DynName(format!("RepParagon{cat_idx}_{fac_idx}"));
    let label_id = DynName(format!("RepParagon{cat_idx}_{fac_idx}Label"));
    let hide = paragon.is_none();
    let reward_pending = paragon.is_some_and(|p| p.reward_pending);
    let icon_text = if reward_pending { "★" } else { "◆" };
    rsx! {
        r#frame {
            name: id,
            width: {PARAGON_ICON_SIZE},
            height: {PARAGON_ICON_SIZE},
            hidden: hide,
            background_color: PARAGON_ICON_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {PARAGON_ICON_X},
                y: {-(FACTION_ROW_H - PARAGON_ICON_SIZE) / 2.0},
            }
            fontstring {
                name: label_id,
                width: {PARAGON_ICON_SIZE},
                height: {PARAGON_ICON_SIZE},
                text: icon_text,
                font_size: 10.0,
                font_color: PARAGON_ICON_TEXT,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
    }
}

// --- Reputation detail tooltip ---

fn build_tooltip(state: &ReputationFrameState) -> Element {
    let hide = state.hovered_faction.is_none();
    let (content, tooltip_h) = match state.hovered_faction {
        Some((ci, fi)) => {
            let faction = &state.categories[ci].factions[fi];
            tooltip_content(faction)
        }
        None => (
            rsx! {},
            TOOLTIP_HEADER_H + 2.0 * TOOLTIP_LINE_H + 2.0 * TOOLTIP_INSET,
        ),
    };
    rsx! {
        r#frame {
            name: "RepTooltip",
            width: {TOOLTIP_W},
            height: {tooltip_h},
            hidden: hide,
            background_color: TOOLTIP_BG,
            anchor {
                point: AnchorPoint::TopRight,
                relative_point: AnchorPoint::TopLeft,
                x: "4",
                y: "0",
            }
            {tooltip_border()}
            {content}
        }
    }
}

fn tooltip_border() -> Element {
    rsx! {
        r#frame {
            name: "RepTooltipBorder",
            width: {TOOLTIP_W},
            height: "1",
            background_color: TOOLTIP_BORDER,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: "0",
            }
        }
    }
}

fn rep_tooltip_line(
    name: &str,
    text: &str,
    h: f32,
    font_size: f32,
    color: &str,
    y: f32,
) -> Element {
    rsx! {
        fontstring {
            name: DynName(name.into()),
            width: {TOOLTIP_W - 2.0 * TOOLTIP_INSET},
            height: {h},
            text: text,
            font_size: font_size,
            font_color: color,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {TOOLTIP_INSET}, y: {y} }
        }
    }
}

fn tooltip_content(faction: &FactionEntry) -> (Element, f32) {
    let has_paragon = faction.paragon.is_some();
    let line_count = if has_paragon { 3u32 } else { 2 };
    let h = TOOLTIP_INSET * 2.0 + TOOLTIP_HEADER_H + line_count as f32 * TOOLTIP_LINE_H;
    let standing_text = format!("Standing: {}", faction.standing.label());
    let progress_text = format!("Progress: {}", faction.progress_text());
    let paragon_line = faction
        .paragon
        .as_ref()
        .map(|p| format!("Paragon: {}", p.progress_text()))
        .unwrap_or_default();
    let hide_paragon = !has_paragon;
    let paragon_y = TOOLTIP_INSET + TOOLTIP_HEADER_H + 2.0 * TOOLTIP_LINE_H;
    let elems = rsx! {
        {rep_tooltip_line("RepTooltipTitle", &faction.name, TOOLTIP_HEADER_H, 12.0, TOOLTIP_HEADER_COLOR, -TOOLTIP_INSET)}
        {rep_tooltip_line("RepTooltipStanding", &standing_text, TOOLTIP_LINE_H, 10.0, faction.standing.bar_color(), -(TOOLTIP_INSET + TOOLTIP_HEADER_H))}
        {rep_tooltip_line("RepTooltipProgress", &progress_text, TOOLTIP_LINE_H, 10.0, TOOLTIP_TEXT_COLOR, -(TOOLTIP_INSET + TOOLTIP_HEADER_H + TOOLTIP_LINE_H))}
        fontstring {
            name: "RepTooltipParagon",
            width: {TOOLTIP_W - 2.0 * TOOLTIP_INSET},
            height: {TOOLTIP_LINE_H},
            hidden: hide_paragon,
            text: {paragon_line.as_str()},
            font_size: 10.0,
            font_color: TOOLTIP_TEXT_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {TOOLTIP_INSET}, y: {-paragon_y} }
        }
    };
    (elems, h)
}

#[cfg(test)]
#[cfg(test)]
#[path = "reputation_frame_component_tests.rs"]
mod tests;
