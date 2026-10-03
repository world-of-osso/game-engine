use ui_toolkit::layout::LayoutRect;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use super::*;
use crate::ui::screens::menu_character_layout_test_support::compute_layout;
use crate::ui::screens::screen_test_helpers::fontstring_text;

fn build(state: GuildBankFrameState) -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(guild_bank_frame_screen).sync(&shared, &mut reg);
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

/// One purchased tab with 20 Linen in slot 1, seen by the Guild Master.
fn leader_view() -> GuildBankFrameState {
    let mut slots = vec![None; SLOTS];
    slots[0] = Some(SlotItem {
        icon_fdid: 132_889,
        count: 20,
        quality_border: WHITE.into(),
    });
    GuildBankFrameState {
        visible: true,
        title: "Bank Testers".into(),
        mode: GuildBankModeView::Bank,
        tabs: vec![GuildSideTab {
            icon_fdid: 134_400,
            selected: true,
        }],
        buy_tab: Some(false),
        tab_title: Some(("Tab 1".into(), "(Full Access)".into(), "0.13,1.0,0.13,1.0")),
        limit_text: Some("Remaining Daily Withdrawals for Tab 1:  Unlimited".into()),
        locked: false,
        slots,
        buy: None,
        error_message: None,
        log_lines: Vec::new(),
        info: None,
        money: 50_000,
        withdraw_limit: None,
        can_withdraw: true,
        money_prompt: None,
    }
}

#[test]
fn seven_columns_of_fourteen_hold_the_ninety_eight_slots() {
    let reg = build(leader_view());
    // Column1 18,-59 + Button1 7,-3; Button2 7 below; Button8 12 right; Column2 3 apart.
    assert_eq!(offset(&reg, "GuildBankFrameItem1"), (25.0, 62.0));
    assert_eq!(offset(&reg, "GuildBankFrameItem2"), (25.0, 106.0));
    assert_eq!(offset(&reg, "GuildBankFrameItem8"), (74.0, 62.0));
    assert_eq!(offset(&reg, "GuildBankFrameItem15"), (128.0, 62.0));
    assert_eq!(
        offset(&reg, "GuildBankFrameColumn7Background"),
        (636.0, 59.0)
    );
    assert!(!exists(&reg, "GuildBankFrameItem99"));
    assert_eq!(fontstring_text(&reg, "GuildBankFrameItem1Count"), "20");
    assert_eq!(
        onclick(&reg, "GuildBankFrameItem1"),
        Some("guild_bank_slot:1".into())
    );
    assert_eq!(fontstring_text(&reg, "GuildBankFrameTabTitle"), "Tab 1");
    assert_eq!(
        fontstring_text(&reg, "GuildBankFrameTabTitleAccess"),
        "(Full Access)"
    );
    assert_eq!(
        fontstring_text(&reg, "GuildBankFrameLimitLabel"),
        "Remaining Daily Withdrawals for Tab 1:  Unlimited"
    );
}

#[test]
fn tabs_stack_down_the_right_edge_and_modes_sit_below() {
    let reg = build(leader_view());
    assert_eq!(offset(&reg, "GuildBankTab1"), (749.0, 17.0));
    assert_eq!(offset(&reg, "GuildBankTab2"), (749.0, 67.0));
    assert_eq!(
        onclick(&reg, "GuildBankTab2"),
        Some("guild_bank_buy_tab_tab".into())
    );
    assert!(exists(&reg, "GuildBankTab1Checked"));
    assert_eq!(
        onclick(&reg, "GuildBankFrameTab2"),
        Some("guild_bank_mode:log".into())
    );
    // The shown mode's tab takes no click.
    assert_eq!(onclick(&reg, "GuildBankFrameTab1"), None);
    assert_eq!(
        onclick(&reg, "GuildBankFrameDepositButton"),
        Some("guild_bank_money_deposit".into())
    );
    assert!(exists(&reg, "GuildBankFrameMoneyFrameUnlimitedLabel"));
    assert_eq!(
        fontstring_text(&reg, "GuildBankFrameMoneyFrameMoneyAmount0"),
        "5"
    );
}

#[test]
fn a_member_without_withdraw_rights_sees_the_limit_and_a_disabled_withdraw() {
    let reg = build(GuildBankFrameState {
        buy_tab: None,
        withdraw_limit: Some(0),
        can_withdraw: false,
        locked: true,
        ..leader_view()
    });
    assert!(!exists(&reg, "GuildBankTab2"));
    assert_eq!(onclick(&reg, "GuildBankFrameWithdrawButton"), None);
    assert!(!exists(&reg, "GuildBankFrameMoneyFrameUnlimitedLabel"));
    assert_eq!(
        fontstring_text(&reg, "GuildBankFrameMoneyFrameWithdrawMoneyAmount0"),
        "0"
    );
}

#[test]
fn log_mode_lists_lines_and_buy_mode_shows_the_price() {
    let reg = build(GuildBankFrameState {
        mode: GuildBankModeView::Log,
        slots: Vec::new(),
        log_lines: vec![
            "Banker deposited Linen Cloth x 20 ( 1 min ago )".into(),
            "Bankalt withdrew Linen Cloth x 20 ( 1 min ago )".into(),
        ],
        ..leader_view()
    });
    assert!(!exists(&reg, "GuildBankFrameItem1"));
    assert_eq!(
        fontstring_text(&reg, "GuildBankFrameLogLine2"),
        "Bankalt withdrew Linen Cloth x 20 ( 1 min ago )"
    );
    assert_eq!(offset(&reg, "GuildBankFrameLogLine1"), (24.0, 64.0));
    assert!(!exists(&reg, "GuildBankFrameDepositButton"));

    let reg = build(GuildBankFrameState {
        buy: Some(BuyTabView {
            cost: 2_500_000,
            purchased: 1,
            can_afford: true,
        }),
        slots: Vec::new(),
        ..leader_view()
    });
    assert_eq!(
        fontstring_text(&reg, "GuildBankFrameBuyInfoPurchasedText"),
        "(1/6 tabs purchased)"
    );
    assert_eq!(
        onclick(&reg, "GuildBankFrameBuyInfoPurchaseButton"),
        Some("guild_bank_buy_tab".into())
    );
    assert_eq!(
        fontstring_text(&reg, "GuildBankFrameBuyInfoMoneyAmount0"),
        "250"
    );
}
