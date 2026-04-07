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

pub const FRAME_W: f32 = 504.0;
pub const FRAME_H: f32 = 480.0;
const HEADER_H: f32 = 30.0;
const TAB_H: f32 = 28.0;
const TAB_GAP: f32 = 4.0;
const TAB_INSET: f32 = 12.0;
const SIDEBAR_W: f32 = 175.0;
const SIDEBAR_INSET: f32 = 8.0;
const SIDEBAR_TOP: f32 = HEADER_H + TAB_GAP + TAB_H + TAB_GAP;
const CAT_ROW_H: f32 = 20.0;
const CAT_ROW_GAP: f32 = 2.0;
const CAT_INDENT: f32 = 16.0;
const CONTENT_INSET: f32 = 4.0;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const TAB_BG_ACTIVE: &str = "0.2,0.15,0.05,0.95";
const TAB_BG_INACTIVE: &str = "0.08,0.07,0.06,0.88";
const TAB_TEXT_ACTIVE: &str = "1.0,0.82,0.0,1.0";
const TAB_TEXT_INACTIVE: &str = "0.6,0.6,0.6,1.0";
const SIDEBAR_BG: &str = "0.0,0.0,0.0,0.4";
const CAT_BG_SELECTED: &str = "0.2,0.15,0.05,0.95";
const CAT_BG_NORMAL: &str = "0.0,0.0,0.0,0.0";
const CAT_TEXT_SELECTED: &str = "1.0,0.82,0.0,1.0";
const CAT_TEXT_NORMAL: &str = "1.0,1.0,1.0,1.0";
const CAT_TEXT_HEADER: &str = "0.8,0.8,0.8,1.0";
const CONTENT_BG: &str = "0.0,0.0,0.0,0.3";
const CONTENT_PLACEHOLDER_COLOR: &str = "0.5,0.5,0.5,1.0";
const ROW_H: f32 = 56.0;
const ROW_GAP: f32 = 2.0;
const ROW_INSET: f32 = 4.0;
const ICON_SIZE: f32 = 40.0;
const ICON_INSET: f32 = 6.0;
const PROGRESS_BAR_H: f32 = 12.0;
const PROGRESS_BAR_W: f32 = 160.0;
const CHECK_SIZE: f32 = 16.0;
const ROW_BG_COMPLETE: &str = "0.08,0.12,0.06,0.85";
const ROW_BG_INCOMPLETE: &str = "0.05,0.05,0.05,0.75";
const ICON_BG: &str = "0.15,0.12,0.02,0.95";
const NAME_COLOR: &str = "1.0,1.0,1.0,1.0";
const DESC_COLOR: &str = "0.7,0.7,0.7,1.0";
const POINTS_COLOR: &str = "1.0,0.82,0.0,1.0";
const PROGRESS_BG: &str = "0.1,0.1,0.1,0.9";
const PROGRESS_FILL: &str = "0.2,0.6,0.1,0.9";
const PROGRESS_TEXT_COLOR: &str = "1.0,1.0,1.0,1.0";
const CHECK_COMPLETE: &str = "0.0,1.0,0.0,1.0";
const CHECK_INCOMPLETE: &str = "0.3,0.3,0.3,0.6";

pub const MAX_VISIBLE_CATEGORIES: usize = 16;
pub const MAX_VISIBLE_ACHIEVEMENTS: usize = 6;

#[derive(Clone, Debug, PartialEq)]
pub struct AchievementCategory {
    pub name: String,
    pub is_child: bool,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AchievementTab {
    pub name: String,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AchievementRow {
    pub name: String,
    pub description: String,
    pub points: u32,
    pub icon_fdid: u32,
    pub completed: bool,
    /// Progress as 0.0..=1.0 fraction.
    pub progress: f32,
    /// Display text for progress bar, e.g. "3 / 5".
    pub progress_text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AchievementFrameState {
    pub visible: bool,
    pub tabs: Vec<AchievementTab>,
    pub categories: Vec<AchievementCategory>,
    pub achievements: Vec<AchievementRow>,
    pub total_points: u32,
}

impl AchievementRow {
    /// Returns the progress percentage as 0–100.
    pub fn progress_pct(&self) -> u32 {
        (self.progress.clamp(0.0, 1.0) * 100.0).round() as u32
    }
}

impl AchievementFrameState {
    /// Toggle the completion state of an achievement by index.
    pub fn toggle_completion(&mut self, index: usize) {
        if let Some(row) = self.achievements.get_mut(index) {
            row.completed = !row.completed;
            if row.completed {
                row.progress = 1.0;
            }
        }
    }

    /// Percentage of achievements completed (0–100).
    pub fn completion_pct(&self) -> u32 {
        if self.achievements.is_empty() {
            return 0;
        }
        let done = self.achievements.iter().filter(|a| a.completed).count();
        ((done as f32 / self.achievements.len() as f32) * 100.0).round() as u32
    }

    /// Filter achievements by category name, returning only those whose name
    /// contains the query (case-insensitive).
    pub fn filter_by_name(&self, query: &str) -> Vec<&AchievementRow> {
        let q = query.to_lowercase();
        self.achievements
            .iter()
            .filter(|a| a.name.to_lowercase().contains(&q))
            .collect()
    }

    /// Select a category by index, deselecting all others.
    pub fn select_category(&mut self, index: usize) {
        for (i, cat) in self.categories.iter_mut().enumerate() {
            cat.selected = i == index;
        }
    }
}

pub fn achievement_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<AchievementFrameState>()
        .expect("AchievementFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "AchievementFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "370",
                y: "-80",
            }
            {title_bar(state.total_points)}
            {tab_row(&state.tabs)}
            {category_sidebar(&state.categories)}
            {content_area(&state.achievements)}
        }
    }
}

fn title_bar(total_points: u32) -> Element {
    let text = format!("Achievements ({total_points})");
    rsx! {
        fontstring {
            name: "AchievementFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: {text.as_str()},
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

fn tab_row(tabs: &[AchievementTab]) -> Element {
    let count = tabs.len().max(1) as f32;
    let tab_w = (FRAME_W - 2.0 * TAB_INSET - (count - 1.0) * TAB_GAP) / count;
    tabs.iter()
        .enumerate()
        .flat_map(|(i, tab)| {
            let x = TAB_INSET + i as f32 * (tab_w + TAB_GAP);
            let y = -(HEADER_H + TAB_GAP);
            tab_button(i, tab, tab_w, x, y)
        })
        .collect()
}

fn tab_button(i: usize, tab: &AchievementTab, tab_w: f32, x: f32, y: f32) -> Element {
    let tab_id = DynName(format!("AchievementTab{i}"));
    let label_id = DynName(format!("AchievementTab{i}Label"));
    let (bg, color) = if tab.active {
        (TAB_BG_ACTIVE, TAB_TEXT_ACTIVE)
    } else {
        (TAB_BG_INACTIVE, TAB_TEXT_INACTIVE)
    };
    rsx! {
        r#frame {
            name: tab_id,
            width: {tab_w},
            height: {TAB_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            {tab_label(label_id, &tab.name, tab_w, color)}
        }
    }
}

fn tab_label(id: DynName, text: &str, w: f32, color: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {w},
            height: {TAB_H},
            text: text,
            font_size: 11.0,
            font_color: color,
            justify_h: "CENTER",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
        }
    }
}

fn category_sidebar(categories: &[AchievementCategory]) -> Element {
    let sidebar_h = FRAME_H - SIDEBAR_TOP - SIDEBAR_INSET;
    let sidebar_y = -SIDEBAR_TOP;
    let cats: Element = categories
        .iter()
        .enumerate()
        .take(MAX_VISIBLE_CATEGORIES)
        .flat_map(|(i, cat)| {
            let row_y = -(i as f32 * (CAT_ROW_H + CAT_ROW_GAP));
            category_row(i, cat, row_y)
        })
        .collect();
    rsx! {
        r#frame {
            name: "AchievementCategorySidebar",
            width: {SIDEBAR_W},
            height: {sidebar_h},
            background_color: SIDEBAR_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SIDEBAR_INSET},
                y: {sidebar_y},
            }
            {cats}
        }
    }
}

fn category_row(idx: usize, cat: &AchievementCategory, y: f32) -> Element {
    let row_id = DynName(format!("AchievementCat{idx}"));
    let label_id = DynName(format!("AchievementCat{idx}Label"));
    let x_offset = if cat.is_child { CAT_INDENT } else { 4.0 };
    let label_w = SIDEBAR_W - x_offset - 4.0;
    let bg = if cat.selected {
        CAT_BG_SELECTED
    } else {
        CAT_BG_NORMAL
    };
    let color = category_text_color(cat);
    rsx! {
        r#frame {
            name: row_id,
            width: {SIDEBAR_W},
            height: {CAT_ROW_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: {y},
            }
            {category_row_label(label_id, &cat.name, label_w, x_offset, color)}
        }
    }
}

fn category_text_color(cat: &AchievementCategory) -> &'static str {
    if cat.selected {
        CAT_TEXT_SELECTED
    } else if cat.is_child {
        CAT_TEXT_NORMAL
    } else {
        CAT_TEXT_HEADER
    }
}

fn category_row_label(id: DynName, text: &str, w: f32, x: f32, color: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {w},
            height: {CAT_ROW_H},
            text: text,
            font_size: 10.0,
            font_color: color,
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

fn build_achievement_rows(achievements: &[AchievementRow], row_w: f32) -> Element {
    achievements
        .iter()
        .enumerate()
        .take(MAX_VISIBLE_ACHIEVEMENTS)
        .flat_map(|(i, row)| {
            achievement_row(i, row, row_w, -(ROW_INSET + i as f32 * (ROW_H + ROW_GAP)))
        })
        .collect()
}

fn content_area(achievements: &[AchievementRow]) -> Element {
    let content_x = SIDEBAR_INSET + SIDEBAR_W + CONTENT_INSET;
    let content_y = -SIDEBAR_TOP;
    let content_w = FRAME_W - content_x - SIDEBAR_INSET;
    let content_h = FRAME_H - SIDEBAR_TOP - SIDEBAR_INSET;
    let has_rows = !achievements.is_empty();
    let rows = build_achievement_rows(achievements, content_w - 2.0 * ROW_INSET);
    rsx! {
        r#frame {
            name: "AchievementContentArea",
            width: {content_w},
            height: {content_h},
            background_color: CONTENT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {content_x},
                y: {content_y},
            }
            {content_placeholder(content_w, has_rows)}
            {rows}
        }
    }
}

fn content_placeholder(w: f32, has_rows: bool) -> Element {
    rsx! {
        fontstring {
            name: "AchievementContentPlaceholder",
            width: {w},
            height: 20.0,
            text: "Select a category",
            font_size: 11.0,
            font_color: CONTENT_PLACEHOLDER_COLOR,
            hidden: has_rows,
            justify_h: "CENTER",
            anchor { point: AnchorPoint::Top, relative_point: AnchorPoint::Top, x: "0", y: "-20" }
        }
    }
}

fn achievement_row(idx: usize, row: &AchievementRow, row_w: f32, y: f32) -> Element {
    let row_id = DynName(format!("AchievementRow{idx}"));
    let bg = if row.completed {
        ROW_BG_COMPLETE
    } else {
        ROW_BG_INCOMPLETE
    };
    let text_x = ICON_INSET + ICON_SIZE + ICON_INSET;
    let text_w = row_w - text_x - CHECK_SIZE - ICON_INSET;
    rsx! {
        r#frame {
            name: row_id,
            width: {row_w},
            height: {ROW_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {ROW_INSET},
                y: {y},
            }
            {row_icon(idx, row.icon_fdid)}
            {row_name(idx, &row.name, text_x, text_w)}
            {row_description(idx, &row.description, text_x, text_w)}
            {row_progress_bar(idx, row, text_x)}
            {row_points(idx, row.points, row_w)}
            {row_checkmark(idx, row.completed, row_w)}
        }
    }
}

fn row_icon(idx: usize, icon_fdid: u32) -> Element {
    let icon_id = DynName(format!("AchievementRow{idx}Icon"));
    let tex_id = DynName(format!("AchievementRow{idx}IconTex"));
    let icon_y = -((ROW_H - ICON_SIZE) / 2.0);
    rsx! {
        r#frame {
            name: icon_id,
            width: {ICON_SIZE},
            height: {ICON_SIZE},
            background_color: ICON_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {ICON_INSET},
                y: {icon_y},
            }
            texture {
                name: tex_id,
                width: {ICON_SIZE},
                height: {ICON_SIZE},
                texture_fdid: {icon_fdid},
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
        }
    }
}

fn row_name(idx: usize, name: &str, x: f32, w: f32) -> Element {
    let name_id = DynName(format!("AchievementRow{idx}Name"));
    rsx! {
        fontstring {
            name: name_id,
            width: {w},
            height: 16.0,
            text: name,
            font_size: 11.0,
            font_color: NAME_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: "-4",
            }
        }
    }
}

fn row_description(idx: usize, desc: &str, x: f32, w: f32) -> Element {
    let desc_id = DynName(format!("AchievementRow{idx}Desc"));
    rsx! {
        fontstring {
            name: desc_id,
            width: {w},
            height: 14.0,
            text: desc,
            font_size: 9.0,
            font_color: DESC_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: "-20",
            }
        }
    }
}

fn row_progress_bar(idx: usize, row: &AchievementRow, x: f32) -> Element {
    let bar_id = DynName(format!("AchievementRow{idx}ProgressBg"));
    let fill_id = DynName(format!("AchievementRow{idx}ProgressFill"));
    let text_id = DynName(format!("AchievementRow{idx}ProgressText"));
    let fill_w = PROGRESS_BAR_W * row.progress.clamp(0.0, 1.0);
    rsx! {
        r#frame {
            name: bar_id,
            width: {PROGRESS_BAR_W},
            height: {PROGRESS_BAR_H},
            background_color: PROGRESS_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {-(ROW_H - PROGRESS_BAR_H - 4.0)},
            }
            {progress_fill(fill_id, fill_w)}
            {progress_text(text_id, &row.progress_text)}
        }
    }
}

fn progress_fill(id: DynName, w: f32) -> Element {
    rsx! {
        r#frame {
            name: id,
            width: {w},
            height: {PROGRESS_BAR_H},
            background_color: PROGRESS_FILL,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
        }
    }
}

fn progress_text(id: DynName, text: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {PROGRESS_BAR_W},
            height: {PROGRESS_BAR_H},
            text: text,
            font_size: 8.0,
            font_color: PROGRESS_TEXT_COLOR,
            justify_h: "CENTER",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
        }
    }
}

fn row_points(idx: usize, points: u32, row_w: f32) -> Element {
    let pts_id = DynName(format!("AchievementRow{idx}Points"));
    let text = format!("{points}");
    rsx! {
        fontstring {
            name: pts_id,
            width: {CHECK_SIZE + ICON_INSET},
            height: 14.0,
            text: {text.as_str()},
            font_size: 9.0,
            font_color: POINTS_COLOR,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {row_w - CHECK_SIZE - ICON_INSET},
                y: "-4",
            }
        }
    }
}

fn row_checkmark(idx: usize, completed: bool, row_w: f32) -> Element {
    let check_id = DynName(format!("AchievementRow{idx}Check"));
    let color = if completed {
        CHECK_COMPLETE
    } else {
        CHECK_INCOMPLETE
    };
    let text = if completed { "\u{2713}" } else { "\u{25CB}" };
    rsx! {
        fontstring {
            name: check_id,
            width: {CHECK_SIZE},
            height: {CHECK_SIZE},
            text: text,
            font_size: 14.0,
            font_color: color,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {row_w - CHECK_SIZE - 2.0},
                y: {-((ROW_H - CHECK_SIZE) / 2.0)},
            }
        }
    }
}

#[cfg(test)]
#[path = "achievement_frame_component_tests.rs"]
mod tests;
