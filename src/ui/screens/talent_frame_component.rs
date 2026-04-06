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

pub const FRAME_W: f32 = 400.0;
pub const FRAME_H: f32 = 500.0;
const HEADER_H: f32 = 30.0;
const TAB_H: f32 = 28.0;
const TAB_GAP: f32 = 4.0;
const NODE_W: f32 = 40.0;
const NODE_H: f32 = 48.0;
const NODE_GAP: f32 = 8.0;
const GRID_COLS: usize = 4;
const GRID_ROWS: usize = 7;
const GRID_INSET: f32 = 16.0;
const FOOTER_H: f32 = 28.0;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const TAB_BG_ACTIVE: &str = "0.2,0.15,0.05,0.95";
const TAB_BG_INACTIVE: &str = "0.08,0.07,0.06,0.88";
const TAB_TEXT_ACTIVE: &str = "1.0,0.82,0.0,1.0";
const TAB_TEXT_INACTIVE: &str = "0.6,0.6,0.6,1.0";
const NODE_BG_ACTIVE: &str = "0.15,0.12,0.02,0.95";
const NODE_BG_INACTIVE: &str = "0.05,0.05,0.05,0.85";
const NODE_NAME_COLOR: &str = "1.0,1.0,1.0,1.0";
const NODE_POINTS_ACTIVE: &str = "1.0,0.82,0.0,1.0";
const NODE_POINTS_INACTIVE: &str = "0.5,0.5,0.5,1.0";
const FOOTER_COLOR: &str = "0.8,0.8,0.8,1.0";

pub const TALENT_COUNT: usize = GRID_ROWS * GRID_COLS;

#[derive(Clone, Debug, PartialEq)]
pub struct TalentNodeState {
    pub name: String,
    pub points: String,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TalentSpecTab {
    pub name: String,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TalentFrameState {
    pub visible: bool,
    pub spec_tabs: Vec<TalentSpecTab>,
    pub talents: Vec<TalentNodeState>,
    pub points_remaining: u16,
}

pub fn talent_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<TalentFrameState>()
        .expect("TalentFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "TalentFrame",
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
            {talent_title_bar()}
            {spec_tab_row(&state.spec_tabs)}
            {talent_grid(&state.talents)}
            {points_remaining_footer(state.points_remaining)}
        }
    }
}

fn talent_title_bar() -> Element {
    rsx! {
        fontstring {
            name: "TalentFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Talents",
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

fn spec_tab_row(tabs: &[TalentSpecTab]) -> Element {
    let tab_w = (FRAME_W - 2.0 * GRID_INSET - (tabs.len() as f32 - 1.0) * TAB_GAP)
        / tabs.len().max(1) as f32;
    tabs.iter()
        .enumerate()
        .flat_map(|(i, tab)| {
            let x = GRID_INSET + i as f32 * (tab_w + TAB_GAP);
            let y = -(HEADER_H + TAB_GAP);
            spec_tab(i, tab, tab_w, x, y)
        })
        .collect()
}

fn spec_tab(i: usize, tab: &TalentSpecTab, tab_w: f32, x: f32, y: f32) -> Element {
    let tab_id = DynName(format!("TalentSpecTab{i}"));
    let label_id = DynName(format!("TalentSpecTab{i}Label"));
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

fn talent_grid(talents: &[TalentNodeState]) -> Element {
    let grid_top = HEADER_H + TAB_GAP + TAB_H + NODE_GAP;
    talents
        .iter()
        .enumerate()
        .take(TALENT_COUNT)
        .flat_map(|(i, talent)| {
            let col = i % GRID_COLS;
            let row = i / GRID_COLS;
            let x = GRID_INSET + col as f32 * (NODE_W + NODE_GAP);
            let y = -(grid_top + row as f32 * (NODE_H + NODE_GAP));
            talent_node(i, talent, x, y)
        })
        .collect()
}

fn talent_node(idx: usize, talent: &TalentNodeState, x: f32, y: f32) -> Element {
    let node_id = DynName(format!("TalentNode{idx}"));
    let bg = if talent.active {
        NODE_BG_ACTIVE
    } else {
        NODE_BG_INACTIVE
    };
    rsx! {
        r#frame {
            name: node_id,
            width: {NODE_W},
            height: {NODE_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            {talent_node_name(idx, &talent.name)}
            {talent_node_points(idx, &talent.points, talent.active)}
        }
    }
}

fn talent_node_name(idx: usize, name: &str) -> Element {
    let name_id = DynName(format!("TalentNode{idx}Name"));
    rsx! {
        fontstring {
            name: name_id,
            width: {NODE_W},
            height: 20.0,
            text: name,
            font_size: 7.0,
            font_color: NODE_NAME_COLOR,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                x: "0",
                y: "-4",
            }
        }
    }
}

fn talent_node_points(idx: usize, points: &str, active: bool) -> Element {
    let pts_id = DynName(format!("TalentNode{idx}Points"));
    let pts_color = if active {
        NODE_POINTS_ACTIVE
    } else {
        NODE_POINTS_INACTIVE
    };
    rsx! {
        fontstring {
            name: pts_id,
            width: {NODE_W},
            height: 14.0,
            text: points,
            font_size: 8.0,
            font_color: pts_color,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::Bottom,
                relative_point: AnchorPoint::Bottom,
                x: "0",
                y: "4",
            }
        }
    }
}

fn points_remaining_footer(points: u16) -> Element {
    let text = format!("Points Remaining: {points}");
    let y = -(FRAME_H - FOOTER_H);
    rsx! {
        fontstring {
            name: "TalentFramePointsRemaining",
            width: {FRAME_W},
            height: {FOOTER_H},
            text: {text.as_str()},
            font_size: 11.0,
            font_color: FOOTER_COLOR,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: {y},
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn make_test_state() -> TalentFrameState {
        TalentFrameState {
            visible: true,
            spec_tabs: vec![
                TalentSpecTab {
                    name: "Protection".to_string(),
                    active: true,
                },
                TalentSpecTab {
                    name: "Holy".to_string(),
                    active: false,
                },
                TalentSpecTab {
                    name: "Retribution".to_string(),
                    active: false,
                },
            ],
            talents: (0..TALENT_COUNT)
                .map(|i| TalentNodeState {
                    name: format!("Talent{i}"),
                    points: "0/1".to_string(),
                    active: false,
                })
                .collect(),
            points_remaining: 51,
        }
    }

    #[test]
    fn talent_frame_screen_builds_expected_frames() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        let mut screen = Screen::new(talent_frame_screen);
        screen.sync(&shared, &mut registry);

        assert!(registry.get_by_name("TalentFrame").is_some());
        assert!(registry.get_by_name("TalentFrameTitle").is_some());
        assert!(registry.get_by_name("TalentFramePointsRemaining").is_some());
    }

    #[test]
    fn talent_frame_builds_talent_grid() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(talent_frame_screen).sync(&shared, &mut registry);

        for i in 0..TALENT_COUNT {
            assert!(
                registry.get_by_name(&format!("TalentNode{i}")).is_some(),
                "TalentNode{i} missing"
            );
        }
    }

    #[test]
    fn talent_frame_builds_spec_tabs() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(talent_frame_screen).sync(&shared, &mut registry);

        for i in 0..3 {
            assert!(
                registry.get_by_name(&format!("TalentSpecTab{i}")).is_some(),
                "TalentSpecTab{i} missing"
            );
        }
    }
}
