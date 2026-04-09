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

const REPAIR_BTN_W: f32 = 80.0;
const REPAIR_BTN_H: f32 = 22.0;
const REPAIR_GAP: f32 = 8.0;
const MONEY_W: f32 = 120.0;
const MONEY_H: f32 = 16.0;
const REPAIR_BTN_BG: &str = "0.15,0.12,0.05,0.95";
const REPAIR_BTN_TEXT_COLOR: &str = "1.0,0.82,0.0,1.0";
const MONEY_COLOR: &str = "1.0,0.82,0.0,1.0";
const EMPTY_TEXT_COLOR: &str = "0.72,0.72,0.72,1.0";

pub const MERCHANT_ITEM_ROWS: usize = 10;
pub const ACTION_CLOSE: &str = "merchant_close";
pub const ACTION_PAGE_PREV: &str = "merchant_page_prev";
pub const ACTION_PAGE_NEXT: &str = "merchant_page_next";
pub const ACTION_REPAIR_ALL: &str = "merchant_repair_all";
pub const ACTION_GUILD_REPAIR: &str = "merchant_guild_repair";
pub const ACTION_TAB_PREFIX: &str = "merchant_tab:";
pub const ACTION_BUY_PREFIX: &str = "merchant_buy:";
pub const ACTION_BUYBACK_PREFIX: &str = "merchant_buyback:";

#[derive(Clone, Debug, PartialEq)]
pub struct MerchantTab {
    pub name: String,
    pub active: bool,
    pub action: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MerchantItem {
    pub name: String,
    pub price: String,
    pub icon_fdid: u32,
    pub action: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MerchantFrameState {
    pub visible: bool,
    pub tabs: Vec<MerchantTab>,
    pub items: Vec<MerchantItem>,
    pub page: usize,
    pub total_pages: usize,
    pub player_money: String,
    pub empty_text: Option<String>,
}

impl Default for MerchantFrameState {
    fn default() -> Self {
        Self {
            visible: false,
            tabs: vec![
                MerchantTab {
                    name: "Buy".into(),
                    active: true,
                    action: format!("{ACTION_TAB_PREFIX}buy"),
                },
                MerchantTab {
                    name: "Sell".into(),
                    active: false,
                    action: format!("{ACTION_TAB_PREFIX}sell"),
                },
                MerchantTab {
                    name: "Buyback".into(),
                    active: false,
                    action: format!("{ACTION_TAB_PREFIX}buyback"),
                },
            ],
            items: vec![],
            page: 1,
            total_pages: 1,
            player_money: "0g 0s 0c".into(),
            empty_text: None,
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
            {item_grid(&state.items, state.empty_text.as_deref())}
            {repair_buttons()}
            {money_display(&state.player_money)}
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
            onclick: {tab.action.as_str()},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            {merchant_tab_label(label_id, &tab.name, tab_w, color)}
        }
    }
}

fn merchant_tab_label(id: DynName, text: &str, w: f32, color: &str) -> Element {
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

fn item_grid(items: &[MerchantItem], empty_text: Option<&str>) -> Element {
    let content_w = FRAME_W - 2.0 * CONTENT_INSET;
    let rows: Element = items
        .iter()
        .enumerate()
        .take(MERCHANT_ITEM_ROWS)
        .flat_map(|(i, item)| merchant_item_row(i, item, content_w))
        .collect();
    let content: Element = if items.is_empty() {
        empty_text
            .and_then(empty_state_text)
            .into_iter()
            .flatten()
            .collect()
    } else {
        rows
    };
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
            {content}
        }
    }
}

fn merchant_item_row(idx: usize, item: &MerchantItem, parent_w: f32) -> Element {
    let row_id = DynName(format!("MerchantItem{idx}"));
    let y = -(ITEM_INSET + idx as f32 * (ITEM_ROW_H + ITEM_ROW_GAP));
    let row_w = parent_w - 2.0 * ITEM_INSET;
    let text_x = ITEM_ICON_SIZE + 8.0;
    rsx! {
        r#frame {
            name: row_id,
            width: {row_w},
            height: {ITEM_ROW_H},
            onclick: {item.action.as_str()},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {ITEM_INSET},
                y: {y},
            }
            {merchant_item_icon(DynName(format!("MerchantItem{idx}Icon")))}
            {merchant_item_name(DynName(format!("MerchantItem{idx}Name")), &item.name, row_w - text_x - 60.0, text_x)}
            {merchant_item_price(DynName(format!("MerchantItem{idx}Price")), &item.price)}
        }
    }
}

fn empty_state_text(text: &str) -> Option<Element> {
    Some(rsx! {
        fontstring {
            name: "MerchantEmptyText",
            width: {FRAME_W - 2.0 * CONTENT_INSET - 2.0 * ITEM_INSET},
            height: 18.0,
            text: text,
            font_size: 11.0,
            font_color: EMPTY_TEXT_COLOR,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
            }
        }
    })
}

fn merchant_item_icon(id: DynName) -> Element {
    rsx! {
        r#frame {
            name: id,
            width: {ITEM_ICON_SIZE},
            height: {ITEM_ICON_SIZE},
            background_color: ITEM_ICON_BG,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "0", y: {-((ITEM_ROW_H - ITEM_ICON_SIZE) / 2.0)} }
        }
    }
}

fn merchant_item_name(id: DynName, text: &str, w: f32, x: f32) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {w},
            height: 16.0,
            text: text,
            font_size: 10.0,
            font_color: ITEM_NAME_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {x}, y: {-((ITEM_ROW_H - 16.0) / 2.0)} }
        }
    }
}

fn merchant_item_price(id: DynName, text: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: 56.0,
            height: 16.0,
            text: text,
            font_size: 9.0,
            font_color: ITEM_PRICE_COLOR,
            justify_h: "RIGHT",
            anchor { point: AnchorPoint::TopRight, relative_point: AnchorPoint::TopRight, x: "0", y: {-((ITEM_ROW_H - 16.0) / 2.0)} }
        }
    }
}

fn repair_btn(name: &str, label: &str, x: f32, y: f32) -> Element {
    let btn_id = DynName(name.into());
    let text_id = DynName(format!("{name}Text"));
    let action = match name {
        "MerchantRepairButton" => ACTION_REPAIR_ALL,
        "MerchantGuildRepairButton" => ACTION_GUILD_REPAIR,
        _ => "",
    };
    rsx! {
        r#frame {
            name: btn_id,
            width: {REPAIR_BTN_W},
            height: {REPAIR_BTN_H},
            background_color: REPAIR_BTN_BG,
            onclick: action,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            fontstring {
                name: text_id,
                width: {REPAIR_BTN_W},
                height: {REPAIR_BTN_H},
                text: label,
                font_size: 10.0,
                font_color: REPAIR_BTN_TEXT_COLOR,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
    }
}

fn repair_buttons() -> Element {
    let grid_h = MERCHANT_ITEM_ROWS as f32 * (ITEM_ROW_H + ITEM_ROW_GAP);
    let y = -(CONTENT_TOP + grid_h + 8.0);
    rsx! {
        {repair_btn("MerchantRepairButton", "Repair All", CONTENT_INSET, y)}
        {repair_btn("MerchantGuildRepairButton", "Guild Repair", CONTENT_INSET + REPAIR_BTN_W + REPAIR_GAP, y)}
    }
}

fn money_display(money: &str) -> Element {
    let grid_h = MERCHANT_ITEM_ROWS as f32 * (ITEM_ROW_H + ITEM_ROW_GAP);
    let y = -(CONTENT_TOP + grid_h + 8.0);
    rsx! {
        fontstring {
            name: "MerchantMoneyDisplay",
            width: {MONEY_W},
            height: {MONEY_H},
            text: money,
            font_size: 10.0,
            font_color: MONEY_COLOR,
            justify_h: "RIGHT",
            anchor {
                point: AnchorPoint::TopRight,
                relative_point: AnchorPoint::TopRight,
                x: {-CONTENT_INSET},
                y: {y},
            }
        }
    }
}

fn page_nav_button(name: &str, label: &str, x: f32, y: f32) -> Element {
    let btn_id = DynName(name.into());
    let text_id = DynName(format!("{name}Text"));
    let action = match name {
        "MerchantPagePrev" => ACTION_PAGE_PREV,
        "MerchantPageNext" => ACTION_PAGE_NEXT,
        _ => "",
    };
    rsx! {
        r#frame {
            name: btn_id,
            width: {PAGE_BTN_W},
            height: {PAGE_BTN_H},
            background_color: PAGE_BTN_BG,
            onclick: action,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            fontstring {
                name: text_id,
                width: {PAGE_BTN_W},
                height: {PAGE_BTN_H},
                text: label,
                font_size: 12.0,
                font_color: PAGE_BTN_TEXT,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
    }
}

fn page_buttons(page: usize, total: usize) -> Element {
    let page_text = format!("Page {page}/{total}");
    let y = -(FRAME_H - PAGE_BTN_H - 8.0);
    let center_x = FRAME_W / 2.0;
    rsx! {
        {page_nav_button("MerchantPagePrev", "<", center_x - PAGE_BTN_W - PAGE_BTN_GAP - 30.0, y)}
        fontstring {
            name: "MerchantPageLabel",
            width: 60.0,
            height: {PAGE_BTN_H},
            text: {page_text.as_str()},
            font_size: 10.0,
            font_color: PAGE_BTN_TEXT,
            justify_h: "CENTER",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {center_x - 30.0}, y: {y} }
        }
        {page_nav_button("MerchantPageNext", ">", center_x + 30.0 + PAGE_BTN_GAP, y)}
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/merchant_frame_component_tests.rs"]
mod tests;
