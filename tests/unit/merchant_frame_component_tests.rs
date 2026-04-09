use game_engine::ui::screens::merchant_frame_component::*;
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
                action: "merchant_buy:1".into(),
            },
            MerchantItem {
                name: "Light Shot".into(),
                price: "10c".into(),
                icon_fdid: 0,
                action: "merchant_buy:2".into(),
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

fn fontstring_text(reg: &FrameRegistry, name: &str) -> String {
    use ui_toolkit::frame::WidgetData;
    let id = reg.get_by_name(name).expect(name);
    let frame = reg.get(id).expect("frame data");
    match frame.widget_data.as_ref() {
        Some(WidgetData::FontString(fs)) => fs.text.clone(),
        _ => panic!("{name} is not a FontString"),
    }
}

fn build_with_state(state: MerchantFrameState) -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(merchant_frame_screen).sync(&shared, &mut reg);
    reg
}

const FRAME_X: f32 = 50.0;
const FRAME_Y: f32 = 80.0;

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

#[test]
fn builds_repair_buttons() {
    let reg = build_registry();
    assert!(reg.get_by_name("MerchantRepairButton").is_some());
    assert!(reg.get_by_name("MerchantGuildRepairButton").is_some());
}

#[test]
fn builds_money_display() {
    let reg = build_registry();
    assert!(reg.get_by_name("MerchantMoneyDisplay").is_some());
}

#[test]
fn coord_repair_button_spacing() {
    let reg = layout_registry();
    let repair = rect(&reg, "MerchantRepairButton");
    let guild = rect(&reg, "MerchantGuildRepairButton");
    assert!((repair.width - REPAIR_BTN_W).abs() < 1.0);
    let spacing = guild.x - repair.x;
    let expected = REPAIR_BTN_W + REPAIR_GAP;
    assert!(
        (spacing - expected).abs() < 1.0,
        "spacing: expected {expected}, got {spacing}"
    );
}

#[test]
fn title_text() {
    let reg = build_registry();
    assert_eq!(fontstring_text(&reg, "MerchantFrameTitle"), "Merchant");
}

#[test]
fn tab_labels() {
    let reg = build_registry();
    assert_eq!(fontstring_text(&reg, "MerchantTab0Label"), "Buy");
    assert_eq!(fontstring_text(&reg, "MerchantTab1Label"), "Sell");
    assert_eq!(fontstring_text(&reg, "MerchantTab2Label"), "Buyback");
}

#[test]
fn item_name_and_price() {
    let reg = build_registry();
    assert_eq!(fontstring_text(&reg, "MerchantItem0Name"), "Rough Arrow");
    assert_eq!(fontstring_text(&reg, "MerchantItem0Price"), "10c");
    assert_eq!(fontstring_text(&reg, "MerchantItem1Name"), "Light Shot");
    assert_eq!(fontstring_text(&reg, "MerchantItem1Price"), "10c");
}

#[test]
fn repair_button_labels() {
    let reg = build_registry();
    assert_eq!(
        fontstring_text(&reg, "MerchantRepairButtonText"),
        "Repair All"
    );
    assert_eq!(
        fontstring_text(&reg, "MerchantGuildRepairButtonText"),
        "Guild Repair"
    );
}

#[test]
fn money_display_text() {
    let reg = build_registry();
    assert_eq!(fontstring_text(&reg, "MerchantMoneyDisplay"), "0g 0s 0c");
}

#[test]
fn money_display_custom() {
    let mut state = make_test_state();
    state.player_money = "150g 30s 5c".into();
    let reg = build_with_state(state);
    assert_eq!(fontstring_text(&reg, "MerchantMoneyDisplay"), "150g 30s 5c");
}

#[test]
fn page_label_text() {
    let reg = build_registry();
    assert_eq!(fontstring_text(&reg, "MerchantPageLabel"), "Page 1/1");
}

#[test]
fn page_label_multipage() {
    let mut state = make_test_state();
    state.page = 2;
    state.total_pages = 5;
    let reg = build_with_state(state);
    assert_eq!(fontstring_text(&reg, "MerchantPageLabel"), "Page 2/5");
}

#[test]
fn page_button_labels() {
    let reg = build_registry();
    assert_eq!(fontstring_text(&reg, "MerchantPagePrevText"), "<");
    assert_eq!(fontstring_text(&reg, "MerchantPageNextText"), ">");
}

#[test]
fn item_rows_capped() {
    let items: Vec<MerchantItem> = (0..15)
        .map(|i| MerchantItem {
            name: format!("Item {i}"),
            price: "1g".into(),
            icon_fdid: 0,
            action: format!("merchant_buy:{i}"),
        })
        .collect();
    let reg = build_with_state(MerchantFrameState {
        visible: true,
        items,
        ..Default::default()
    });
    for i in 0..MERCHANT_ITEM_ROWS {
        assert!(
            reg.get_by_name(&format!("MerchantItem{i}")).is_some(),
            "MerchantItem{i} missing"
        );
    }
    assert!(
        reg.get_by_name(&format!("MerchantItem{MERCHANT_ITEM_ROWS}"))
            .is_none()
    );
}

#[test]
fn renders_buyback_items_when_buyback_tab_is_active() {
    let reg = build_with_state(MerchantFrameState {
        visible: true,
        tabs: vec![
            MerchantTab {
                name: "Buy".into(),
                active: false,
                action: format!("{ACTION_TAB_PREFIX}buy"),
            },
            MerchantTab {
                name: "Sell".into(),
                active: false,
                action: format!("{ACTION_TAB_PREFIX}sell"),
            },
            MerchantTab {
                name: "Buyback".into(),
                active: true,
                action: format!("{ACTION_TAB_PREFIX}buyback"),
            },
        ],
        items: vec![MerchantItem {
            name: "Bent Sword".into(),
            price: "25s".into(),
            icon_fdid: 0,
            action: "merchant_buyback:3".into(),
        }],
        ..Default::default()
    });

    assert_eq!(fontstring_text(&reg, "MerchantItem0Name"), "Bent Sword");
    assert_eq!(fontstring_text(&reg, "MerchantItem0Price"), "25s");
}

#[test]
fn renders_empty_text_for_empty_buyback_tab() {
    let reg = build_with_state(MerchantFrameState {
        visible: true,
        tabs: vec![
            MerchantTab {
                name: "Buy".into(),
                active: false,
                action: format!("{ACTION_TAB_PREFIX}buy"),
            },
            MerchantTab {
                name: "Sell".into(),
                active: false,
                action: format!("{ACTION_TAB_PREFIX}sell"),
            },
            MerchantTab {
                name: "Buyback".into(),
                active: true,
                action: format!("{ACTION_TAB_PREFIX}buyback"),
            },
        ],
        empty_text: Some("No items available for buyback.".into()),
        ..Default::default()
    });

    assert_eq!(
        fontstring_text(&reg, "MerchantEmptyText"),
        "No items available for buyback."
    );
}
