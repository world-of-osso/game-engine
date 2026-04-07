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
pub const FRAME_H: f32 = 460.0;
const HEADER_H: f32 = 28.0;
const BUTTON_W: f32 = 240.0;
const BUTTON_H: f32 = 36.0;
const BUTTON_GAP: f32 = 8.0;
const BUTTON_TOP: f32 = HEADER_H + 20.0;
const CONTENT_INSET: f32 = 8.0;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const BUTTON_BG: &str = "0.15,0.12,0.05,0.95";
const BUTTON_TEXT_COLOR: &str = "1.0,0.82,0.0,1.0";
const CONTENT_BG: &str = "0.0,0.0,0.0,0.3";

// Article / detail / ticket layout
const ARTICLE_ROW_H: f32 = 22.0;
const ARTICLE_ROW_GAP: f32 = 1.0;
const ARTICLE_INSET: f32 = 4.0;
const DETAIL_TITLE_H: f32 = 20.0;
const DETAIL_BODY_INSET: f32 = 4.0;
const TICKET_INPUT_H: f32 = 26.0;
const TICKET_TEXTAREA_H: f32 = 80.0;
const TICKET_LABEL_W: f32 = 80.0;
const TICKET_BTN_W: f32 = 100.0;
const TICKET_BTN_H: f32 = 26.0;
const ARTICLE_TITLE_COLOR: &str = "1.0,1.0,1.0,1.0";
const ARTICLE_SELECTED_BG: &str = "0.2,0.15,0.05,0.95";
const ARTICLE_NORMAL_BG: &str = "0.0,0.0,0.0,0.0";
const ARTICLE_SELECTED_COLOR: &str = "1.0,0.82,0.0,1.0";
const DETAIL_TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const DETAIL_BODY_COLOR: &str = "0.8,0.8,0.8,1.0";
const TICKET_LABEL_COLOR: &str = "0.8,0.8,0.8,1.0";
const TICKET_INPUT_BG: &str = "0.1,0.1,0.1,0.9";
const TICKET_INPUT_COLOR: &str = "1.0,1.0,1.0,1.0";
const TICKET_BTN_BG: &str = "0.15,0.12,0.05,0.95";
const TICKET_BTN_TEXT: &str = "1.0,0.82,0.0,1.0";

pub const CATEGORY_BUTTONS: &[&str] = &["Knowledge Base", "Submit Ticket", "Bug Report"];
pub const MAX_ARTICLES: usize = 10;

#[derive(Clone, Debug, PartialEq)]
pub struct ArticleEntry {
    pub title: String,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ArticleDetail {
    pub title: String,
    pub body: String,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TicketFormState {
    pub category: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HelpFrameState {
    pub visible: bool,
    pub articles: Vec<ArticleEntry>,
    pub detail: ArticleDetail,
    pub ticket: TicketFormState,
}

impl Default for HelpFrameState {
    fn default() -> Self {
        Self {
            visible: false,
            articles: vec![],
            detail: ArticleDetail::default(),
            ticket: TicketFormState::default(),
        }
    }
}

pub fn help_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<HelpFrameState>()
        .expect("HelpFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "HelpFrame",
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
            {category_buttons()}
            {content_area()}
            {article_list_panel(&state.articles)}
            {article_detail_panel(&state.detail)}
            {ticket_form_panel(&state.ticket)}
        }
    }
}

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "HelpFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Help",
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

fn category_buttons() -> Element {
    let x_start = (FRAME_W - BUTTON_W) / 2.0;
    CATEGORY_BUTTONS
        .iter()
        .enumerate()
        .flat_map(|(i, label)| {
            let y = -(BUTTON_TOP + i as f32 * (BUTTON_H + BUTTON_GAP));
            category_button(i, label, x_start, y)
        })
        .collect()
}

fn category_button(idx: usize, label: &str, x: f32, y: f32) -> Element {
    let btn_name = DynName(format!("HelpCategoryBtn{idx}"));
    let txt_name = DynName(format!("HelpCategoryBtn{idx}Text"));
    rsx! {
        r#frame {
            name: btn_name,
            width: {BUTTON_W},
            height: {BUTTON_H},
            background_color: BUTTON_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            fontstring {
                name: txt_name,
                width: {BUTTON_W},
                height: {BUTTON_H},
                text: label,
                font_size: 12.0,
                font_color: BUTTON_TEXT_COLOR,
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
    let buttons_h = CATEGORY_BUTTONS.len() as f32 * (BUTTON_H + BUTTON_GAP);
    let content_y = -(BUTTON_TOP + buttons_h);
    let content_w = FRAME_W - 2.0 * CONTENT_INSET;
    let content_h = FRAME_H - BUTTON_TOP - buttons_h - CONTENT_INSET;
    rsx! {
        r#frame {
            name: "HelpContentArea",
            width: {content_w},
            height: {content_h},
            background_color: CONTENT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {CONTENT_INSET},
                y: {content_y},
            }
        }
    }
}

fn article_list_panel(articles: &[ArticleEntry]) -> Element {
    let buttons_h = CATEGORY_BUTTONS.len() as f32 * (BUTTON_H + BUTTON_GAP);
    let panel_y = -(BUTTON_TOP + buttons_h);
    let panel_w = FRAME_W - 2.0 * CONTENT_INSET;
    let panel_h = FRAME_H - BUTTON_TOP - buttons_h - CONTENT_INSET;
    let rows: Element = articles
        .iter()
        .enumerate()
        .take(MAX_ARTICLES)
        .flat_map(|(i, a)| article_row(i, a, panel_w))
        .collect();
    rsx! {
        r#frame {
            name: "HelpArticleList",
            width: {panel_w},
            height: {panel_h},
            hidden: true,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {CONTENT_INSET},
                y: {panel_y},
            }
            {rows}
        }
    }
}

fn article_row(idx: usize, article: &ArticleEntry, w: f32) -> Element {
    let row_id = DynName(format!("HelpArticle{idx}"));
    let label_id = DynName(format!("HelpArticle{idx}Title"));
    let (bg, color) = if article.selected {
        (ARTICLE_SELECTED_BG, ARTICLE_SELECTED_COLOR)
    } else {
        (ARTICLE_NORMAL_BG, ARTICLE_TITLE_COLOR)
    };
    let y = -(ARTICLE_INSET + idx as f32 * (ARTICLE_ROW_H + ARTICLE_ROW_GAP));
    rsx! {
        r#frame {
            name: row_id,
            width: {w - 2.0 * ARTICLE_INSET},
            height: {ARTICLE_ROW_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {ARTICLE_INSET},
                y: {y},
            }
            {article_row_label(label_id, &article.title, w - 4.0 * ARTICLE_INSET, color)}
        }
    }
}

fn article_row_label(id: DynName, text: &str, w: f32, color: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {w},
            height: {ARTICLE_ROW_H},
            text: text,
            font_size: 10.0,
            font_color: color,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "4", y: "0" }
        }
    }
}

fn article_detail_panel(detail: &ArticleDetail) -> Element {
    let buttons_h = CATEGORY_BUTTONS.len() as f32 * (BUTTON_H + BUTTON_GAP);
    let panel_y = -(BUTTON_TOP + buttons_h);
    let panel_w = FRAME_W - 2.0 * CONTENT_INSET;
    let panel_h = FRAME_H - BUTTON_TOP - buttons_h - CONTENT_INSET;
    rsx! {
        r#frame {
            name: "HelpArticleDetail",
            width: {panel_w},
            height: {panel_h},
            hidden: true,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {CONTENT_INSET},
                y: {panel_y},
            }
            fontstring {
                name: "HelpArticleDetailTitle",
                width: {panel_w - 2.0 * DETAIL_BODY_INSET},
                height: {DETAIL_TITLE_H},
                text: {detail.title.as_str()},
                font_size: 13.0,
                font_color: DETAIL_TITLE_COLOR,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {DETAIL_BODY_INSET},
                    y: {-DETAIL_BODY_INSET},
                }
            }
            fontstring {
                name: "HelpArticleDetailBody",
                width: {panel_w - 2.0 * DETAIL_BODY_INSET},
                height: {panel_h - DETAIL_TITLE_H - 3.0 * DETAIL_BODY_INSET},
                text: {detail.body.as_str()},
                font_size: 10.0,
                font_color: DETAIL_BODY_COLOR,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {DETAIL_BODY_INSET},
                    y: {-(DETAIL_BODY_INSET + DETAIL_TITLE_H)},
                }
            }
        }
    }
}

fn ticket_form_panel(ticket: &TicketFormState) -> Element {
    let buttons_h = CATEGORY_BUTTONS.len() as f32 * (BUTTON_H + BUTTON_GAP);
    let panel_y = -(BUTTON_TOP + buttons_h);
    let panel_w = FRAME_W - 2.0 * CONTENT_INSET;
    let panel_h = FRAME_H - BUTTON_TOP - buttons_h - CONTENT_INSET;
    let input_w = panel_w - TICKET_LABEL_W - 3.0 * ARTICLE_INSET;
    rsx! {
        r#frame {
            name: "HelpTicketForm",
            width: {panel_w},
            height: {panel_h},
            hidden: true,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {CONTENT_INSET},
                y: {panel_y},
            }
            fontstring {
                name: "HelpTicketCategoryLabel",
                width: {TICKET_LABEL_W},
                height: {TICKET_INPUT_H},
                text: "Category:",
                font_size: 10.0,
                font_color: TICKET_LABEL_COLOR,
                justify_h: "RIGHT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {ARTICLE_INSET},
                    y: {-ARTICLE_INSET},
                }
            }
            r#frame {
                name: "HelpTicketCategoryInput",
                width: {input_w},
                height: {TICKET_INPUT_H},
                background_color: TICKET_INPUT_BG,
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {ARTICLE_INSET + TICKET_LABEL_W + ARTICLE_INSET},
                    y: {-ARTICLE_INSET},
                }
            }
            fontstring {
                name: "HelpTicketDescLabel",
                width: {TICKET_LABEL_W},
                height: {TICKET_INPUT_H},
                text: "Description:",
                font_size: 10.0,
                font_color: TICKET_LABEL_COLOR,
                justify_h: "RIGHT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {ARTICLE_INSET},
                    y: {-(ARTICLE_INSET + TICKET_INPUT_H + ARTICLE_INSET)},
                }
            }
            r#frame {
                name: "HelpTicketDescInput",
                width: {input_w},
                height: {TICKET_TEXTAREA_H},
                background_color: TICKET_INPUT_BG,
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {ARTICLE_INSET + TICKET_LABEL_W + ARTICLE_INSET},
                    y: {-(ARTICLE_INSET + TICKET_INPUT_H + ARTICLE_INSET)},
                }
            }
            r#frame {
                name: "HelpTicketSubmitButton",
                width: {TICKET_BTN_W},
                height: {TICKET_BTN_H},
                background_color: TICKET_BTN_BG,
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {ARTICLE_INSET + TICKET_LABEL_W + ARTICLE_INSET},
                    y: {-(ARTICLE_INSET + TICKET_INPUT_H + ARTICLE_INSET + TICKET_TEXTAREA_H + ARTICLE_INSET)},
                }
                fontstring {
                    name: "HelpTicketSubmitButtonText",
                    width: {TICKET_BTN_W},
                    height: {TICKET_BTN_H},
                    text: "Submit",
                    font_size: 10.0,
                    font_color: TICKET_BTN_TEXT,
                    justify_h: "CENTER",
                    anchor {
                        point: AnchorPoint::TopLeft,
                        relative_point: AnchorPoint::TopLeft,
                    }
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

    fn build_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(HelpFrameState {
            visible: true,
            ..Default::default()
        });
        Screen::new(help_frame_screen).sync(&shared, &mut reg);
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
        assert!(reg.get_by_name("HelpFrame").is_some());
        assert!(reg.get_by_name("HelpFrameTitle").is_some());
    }

    #[test]
    fn builds_category_buttons() {
        let reg = build_registry();
        for i in 0..CATEGORY_BUTTONS.len() {
            assert!(
                reg.get_by_name(&format!("HelpCategoryBtn{i}")).is_some(),
                "HelpCategoryBtn{i} missing"
            );
            assert!(
                reg.get_by_name(&format!("HelpCategoryBtn{i}Text"))
                    .is_some(),
                "HelpCategoryBtn{i}Text missing"
            );
        }
    }

    #[test]
    fn builds_content_area() {
        let reg = build_registry();
        assert!(reg.get_by_name("HelpContentArea").is_some());
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(HelpFrameState::default());
        Screen::new(help_frame_screen).sync(&shared, &mut reg);
        let id = reg.get_by_name("HelpFrame").expect("frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    // --- Coord validation ---

    #[test]
    fn coord_main_frame_centered() {
        let reg = layout_registry();
        let r = rect(&reg, "HelpFrame");
        let expected_x = (1920.0 - FRAME_W) / 2.0;
        let expected_y = (1080.0 - FRAME_H) / 2.0;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.y - expected_y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
    }

    #[test]
    fn coord_first_button() {
        let reg = layout_registry();
        let frame_x = (1920.0 - FRAME_W) / 2.0;
        let frame_y = (1080.0 - FRAME_H) / 2.0;
        let r = rect(&reg, "HelpCategoryBtn0");
        let expected_x = frame_x + (FRAME_W - BUTTON_W) / 2.0;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.y - (frame_y + BUTTON_TOP)).abs() < 1.0);
        assert!((r.width - BUTTON_W).abs() < 1.0);
        assert!((r.height - BUTTON_H).abs() < 1.0);
    }

    #[test]
    fn coord_button_spacing() {
        let reg = layout_registry();
        let b0 = rect(&reg, "HelpCategoryBtn0");
        let b1 = rect(&reg, "HelpCategoryBtn1");
        let spacing = b1.y - b0.y;
        let expected = BUTTON_H + BUTTON_GAP;
        assert!(
            (spacing - expected).abs() < 1.0,
            "spacing: expected {expected}, got {spacing}"
        );
    }

    // --- Article / detail / ticket tests ---

    fn make_article_state() -> HelpFrameState {
        HelpFrameState {
            visible: true,
            articles: vec![
                ArticleEntry {
                    title: "Getting Started".into(),
                    selected: true,
                },
                ArticleEntry {
                    title: "Combat Basics".into(),
                    selected: false,
                },
            ],
            detail: ArticleDetail {
                title: "Getting Started".into(),
                body: "Welcome to the game!".into(),
            },
            ticket: TicketFormState {
                category: "Bug".into(),
                description: String::new(),
            },
        }
    }

    fn article_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_article_state());
        Screen::new(help_frame_screen).sync(&shared, &mut reg);
        reg
    }

    #[test]
    fn builds_article_list() {
        let reg = article_registry();
        assert!(reg.get_by_name("HelpArticleList").is_some());
        for i in 0..2 {
            assert!(
                reg.get_by_name(&format!("HelpArticle{i}")).is_some(),
                "HelpArticle{i} missing"
            );
        }
    }

    #[test]
    fn builds_article_detail() {
        let reg = article_registry();
        assert!(reg.get_by_name("HelpArticleDetail").is_some());
        assert!(reg.get_by_name("HelpArticleDetailTitle").is_some());
        assert!(reg.get_by_name("HelpArticleDetailBody").is_some());
    }

    #[test]
    fn builds_ticket_form() {
        let reg = article_registry();
        assert!(reg.get_by_name("HelpTicketForm").is_some());
        assert!(reg.get_by_name("HelpTicketCategoryInput").is_some());
        assert!(reg.get_by_name("HelpTicketDescInput").is_some());
        assert!(reg.get_by_name("HelpTicketSubmitButton").is_some());
    }

    // --- Additional coord validation ---

    fn article_layout_registry() -> FrameRegistry {
        let mut reg = article_registry();
        recompute_layouts(&mut reg);
        reg
    }

    #[test]
    fn coord_article_list_position() {
        let reg = article_layout_registry();
        let frame_x = (1920.0 - FRAME_W) / 2.0;
        let frame_y = (1080.0 - FRAME_H) / 2.0;
        let r = rect(&reg, "HelpArticleList");
        let buttons_h = CATEGORY_BUTTONS.len() as f32 * (BUTTON_H + BUTTON_GAP);
        let expected_y = frame_y + BUTTON_TOP + buttons_h;
        assert!((r.x - (frame_x + CONTENT_INSET)).abs() < 1.0);
        assert!(
            (r.y - expected_y).abs() < 1.0,
            "y: expected {expected_y}, got {}",
            r.y
        );
    }

    #[test]
    fn coord_ticket_submit_button() {
        let reg = article_layout_registry();
        let r = rect(&reg, "HelpTicketSubmitButton");
        assert!((r.width - TICKET_BTN_W).abs() < 1.0);
        assert!((r.height - TICKET_BTN_H).abs() < 1.0);
    }
}
