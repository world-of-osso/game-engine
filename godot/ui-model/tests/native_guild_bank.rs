//! Behavioral guild-vault contract: portable native session, no ECS or renderer.
use game_engine_ui_model::bank_data::{GuildBankMode, GuildBankRequest};
use game_engine_ui_model::guild_bank::{GuildBankEffect, GuildBankSession};
use game_engine_ui_model::merchant::Click;
use shared::protocol::*;

fn contents(object: u64) -> GuildBankContents {
    GuildBankContents {
        object,
        guild_name: "Test Guild".into(),
        money: 50_000,
        withdraw_money_remaining: Some(10_000),
        next_tab_cost: Some(100_000),
        is_leader: true,
        tabs: vec![GuildBankTabView {
            name: "Supplies".into(),
            icon: 132_074,
            viewable: true,
            can_deposit: true,
            withdrawals_per_day: Some(2),
            remaining_withdrawals: Some(2),
            text: "Raid supplies".into(),
            slots: vec![Some(ItemStack {
                definition_source: shared::item_data::ItemDefinitionSource::Retail,
                item_guid: 71,
                item_id: 2589,
                count: 20,
                durability: None,
                soulbound: false,
            })],
        }],
    }
}
fn opened() -> GuildBankSession {
    let mut session = GuildBankSession::default();
    assert!(session.open_role(42, NpcRole::GuildBanker));
    session.apply_contents(contents(42));
    session
}
fn request(request: GuildBankRequest) -> GuildBankEffect {
    GuildBankEffect::Request {
        object: 42,
        request,
    }
}

#[test]
fn role_and_vault_bind_pending_contents_and_reject_foreign_updates() {
    let mut session = GuildBankSession::default();
    session.apply_contents(contents(42));
    assert!(!session.open_role(42, NpcRole::Vendor));
    assert!(!session.state.is_open());
    session.apply_contents(contents(42));
    assert!(session.open_role(42, NpcRole::GuildBanker));
    assert_eq!(session.frame_state().title, "Test Guild");
    let mut foreign = contents(99);
    foreign.guild_name = "Foreign".into();
    assert!(session.apply_contents(foreign).is_empty());
    assert_eq!(session.state.contents.as_ref().unwrap().object, 42);
    assert!(!session.close_for(99));
    assert!(session.close_for(42));
    assert!(!session.frame_state().visible);
    assert!(
        session
            .click("guild_bank_slot:1", Click::RIGHT, &Default::default())
            .is_empty()
    );
}

#[test]
fn item_money_requests_leave_inventory_and_guild_money_authoritative() {
    let mut session = opened();
    let before = session.state.contents.clone();
    assert_eq!(
        session.click("guild_bank_slot:1", Click::RIGHT, &Default::default()),
        vec![request(GuildBankRequest::Withdraw { tab: 0, slot: 0 })]
    );
    assert!(
        session
            .click("guild_bank_slot:1", Click::LEFT, &Default::default())
            .is_empty()
    );
    session.apply_inventory(InventorySnapshot {
        bags: vec![BagContents {
            bag: 0,
            size: 16,
            items: vec![BagSlotItem {
                slot: 0,
                item: ItemStack {
                    definition_source: shared::item_data::ItemDefinitionSource::Retail,
                    item_guid: 91,
                    item_id: 2589,
                    count: 3,
                    durability: None,
                    soulbound: false,
                },
            }],
        }],
    });
    let inventory = session.inventory.clone();
    assert_eq!(
        session.deposit_bag(0, 0),
        vec![request(GuildBankRequest::Deposit {
            tab: 0,
            item_guid: 91
        })]
    );
    session.money = 120_000;
    assert!(
        session
            .click("guild_bank_money_deposit", Click::LEFT, &Default::default())
            .is_empty()
    );
    let texts = [
        ("GuildBankMoneyGold".to_string(), "1".to_string()),
        ("GuildBankMoneySilver".to_string(), "2".to_string()),
        ("GuildBankMoneyCopper".to_string(), "3".to_string()),
    ]
    .into();
    assert_eq!(
        session.click("guild_bank_money_accept", Click::LEFT, &texts),
        vec![request(GuildBankRequest::Money {
            copper: 10_203,
            deposit: true
        })]
    );
    assert_eq!(session.money, 120_000);
    assert_eq!(session.inventory, inventory);
    assert_eq!(session.state.contents, before);
}

#[test]
fn log_refresh_and_stale_log_answers_follow_original_provider_ordering() {
    let mut session = opened();
    assert_eq!(
        session.set_mode(GuildBankMode::Log),
        vec![request(GuildBankRequest::QueryLog { tab: Some(0) })]
    );
    session.apply_log(GuildBankLog {
        tab: None,
        entries: vec![],
    });
    assert!(session.state.log.is_none());
    session.apply_log(GuildBankLog {
        tab: Some(0),
        entries: vec![GuildBankLogEntry {
            kind: GuildBankLogKind::DepositItem,
            actor: "Alice".into(),
            item_id: 2589,
            item_name: "Linen Cloth".into(),
            count: 20,
            copper: 0,
            seconds_ago: 60,
        }],
    });
    assert_eq!(
        session.frame_state().log_lines,
        vec!["Alice deposited Linen Cloth x 20 ( 1 min ago )"]
    );
    assert_eq!(
        session.apply_contents(contents(42)),
        vec![request(GuildBankRequest::QueryLog { tab: Some(0) })]
    );
    assert!(session.state.log.is_none());
    assert_eq!(
        session.set_mode(GuildBankMode::MoneyLog),
        vec![request(GuildBankRequest::QueryLog { tab: None })]
    );
    session.apply_log(GuildBankLog {
        tab: Some(0),
        entries: vec![],
    });
    assert!(session.state.log.is_none());
}

#[test]
fn the_money_log_shows_guild_repairs_in_retail_wording() {
    let mut session = opened();
    session.set_mode(GuildBankMode::MoneyLog);
    // GUILDBANK_REPAIR_MONEY_FORMAT "%s withdrew %s for repairs" (GB.lua:797-798).
    session.apply_log(GuildBankLog {
        tab: None,
        entries: vec![GuildBankLogEntry {
            kind: GuildBankLogKind::RepairMoney,
            actor: "Alice".into(),
            item_id: 0,
            item_name: String::new(),
            count: 0,
            copper: 15_016,
            seconds_ago: 5,
        }],
    });
    assert_eq!(
        session.frame_state().log_lines,
        vec!["Alice withdrew 1g 50s 16c for repairs ( 1 min ago )"]
    );
}

#[test]
fn tabs_permissions_and_allowances_are_server_supplied_not_invented_rules() {
    let mut session = opened();
    for (deposit, withdrawals, suffix) in [
        (true, Some(2), "(Full Access)"),
        (false, Some(2), "(Withdraw Only)"),
        (true, Some(0), "(Deposit Only)"),
        (false, Some(0), "(Locked)"),
    ] {
        let mut c = contents(42);
        c.tabs[0].can_deposit = deposit;
        c.tabs[0].withdrawals_per_day = withdrawals;
        c.tabs[0].remaining_withdrawals = Some(0);
        c.withdraw_money_remaining = Some(0);
        session.apply_contents(c);
        let view = session.frame_state();
        assert_eq!(view.tab_title.unwrap().1, suffix);
        assert_eq!(
            view.limit_text.unwrap(),
            "Remaining Daily Withdrawals for Supplies:  None"
        );
        assert!(!view.can_withdraw);
        assert_eq!(view.locked, suffix == "(Locked)");
    }
    let mut c = contents(42);
    c.tabs[0].viewable = false;
    c.tabs[0].slots.clear();
    session.apply_contents(c);
    assert!(session.frame_state().slots.is_empty());
    session.select_tab(999);
    assert_eq!(session.state.tab, 0);
}

#[test]
fn authoritative_inventory_delta_and_tab_purchase_replace_only_server_state() {
    let mut session = opened();
    session.apply_inventory(InventorySnapshot {
        bags: vec![BagContents {
            bag: 0,
            size: 16,
            items: vec![],
        }],
    });
    session.apply_inventory_delta(InventoryDelta {
        changes: vec![InventorySlotChange {
            location: ItemLocation::Bag { bag: 0, slot: 2 },
            item: Some(ItemStack {
                definition_source: shared::item_data::ItemDefinitionSource::Retail,
                item_guid: 81,
                item_id: 2589,
                count: 9,
                durability: None,
                soulbound: false,
            }),
        }],
    });
    assert_eq!(session.inventory.slot(0, 2).unwrap().count, 9);
    session.select_tab(1);
    let mut purchased = contents(42);
    let mut new_tab = purchased.tabs[0].clone();
    new_tab.name = "New tab".into();
    new_tab.slots = vec![None; 98];
    purchased.tabs.push(new_tab);
    session.apply_contents(purchased);
    assert_eq!(session.state.tab, 1);
    assert!(session.frame_state().buy.is_none());
    assert_eq!(session.frame_state().tab_title.unwrap().0, "New tab");
    session.close();
    assert!(session.inventory.slot(0, 2).is_some());
}

#[test]
fn no_tabs_nonleader_money_cancel_and_purchase_cancel_emit_no_requests() {
    let mut session = opened();
    let mut empty = contents(42);
    empty.tabs.clear();
    empty.is_leader = false;
    empty.next_tab_cost = None;
    empty.withdraw_money_remaining = None;
    session.apply_contents(empty);
    let view = session.frame_state();
    assert_eq!(
        view.error_message.as_deref(),
        Some("Your guild has not purchased any guild bank space.")
    );
    assert!(view.buy.is_none());
    assert!(view.withdraw_limit.is_none());
    assert!(
        session
            .click("guild_bank_money_deposit", Click::LEFT, &Default::default())
            .is_empty()
    );
    assert_eq!(session.frame_state().money_prompt, Some(true));
    assert!(
        session
            .click("guild_bank_money_cancel", Click::LEFT, &Default::default())
            .is_empty()
    );
    assert!(session.frame_state().money_prompt.is_none());
    session.apply_contents(contents(42));
    session.click("guild_bank_buy_tab", Click::LEFT, &Default::default());
    assert!(session.confirm_purchase(false).is_empty());
    assert!(session.purchase_confirmation.is_none());
}

#[test]
fn purchase_info_denial_and_close_are_bound_to_current_vault() {
    let mut session = opened();
    session.money = 120_000;
    session.select_tab(1);
    assert_eq!(session.frame_state().buy.unwrap().cost, 100_000);
    assert!(
        session
            .click("guild_bank_buy_tab", Click::LEFT, &Default::default())
            .is_empty()
    );
    assert!(session.purchase_confirmation.is_some());
    assert_eq!(
        session.confirm_purchase(true),
        vec![request(GuildBankRequest::BuyTab)]
    );
    assert!(session.confirm_purchase(true).is_empty());
    session.select_tab(0);
    session.set_mode(GuildBankMode::Info);
    assert_eq!(
        session.frame_state().info,
        Some(("Raid supplies".into(), true))
    );
    let texts = [(
        "GuildBankTabInfoEditBox".to_string(),
        "New info".to_string(),
    )]
    .into();
    assert_eq!(
        session.click("guild_bank_save_info", Click::LEFT, &texts),
        vec![request(GuildBankRequest::SetTabText {
            tab: 0,
            text: "New info".into()
        })]
    );
    session.apply_failed(GuildBankFailed {
        object: 99,
        error: GuildBankError::Permissions,
    });
    assert!(session.error.is_none());
    session.apply_failed(GuildBankFailed {
        object: 42,
        error: GuildBankError::Permissions,
    });
    assert_eq!(
        session.error.as_deref(),
        Some("You don't have permission to do that.")
    );
    assert_eq!(
        session.close(),
        vec![GuildBankEffect::CloseInteraction { npc: 42 }]
    );
    assert!(session.close().is_empty());
    assert!(session.state.contents.is_none());
}
