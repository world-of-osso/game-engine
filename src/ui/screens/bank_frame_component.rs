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

pub const FRAME_W: f32 = 425.0;
pub const FRAME_H: f32 = 430.0;
const HEADER_H: f32 = 28.0;
const SLOT_SIZE: f32 = 36.0;
const SLOT_GAP: f32 = 4.0;
const GRID_COLS: usize = 7;
const INSET: f32 = 8.0;
const BAG_SLOT_SIZE: f32 = 30.0;
const BAG_SLOT_GAP: f32 = 4.0;
const BAG_ROW_TOP: f32 = 208.0;
const TAB_H: f32 = 28.0;
const TAB_GAP: f32 = 4.0;
const TAB_INSET: f32 = 12.0;

pub const BANK_SLOT_COUNT: usize = 28;
pub const BANK_BAG_SLOT_COUNT: usize = 7;
pub const REAGENT_SLOT_COUNT: usize = 98;
const REAGENT_GRID_COLS: usize = 7;
const REAGENT_GRID_TOP: f32 = HEADER_H + TAB_GAP + TAB_H + TAB_GAP;
const PURCHASE_BTN_W: f32 = 140.0;
const PURCHASE_BTN_H: f32 = 28.0;
const PURCHASE_BTN_BG: &str = "0.2,0.15,0.05,0.95";
const PURCHASE_BTN_TEXT: &str = "1.0,0.82,0.0,1.0";
const LOCKED_SLOT_BG: &str = "0.04,0.04,0.04,0.6";

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const SLOT_BG: &str = "0.08,0.07,0.06,0.88";
const BAG_SLOT_BG: &str = "0.08,0.07,0.06,0.88";
const TAB_BG_ACTIVE: &str = "0.2,0.15,0.05,0.95";
const TAB_BG_INACTIVE: &str = "0.08,0.07,0.06,0.88";
const TAB_TEXT_ACTIVE: &str = "1.0,0.82,0.0,1.0";
const TAB_TEXT_INACTIVE: &str = "0.6,0.6,0.6,1.0";
const BAG_ROW_LABEL_COLOR: &str = "0.8,0.8,0.8,1.0";

#[derive(Clone, Debug, PartialEq)]
pub struct BankTab {
    pub name: String,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BankFrameState {
    pub visible: bool,
    pub tabs: Vec<BankTab>,
    /// Number of reagent bank slots unlocked (0 = not purchased).
    pub reagent_slots_unlocked: usize,
}

impl Default for BankFrameState {
    fn default() -> Self {
        Self {
            visible: false,
            tabs: vec![
                BankTab {
                    name: "Bank".into(),
                    active: true,
                },
                BankTab {
                    name: "Reagent Bank".into(),
                    active: false,
                },
            ],
            reagent_slots_unlocked: 0,
        }
    }
}

pub fn bank_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<BankFrameState>()
        .expect("BankFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "BankFrame",
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
            {bank_slot_grid()}
            {bag_slots_row()}
            {reagent_bank_tab(state.reagent_slots_unlocked)}
        }
    }
}

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "BankFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Bank",
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

fn tab_row(tabs: &[BankTab]) -> Element {
    let count = tabs.len().max(1) as f32;
    let tab_w = (FRAME_W - 2.0 * TAB_INSET - (count - 1.0) * TAB_GAP) / count;
    let tab_y = -(HEADER_H + TAB_GAP);
    tabs.iter()
        .enumerate()
        .flat_map(|(i, tab)| {
            let x = TAB_INSET + i as f32 * (tab_w + TAB_GAP);
            tab_button(i, tab, tab_w, x, tab_y)
        })
        .collect()
}

fn tab_button(i: usize, tab: &BankTab, tab_w: f32, x: f32, y: f32) -> Element {
    let tab_id = DynName(format!("BankTab{i}"));
    let label_id = DynName(format!("BankTab{i}Label"));
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
            {bank_tab_label(label_id, &tab.name, tab_w, color)}
        }
    }
}

fn bank_tab_label(id: DynName, text: &str, w: f32, color: &str) -> Element {
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

fn bank_slot_grid() -> Element {
    let grid_top = HEADER_H + TAB_GAP + TAB_H + TAB_GAP;
    (0..BANK_SLOT_COUNT)
        .flat_map(|i| {
            let col = i % GRID_COLS;
            let row = i / GRID_COLS;
            let x = INSET + col as f32 * (SLOT_SIZE + SLOT_GAP);
            let y = -(grid_top + row as f32 * (SLOT_SIZE + SLOT_GAP));
            bank_slot(i, x, y)
        })
        .collect()
}

fn bank_slot(index: usize, x: f32, y: f32) -> Element {
    let slot_name = DynName(format!("BankSlot{index}"));
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

fn bag_slots_row() -> Element {
    let label = bag_row_label();
    let slots: Element = (0..BANK_BAG_SLOT_COUNT)
        .flat_map(|i| {
            let x = INSET + i as f32 * (BAG_SLOT_SIZE + BAG_SLOT_GAP);
            bank_bag_slot(i, x)
        })
        .collect();
    rsx! {
        {label}
        {slots}
    }
}

fn bag_row_label() -> Element {
    rsx! {
        fontstring {
            name: "BankBagSlotsLabel",
            width: {FRAME_W - 2.0 * INSET},
            height: 16.0,
            text: "Bag Slots",
            font_size: 10.0,
            font_color: BAG_ROW_LABEL_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INSET},
                y: {-(BAG_ROW_TOP - 18.0)},
            }
        }
    }
}

fn bank_bag_slot(index: usize, x: f32) -> Element {
    let slot_name = DynName(format!("BankBagSlot{index}"));
    rsx! {
        r#frame {
            name: slot_name,
            width: {BAG_SLOT_SIZE},
            height: {BAG_SLOT_SIZE},
            background_color: BAG_SLOT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {-BAG_ROW_TOP},
            }
        }
    }
}

// --- Reagent Bank Tab ---

fn reagent_bank_tab(slots_unlocked: usize) -> Element {
    let content_y = -REAGENT_GRID_TOP;
    let content_w = FRAME_W - 2.0 * INSET;
    let content_h = FRAME_H - REAGENT_GRID_TOP - INSET;
    let grid = reagent_slot_grid(slots_unlocked);
    let purchase = purchase_slot_button(slots_unlocked);
    rsx! {
        r#frame {
            name: "ReagentBankTab",
            width: {content_w},
            height: {content_h},
            hidden: true,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INSET},
                y: {content_y},
            }
            {grid}
            {purchase}
        }
    }
}

fn reagent_slot_grid(slots_unlocked: usize) -> Element {
    (0..REAGENT_SLOT_COUNT)
        .flat_map(|i| {
            let col = i % REAGENT_GRID_COLS;
            let row = i / REAGENT_GRID_COLS;
            let x = col as f32 * (SLOT_SIZE + SLOT_GAP);
            let y = -(row as f32 * (SLOT_SIZE + SLOT_GAP));
            let locked = i >= slots_unlocked;
            reagent_slot(i, x, y, locked)
        })
        .collect()
}

fn reagent_slot(index: usize, x: f32, y: f32, locked: bool) -> Element {
    let slot_name = DynName(format!("ReagentBankSlot{index}"));
    let bg = if locked { LOCKED_SLOT_BG } else { SLOT_BG };
    rsx! {
        r#frame {
            name: slot_name,
            width: {SLOT_SIZE},
            height: {SLOT_SIZE},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
        }
    }
}

fn purchase_slot_button(slots_unlocked: usize) -> Element {
    let all_unlocked = slots_unlocked >= REAGENT_SLOT_COUNT;
    let label = if all_unlocked {
        "All Slots Unlocked"
    } else {
        "Purchase Reagent Slot"
    };
    let rows = REAGENT_SLOT_COUNT.div_ceil(REAGENT_GRID_COLS);
    let grid_h = rows as f32 * SLOT_SIZE + (rows - 1) as f32 * SLOT_GAP;
    let btn_y = -(grid_h + INSET);
    rsx! {
        r#frame {
            name: "ReagentBankPurchaseButton",
            width: {PURCHASE_BTN_W},
            height: {PURCHASE_BTN_H},
            background_color: PURCHASE_BTN_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: {btn_y},
            }
            fontstring {
                name: "ReagentBankPurchaseButtonText",
                width: {PURCHASE_BTN_W},
                height: {PURCHASE_BTN_H},
                text: label,
                font_size: 10.0,
                font_color: PURCHASE_BTN_TEXT,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
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

    fn make_test_state() -> BankFrameState {
        BankFrameState {
            visible: true,
            ..Default::default()
        }
    }

    fn build_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(bank_frame_screen).sync(&shared, &mut reg);
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
        assert!(reg.get_by_name("BankFrame").is_some());
        assert!(reg.get_by_name("BankFrameTitle").is_some());
    }

    #[test]
    fn builds_28_bank_slots() {
        let reg = build_registry();
        for i in 0..BANK_SLOT_COUNT {
            assert!(
                reg.get_by_name(&format!("BankSlot{i}")).is_some(),
                "BankSlot{i} missing"
            );
        }
        assert!(
            reg.get_by_name(&format!("BankSlot{BANK_SLOT_COUNT}"))
                .is_none()
        );
    }

    #[test]
    fn builds_7_bag_slots() {
        let reg = build_registry();
        assert!(reg.get_by_name("BankBagSlotsLabel").is_some());
        for i in 0..BANK_BAG_SLOT_COUNT {
            assert!(
                reg.get_by_name(&format!("BankBagSlot{i}")).is_some(),
                "BankBagSlot{i} missing"
            );
        }
    }

    #[test]
    fn builds_tabs() {
        let reg = build_registry();
        assert!(reg.get_by_name("BankTab0").is_some());
        assert!(reg.get_by_name("BankTab1").is_some());
        assert!(reg.get_by_name("BankTab0Label").is_some());
        assert!(reg.get_by_name("BankTab1Label").is_some());
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        let mut state = make_test_state();
        state.visible = false;
        shared.insert(state);
        Screen::new(bank_frame_screen).sync(&shared, &mut reg);

        let id = reg.get_by_name("BankFrame").expect("frame");
        let frame = reg.get(id).expect("data");
        assert!(frame.hidden);
    }

    // --- Coord validation ---

    const FRAME_X: f32 = 50.0;
    const FRAME_Y: f32 = 80.0;

    #[test]
    fn coord_main_frame() {
        let reg = layout_registry();
        let r = rect(&reg, "BankFrame");
        assert!((r.x - FRAME_X).abs() < 1.0);
        assert!((r.y - FRAME_Y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
        assert!((r.height - FRAME_H).abs() < 1.0);
    }

    #[test]
    fn coord_first_bank_slot() {
        let reg = layout_registry();
        let r = rect(&reg, "BankSlot0");
        let grid_top = HEADER_H + TAB_GAP + TAB_H + TAB_GAP;
        assert!((r.x - (FRAME_X + INSET)).abs() < 1.0);
        assert!((r.y - (FRAME_Y + grid_top)).abs() < 1.0);
        assert!((r.width - SLOT_SIZE).abs() < 1.0);
    }

    #[test]
    fn coord_bag_slot_row() {
        let reg = layout_registry();
        let r = rect(&reg, "BankBagSlot0");
        assert!((r.x - (FRAME_X + INSET)).abs() < 1.0);
        assert!((r.y - (FRAME_Y + BAG_ROW_TOP)).abs() < 1.0);
        assert!((r.width - BAG_SLOT_SIZE).abs() < 1.0);
    }

    // --- Reagent bank tests ---

    #[test]
    fn reagent_tab_builds_root() {
        let reg = build_registry();
        assert!(reg.get_by_name("ReagentBankTab").is_some());
    }

    #[test]
    fn reagent_tab_builds_98_slots() {
        let reg = build_registry();
        for i in 0..REAGENT_SLOT_COUNT {
            assert!(
                reg.get_by_name(&format!("ReagentBankSlot{i}")).is_some(),
                "ReagentBankSlot{i} missing"
            );
        }
        assert!(
            reg.get_by_name(&format!("ReagentBankSlot{REAGENT_SLOT_COUNT}"))
                .is_none()
        );
    }

    #[test]
    fn reagent_tab_builds_purchase_button() {
        let reg = build_registry();
        assert!(reg.get_by_name("ReagentBankPurchaseButton").is_some());
        assert!(reg.get_by_name("ReagentBankPurchaseButtonText").is_some());
    }

    // --- Additional coord validation ---

    #[test]
    fn coord_tabs() {
        let reg = layout_registry();
        let tab_count = 2.0_f32;
        let tab_w = (FRAME_W - 2.0 * TAB_INSET - (tab_count - 1.0) * TAB_GAP) / tab_count;
        let tab_y = FRAME_Y + HEADER_H + TAB_GAP;
        let t0 = rect(&reg, "BankTab0");
        assert!((t0.x - (FRAME_X + TAB_INSET)).abs() < 1.0);
        assert!((t0.y - tab_y).abs() < 1.0);
        assert!((t0.width - tab_w).abs() < 1.0);
    }

    #[test]
    fn coord_reagent_first_slot() {
        let reg = layout_registry();
        let tab_x = FRAME_X + INSET;
        let tab_y = FRAME_Y + REAGENT_GRID_TOP;
        let r = rect(&reg, "ReagentBankSlot0");
        assert!(
            (r.x - tab_x).abs() < 1.0,
            "x: expected {tab_x}, got {}",
            r.x
        );
        assert!(
            (r.y - tab_y).abs() < 1.0,
            "y: expected {tab_y}, got {}",
            r.y
        );
        assert!((r.width - SLOT_SIZE).abs() < 1.0);
    }

    #[test]
    fn coord_purchase_button_dimensions() {
        let reg = layout_registry();
        let r = rect(&reg, "ReagentBankPurchaseButton");
        assert!((r.width - PURCHASE_BTN_W).abs() < 1.0);
        assert!((r.height - PURCHASE_BTN_H).abs() < 1.0);
    }

    #[test]
    fn coord_second_bank_slot_column() {
        let reg = layout_registry();
        let r = rect(&reg, "BankSlot1");
        let grid_top = HEADER_H + TAB_GAP + TAB_H + TAB_GAP;
        let expected_x = FRAME_X + INSET + SLOT_SIZE + SLOT_GAP;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.y - (FRAME_Y + grid_top)).abs() < 1.0);
    }

    // --- Text content tests ---

    fn fontstring_text(reg: &FrameRegistry, name: &str) -> String {
        use ui_toolkit::frame::WidgetData;
        let id = reg.get_by_name(name).expect(name);
        let frame = reg.get(id).expect("frame data");
        match frame.widget_data.as_ref() {
            Some(WidgetData::FontString(fs)) => fs.text.clone(),
            _ => panic!("{name} is not a FontString"),
        }
    }

    fn build_with_state(state: BankFrameState) -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(state);
        Screen::new(bank_frame_screen).sync(&shared, &mut reg);
        reg
    }

    #[test]
    fn tab_labels_show_names() {
        let reg = build_registry();
        assert_eq!(fontstring_text(&reg, "BankTab0Label"), "Bank");
        assert_eq!(fontstring_text(&reg, "BankTab1Label"), "Reagent Bank");
    }

    #[test]
    fn tab_switching_preserves_labels() {
        let mut state = make_test_state();
        state.tabs[0].active = false;
        state.tabs[1].active = true;
        let reg = build_with_state(state);

        assert!(reg.get_by_name("BankTab0").is_some());
        assert!(reg.get_by_name("BankTab1").is_some());
        assert_eq!(fontstring_text(&reg, "BankTab0Label"), "Bank");
        assert_eq!(fontstring_text(&reg, "BankTab1Label"), "Reagent Bank");
    }

    #[test]
    fn purchase_button_text_when_locked() {
        let reg = build_registry();
        assert_eq!(
            fontstring_text(&reg, "ReagentBankPurchaseButtonText"),
            "Purchase Reagent Slot"
        );
    }

    #[test]
    fn purchase_button_text_when_all_unlocked() {
        let mut state = make_test_state();
        state.reagent_slots_unlocked = REAGENT_SLOT_COUNT;
        let reg = build_with_state(state);
        assert_eq!(
            fontstring_text(&reg, "ReagentBankPurchaseButtonText"),
            "All Slots Unlocked"
        );
    }

    #[test]
    fn title_text() {
        let reg = build_registry();
        assert_eq!(fontstring_text(&reg, "BankFrameTitle"), "Bank");
    }

    #[test]
    fn bag_slots_label_text() {
        let reg = build_registry();
        assert_eq!(fontstring_text(&reg, "BankBagSlotsLabel"), "Bag Slots");
    }
}
