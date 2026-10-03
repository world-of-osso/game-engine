use ui_toolkit::frame::WidgetData;
use ui_toolkit::layout::LayoutRect;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use super::*;
use crate::ui::screens::menu_character_layout_test_support::compute_layout;
use crate::ui::screens::screen_test_helpers::fontstring_text;

fn build(state: TradeFrameState) -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(trade_frame_screen).sync(&shared, &mut reg);
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

fn linen() -> TradeItemView {
    TradeItemView {
        item: SlotItem {
            icon_fdid: 132_889,
            count: 20,
            quality_border: WHITE.into(),
        },
        name: "Linen Cloth".into(),
        name_color: WHITE.into(),
    }
}

fn open_trade() -> TradeFrameState {
    let mut player_items = vec![None; TRADE_SLOTS];
    player_items[0] = Some(linen());
    let mut recipient_items = vec![None; TRADE_SLOTS];
    recipient_items[1] = Some(TradeItemView {
        name: "Peacebloom".into(),
        name_color: "0.12,1.0,0.0,1.0".into(),
        ..linen()
    });
    TradeFrameState {
        visible: true,
        player_name: "Tradea".into(),
        recipient_name: "Tradeb".into(),
        player_items,
        recipient_items,
        recipient_money: 10_503,
        player_accepted: false,
        recipient_accepted: true,
    }
}

#[test]
fn both_sides_show_seven_slots_at_the_retail_offsets() {
    let reg = build(open_trade());
    let root = rect(&reg, FRAME_NAME);
    assert_eq!((root.width, root.height), (344.0, 446.0));
    assert_eq!(offset(&reg, "TradePlayerItem1ItemButton"), (14.0, 89.0));
    assert_eq!(offset(&reg, "TradePlayerItem2ItemButton"), (14.0, 133.0));
    assert_eq!(
        offset(&reg, "TradeRecipientItem6ItemButton"),
        (182.0, 309.0)
    );
    assert_eq!(
        offset(&reg, "TradeRecipientItem7ItemButton"),
        (182.0, 374.0)
    );
    assert_eq!(fontstring_text(&reg, "TradeFramePlayerNameText"), "Tradea");
    assert_eq!(
        fontstring_text(&reg, "TradeFrameRecipientNameText"),
        "Tradeb"
    );
}

#[test]
fn offered_items_show_names_in_quality_colour_and_only_player_slots_click() {
    let reg = build(open_trade());
    assert_eq!(fontstring_text(&reg, "TradePlayerItem1Name"), "Linen Cloth");
    assert_eq!(
        text_color(&reg, "TradeRecipientItem2Name"),
        [0.12, 1.0, 0.0, 1.0]
    );
    assert_eq!(
        onclick(&reg, "TradePlayerItem1ItemButton").as_deref(),
        Some("trade_player_slot:0")
    );
    assert_eq!(onclick(&reg, "TradeRecipientItem2ItemButton"), None);
    assert_eq!(
        onclick(&reg, "TradePlayerItem7ItemButton").as_deref(),
        Some("trade_player_slot:6")
    );
    // An empty seventh slot shows the enchant icon.
    assert!(exists(&reg, "TradePlayerItem7EnchantIcon"));
}

#[test]
fn accept_highlights_the_accepted_side_and_disables_trade_for_the_player() {
    let reg = build(open_trade());
    assert!(exists(&reg, "TradeHighlightRecipientTop"));
    assert!(!exists(&reg, "TradeHighlightPlayerTop"));
    assert_eq!(
        onclick(&reg, "TradeFrameTradeButton").as_deref(),
        Some(ACTION_TRADE)
    );

    let mut accepted = open_trade();
    accepted.player_accepted = true;
    let reg = build(accepted);
    assert!(exists(&reg, "TradeHighlightPlayerEnchantBottom"));
    assert_eq!(onclick(&reg, "TradeFrameTradeButton"), None);
    assert_eq!(
        onclick(&reg, "TradeFrameCancelButton").as_deref(),
        Some(ACTION_CANCEL)
    );
}

#[test]
fn the_partners_money_shows_in_coins() {
    let reg = build(open_trade());
    let amounts: Vec<String> = (0..3)
        .map(|i| fontstring_text(&reg, &format!("TradeRecipientMoneyFrameAmount{i}")))
        .collect();
    assert_eq!(amounts, ["1", "5", "3"]);
    // The compact entry fits the 162 wide money inset at 4,-58.
    let copper = rect(&reg, &format!("{}Unit", MONEY_BOXES.copper));
    let root = rect(&reg, FRAME_NAME);
    assert!(copper.x + copper.width - root.x <= 4.0 + 162.0);
    assert_eq!(offset(&reg, MONEY_BOXES.gold), (11.0, 61.0));
}
