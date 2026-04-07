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

pub const FRAME_W: f32 = 608.0;
pub const FRAME_H: f32 = 486.0;
const HEADER_H: f32 = 30.0;
const TAB_H: f32 = 28.0;
const TAB_GAP: f32 = 4.0;
const TAB_INSET: f32 = 12.0;
const CONTENT_INSET: f32 = 8.0;
const CONTENT_TOP: f32 = HEADER_H + TAB_GAP + TAB_H + TAB_GAP;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const TAB_BG_ACTIVE: &str = "0.2,0.15,0.05,0.95";
const TAB_BG_INACTIVE: &str = "0.08,0.07,0.06,0.88";
const TAB_TEXT_ACTIVE: &str = "1.0,0.82,0.0,1.0";
const TAB_TEXT_INACTIVE: &str = "0.6,0.6,0.6,1.0";
const CONTENT_BG: &str = "0.0,0.0,0.0,0.3";
const CONTENT_PLACEHOLDER_COLOR: &str = "0.5,0.5,0.5,1.0";

#[derive(Clone, Debug, PartialEq)]
pub struct AuctionTab {
    pub name: String,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AuctionHouseFrameState {
    pub visible: bool,
    pub tabs: Vec<AuctionTab>,
}

impl Default for AuctionHouseFrameState {
    fn default() -> Self {
        Self {
            visible: false,
            tabs: vec![
                AuctionTab {
                    name: "Browse".into(),
                    active: true,
                },
                AuctionTab {
                    name: "Sell".into(),
                    active: false,
                },
                AuctionTab {
                    name: "Auctions".into(),
                    active: false,
                },
            ],
        }
    }
}

pub fn auction_house_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<AuctionHouseFrameState>()
        .expect("AuctionHouseFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "AuctionHouseFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "100",
                y: "-80",
            }
            {title_bar()}
            {tab_row(&state.tabs)}
            {content_area()}
        }
    }
}

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "AuctionHouseFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Auction House",
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

fn tab_row(tabs: &[AuctionTab]) -> Element {
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

fn tab_button(i: usize, tab: &AuctionTab, tab_w: f32, x: f32, y: f32) -> Element {
    let tab_id = DynName(format!("AuctionHouseTab{i}"));
    let label_id = DynName(format!("AuctionHouseTab{i}Label"));
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

fn content_area() -> Element {
    let content_y = -CONTENT_TOP;
    let content_w = FRAME_W - 2.0 * CONTENT_INSET;
    let content_h = FRAME_H - CONTENT_TOP - CONTENT_INSET;
    rsx! {
        r#frame {
            name: "AuctionHouseContentArea",
            width: {content_w},
            height: {content_h},
            background_color: CONTENT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {CONTENT_INSET},
                y: {content_y},
            }
            fontstring {
                name: "AuctionHouseContentPlaceholder",
                width: {content_w},
                height: 20.0,
                text: "Browse the Auction House",
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
    use ui_toolkit::layout::{LayoutRect, recompute_layouts};
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn make_test_state() -> AuctionHouseFrameState {
        AuctionHouseFrameState {
            visible: true,
            ..Default::default()
        }
    }

    fn build_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(auction_house_frame_screen).sync(&shared, &mut reg);
        reg
    }

    fn layout_registry() -> FrameRegistry {
        let mut reg = build_registry();
        recompute_layouts(&mut reg);
        reg
    }

    fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
        reg.get(reg.get_by_name(name).expect(name))
            .and_then(|f| f.layout_rect.clone())
            .unwrap_or_else(|| panic!("{name} has no layout_rect"))
    }

    #[test]
    fn builds_expected_frames() {
        let reg = build_registry();
        assert!(reg.get_by_name("AuctionHouseFrame").is_some());
        assert!(reg.get_by_name("AuctionHouseFrameTitle").is_some());
        assert!(reg.get_by_name("AuctionHouseContentArea").is_some());
        assert!(reg.get_by_name("AuctionHouseContentPlaceholder").is_some());
    }

    #[test]
    fn builds_three_tabs() {
        let reg = build_registry();
        for i in 0..3 {
            assert!(
                reg.get_by_name(&format!("AuctionHouseTab{i}")).is_some(),
                "AuctionHouseTab{i} missing"
            );
            assert!(
                reg.get_by_name(&format!("AuctionHouseTab{i}Label"))
                    .is_some(),
                "AuctionHouseTab{i}Label missing"
            );
        }
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        let mut state = make_test_state();
        state.visible = false;
        shared.insert(state);
        Screen::new(auction_house_frame_screen).sync(&shared, &mut reg);

        let id = reg.get_by_name("AuctionHouseFrame").expect("frame");
        let frame = reg.get(id).expect("data");
        assert!(frame.hidden);
    }

    // --- Coord validation ---

    const FRAME_X: f32 = 100.0;
    const FRAME_Y: f32 = 80.0;

    #[test]
    fn coord_main_frame() {
        let reg = layout_registry();
        let r = rect(&reg, "AuctionHouseFrame");
        assert!((r.x - FRAME_X).abs() < 1.0);
        assert!((r.y - FRAME_Y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
        assert!((r.height - FRAME_H).abs() < 1.0);
    }

    #[test]
    fn coord_tabs() {
        let reg = layout_registry();
        let tab_count = 3.0_f32;
        let tab_w = (FRAME_W - 2.0 * TAB_INSET - (tab_count - 1.0) * TAB_GAP) / tab_count;
        let tab_y = FRAME_Y + HEADER_H + TAB_GAP;
        let tab0 = rect(&reg, "AuctionHouseTab0");
        assert!((tab0.x - (FRAME_X + TAB_INSET)).abs() < 1.0);
        assert!((tab0.y - tab_y).abs() < 1.0);
        assert!((tab0.width - tab_w).abs() < 1.0);
        let tab2 = rect(&reg, "AuctionHouseTab2");
        let expected_x2 = FRAME_X + TAB_INSET + 2.0 * (tab_w + TAB_GAP);
        assert!((tab2.x - expected_x2).abs() < 1.0);
    }

    #[test]
    fn coord_content_area() {
        let reg = layout_registry();
        let r = rect(&reg, "AuctionHouseContentArea");
        assert!((r.x - (FRAME_X + CONTENT_INSET)).abs() < 1.0);
        assert!((r.y - (FRAME_Y + CONTENT_TOP)).abs() < 1.0);
        let expected_w = FRAME_W - 2.0 * CONTENT_INSET;
        let expected_h = FRAME_H - CONTENT_TOP - CONTENT_INSET;
        assert!((r.width - expected_w).abs() < 1.0);
        assert!((r.height - expected_h).abs() < 1.0);
    }
}
