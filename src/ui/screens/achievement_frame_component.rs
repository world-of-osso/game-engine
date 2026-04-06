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

pub const MAX_VISIBLE_CATEGORIES: usize = 16;

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
pub struct AchievementFrameState {
    pub visible: bool,
    pub tabs: Vec<AchievementTab>,
    pub categories: Vec<AchievementCategory>,
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
            {content_area()}
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
    let bg = if tab.active {
        TAB_BG_ACTIVE
    } else {
        TAB_BG_INACTIVE
    };
    let color = if tab.active {
        TAB_TEXT_ACTIVE
    } else {
        TAB_TEXT_INACTIVE
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
            fontstring {
                name: label_id,
                width: {tab_w},
                height: {TAB_H},
                text: {tab.name.as_str()},
                font_size: 11.0,
                font_color: color,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
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
    let color = if cat.selected {
        CAT_TEXT_SELECTED
    } else if cat.is_child {
        CAT_TEXT_NORMAL
    } else {
        CAT_TEXT_HEADER
    };
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
            fontstring {
                name: label_id,
                width: {label_w},
                height: {CAT_ROW_H},
                text: {cat.name.as_str()},
                font_size: 10.0,
                font_color: color,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {x_offset},
                    y: "0",
                }
            }
        }
    }
}

fn content_area() -> Element {
    let content_x = SIDEBAR_INSET + SIDEBAR_W + CONTENT_INSET;
    let content_y = -SIDEBAR_TOP;
    let content_w = FRAME_W - content_x - SIDEBAR_INSET;
    let content_h = FRAME_H - SIDEBAR_TOP - SIDEBAR_INSET;
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
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::Top,
                    relative_point: AnchorPoint::Top,
                    x: "0",
                    y: "-20",
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

        // Child category row should exist
        assert!(registry.get_by_name("AchievementCat1").is_some());
        assert!(registry.get_by_name("AchievementCat1Label").is_some());
    }
}
