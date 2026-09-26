use ui_toolkit::frame::WidgetData;
use ui_toolkit::layout::LayoutRect;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use super::*;
use crate::ui::screens::menu_character_layout_test_support::compute_layout;
use crate::ui::screens::screen_test_helpers::fontstring_text;

fn build(state: MerchantFrameState) -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(merchant_frame_screen).sync(&shared, &mut reg);
    compute_layout(&mut reg);
    reg
}

fn exists(reg: &FrameRegistry, name: &str) -> bool {
    reg.get_by_name(name).is_some()
}

fn onclick(reg: &FrameRegistry, name: &str) -> Option<String> {
    reg.get(reg.get_by_name(name).expect(name))
        .unwrap()
        .onclick
        .clone()
        .filter(|action| !action.is_empty())
}

fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
    reg.get(reg.get_by_name(name).expect(name))
        .and_then(|frame| frame.layout_rect.clone())
        .unwrap_or_else(|| panic!("{name} has no layout rect"))
}

/// Offset of `name` from the MerchantFrame's top-left.
fn offset(reg: &FrameRegistry, name: &str) -> (f32, f32) {
    let root = rect(reg, FRAME_NAME);
    let child = rect(reg, name);
    (child.x - root.x, child.y - root.y)
}

fn text_color(reg: &FrameRegistry, name: &str) -> [f32; 4] {
    match reg
        .get(reg.get_by_name(name).expect(name))
        .unwrap()
        .widget_data
        .as_ref()
    {
        Some(WidgetData::FontString(fs)) => fs.color,
        _ => panic!("{name} is not a FontString"),
    }
}

fn texture_color(reg: &FrameRegistry, name: &str) -> [f32; 4] {
    match reg
        .get(reg.get_by_name(name).expect(name))
        .unwrap()
        .widget_data
        .as_ref()
    {
        Some(WidgetData::Texture(texture)) => texture.vertex_color,
        _ => panic!("{name} is not a Texture"),
    }
}

/// Godric Rothgar's first items, as the server sends them (world.db prices).
fn cell(name: &str, price: u64, action: &str) -> MerchantCell {
    MerchantCell {
        name: name.into(),
        name_color: "1.0,1.0,1.0,1.0",
        icon_fdid: 134_582,
        count: 1,
        stock: None,
        price,
        price_gray: false,
        tint: CellTint::Normal,
        action: action.into(),
    }
}

fn godric() -> MerchantFrameState {
    MerchantFrameState {
        visible: true,
        title: "Godric Rothgar".into(),
        cells: vec![
            cell("Large Round Shield", 78, "merchant_item:0"),
            cell("Tarnished Chain Vest", 89, "merchant_item:1"),
            cell("Tarnished Chain Belt", 45, "merchant_item:2"),
        ],
        repair: Some(true),
        money: 10_503,
        ..Default::default()
    }
}

#[test]
fn frame_is_the_retail_336_by_444_window_titled_with_the_vendor() {
    let reg = build(godric());
    let root = rect(&reg, FRAME_NAME);
    assert_eq!((root.width, root.height), (336.0, 444.0));
    assert_eq!(
        fontstring_text(&reg, "MerchantFrameTitleText"),
        "Godric Rothgar"
    );
    assert_eq!(
        onclick(&reg, "MerchantFrameCloseButton").as_deref(),
        Some(ACTION_CLOSE)
    );
}

#[test]
fn items_fill_the_two_column_grid_and_empty_slots_do_nothing() {
    let reg = build(godric());
    assert_eq!(offset(&reg, "MerchantItem1"), (11.0, 69.0));
    assert_eq!(offset(&reg, "MerchantItem2"), (176.0, 69.0));
    assert_eq!(offset(&reg, "MerchantItem3"), (11.0, 121.0));
    assert_eq!(offset(&reg, "MerchantItem10"), (176.0, 277.0));
    assert_eq!(
        fontstring_text(&reg, "MerchantItem2Name"),
        "Tarnished Chain Vest"
    );
    assert_eq!(
        onclick(&reg, "MerchantItem2").as_deref(),
        Some("merchant_item:1")
    );
    assert_eq!(onclick(&reg, "MerchantItem4"), None);
    assert!(!exists(&reg, "MerchantItem4Name"));
    assert!(!exists(&reg, "MerchantItem11"));
}

#[test]
fn prices_show_coins_and_unaffordable_prices_turn_gray() {
    let mut state = godric();
    state.cells[1].price_gray = true;
    state.cells[0].price = 10_503;
    let reg = build(state);

    let amounts: Vec<String> = (0..3)
        .map(|i| fontstring_text(&reg, &format!("MerchantItem1MoneyFrameAmount{i}")))
        .collect();
    assert_eq!(amounts, ["1", "5", "3"]);
    assert_eq!(
        fontstring_text(&reg, "MerchantItem2MoneyFrameAmount0"),
        "89"
    );
    assert!(!exists(&reg, "MerchantItem2MoneyFrameAmount1"));
    assert_eq!(text_color(&reg, "MerchantItem1MoneyFrameAmount0"), [1.0; 4]);
    assert_eq!(
        text_color(&reg, "MerchantItem2MoneyFrameAmount0"),
        [0.5, 0.5, 0.5, 1.0]
    );
}

#[test]
fn unusable_items_are_tinted_red_and_limited_stock_shows_its_count() {
    let mut state = godric();
    state.cells[0].tint = CellTint::Unusable;
    state.cells[2].stock = Some(1);
    let reg = build(state);

    assert_eq!(
        texture_color(&reg, "MerchantItem1SlotTexture"),
        [1.0, 0.0, 0.0, 1.0]
    );
    assert_eq!(texture_color(&reg, "MerchantItem2SlotTexture"), [1.0; 4]);
    assert_eq!(fontstring_text(&reg, "MerchantItem3ItemButtonStock"), "(1)");
    assert!(!exists(&reg, "MerchantItem1ItemButtonStock"));
}

#[test]
fn paging_shows_only_with_more_than_ten_items_and_disables_the_first_prev() {
    let reg = build(godric());
    assert!(!exists(&reg, "MerchantPageText"));

    let mut state = godric();
    state.page_text = Some("Page 1 of 2".into());
    state.next_enabled = true;
    let reg = build(state);
    assert_eq!(fontstring_text(&reg, "MerchantPageText"), "Page 1 of 2");
    assert_eq!(onclick(&reg, "MerchantPrevPageButton"), None);
    assert_eq!(
        onclick(&reg, "MerchantNextPageButton").as_deref(),
        Some(ACTION_PAGE_NEXT)
    );
}

#[test]
fn repair_buttons_follow_the_vendor_and_the_repair_cost() {
    let reg = build(godric());
    assert_eq!(
        onclick(&reg, "MerchantRepairAllButton").as_deref(),
        Some(ACTION_REPAIR_ALL)
    );
    // BOTTOMRIGHT at the frame's BOTTOMLEFT 118,33.
    assert_eq!(offset(&reg, "MerchantRepairAllButton"), (82.0, 375.0));

    let mut state = godric();
    state.repair = Some(false);
    let reg = build(state);
    assert_eq!(onclick(&reg, "MerchantRepairAllButton"), None);

    let mut state = godric();
    state.repair = None;
    let reg = build(state);
    assert!(!exists(&reg, "MerchantRepairAllButton"));
}

#[test]
fn sell_all_junk_sits_right_of_repair_all_and_is_enabled_only_with_junk() {
    let mut state = godric();
    state.has_junk = true;
    let reg = build(state);
    assert_eq!(
        onclick(&reg, "MerchantSellAllJunkButton").as_deref(),
        Some(ACTION_SELL_ALL_JUNK)
    );
    // RIGHT at MerchantRepairAllButton LEFT +80 (MF.lua:943).
    assert_eq!(offset(&reg, "MerchantSellAllJunkButton"), (126.0, 375.0));

    let reg = build(godric());
    assert_eq!(onclick(&reg, "MerchantSellAllJunkButton"), None);
    assert_eq!(
        texture_color(&reg, "MerchantSellAllJunkButtonIcon"),
        [0.4, 0.4, 0.4, 1.0]
    );

    // A vendor that cannot repair: BOTTOMRIGHT -148,33 (MF.lua:954).
    let mut state = godric();
    state.repair = None;
    let reg = build(state);
    assert_eq!(offset(&reg, "MerchantSellAllJunkButton"), (152.0, 375.0));
}

#[test]
fn the_frame_background_takes_cursor_drops() {
    let reg = build(godric());
    assert_eq!(onclick(&reg, FRAME_NAME).as_deref(), Some(ACTION_FRAME));
}

#[test]
fn player_money_sits_bottom_right_as_gold_silver_copper() {
    let reg = build(godric());
    let amounts: Vec<String> = (0..3)
        .map(|i| fontstring_text(&reg, &format!("MerchantMoneyFrameAmount{i}")))
        .collect();
    assert_eq!(amounts, ["1", "5", "3"]);
    let (right, bottom) = {
        let coin = rect(&reg, "MerchantMoneyFrameCoin2");
        let root = rect(&reg, FRAME_NAME);
        (coin.x + coin.width - root.x, coin.y + coin.height - root.y)
    };
    assert!((right - 326.0).abs() < 5.0, "money right edge {right}");
    assert!(bottom <= 436.0 && bottom > 425.0, "money bottom {bottom}");
}

#[test]
fn buyback_tab_shows_twelve_slots_and_hides_the_merchant_controls() {
    let mut state = godric();
    state.buyback_tab = true;
    state.cells = vec![cell("Linen Cloth", 65, "merchant_item:0")];
    let reg = build(state);

    assert_eq!(offset(&reg, "MerchantItem3"), (11.0, 128.0));
    assert_eq!(offset(&reg, "MerchantItem11"), (11.0, 364.0));
    assert!(exists(&reg, "MerchantItem12"));
    assert!(!exists(&reg, "MerchantRepairAllButton"));
    assert!(!exists(&reg, "MerchantSellAllJunkButton"));
    assert!(!exists(&reg, "MerchantBuyBackItem"));
    assert_eq!(onclick(&reg, "MerchantFrameTab2"), None);
    assert_eq!(
        onclick(&reg, "MerchantFrameTab1").as_deref(),
        Some(ACTION_TAB_MERCHANT)
    );
}

#[test]
fn last_sale_sits_in_the_buyback_slot_under_the_grid() {
    let reg = build(godric());
    assert_eq!(onclick(&reg, "MerchantBuyBackItem"), None);
    assert!(exists(&reg, "MerchantBuyBackItemUndoArrow"));

    let mut state = godric();
    state.last_buyback = Some(cell("Linen Cloth", 65, ACTION_BUYBACK_LAST));
    let reg = build(state);
    assert_eq!(offset(&reg, "MerchantBuyBackItem"), (206.0, 374.0));
    assert_eq!(
        fontstring_text(&reg, "MerchantBuyBackItemName"),
        "Linen Cloth"
    );
    assert_eq!(
        onclick(&reg, "MerchantBuyBackItem").as_deref(),
        Some(ACTION_BUYBACK_LAST)
    );
}
