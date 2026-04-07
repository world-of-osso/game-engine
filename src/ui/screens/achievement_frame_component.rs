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

fn content_area(achievements: &[AchievementRow]) -> Element {
    let content_x = SIDEBAR_INSET + SIDEBAR_W + CONTENT_INSET;
    let content_y = -SIDEBAR_TOP;
    let content_w = FRAME_W - content_x - SIDEBAR_INSET;
    let content_h = FRAME_H - SIDEBAR_TOP - SIDEBAR_INSET;
    let has_rows = !achievements.is_empty();
    let rows: Element = achievements
        .iter()
        .enumerate()
        .take(MAX_VISIBLE_ACHIEVEMENTS)
        .flat_map(|(i, row)| {
            let row_y = -(ROW_INSET + i as f32 * (ROW_H + ROW_GAP));
            achievement_row(i, row, content_w - 2.0 * ROW_INSET, row_y)
        })
        .collect();
    let placeholder_hidden = has_rows;
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
            fontstring {
                name: "AchievementContentPlaceholder",
                width: {content_w},
                height: 20.0,
                text: "Select a category",
                font_size: 11.0,
                font_color: CONTENT_PLACEHOLDER_COLOR,
                hidden: placeholder_hidden,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::Top,
                    relative_point: AnchorPoint::Top,
                    x: "0",
                    y: "-20",
                }
            }
            {rows}
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
            r#frame {
                name: fill_id,
                width: {fill_w},
                height: {PROGRESS_BAR_H},
                background_color: PROGRESS_FILL,
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
            fontstring {
                name: text_id,
                width: {PROGRESS_BAR_W},
                height: {PROGRESS_BAR_H},
                text: {row.progress_text.as_str()},
                font_size: 8.0,
                font_color: PROGRESS_TEXT_COLOR,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
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
mod tests {
    use super::*;
    use ui_toolkit::layout::{LayoutRect, recompute_layouts};
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn default_categories() -> Vec<AchievementCategory> {
        vec![
            AchievementCategory {
                name: "General".into(),
                is_child: false,
                selected: true,
            },
            AchievementCategory {
                name: "Quests".into(),
                is_child: false,
                selected: false,
            },
            AchievementCategory {
                name: "Exploration".into(),
                is_child: false,
                selected: false,
            },
            AchievementCategory {
                name: "PvP".into(),
                is_child: false,
                selected: false,
            },
            AchievementCategory {
                name: "Dungeons & Raids".into(),
                is_child: false,
                selected: false,
            },
            AchievementCategory {
                name: "Professions".into(),
                is_child: false,
                selected: false,
            },
            AchievementCategory {
                name: "Reputation".into(),
                is_child: false,
                selected: false,
            },
            AchievementCategory {
                name: "World Events".into(),
                is_child: false,
                selected: false,
            },
            AchievementCategory {
                name: "Feats of Strength".into(),
                is_child: false,
                selected: false,
            },
        ]
    }

    fn sample_achievements() -> Vec<AchievementRow> {
        vec![
            AchievementRow {
                name: "Level 10".into(),
                description: "Reach level 10.".into(),
                points: 10,
                icon_fdid: 236562,
                completed: true,
                progress: 1.0,
                progress_text: "10 / 10".into(),
            },
            AchievementRow {
                name: "Level 20".into(),
                description: "Reach level 20.".into(),
                points: 10,
                icon_fdid: 236563,
                completed: false,
                progress: 0.75,
                progress_text: "15 / 20".into(),
            },
            AchievementRow {
                name: "Level 40".into(),
                description: "Reach level 40.".into(),
                points: 10,
                icon_fdid: 236565,
                completed: false,
                progress: 0.0,
                progress_text: "0 / 40".into(),
            },
        ]
    }

    fn make_test_state() -> AchievementFrameState {
        AchievementFrameState {
            visible: true,
            tabs: vec![
                AchievementTab {
                    name: "Achievements".into(),
                    active: true,
                },
                AchievementTab {
                    name: "Statistics".into(),
                    active: false,
                },
            ],
            categories: default_categories(),
            achievements: sample_achievements(),
            total_points: 0,
        }
    }

    #[test]
    fn achievement_frame_builds_expected_frames() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        let mut screen = Screen::new(achievement_frame_screen);
        screen.sync(&shared, &mut registry);

        assert!(registry.get_by_name("AchievementFrame").is_some());
        assert!(registry.get_by_name("AchievementFrameTitle").is_some());
        assert!(registry.get_by_name("AchievementCategorySidebar").is_some());
        assert!(registry.get_by_name("AchievementContentArea").is_some());
        assert!(
            registry
                .get_by_name("AchievementContentPlaceholder")
                .is_some()
        );
    }

    #[test]
    fn achievement_frame_builds_tabs() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(achievement_frame_screen).sync(&shared, &mut registry);

        assert!(registry.get_by_name("AchievementTab0").is_some());
        assert!(registry.get_by_name("AchievementTab1").is_some());
        assert!(registry.get_by_name("AchievementTab0Label").is_some());
        assert!(registry.get_by_name("AchievementTab1Label").is_some());
    }

    #[test]
    fn achievement_frame_builds_category_rows() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(achievement_frame_screen).sync(&shared, &mut registry);

        for i in 0..9 {
            assert!(
                registry
                    .get_by_name(&format!("AchievementCat{i}"))
                    .is_some(),
                "AchievementCat{i} missing"
            );
            assert!(
                registry
                    .get_by_name(&format!("AchievementCat{i}Label"))
                    .is_some(),
                "AchievementCat{i}Label missing"
            );
        }
    }

    #[test]
    fn achievement_frame_hidden_when_not_visible() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        let mut state = make_test_state();
        state.visible = false;
        shared.insert(state);
        Screen::new(achievement_frame_screen).sync(&shared, &mut registry);

        let frame_id = registry
            .get_by_name("AchievementFrame")
            .expect("AchievementFrame");
        let frame = registry.get(frame_id).expect("frame data");
        assert!(frame.hidden, "frame should be hidden when visible=false");
    }

    #[test]
    fn achievement_frame_child_categories_indented() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        let mut state = make_test_state();
        state.categories.insert(
            1,
            AchievementCategory {
                name: "Level".into(),
                is_child: true,
                selected: false,
            },
        );
        shared.insert(state);
        Screen::new(achievement_frame_screen).sync(&shared, &mut registry);

        assert!(registry.get_by_name("AchievementCat1").is_some());
        assert!(registry.get_by_name("AchievementCat1Label").is_some());
    }

    #[test]
    fn achievement_frame_builds_achievement_rows() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(achievement_frame_screen).sync(&shared, &mut registry);

        for i in 0..3 {
            assert!(
                registry
                    .get_by_name(&format!("AchievementRow{i}"))
                    .is_some(),
                "AchievementRow{i} missing"
            );
            assert!(
                registry
                    .get_by_name(&format!("AchievementRow{i}Icon"))
                    .is_some(),
                "AchievementRow{i}Icon missing"
            );
            assert!(
                registry
                    .get_by_name(&format!("AchievementRow{i}Name"))
                    .is_some(),
                "AchievementRow{i}Name missing"
            );
            assert!(
                registry
                    .get_by_name(&format!("AchievementRow{i}Desc"))
                    .is_some(),
                "AchievementRow{i}Desc missing"
            );
            assert!(
                registry
                    .get_by_name(&format!("AchievementRow{i}ProgressBg"))
                    .is_some(),
                "AchievementRow{i}ProgressBg missing"
            );
            assert!(
                registry
                    .get_by_name(&format!("AchievementRow{i}ProgressFill"))
                    .is_some(),
                "AchievementRow{i}ProgressFill missing"
            );
            assert!(
                registry
                    .get_by_name(&format!("AchievementRow{i}Check"))
                    .is_some(),
                "AchievementRow{i}Check missing"
            );
            assert!(
                registry
                    .get_by_name(&format!("AchievementRow{i}Points"))
                    .is_some(),
                "AchievementRow{i}Points missing"
            );
        }
    }

    #[test]
    fn achievement_row_icons_have_texture_frames() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(achievement_frame_screen).sync(&shared, &mut registry);

        for i in 0..3 {
            assert!(
                registry
                    .get_by_name(&format!("AchievementRow{i}IconTex"))
                    .is_some(),
                "AchievementRow{i}IconTex missing"
            );
        }
    }

    #[test]
    fn achievement_frame_empty_shows_placeholder() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        let mut state = make_test_state();
        state.achievements.clear();
        shared.insert(state);
        Screen::new(achievement_frame_screen).sync(&shared, &mut registry);

        let ph_id = registry
            .get_by_name("AchievementContentPlaceholder")
            .expect("placeholder");
        let ph = registry.get(ph_id).expect("frame data");
        assert!(
            !ph.hidden,
            "placeholder should be visible when no achievements"
        );
    }

    #[test]
    fn achievement_frame_with_rows_hides_placeholder() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(achievement_frame_screen).sync(&shared, &mut registry);

        let ph_id = registry
            .get_by_name("AchievementContentPlaceholder")
            .expect("placeholder");
        let ph = registry.get(ph_id).expect("frame data");
        assert!(
            ph.hidden,
            "placeholder should be hidden when achievements present"
        );
    }

    #[test]
    fn achievement_row_progress_fill_width_matches_fraction() {
        use ui_toolkit::frame::Dimension;

        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(achievement_frame_screen).sync(&shared, &mut registry);

        // Row 0 is completed (progress=1.0), fill should be full width
        let fill_id = registry
            .get_by_name("AchievementRow0ProgressFill")
            .expect("fill");
        let fill = registry.get(fill_id).expect("frame data");
        assert_eq!(fill.width, Dimension::Fixed(PROGRESS_BAR_W));

        // Row 1 has progress=0.75
        let fill_id = registry
            .get_by_name("AchievementRow1ProgressFill")
            .expect("fill");
        let fill = registry.get(fill_id).expect("frame data");
        assert_eq!(fill.width, Dimension::Fixed(PROGRESS_BAR_W * 0.75));
    }

    // --- Coord validation helpers ---

    fn layout_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(achievement_frame_screen).sync(&shared, &mut reg);
        recompute_layouts(&mut reg);
        reg
    }

    fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
        reg.get(reg.get_by_name(name).expect(name))
            .and_then(|f| f.layout_rect.clone())
            .unwrap_or_else(|| panic!("{name} has no layout_rect"))
    }

    fn assert_rect(reg: &FrameRegistry, name: &str, expected: LayoutRect) {
        let actual = rect(reg, name);
        let ok = (actual.x - expected.x).abs() < 1.0
            && (actual.y - expected.y).abs() < 1.0
            && (actual.width - expected.width).abs() < 1.0
            && (actual.height - expected.height).abs() < 1.0;
        assert!(
            ok,
            "{name} rect mismatch:\n  expected: ({}, {}, {}×{})\n  actual:   ({}, {}, {}×{})",
            expected.x,
            expected.y,
            expected.width,
            expected.height,
            actual.x,
            actual.y,
            actual.width,
            actual.height,
        );
    }

    // --- Coord validation tests ---

    const FRAME_X: f32 = 370.0;
    const FRAME_Y: f32 = 80.0;

    #[test]
    fn coord_main_frame() {
        let reg = layout_registry();
        assert_rect(
            &reg,
            "AchievementFrame",
            LayoutRect {
                x: FRAME_X,
                y: FRAME_Y,
                width: FRAME_W,
                height: FRAME_H,
            },
        );
    }

    #[test]
    fn coord_title() {
        let reg = layout_registry();
        assert_rect(
            &reg,
            "AchievementFrameTitle",
            LayoutRect {
                x: FRAME_X,
                y: FRAME_Y,
                width: FRAME_W,
                height: HEADER_H,
            },
        );
    }

    #[test]
    fn coord_tabs() {
        let reg = layout_registry();
        let tab_count = 2.0_f32;
        let tab_w = (FRAME_W - 2.0 * TAB_INSET - (tab_count - 1.0) * TAB_GAP) / tab_count;
        let tab_y = FRAME_Y + HEADER_H + TAB_GAP;
        assert_rect(
            &reg,
            "AchievementTab0",
            LayoutRect {
                x: FRAME_X + TAB_INSET,
                y: tab_y,
                width: tab_w,
                height: TAB_H,
            },
        );
        assert_rect(
            &reg,
            "AchievementTab1",
            LayoutRect {
                x: FRAME_X + TAB_INSET + tab_w + TAB_GAP,
                y: tab_y,
                width: tab_w,
                height: TAB_H,
            },
        );
    }

    #[test]
    fn coord_sidebar() {
        let reg = layout_registry();
        let sidebar_y = FRAME_Y + SIDEBAR_TOP;
        let sidebar_h = FRAME_H - SIDEBAR_TOP - SIDEBAR_INSET;
        assert_rect(
            &reg,
            "AchievementCategorySidebar",
            LayoutRect {
                x: FRAME_X + SIDEBAR_INSET,
                y: sidebar_y,
                width: SIDEBAR_W,
                height: sidebar_h,
            },
        );
    }

    #[test]
    fn coord_content_area() {
        let reg = layout_registry();
        let content_x = FRAME_X + SIDEBAR_INSET + SIDEBAR_W + CONTENT_INSET;
        let content_y = FRAME_Y + SIDEBAR_TOP;
        let content_w = FRAME_W - (SIDEBAR_INSET + SIDEBAR_W + CONTENT_INSET) - SIDEBAR_INSET;
        let content_h = FRAME_H - SIDEBAR_TOP - SIDEBAR_INSET;
        assert_rect(
            &reg,
            "AchievementContentArea",
            LayoutRect {
                x: content_x,
                y: content_y,
                width: content_w,
                height: content_h,
            },
        );
    }

    #[test]
    fn coord_first_category_row() {
        let reg = layout_registry();
        let sidebar_x = FRAME_X + SIDEBAR_INSET;
        let sidebar_y = FRAME_Y + SIDEBAR_TOP;
        assert_rect(
            &reg,
            "AchievementCat0",
            LayoutRect {
                x: sidebar_x,
                y: sidebar_y,
                width: SIDEBAR_W,
                height: CAT_ROW_H,
            },
        );
    }

    #[test]
    fn coord_first_achievement_row() {
        let reg = layout_registry();
        let content_x = FRAME_X + SIDEBAR_INSET + SIDEBAR_W + CONTENT_INSET;
        let content_y = FRAME_Y + SIDEBAR_TOP;
        let content_w = FRAME_W - (SIDEBAR_INSET + SIDEBAR_W + CONTENT_INSET) - SIDEBAR_INSET;
        let row_w = content_w - 2.0 * ROW_INSET;
        assert_rect(
            &reg,
            "AchievementRow0",
            LayoutRect {
                x: content_x + ROW_INSET,
                y: content_y + ROW_INSET,
                width: row_w,
                height: ROW_H,
            },
        );
    }
}
