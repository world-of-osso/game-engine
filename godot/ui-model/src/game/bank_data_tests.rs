use super::*;
use shared::protocol::{BankTabView, GuildBankLogEntry, GuildBankTabView};

fn bank(bank: BankType, tabs: usize) -> BankContents {
    BankContents {
        bank,
        tabs: (0..tabs)
            .map(|index| BankTabView {
                name: format!("Tab {}", index + 1),
                icon: 134_400,
                deposit_flags: 0,
                slots: vec![None; 98],
            })
            .collect(),
        next_tab_cost: Some(1_000_000),
        money: None,
    }
}

fn guild(object: u64, tabs: usize) -> GuildBankContents {
    GuildBankContents {
        object,
        guild_name: "Bank Testers".into(),
        tabs: (0..tabs)
            .map(|_| GuildBankTabView {
                name: "Tab".into(),
                icon: 0,
                viewable: true,
                can_deposit: true,
                withdrawals_per_day: None,
                remaining_withdrawals: None,
                text: String::new(),
                slots: vec![None; 98],
            })
            .collect(),
        money: 0,
        withdraw_money_remaining: None,
        next_tab_cost: None,
        is_leader: true,
    }
}

#[test]
fn a_new_banker_opens_on_the_character_bank_and_keeps_the_reagent_choice() {
    let mut state = BankState::default();
    state.include_reagents = true;
    state.open(7);
    state.apply(bank(BankType::Character, 2));
    state.apply(bank(BankType::Account, 1));
    state.show(BankType::Account);
    state.open(9);
    assert_eq!(state.npc, Some(9));
    assert_eq!(state.shown, BankType::Character);
    assert!(state.include_reagents);
    state.close();
    assert!(state.character.is_none());
}

#[test]
fn contents_that_arrive_before_the_opening_are_kept() {
    let mut state = BankState::default();
    state.apply(bank(BankType::Account, 1));
    state.open(7);
    assert_eq!(state.account.as_ref().unwrap().tabs.len(), 1);

    let mut guild_state = GuildBankState::default();
    guild_state.apply(guild(9, 2));
    guild_state.open(9);
    assert_eq!(guild_state.contents.as_ref().unwrap().tabs.len(), 2);
    guild_state.close();
    guild_state.apply(guild(9, 2));
    guild_state.open(11);
    assert!(guild_state.contents.is_none());
}

#[test]
fn side_tabs_select_purchased_tabs_and_the_purchase_tab_per_bank() {
    let mut state = BankState::default();
    state.open(7);
    state.apply(bank(BankType::Character, 3));
    let mut full = bank(BankType::Account, 5);
    full.next_tab_cost = None;
    state.apply(full);
    state.select_tab(2);
    state.select_tab(5);
    assert_eq!(state.selected_tab(BankType::Character), 2);
    // Tab index 3 is the purchase tab while a tab can be bought.
    state.select_tab(3);
    assert!(state.shows_purchase_prompt());
    state.show(BankType::Account);
    state.select_tab(5);
    assert_eq!(state.selected_tab(BankType::Account), 0);
    state.select_tab(4);
    assert_eq!(state.selected_tab(BankType::Account), 4);
    // Buying the tab keeps index 3 selected: it is now the new tab.
    state.apply(bank(BankType::Character, 4));
    assert_eq!(state.selected_tab(BankType::Character), 3);
    state.show(BankType::Character);
    assert!(!state.shows_purchase_prompt());
}

#[test]
fn a_bank_without_tabs_shows_the_purchase_prompt() {
    let mut state = BankState::default();
    state.open(7);
    state.apply(bank(BankType::Character, 0));
    assert!(state.shows_purchase_prompt());
    // The first purchase fills the selected index 0.
    state.apply(bank(BankType::Character, 1));
    assert!(!state.shows_purchase_prompt());
}

#[test]
fn guild_log_modes_query_their_log_and_drop_stale_answers() {
    let mut state = GuildBankState::default();
    state.open(9);
    state.apply(guild(9, 2));
    assert_eq!(
        state.set_mode(GuildBankMode::Log),
        Some(GuildBankRequest::QueryLog { tab: Some(0) })
    );
    assert_eq!(
        state.select_tab(1),
        Some(GuildBankRequest::QueryLog { tab: Some(1) })
    );
    state.apply_log(GuildBankLog {
        tab: Some(0),
        entries: Vec::new(),
    });
    assert!(state.log.is_none());
    state.apply_log(GuildBankLog {
        tab: Some(1),
        entries: Vec::new(),
    });
    assert!(state.log.is_some());
    assert_eq!(
        state.set_mode(GuildBankMode::MoneyLog),
        Some(GuildBankRequest::QueryLog { tab: None })
    );
    assert_eq!(state.set_mode(GuildBankMode::Bank), None);
    // Contents of another vault are ignored.
    state.apply(guild(11, 5));
    assert_eq!(state.contents.as_ref().unwrap().tabs.len(), 2);
}

#[test]
fn log_lines_use_retail_formats() {
    let entry = |kind, count, copper, seconds_ago| GuildBankLogEntry {
        kind,
        actor: "Bankalt".into(),
        item_id: 2589,
        item_name: "Linen Cloth".into(),
        count,
        copper,
        seconds_ago,
    };
    assert_eq!(
        log_line(&entry(GuildBankLogKind::DepositItem, 20, 0, 30)),
        "Bankalt deposited Linen Cloth x 20 ( 1 min ago )"
    );
    assert_eq!(
        log_line(&entry(GuildBankLogKind::WithdrawMoney, 0, 12_345, 7_300)),
        "Bankalt withdrew 1g 23s 45c ( 2 hours ago )"
    );
    assert_eq!(
        log_line(&entry(GuildBankLogKind::BuyTab, 0, 1_000_000, 90_000)),
        "Bankalt purchased a guild bank tab for 100g ( 1 day ago )"
    );
}
