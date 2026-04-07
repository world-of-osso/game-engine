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

pub const FRAME_W: f32 = 340.0;
pub const FRAME_H: f32 = 480.0;
const HEADER_H: f32 = 28.0;
const TAB_H: f32 = 28.0;
const TAB_GAP: f32 = 4.0;
const TAB_INSET: f32 = 8.0;
const CONTENT_TOP: f32 = HEADER_H + TAB_GAP + TAB_H + TAB_GAP;
const ITEM_ROW_H: f32 = 32.0;
const ITEM_ROW_GAP: f32 = 1.0;
const ITEM_INSET: f32 = 4.0;
const ITEM_ICON_SIZE: f32 = 24.0;
const PAGE_BTN_W: f32 = 30.0;
const PAGE_BTN_H: f32 = 22.0;
const PAGE_BTN_GAP: f32 = 8.0;
const CONTENT_INSET: f32 = 8.0;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const TAB_BG_ACTIVE: &str = "0.2,0.15,0.05,0.95";
const TAB_BG_INACTIVE: &str = "0.08,0.07,0.06,0.88";
const TAB_TEXT_ACTIVE: &str = "1.0,0.82,0.0,1.0";
const TAB_TEXT_INACTIVE: &str = "0.6,0.6,0.6,1.0";
const CONTENT_BG: &str = "0.0,0.0,0.0,0.3";
const ITEM_ICON_BG: &str = "0.1,0.1,0.1,0.9";
const ITEM_NAME_COLOR: &str = "1.0,1.0,1.0,1.0";
const ITEM_PRICE_COLOR: &str = "1.0,0.82,0.0,1.0";
const PAGE_BTN_BG: &str = "0.15,0.12,0.05,0.95";
const PAGE_BTN_TEXT: &str = "1.0,0.82,0.0,1.0";

pub const MERCHANT_ITEM_ROWS: usize = 10;

#[derive(Clone, Debug, PartialEq)]
pub struct MerchantTab {
    pub name: String,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MerchantItem {
    pub name: String,
    pub price: String,
    pub icon_fdid: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MerchantFrameState {
    pub visible: bool,
    pub tabs: Vec<MerchantTab>,
    pub items: Vec<MerchantItem>,
    pub page: usize,
    pub total_pages: usize,
}

impl Default for MerchantFrameState {
    fn default() -> Self {
        Self {
            visible: false,
            tabs: vec![
                MerchantTab {
                    name: "Buy".into(),
                    active: true,
                },
                MerchantTab {
                    name: "Sell".into(),
                    active: false,
                },
                MerchantTab {
                    name: "Buyback".into(),
                    active: false,
                },
            ],
            items: vec![],
            page: 1,
            total_pages: 1,
        }
    }
}

pub fn merchant_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<MerchantFrameState>()
        .expect("MerchantFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "MerchantFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "50",
                y: "-80",
            }
            {title_bar()}
            {tab_row(&state.tabs)}
            {item_grid(&state.items)}
            {page_buttons(state.page, state.total_pages)}
        }
    }
}

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "MerchantFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Merchant",
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

fn tab_row(tabs: &[MerchantTab]) -> Element {
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

fn tab_button(i: usize, tab: &MerchantTab, tab_w: f32, x: f32, y: f32) -> Element {
    let tab_id = DynName(format!("MerchantTab{i}"));
    let label_id = DynName(format!("MerchantTab{i}Label"));
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

fn item_grid(items: &[MerchantItem]) -> Element {
    let content_w = FRAME_W - 2.0 * CONTENT_INSET;
    let rows: Element = items
        .iter()
        .enumerate()
        .take(MERCHANT_ITEM_ROWS)
        .flat_map(|(i, item)| merchant_item_row(i, item, content_w))
        .collect();
    rsx! {
        r#frame {
            name: "MerchantItemGrid",
            width: {content_w},
            height: {MERCHANT_ITEM_ROWS as f32 * (ITEM_ROW_H + ITEM_ROW_GAP)},
            background_color: CONTENT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {CONTENT_INSET},
                y: {-CONTENT_TOP},
            }
            {rows}
        }
    }
}

fn merchant_item_row(idx: usize, item: &MerchantItem, parent_w: f32) -> Element {
    let row_id = DynName(format!("MerchantItem{idx}"));
    let icon_id = DynName(format!("MerchantItem{idx}Icon"));
    let name_id = DynName(format!("MerchantItem{idx}Name"));
    let price_id = DynName(format!("MerchantItem{idx}Price"));
    let y = -(ITEM_INSET + idx as f32 * (ITEM_ROW_H + ITEM_ROW_GAP));
    let row_w = parent_w - 2.0 * ITEM_INSET;
    let text_x = ITEM_ICON_SIZE + 8.0;
    rsx! {
        r#frame {
            name: row_id,
            width: {row_w},
            height: {ITEM_ROW_H},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {ITEM_INSET},
                y: {y},
            }
            r#frame {
                name: icon_id,
                width: {ITEM_ICON_SIZE},
                height: {ITEM_ICON_SIZE},
                background_color: ITEM_ICON_BG,
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "0",
                    y: {-((ITEM_ROW_H - ITEM_ICON_SIZE) / 2.0)},
                }
            }
            fontstring {
                name: name_id,
                width: {row_w - text_x - 60.0},
                height: 16.0,
                text: {item.name.as_str()},
                font_size: 10.0,
                font_color: ITEM_NAME_COLOR,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {text_x},
                    y: {-((ITEM_ROW_H - 16.0) / 2.0)},
                }
            }
            fontstring {
                name: price_id,
                width: 56.0,
                height: 16.0,
                text: {item.price.as_str()},
                font_size: 9.0,
                font_color: ITEM_PRICE_COLOR,
                justify_h: "RIGHT",
                anchor {
                    point: AnchorPoint::TopRight,
                    relative_point: AnchorPoint::TopRight,
                    x: "0",
                    y: {-((ITEM_ROW_H - 16.0) / 2.0)},
                }
            }
        }
    }
}

fn page_buttons(page: usize, total: usize) -> Element {
    let page_text = format!("Page {page}/{total}");
    let y = -(FRAME_H - PAGE_BTN_H - 8.0);
    let center_x = FRAME_W / 2.0;
    rsx! {
        r#frame {
            name: "MerchantPagePrev",
            width: {PAGE_BTN_W},
            height: {PAGE_BTN_H},
            background_color: PAGE_BTN_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {center_x - PAGE_BTN_W - PAGE_BTN_GAP - 30.0},
                y: {y},
            }
            fontstring {
                name: "MerchantPagePrevText",
                width: {PAGE_BTN_W},
                height: {PAGE_BTN_H},
                text: "<",
                font_size: 12.0,
                font_color: PAGE_BTN_TEXT,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
        fontstring {
            name: "MerchantPageLabel",
            width: 60.0,
            height: {PAGE_BTN_H},
            text: {page_text.as_str()},
            font_size: 10.0,
            font_color: PAGE_BTN_TEXT,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {center_x - 30.0},
                y: {y},
            }
        }
        r#frame {
            name: "MerchantPageNext",
            width: {PAGE_BTN_W},
            height: {PAGE_BTN_H},
            background_color: PAGE_BTN_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {center_x + 30.0 + PAGE_BTN_GAP},
                y: {y},
            }
            fontstring {
                name: "MerchantPageNextText",
                width: {PAGE_BTN_W},
                height: {PAGE_BTN_H},
                text: ">",
                font_size: 12.0,
                font_color: PAGE_BTN_TEXT,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
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

    fn make_test_state() -> MerchantFrameState {
        MerchantFrameState {
            visible: true,
            items: vec![
                MerchantItem {
                    name: "Rough Arrow".into(),
                    price: "10c".into(),
                    icon_fdid: 0,
                },
                MerchantItem {
                    name: "Light Shot".into(),
                    price: "10c".into(),
                    icon_fdid: 0,
                },
            ],
            ..Default::default()
        }
    }

    fn build_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(merchant_frame_screen).sync(&shared, &mut reg);
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
    fn builds_frame_and_title() {
        let reg = build_registry();
        assert!(reg.get_by_name("MerchantFrame").is_some());
        assert!(reg.get_by_name("MerchantFrameTitle").is_some());
    }

    #[test]
    fn builds_tabs() {
        let reg = build_registry();
        for i in 0..3 {
            assert!(reg.get_by_name(&format!("MerchantTab{i}")).is_some());
        }
    }

    #[test]
    fn builds_item_rows() {
        let reg = build_registry();
        assert!(reg.get_by_name("MerchantItemGrid").is_some());
        for i in 0..2 {
            assert!(reg.get_by_name(&format!("MerchantItem{i}")).is_some());
            assert!(reg.get_by_name(&format!("MerchantItem{i}Icon")).is_some());
            assert!(reg.get_by_name(&format!("MerchantItem{i}Name")).is_some());
            assert!(reg.get_by_name(&format!("MerchantItem{i}Price")).is_some());
        }
    }

    #[test]
    fn builds_page_buttons() {
        let reg = build_registry();
        assert!(reg.get_by_name("MerchantPagePrev").is_some());
        assert!(reg.get_by_name("MerchantPageNext").is_some());
        assert!(reg.get_by_name("MerchantPageLabel").is_some());
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(MerchantFrameState::default());
        Screen::new(merchant_frame_screen).sync(&shared, &mut reg);
        let id = reg.get_by_name("MerchantFrame").expect("frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    // --- Coord validation ---

    const FRAME_X: f32 = 50.0;
    const FRAME_Y: f32 = 80.0;

    #[test]
    fn coord_main_frame() {
        let reg = layout_registry();
        let r = rect(&reg, "MerchantFrame");
        assert!((r.x - FRAME_X).abs() < 1.0);
        assert!((r.y - FRAME_Y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
    }

    #[test]
    fn coord_item_grid() {
        let reg = layout_registry();
        let r = rect(&reg, "MerchantItemGrid");
        assert!((r.x - (FRAME_X + CONTENT_INSET)).abs() < 1.0);
        assert!((r.y - (FRAME_Y + CONTENT_TOP)).abs() < 1.0);
    }

    #[test]
    fn coord_page_buttons() {
        let reg = layout_registry();
        let prev = rect(&reg, "MerchantPagePrev");
        let next = rect(&reg, "MerchantPageNext");
        assert!((prev.width - PAGE_BTN_W).abs() < 1.0);
        assert!((next.width - PAGE_BTN_W).abs() < 1.0);
        assert!(next.x > prev.x, "next should be right of prev");
    }
}
