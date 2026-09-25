use std::sync::mpsc;

use bevy::ecs::system::RunSystemOnce;
use game_engine::bank_data::GuildBankMode;
use game_engine::network_runtime::messages::{ConnectionSender, Inbox};
use game_engine::network_runtime::worker::NetworkCommand;
use shared::protocol::{BankError, BankTabView, BankType, GuildBankError, GuildBankTabView};

use super::*;

/// Server entity bits of Newton Burnside and the Stormwind Guild Vault in these fixtures.
const BANKER: u64 = 0x0000_0001_0000_0990;
const VAULT: u64 = 0x0000_0001_0000_1D01;

fn fixture() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<BankRequest>()
        .add_message::<GuildBankRequest>()
        .add_message::<NpcFrameEvent>()
        .init_resource::<BankState>()
        .init_resource::<GuildBankState>()
        .init_resource::<UiErrors>()
        .init_resource::<ConnectionSender>()
        .init_resource::<Inbox<BankContents>>()
        .init_resource::<Inbox<BankFailed>>()
        .init_resource::<Inbox<GuildBankContents>>()
        .init_resource::<Inbox<GuildBankLog>>()
        .init_resource::<Inbox<GuildBankFailed>>();
    app
}

fn deliver<M: lightyear::prelude::Message>(app: &mut App, messages: Vec<M>) {
    app.world_mut().insert_resource(Inbox::new(messages));
}

fn run<M, S: IntoSystem<(), (), M>>(app: &mut App, system: S) {
    app.world_mut().run_system_once(system).unwrap();
}

fn warband(slots: Vec<Option<shared::protocol::ItemStack>>) -> BankContents {
    BankContents {
        bank: BankType::Account,
        tabs: vec![BankTabView {
            name: "Tab 1".into(),
            icon: 134_400,
            deposit_flags: 0,
            slots,
        }],
        next_tab_cost: Some(250_000_000),
        money: Some(5_000),
    }
}

fn vault_contents() -> GuildBankContents {
    GuildBankContents {
        object: VAULT,
        guild_name: "Bank Testers".into(),
        tabs: vec![GuildBankTabView {
            name: "Tab 1".into(),
            icon: 134_400,
            viewable: true,
            can_deposit: true,
            withdrawals_per_day: None,
            remaining_withdrawals: None,
            text: String::new(),
            slots: vec![None; 98],
        }],
        money: 50_000,
        withdraw_money_remaining: None,
        next_tab_cost: Some(2_500_000),
        is_leader: true,
    }
}

#[test]
fn the_banker_role_opens_the_bank_and_its_contents_fill_it() {
    let mut app = fixture();
    // Contents can arrive in the same batch as InteractionOpened, before the frame opens.
    deliver(&mut app, vec![warband(vec![None; 98])]);
    run(&mut app, receive_banks);
    app.world_mut().write_message(NpcFrameEvent::Opened {
        npc: BANKER,
        role: NpcRole::Banker,
    });
    run(&mut app, follow_interactions);
    deliver(
        &mut app,
        vec![BankFailed {
            npc: BANKER,
            error: BankError::SoulboundInAccountBank,
        }],
    );
    run(&mut app, receive_banks);
    let bank = app.world().resource::<BankState>();
    assert_eq!(bank.npc, Some(BANKER));
    assert_eq!(bank.account.as_ref().unwrap().money, Some(5_000));
    assert_eq!(
        app.world().resource::<UiErrors>().lines[0].text,
        "Soulbound items cannot be stored in the Warband Bank."
    );

    app.world_mut()
        .write_message(NpcFrameEvent::Closed { npc: BANKER });
    run(&mut app, follow_interactions);
    assert!(!app.world().resource::<BankState>().is_open());
}

#[test]
fn the_guild_vault_role_opens_the_guild_bank_and_a_change_refreshes_the_shown_log() {
    let mut app = fixture();
    app.world_mut().write_message(NpcFrameEvent::Opened {
        npc: VAULT,
        role: NpcRole::GuildBanker,
    });
    run(&mut app, follow_interactions);
    deliver(&mut app, vec![vault_contents()]);
    run(&mut app, receive_banks);
    assert_eq!(
        app.world().resource::<GuildBankState>().contents,
        Some(vault_contents())
    );

    app.world_mut().resource_mut::<GuildBankState>().mode = GuildBankMode::MoneyLog;
    deliver(&mut app, vec![vault_contents()]);
    deliver(
        &mut app,
        vec![GuildBankFailed {
            object: VAULT,
            error: GuildBankError::WithdrawLimit,
        }],
    );
    run(&mut app, receive_banks);
    let queries: Vec<_> = app
        .world_mut()
        .resource_mut::<Messages<GuildBankRequest>>()
        .drain()
        .collect();
    assert_eq!(queries, vec![GuildBankRequest::QueryLog { tab: None }]);
    assert_eq!(
        app.world().resource::<UiErrors>().lines[0].text,
        "You cannot withdraw that much from the guild bank."
    );
}

#[test]
fn bank_and_guild_bank_requests_go_out_only_while_their_frame_is_open() {
    let mut app = fixture();
    let (sender, commands) = mpsc::channel();
    app.insert_resource(ConnectionSender::new(Some(sender)));
    let deposit = BankRequest::Deposit {
        bank: BankType::Character,
        tab: 0,
        item_guid: 41,
    };
    app.world_mut().write_message(deposit.clone());
    app.world_mut()
        .write_message(GuildBankRequest::Withdraw { tab: 0, slot: 3 });
    run(&mut app, send_bank_requests);
    run(&mut app, send_guild_bank_requests);
    assert!(commands.try_recv().is_err());
    app.world_mut()
        .resource_mut::<Messages<BankRequest>>()
        .clear();
    app.world_mut()
        .resource_mut::<Messages<GuildBankRequest>>()
        .clear();

    app.world_mut().resource_mut::<BankState>().open(BANKER);
    app.world_mut().resource_mut::<GuildBankState>().open(VAULT);
    app.world_mut().write_message(deposit);
    app.world_mut().write_message(BankRequest::PurchaseTab {
        bank: BankType::Account,
    });
    app.world_mut()
        .write_message(GuildBankRequest::Withdraw { tab: 0, slot: 3 });
    run(&mut app, send_bank_requests);
    run(&mut app, send_guild_bank_requests);
    for _ in 0..3 {
        assert!(matches!(commands.try_recv(), Ok(NetworkCommand::Apply(_))));
    }
    assert!(commands.try_recv().is_err());
}

#[test]
fn warband_and_vault_contents_feed_the_ipc_status_lists() {
    let mut app = fixture();
    app.init_resource::<WarbankStatusSnapshot>()
        .init_resource::<GuildVaultStatusSnapshot>();
    let mut slots = vec![None; 98];
    slots[4] = Some(shared::protocol::ItemStack {
        item_guid: 77,
        item_id: 2770,
        count: 10,
        durability: None,
        soulbound: false,
    });
    app.world_mut().resource_mut::<BankState>().open(BANKER);
    app.world_mut()
        .resource_mut::<BankState>()
        .apply(warband(slots));
    run(&mut app, sync_bank_status);
    let entries = &app.world().resource::<WarbankStatusSnapshot>().entries;
    assert_eq!(entries.len(), 1);
    assert_eq!(
        (entries[0].slot, entries[0].item_id, entries[0].stack_count),
        (4, 2770, 10)
    );
    assert!(
        app.world()
            .resource::<GuildVaultStatusSnapshot>()
            .entries
            .is_empty()
    );
}
