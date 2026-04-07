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

pub const FRAME_W: f32 = 530.0;
pub const FRAME_H: f32 = 490.0;
const HEADER_H: f32 = 28.0;
const TAB_BTN_SIZE: f32 = 36.0;
const TAB_BTN_GAP: f32 = 4.0;
const TAB_ROW_INSET: f32 = 8.0;
const SLOT_SIZE: f32 = 36.0;
const SLOT_GAP: f32 = 4.0;
const GRID_COLS: usize = 14;
const GRID_INSET: f32 = 8.0;
const GRID_TOP: f32 = HEADER_H + TAB_BTN_SIZE + TAB_ROW_INSET;
const LOG_TAB_H: f32 = 28.0;
const LOG_TAB_GAP: f32 = 4.0;

pub const GUILD_BANK_SLOTS: usize = 98;
pub const MAX_BANK_TABS: usize = 8;
pub const MAX_LOG_ENTRIES: usize = 10;

// Money / deposit-withdraw / log layout
const MONEY_ROW_Y: f32 = FRAME_H - LOG_TAB_H - 8.0 - 30.0;
const MONEY_LABEL_W: f32 = 100.0;
const MONEY_VALUE_W: f32 = 120.0;
const BTN_W: f32 = 80.0;
const BTN_H: f32 = 22.0;
const BTN_GAP: f32 = 8.0;
const LOG_ROW_H: f32 = 16.0;
const LOG_INSET: f32 = 8.0;
const MONEY_COLOR: &str = "1.0,0.82,0.0,1.0";
const MONEY_LABEL_COLOR: &str = "0.8,0.8,0.8,1.0";
const BTN_BG: &str = "0.15,0.12,0.05,0.95";
const BTN_TEXT_COLOR: &str = "1.0,0.82,0.0,1.0";
const LOG_TEXT_COLOR: &str = "0.8,0.8,0.8,1.0";

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const TAB_BG: &str = "0.08,0.07,0.06,0.88";
const TAB_ACTIVE_BG: &str = "0.2,0.15,0.05,0.95";
const SLOT_BG: &str = "0.08,0.07,0.06,0.88";
const LOG_ACTIVE_BG: &str = "0.2,0.15,0.05,0.95";
const LOG_INACTIVE_BG: &str = "0.08,0.07,0.06,0.88";
const LOG_ACTIVE_COLOR: &str = "1.0,0.82,0.0,1.0";
const LOG_INACTIVE_COLOR: &str = "0.6,0.6,0.6,1.0";

#[derive(Clone, Debug, PartialEq)]
pub struct GuildBankTab {
    pub name: String,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransactionEntry {
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GuildBankFrameState {
    pub visible: bool,
    pub tabs: Vec<GuildBankTab>,
    pub log_tab_active: bool,
    pub guild_money: String,
    pub transactions: Vec<TransactionEntry>,
}

impl Default for GuildBankFrameState {
    fn default() -> Self {
        Self {
            visible: false,
            tabs: vec![
                GuildBankTab {
                    name: "Tab 1".into(),
                    active: true,
                },
                GuildBankTab {
                    name: "Tab 2".into(),
                    active: false,
                },
            ],
            log_tab_active: false,
            guild_money: "0g 0s 0c".into(),
            transactions: vec![],
        }
    }
}

pub fn guild_bank_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<GuildBankFrameState>()
        .expect("GuildBankFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "GuildBankFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "80",
                y: "-80",
            }
            {title_bar()}
            {tab_buttons_row(&state.tabs)}
            {slot_grid()}
            {money_row(&state.guild_money)}
            {deposit_withdraw_buttons()}
            {log_tabs(state.log_tab_active)}
            {transaction_log(&state.transactions, state.log_tab_active)}
        }
    }
}

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "GuildBankFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Guild Bank",
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

fn tab_buttons_row(tabs: &[GuildBankTab]) -> Element {
    tabs.iter()
        .enumerate()
        .take(MAX_BANK_TABS)
        .flat_map(|(i, tab)| {
            let x = TAB_ROW_INSET + i as f32 * (TAB_BTN_SIZE + TAB_BTN_GAP);
            tab_button(i, tab, x)
        })
        .collect()
}

fn tab_button(i: usize, tab: &GuildBankTab, x: f32) -> Element {
    let btn_id = DynName(format!("GuildBankTabBtn{i}"));
    let bg = if tab.active { TAB_ACTIVE_BG } else { TAB_BG };
    rsx! {
        r#frame {
            name: btn_id,
            width: {TAB_BTN_SIZE},
            height: {TAB_BTN_SIZE},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {-HEADER_H},
            }
        }
    }
}

fn slot_grid() -> Element {
    (0..GUILD_BANK_SLOTS)
        .flat_map(|i| {
            let col = i % GRID_COLS;
            let row = i / GRID_COLS;
            let x = GRID_INSET + col as f32 * (SLOT_SIZE + SLOT_GAP);
            let y = -(GRID_TOP + row as f32 * (SLOT_SIZE + SLOT_GAP));
            guild_slot(i, x, y)
        })
        .collect()
}

fn guild_slot(index: usize, x: f32, y: f32) -> Element {
    let slot_name = DynName(format!("GuildBankSlot{index}"));
    rsx! {
        r#frame {
            name: slot_name,
            width: {SLOT_SIZE},
            height: {SLOT_SIZE},
            background_color: SLOT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
        }
    }
}

fn log_tabs(log_active: bool) -> Element {
    let items_bg = if log_active {
        LOG_INACTIVE_BG
    } else {
        LOG_ACTIVE_BG
    };
    let items_color = if log_active {
        LOG_INACTIVE_COLOR
    } else {
        LOG_ACTIVE_COLOR
    };
    let log_bg = if log_active {
        LOG_ACTIVE_BG
    } else {
        LOG_INACTIVE_BG
    };
    let log_color = if log_active {
        LOG_ACTIVE_COLOR
    } else {
        LOG_INACTIVE_COLOR
    };
    let y = -(FRAME_H - LOG_TAB_H - 8.0);
    rsx! {
        r#frame {
            name: "GuildBankItemsTab",
            width: 80.0,
            height: {LOG_TAB_H},
            background_color: items_bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {GRID_INSET},
                y: {y},
            }
            fontstring {
                name: "GuildBankItemsTabLabel",
                width: 80.0,
                height: {LOG_TAB_H},
                text: "Items",
                font_size: 10.0,
                font_color: items_color,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
        }
        r#frame {
            name: "GuildBankLogTab",
            width: 80.0,
            height: {LOG_TAB_H},
            background_color: log_bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {GRID_INSET + 80.0 + LOG_TAB_GAP},
                y: {y},
            }
            fontstring {
                name: "GuildBankLogTabLabel",
                width: 80.0,
                height: {LOG_TAB_H},
                text: "Log",
                font_size: 10.0,
                font_color: log_color,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
        }
    }
}

fn money_row(guild_money: &str) -> Element {
    let y = -MONEY_ROW_Y;
    rsx! {
        fontstring {
            name: "GuildBankMoneyLabel",
            width: {MONEY_LABEL_W},
            height: 16.0,
            text: "Guild Gold:",
            font_size: 10.0,
            font_color: MONEY_LABEL_COLOR,
            justify_h: "RIGHT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {GRID_INSET},
                y: {y},
            }
        }
        fontstring {
            name: "GuildBankMoneyValue",
            width: {MONEY_VALUE_W},
            height: 16.0,
            text: guild_money,
            font_size: 10.0,
            font_color: MONEY_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {GRID_INSET + MONEY_LABEL_W + 4.0},
                y: {y},
            }
        }
    }
}

fn deposit_withdraw_buttons() -> Element {
    let y = -(MONEY_ROW_Y + 20.0);
    let deposit_x = GRID_INSET;
    let withdraw_x = GRID_INSET + BTN_W + BTN_GAP;
    rsx! {
        r#frame {
            name: "GuildBankDepositButton",
            width: {BTN_W},
            height: {BTN_H},
            background_color: BTN_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {deposit_x},
                y: {y},
            }
            fontstring {
                name: "GuildBankDepositButtonText",
                width: {BTN_W},
                height: {BTN_H},
                text: "Deposit",
                font_size: 10.0,
                font_color: BTN_TEXT_COLOR,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
        }
        r#frame {
            name: "GuildBankWithdrawButton",
            width: {BTN_W},
            height: {BTN_H},
            background_color: BTN_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {withdraw_x},
                y: {y},
            }
            fontstring {
                name: "GuildBankWithdrawButtonText",
                width: {BTN_W},
                height: {BTN_H},
                text: "Withdraw",
                font_size: 10.0,
                font_color: BTN_TEXT_COLOR,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
        }
    }
}

fn transaction_log(transactions: &[TransactionEntry], visible: bool) -> Element {
    let hide = !visible;
    let grid_rows = (GUILD_BANK_SLOTS + GRID_COLS - 1) / GRID_COLS;
    let log_y = -(GRID_TOP + grid_rows as f32 * (SLOT_SIZE + SLOT_GAP) + LOG_INSET);
    let log_w = FRAME_W - 2.0 * LOG_INSET;
    let rows: Element = transactions
        .iter()
        .enumerate()
        .take(MAX_LOG_ENTRIES)
        .flat_map(|(i, entry)| {
            let row_name = DynName(format!("GuildBankLogEntry{i}"));
            let row_y = -(i as f32 * LOG_ROW_H);
            rsx! {
                fontstring {
                    name: row_name,
                    width: {log_w},
                    height: {LOG_ROW_H},
                    text: {entry.text.as_str()},
                    font_size: 9.0,
                    font_color: LOG_TEXT_COLOR,
                    justify_h: "LEFT",
                    anchor {
                        point: AnchorPoint::TopLeft,
                        relative_point: AnchorPoint::TopLeft,
                        x: "0",
                        y: {row_y},
                    }
                }
            }
        })
        .collect();
    rsx! {
        r#frame {
            name: "GuildBankTransactionLog",
            width: {log_w},
            height: 200.0,
            hidden: hide,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {LOG_INSET},
                y: {log_y},
            }
            {rows}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_toolkit::layout::{LayoutRect, recompute_layouts};
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn make_test_state() -> GuildBankFrameState {
        GuildBankFrameState {
            visible: true,
            ..Default::default()
        }
    }

    fn build_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(guild_bank_frame_screen).sync(&shared, &mut reg);
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
        assert!(reg.get_by_name("GuildBankFrame").is_some());
        assert!(reg.get_by_name("GuildBankFrameTitle").is_some());
    }

    #[test]
    fn builds_tab_buttons() {
        let reg = build_registry();
        assert!(reg.get_by_name("GuildBankTabBtn0").is_some());
        assert!(reg.get_by_name("GuildBankTabBtn1").is_some());
    }

    #[test]
    fn builds_98_slots() {
        let reg = build_registry();
        for i in 0..GUILD_BANK_SLOTS {
            assert!(
                reg.get_by_name(&format!("GuildBankSlot{i}")).is_some(),
                "GuildBankSlot{i} missing"
            );
        }
        assert!(
            reg.get_by_name(&format!("GuildBankSlot{GUILD_BANK_SLOTS}"))
                .is_none()
        );
    }

    #[test]
    fn builds_log_tabs() {
        let reg = build_registry();
        assert!(reg.get_by_name("GuildBankItemsTab").is_some());
        assert!(reg.get_by_name("GuildBankLogTab").is_some());
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(GuildBankFrameState::default());
        Screen::new(guild_bank_frame_screen).sync(&shared, &mut reg);
        let id = reg.get_by_name("GuildBankFrame").expect("frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    // --- Coord validation ---

    const FRAME_X: f32 = 80.0;
    const FRAME_Y: f32 = 80.0;

    #[test]
    fn coord_main_frame() {
        let reg = layout_registry();
        let r = rect(&reg, "GuildBankFrame");
        assert!((r.x - FRAME_X).abs() < 1.0);
        assert!((r.y - FRAME_Y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
    }

    #[test]
    fn coord_first_slot() {
        let reg = layout_registry();
        let r = rect(&reg, "GuildBankSlot0");
        assert!((r.x - (FRAME_X + GRID_INSET)).abs() < 1.0);
        assert!((r.y - (FRAME_Y + GRID_TOP)).abs() < 1.0);
        assert!((r.width - SLOT_SIZE).abs() < 1.0);
    }

    #[test]
    fn coord_first_tab_button() {
        let reg = layout_registry();
        let r = rect(&reg, "GuildBankTabBtn0");
        assert!((r.x - (FRAME_X + TAB_ROW_INSET)).abs() < 1.0);
        assert!((r.y - (FRAME_Y + HEADER_H)).abs() < 1.0);
        assert!((r.width - TAB_BTN_SIZE).abs() < 1.0);
    }

    // --- Money / deposit-withdraw / log tests ---

    #[test]
    fn builds_money_display() {
        let reg = build_registry();
        assert!(reg.get_by_name("GuildBankMoneyLabel").is_some());
        assert!(reg.get_by_name("GuildBankMoneyValue").is_some());
    }

    #[test]
    fn builds_deposit_withdraw_buttons() {
        let reg = build_registry();
        assert!(reg.get_by_name("GuildBankDepositButton").is_some());
        assert!(reg.get_by_name("GuildBankWithdrawButton").is_some());
    }

    #[test]
    fn builds_transaction_log() {
        let mut state = make_test_state();
        state.log_tab_active = true;
        state.transactions = vec![
            TransactionEntry {
                text: "Alice deposited 10g".into(),
            },
            TransactionEntry {
                text: "Bob withdrew Sword".into(),
            },
        ];
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(state);
        Screen::new(guild_bank_frame_screen).sync(&shared, &mut reg);

        assert!(reg.get_by_name("GuildBankTransactionLog").is_some());
        assert!(reg.get_by_name("GuildBankLogEntry0").is_some());
        assert!(reg.get_by_name("GuildBankLogEntry1").is_some());
    }

    #[test]
    fn transaction_log_hidden_when_items_tab() {
        let reg = build_registry();
        let id = reg.get_by_name("GuildBankTransactionLog").expect("log");
        let frame = reg.get(id).expect("data");
        assert!(frame.hidden, "log should be hidden when items tab active");
    }
}
