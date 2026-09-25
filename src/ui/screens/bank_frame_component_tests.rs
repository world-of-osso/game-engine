use ui_toolkit::layout::LayoutRect;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use super::*;
use crate::ui::screens::menu_character_layout_test_support::compute_layout;
use crate::ui::screens::screen_test_helpers::fontstring_text;

fn build(state: BankFrameState) -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(bank_frame_screen).sync(&shared, &mut reg);
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

fn linen() -> SlotItem {
    SlotItem {
        icon_fdid: 132_889,
        count: 20,
        quality_border: WHITE.into(),
    }
}

/// Two purchased character tabs, Linen in slot 1 of the first.
fn character_bank() -> BankFrameState {
    let mut slots = vec![None; 98];
    slots[0] = Some(linen());
    BankFrameState {
        visible: true,
        title: "Bank".into(),
        account: false,
        header: "Tab 1".into(),
        tabs: vec![
            SideTab {
                icon_fdid: 134_400,
                selected: true,
            },
            SideTab {
                icon_fdid: 132_889,
                selected: false,
            },
        ],
        purchase_tab: Some(false),
        slots,
        purchase: None,
        money: None,
        deposit_all_label: "Deposit All Reagents".into(),
        include_reagents: None,
        prompt: None,
    }
}

#[test]
fn the_grid_lays_out_ninety_eight_slots_in_paired_columns_of_seven() {
    let reg = build(character_bank());
    // BF.lua:941-968: 26,-63; 10 below; 8 right within a pair, 19 between pairs.
    assert_eq!(offset(&reg, "BankFrameItem1"), (26.0, 63.0));
    assert_eq!(offset(&reg, "BankFrameItem2"), (26.0, 110.0));
    assert_eq!(offset(&reg, "BankFrameItem8"), (71.0, 63.0));
    assert_eq!(offset(&reg, "BankFrameItem15"), (127.0, 63.0));
    assert_eq!(
        offset(&reg, "BankFrameItem98"),
        (26.0 + 13.0 * 45.0 + 6.0 * 11.0, 345.0)
    );
    assert!(!exists(&reg, "BankFrameItem99"));
    assert_eq!(fontstring_text(&reg, "BankFrameItem1Count"), "20");
    assert!(!exists(&reg, "BankFrameItem2Icon"));
    assert_eq!(onclick(&reg, "BankFrameItem3"), Some("bank_slot:2".into()));
    assert_eq!(fontstring_text(&reg, "BankFrameHeader"), "Tab 1");
}

#[test]
fn side_tabs_stack_down_the_right_edge_with_the_purchase_tab_last() {
    let reg = build(character_bank());
    assert_eq!(offset(&reg, "BankFrameTab1"), (740.0, 25.0));
    assert_eq!(offset(&reg, "BankFrameTab2"), (740.0, 74.0));
    assert_eq!(offset(&reg, "BankFramePurchaseTab"), (740.0, 123.0));
    assert!(exists(&reg, "BankFrameTab1Selected"));
    assert!(!exists(&reg, "BankFrameTab2Selected"));
    assert_eq!(onclick(&reg, "BankFrameTab2"), Some("bank_tab:1".into()));
    assert_eq!(
        onclick(&reg, "BankFramePurchaseTab"),
        Some("bank_purchase_tab".into())
    );
    assert_eq!(
        onclick(&reg, "BankFrameDepositAllButton"),
        Some("bank_auto_deposit".into())
    );
    assert!(!exists(&reg, "BankFrameIncludeReagents"));
    assert!(!exists(&reg, "BankFrameMoneyFrameDepositButton"));
    // The character bank's bottom tab is selected; the Warband tab switches banks.
    assert_eq!(onclick(&reg, "BankFrameTabSystemTab1"), None);
    assert_eq!(
        onclick(&reg, "BankFrameTabSystemTab2"),
        Some("bank_show:account".into())
    );
}

#[test]
fn a_bank_without_tabs_shows_the_purchase_prompt_instead_of_slots() {
    let state = BankFrameState {
        account: true,
        title: "Warband Bank".into(),
        tabs: Vec::new(),
        purchase_tab: Some(true),
        slots: Vec::new(),
        purchase: Some(PurchasePromptView {
            title: "Warband Bank".into(),
            text:
                "The Warband Bank offers storage that is shared with all members of your Warband."
                    .into(),
            cost: 10_000_000,
            can_afford: false,
        }),
        money: Some(MoneyFrameView {
            money: 5_000,
            can_withdraw: true,
            can_deposit: true,
        }),
        ..character_bank()
    };
    let reg = build(state);
    assert!(!exists(&reg, "BankFrameItem1"));
    assert!(!exists(&reg, "BankFrameDepositAllButton"));
    assert_eq!(
        fontstring_text(&reg, "BankFramePurchasePromptTitle"),
        "Warband Bank"
    );
    // 1000g the player can't afford: the Purchase button is disabled.
    assert_eq!(onclick(&reg, "BankFramePurchasePromptButton"), None);
    assert_eq!(
        fontstring_text(&reg, "BankFramePurchasePromptMoneyAmount0"),
        "1000"
    );
    assert_eq!(
        onclick(&reg, "BankFrameMoneyFrameDepositButton"),
        Some("bank_money_deposit".into())
    );
    assert_eq!(
        offset(&reg, "BankFrameMoneyFrameWithdrawButton"),
        (524.0, 434.0)
    );
}

#[test]
fn prompts_show_the_money_entry_and_the_tab_settings() {
    let reg = build(BankFrameState {
        prompt: Some(BankPromptView::Money { deposit: false }),
        ..character_bank()
    });
    assert_eq!(
        fontstring_text(&reg, "BankMoneyPopupText"),
        "Amount to withdraw:"
    );
    assert!(exists(&reg, MONEY_BOXES.gold));
    assert_eq!(
        onclick(&reg, "BankMoneyPopupAccept"),
        Some("bank_money_accept".into())
    );

    let reg = build(BankFrameState {
        prompt: Some(BankPromptView::TabSettings {
            flags: 0x80,
            name_prompt: "Enter Bank Tab Name:".into(),
        }),
        ..character_bank()
    });
    assert!(exists(&reg, TAB_NAME_BOX));
    // Reagents (0x80) is the fourth assignment and the only one checked.
    assert!(exists(&reg, "BankFrameTabSettingsAssign3Check"));
    assert!(!exists(&reg, "BankFrameTabSettingsAssign0Check"));
    assert_eq!(
        onclick(&reg, "BankFrameTabSettingsAssign0"),
        Some("bank_settings_flag:2".into())
    );
}
