use bevy::prelude::MouseButton;
use game_engine::bank_data::{BankPrompt, GuildBankMode};
use shared::protocol::{
    BankContents, BankTabView, BankType, GuildBankContents, GuildBankLog, GuildBankLogEntry,
    GuildBankLogKind, GuildBankTabView, ItemStack,
};

use super::actions::{self, InputTexts};
use super::view;
use super::*;

const BANKER: u64 = 0x0000_0001_0000_0990;
const VAULT: u64 = 0x0000_0001_0000_1D01;

fn linen(guid: u64) -> ItemStack {
    ItemStack {
        item_guid: guid,
        item_id: 2589,
        count: 20,
        durability: None,
        soulbound: false,
    }
}

fn contents(bank: BankType, tabs: usize, next: Option<u64>) -> BankContents {
    let mut slots = vec![None; 98];
    slots[0] = Some(linen(41));
    BankContents {
        bank,
        tabs: (0..tabs)
            .map(|index| BankTabView {
                name: format!("Tab {}", index + 1),
                icon: 134_400,
                deposit_flags: 0x80,
                slots: slots.clone(),
            })
            .collect(),
        next_tab_cost: next,
        money: (bank == BankType::Account).then_some(5_000),
    }
}

fn open_bank() -> BankState {
    let mut bank = BankState::default();
    bank.open(BANKER);
    bank.apply(contents(BankType::Character, 1, Some(1_000_000)));
    bank.apply(contents(BankType::Account, 0, Some(10_000_000)));
    bank
}

fn texts(pairs: &[(&'static str, &str)]) -> InputTexts {
    pairs.iter().map(|(k, v)| (*k, v.to_string())).collect()
}

#[test]
fn the_view_shows_the_selected_tab_or_the_purchase_prompt() {
    let mut bank = open_bank();
    let state = view::bank_frame_state(&bank, true, 100_000);
    assert!(state.visible);
    assert_eq!(state.title, "Bank");
    assert_eq!(state.header, "Tab 1");
    assert_eq!(state.slots.len(), 98);
    assert_eq!(state.slots[0].as_ref().unwrap().count, 20);
    assert_eq!(state.purchase_tab, Some(false));
    assert!(state.money.is_none());
    assert_eq!(state.deposit_all_label, "Deposit All Reagents");

    bank.show(BankType::Account);
    let state = view::bank_frame_state(&bank, true, 100_000);
    assert_eq!(state.title, "Warband Bank");
    assert!(state.slots.is_empty());
    let prompt = state.purchase.unwrap();
    assert_eq!(prompt.cost, 10_000_000);
    assert!(!prompt.can_afford);
    assert_eq!(state.money.unwrap().money, 5_000);
    assert_eq!(state.include_reagents, Some(false));

    bank.close();
    assert!(!view::bank_frame_state(&bank, true, 0).visible);
}

#[test]
fn right_clicks_withdraw_filled_slots_and_open_tab_settings() {
    let mut bank = open_bank();
    let none = InputTexts::new();
    let withdraw = actions::bank_action("bank_slot:0", MouseButton::Right, &mut bank, &none);
    assert_eq!(
        withdraw.requests,
        vec![BankRequest::Withdraw {
            bank: BankType::Character,
            tab: 0,
            slot: 0
        }]
    );
    assert!(
        actions::bank_action("bank_slot:1", MouseButton::Right, &mut bank, &none)
            .requests
            .is_empty()
    );
    assert!(
        actions::bank_action("bank_slot:0", MouseButton::Left, &mut bank, &none)
            .requests
            .is_empty()
    );

    let settings = actions::bank_action("bank_tab:0", MouseButton::Right, &mut bank, &none);
    assert_eq!(
        settings.texts,
        vec![(bank_ui::TAB_NAME_BOX, "Tab 1".to_string())]
    );
    assert_eq!(
        bank.prompt,
        Some(BankPrompt::TabSettings {
            tab: 0,
            flags: 0x80
        })
    );
    actions::bank_action("bank_settings_flag:2", MouseButton::Left, &mut bank, &none);
    let saved = actions::bank_action(
        "bank_settings_accept",
        MouseButton::Left,
        &mut bank,
        &texts(&[(bank_ui::TAB_NAME_BOX, " Cloth ")]),
    );
    assert_eq!(
        saved.requests,
        vec![BankRequest::UpdateTab {
            bank: BankType::Character,
            tab: 0,
            name: "Cloth".into(),
            icon: 134_400,
            deposit_flags: 0x82
        }]
    );
    assert!(bank.prompt.is_none());
}

#[test]
fn warband_money_entry_sends_the_typed_copper() {
    let mut bank = open_bank();
    let none = InputTexts::new();
    actions::bank_action("bank_show:account", MouseButton::Left, &mut bank, &none);
    let opened = actions::bank_action("bank_money_deposit", MouseButton::Left, &mut bank, &none);
    assert_eq!(opened.texts.len(), 3);
    assert_eq!(bank.prompt, Some(BankPrompt::DepositMoney));
    let typed = texts(&[
        (bank_ui::MONEY_BOXES.gold, "12"),
        (bank_ui::MONEY_BOXES.silver, "3"),
        (bank_ui::MONEY_BOXES.copper, ""),
    ]);
    let accepted = actions::bank_action("bank_money_accept", MouseButton::Left, &mut bank, &typed);
    assert_eq!(
        accepted.requests,
        vec![BankRequest::Money {
            bank: BankType::Account,
            copper: 120_300,
            deposit: true
        }]
    );
    // An empty entry sends nothing.
    actions::bank_action("bank_money_withdraw", MouseButton::Left, &mut bank, &none);
    assert!(
        actions::bank_action("bank_money_accept", MouseButton::Left, &mut bank, &none)
            .requests
            .is_empty()
    );
}

#[test]
fn purchase_asks_for_confirmation_with_the_price() {
    let mut bank = open_bank();
    let none = InputTexts::new();
    actions::bank_action("bank_show:account", MouseButton::Left, &mut bank, &none);
    let outcome = actions::bank_action("bank_purchase", MouseButton::Left, &mut bank, &none);
    let popup = outcome.popup.unwrap();
    assert_eq!(popup.key, actions::BUY_BANK_TAB_POPUP);
    assert_eq!(
        popup.text,
        "Do you want to purchase a Warband Bank tab for:\n1000g"
    );
    assert!(outcome.requests.is_empty());
}

fn vault_contents() -> GuildBankContents {
    let mut slots = vec![None; 98];
    slots[13] = Some(linen(55));
    GuildBankContents {
        object: VAULT,
        guild_name: "Bank Testers".into(),
        tabs: vec![GuildBankTabView {
            name: "Tab 1".into(),
            icon: 134_400,
            viewable: true,
            can_deposit: false,
            withdrawals_per_day: Some(2),
            remaining_withdrawals: Some(1),
            text: "Cloth for the tailors".into(),
            slots,
        }],
        money: 50_000,
        withdraw_money_remaining: Some(0),
        next_tab_cost: None,
        is_leader: false,
    }
}

#[test]
fn the_guild_view_shows_access_limits_and_logs() {
    let mut guild = GuildBankState::default();
    guild.open(VAULT);
    guild.apply(vault_contents());
    let state = view::guild_bank_frame_state(&guild, true, 0);
    assert_eq!(state.title, "Bank Testers");
    let (title, access, _) = state.tab_title.clone().unwrap();
    assert_eq!(
        (title.as_str(), access.as_str()),
        ("Tab 1", "(Withdraw Only)")
    );
    assert_eq!(
        state.limit_text.as_deref(),
        Some("Remaining Daily Withdrawals for Tab 1:  1 Stack")
    );
    assert_eq!(state.slots[13].as_ref().unwrap().count, 20);
    assert!(!state.can_withdraw);
    assert_eq!(state.buy_tab, None);

    guild.set_mode(GuildBankMode::Log);
    guild.apply_log(GuildBankLog {
        tab: Some(0),
        entries: vec![GuildBankLogEntry {
            kind: GuildBankLogKind::WithdrawItem,
            actor: "Bankalt".into(),
            item_id: 2589,
            item_name: "Linen Cloth".into(),
            count: 20,
            copper: 0,
            seconds_ago: 120,
        }],
    });
    let state = view::guild_bank_frame_state(&guild, true, 0);
    assert!(state.slots.is_empty());
    assert_eq!(
        state.log_lines,
        vec!["Bankalt withdrew Linen Cloth x 20 ( 2 min ago )".to_string()]
    );
}

#[test]
fn guild_clicks_withdraw_by_slot_id_and_switch_modes() {
    let mut guild = GuildBankState::default();
    guild.open(VAULT);
    guild.apply(vault_contents());
    let none = InputTexts::new();
    let withdraw =
        actions::guild_bank_action("guild_bank_slot:14", MouseButton::Right, &mut guild, &none);
    assert_eq!(
        withdraw.requests,
        vec![GuildBankRequest::Withdraw { tab: 0, slot: 13 }]
    );
    let log = actions::guild_bank_action(
        "guild_bank_mode:moneylog",
        MouseButton::Left,
        &mut guild,
        &none,
    );
    assert_eq!(log.requests, vec![GuildBankRequest::QueryLog { tab: None }]);
    let info =
        actions::guild_bank_action("guild_bank_mode:info", MouseButton::Left, &mut guild, &none);
    assert_eq!(
        info.texts,
        vec![(guild_ui::INFO_BOX, "Cloth for the tailors".to_string())]
    );
    let close =
        actions::guild_bank_action("guild_bank_close", MouseButton::Left, &mut guild, &none);
    assert!(close.close);
}

#[test]
fn closing_the_window_ends_the_interaction_and_closes_the_backpack() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<WindowManager>()
        .init_resource::<BankState>()
        .init_resource::<GuildBankState>()
        .add_message::<NpcInteractionRequest>()
        .add_systems(Update, sync_bank_windows);
    app.world_mut().resource_mut::<BankState>().open(BANKER);
    app.update();
    assert!(
        app.world()
            .resource::<WindowManager>()
            .is_open(WindowId::Bank)
    );
    assert!(
        app.world()
            .resource::<WindowManager>()
            .is_open(WindowId::Bag(0))
    );

    app.world_mut()
        .resource_mut::<WindowManager>()
        .close(WindowId::Bank);
    app.update();
    let requests: Vec<_> = app
        .world_mut()
        .resource_mut::<Messages<NpcInteractionRequest>>()
        .drain()
        .collect();
    assert_eq!(requests, vec![NpcInteractionRequest::Close { npc: BANKER }]);
    assert!(!app.world().resource::<BankState>().is_open());
    assert!(
        !app.world()
            .resource::<WindowManager>()
            .is_open(WindowId::Bag(0))
    );

    // The server ending the vault interaction closes its window.
    app.world_mut().resource_mut::<GuildBankState>().open(VAULT);
    app.update();
    assert!(
        app.world()
            .resource::<WindowManager>()
            .is_open(WindowId::GuildBank)
    );
    app.world_mut().resource_mut::<GuildBankState>().close();
    app.update();
    assert!(
        !app.world()
            .resource::<WindowManager>()
            .is_open(WindowId::GuildBank)
    );
}
