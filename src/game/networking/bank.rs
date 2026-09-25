//! Bank networking: the banker role (`NpcFrameEvent::Opened` / `Banker`) opens
//! [`BankState`] and `BankContents` fill it; a Guild Vault (`GuildBanker`) opens
//! [`GuildBankState`], filled by `GuildBankContents` and `GuildBankLog`. Refusals
//! show their Retail error text, the end of the interaction closes the frame, and
//! frame actions ([`BankRequest`], [`GuildBankRequest`]) go to the open banker or vault.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use game_engine::bank_data::{BankRequest, BankState, GuildBankRequest, GuildBankState};
use game_engine::network_runtime::messages::{MessageReceivers, MessageSenders};
use game_engine::quest_runtime::NpcFrameEvent;
use game_engine::status::{GuildVaultStatusSnapshot, StorageItemEntry, WarbankStatusSnapshot};
use game_engine::ui::ui_errors::UiErrors;
use lightyear::prelude::Message as NetworkMessage;
use shared::protocol::{
    BankAutoDeposit, BankChannel, BankContents, BankDeposit, BankFailed, BankMoneyTransfer,
    BankPurchaseTab, BankUpdateTabSettings, BankWithdraw, GuildBankBuyTab, GuildBankChannel,
    GuildBankContents, GuildBankDeposit, GuildBankFailed, GuildBankLog, GuildBankMoneyTransfer,
    GuildBankQueryLog, GuildBankSetTabInfo, GuildBankSetTabText, GuildBankWithdraw, NpcRole,
};

use crate::game_state::GameState;

pub struct BankNetworkPlugin;

impl Plugin for BankNetworkPlugin {
    fn build(&self, app: &mut App) {
        use game_engine::network_events::{add_message_route, register_message_handler};

        app.init_resource::<BankState>()
            .init_resource::<GuildBankState>()
            .init_resource::<WarbankStatusSnapshot>()
            .init_resource::<GuildVaultStatusSnapshot>()
            .init_resource::<UiErrors>()
            .add_message::<BankRequest>()
            .add_message::<GuildBankRequest>()
            .add_message::<NpcFrameEvent>();
        let handler = register_message_handler::<BankContents, _>(app, receive_banks, in_world);
        add_message_route::<BankFailed>(app, handler);
        add_message_route::<GuildBankContents>(app, handler);
        add_message_route::<GuildBankLog>(app, handler);
        add_message_route::<GuildBankFailed>(app, handler);
        app.add_systems(
            Update,
            (
                follow_interactions,
                send_bank_requests,
                send_guild_bank_requests,
                sync_bank_status,
            )
                .chain()
                .run_if(in_state(GameState::InWorld)),
        );
        app.add_systems(OnExit(GameState::InWorld), reset_banks);
    }
}

fn in_world(world: &World) -> bool {
    *world.resource::<State<GameState>>().get() == GameState::InWorld
}

#[derive(SystemParam)]
struct BankReceivers<'w, 's> {
    contents: MessageReceivers<'w, 's, BankContents>,
    failures: MessageReceivers<'w, 's, BankFailed>,
    guild_contents: MessageReceivers<'w, 's, GuildBankContents>,
    guild_logs: MessageReceivers<'w, 's, GuildBankLog>,
    guild_failures: MessageReceivers<'w, 's, GuildBankFailed>,
}

fn receive_banks(
    mut receivers: BankReceivers,
    mut bank: ResMut<BankState>,
    mut guild: ResMut<GuildBankState>,
    mut guild_requests: MessageWriter<GuildBankRequest>,
    mut errors: ResMut<UiErrors>,
) {
    for inbox in receivers.contents.iter_mut() {
        for contents in inbox.receive() {
            if bank.is_open() {
                bank.apply(contents);
            }
        }
    }
    for inbox in receivers.guild_contents.iter_mut() {
        for contents in inbox.receive() {
            guild.apply(contents);
            // A change while a log is shown refreshes it (Retail `GUILDBANKLOG_UPDATE`).
            let mode = guild.mode;
            if let Some(query) = guild.set_mode(mode) {
                guild_requests.write(query);
            }
        }
    }
    for inbox in receivers.guild_logs.iter_mut() {
        for log in inbox.receive() {
            guild.apply_log(log);
        }
    }
    for inbox in receivers.failures.iter_mut() {
        for failed in inbox.receive() {
            errors.add(failed.error.message());
        }
    }
    for inbox in receivers.guild_failures.iter_mut() {
        for failed in inbox.receive() {
            errors.add(failed.error.message());
        }
    }
}

/// The banker and Guild Vault roles open their frames; the end of the interaction
/// (walked away, another NPC, the frame closed) closes them.
fn follow_interactions(
    mut events: MessageReader<NpcFrameEvent>,
    mut bank: ResMut<BankState>,
    mut guild: ResMut<GuildBankState>,
) {
    for event in events.read() {
        match *event {
            NpcFrameEvent::Opened {
                npc,
                role: NpcRole::Banker,
            } => bank.open(npc),
            NpcFrameEvent::Opened {
                npc,
                role: NpcRole::GuildBanker,
            } => guild.open(npc),
            NpcFrameEvent::Closed { npc } if bank.npc == Some(npc) => bank.close(),
            NpcFrameEvent::Closed { npc } if guild.object == Some(npc) => guild.close(),
            _ => {}
        }
    }
}

#[derive(SystemParam)]
struct BankSenders<'w, 's> {
    deposit: MessageSenders<'w, 's, BankDeposit>,
    withdraw: MessageSenders<'w, 's, BankWithdraw>,
    purchase: MessageSenders<'w, 's, BankPurchaseTab>,
    money: MessageSenders<'w, 's, BankMoneyTransfer>,
    auto_deposit: MessageSenders<'w, 's, BankAutoDeposit>,
    settings: MessageSenders<'w, 's, BankUpdateTabSettings>,
}

fn send_bank_requests(
    mut requests: MessageReader<BankRequest>,
    bank: Res<BankState>,
    mut senders: BankSenders,
) {
    let Some(npc) = bank.npc else {
        requests.clear();
        return;
    };
    for request in requests.read() {
        match request.clone() {
            BankRequest::Deposit {
                bank,
                tab,
                item_guid,
            } => send::<BankChannel, _>(
                &mut senders.deposit,
                BankDeposit {
                    npc,
                    bank,
                    tab,
                    item_guid,
                },
            ),
            BankRequest::Withdraw { bank, tab, slot } => send::<BankChannel, _>(
                &mut senders.withdraw,
                BankWithdraw {
                    npc,
                    bank,
                    tab,
                    slot,
                },
            ),
            BankRequest::PurchaseTab { bank } => {
                send::<BankChannel, _>(&mut senders.purchase, BankPurchaseTab { npc, bank })
            }
            BankRequest::Money {
                bank,
                copper,
                deposit,
            } => send::<BankChannel, _>(
                &mut senders.money,
                BankMoneyTransfer {
                    npc,
                    bank,
                    copper,
                    deposit,
                },
            ),
            BankRequest::AutoDeposit {
                bank,
                include_reagents,
            } => send::<BankChannel, _>(
                &mut senders.auto_deposit,
                BankAutoDeposit {
                    npc,
                    bank,
                    include_reagents,
                },
            ),
            BankRequest::UpdateTab {
                bank,
                tab,
                name,
                icon,
                deposit_flags,
            } => send::<BankChannel, _>(
                &mut senders.settings,
                BankUpdateTabSettings {
                    npc,
                    bank,
                    tab,
                    name,
                    icon,
                    deposit_flags,
                },
            ),
        }
    }
}

#[derive(SystemParam)]
struct GuildBankSenders<'w, 's> {
    deposit: MessageSenders<'w, 's, GuildBankDeposit>,
    withdraw: MessageSenders<'w, 's, GuildBankWithdraw>,
    money: MessageSenders<'w, 's, GuildBankMoneyTransfer>,
    buy: MessageSenders<'w, 's, GuildBankBuyTab>,
    info: MessageSenders<'w, 's, GuildBankSetTabInfo>,
    text: MessageSenders<'w, 's, GuildBankSetTabText>,
    log: MessageSenders<'w, 's, GuildBankQueryLog>,
}

fn send_guild_bank_requests(
    mut requests: MessageReader<GuildBankRequest>,
    guild: Res<GuildBankState>,
    mut senders: GuildBankSenders,
) {
    let Some(object) = guild.object else {
        requests.clear();
        return;
    };
    for request in requests.read() {
        match request.clone() {
            GuildBankRequest::Deposit { tab, item_guid } => send::<GuildBankChannel, _>(
                &mut senders.deposit,
                GuildBankDeposit {
                    object,
                    tab,
                    item_guid,
                },
            ),
            GuildBankRequest::Withdraw { tab, slot } => send::<GuildBankChannel, _>(
                &mut senders.withdraw,
                GuildBankWithdraw { object, tab, slot },
            ),
            GuildBankRequest::Money { copper, deposit } => send::<GuildBankChannel, _>(
                &mut senders.money,
                GuildBankMoneyTransfer {
                    object,
                    copper,
                    deposit,
                },
            ),
            GuildBankRequest::BuyTab => {
                send::<GuildBankChannel, _>(&mut senders.buy, GuildBankBuyTab { object })
            }
            GuildBankRequest::SetTabInfo { tab, name, icon } => send::<GuildBankChannel, _>(
                &mut senders.info,
                GuildBankSetTabInfo {
                    object,
                    tab,
                    name,
                    icon,
                },
            ),
            GuildBankRequest::SetTabText { tab, text } => send::<GuildBankChannel, _>(
                &mut senders.text,
                GuildBankSetTabText { object, tab, text },
            ),
            GuildBankRequest::QueryLog { tab } => {
                send::<GuildBankChannel, _>(&mut senders.log, GuildBankQueryLog { object, tab })
            }
        }
    }
}

fn send<C: lightyear::prelude::Channel, M: NetworkMessage + Clone>(
    senders: &mut MessageSenders<M>,
    message: M,
) {
    for mut sender in senders.iter_mut() {
        sender.send::<C>(message.clone());
    }
}

fn storage_entries<'a>(
    tabs: impl Iterator<Item = &'a [Option<shared::protocol::ItemStack>]>,
) -> Vec<StorageItemEntry> {
    tabs.enumerate()
        .flat_map(|(tab, slots)| {
            slots.iter().enumerate().filter_map(move |(slot, item)| {
                let item = item.as_ref()?;
                Some(StorageItemEntry {
                    slot: (tab * slots.len() + slot) as u32,
                    item_guid: item.item_guid,
                    item_id: item.item_id,
                    name: String::new(),
                    stack_count: item.count,
                })
            })
        })
        .collect()
}

/// The IPC `warbank` / `guild_vault` status lists the last contents the server sent.
fn sync_bank_status(
    bank: Res<BankState>,
    guild: Res<GuildBankState>,
    mut warbank: ResMut<WarbankStatusSnapshot>,
    mut vault: ResMut<GuildVaultStatusSnapshot>,
) {
    if bank.is_changed()
        && let Some(account) = &bank.account
    {
        warbank.entries = storage_entries(account.tabs.iter().map(|tab| tab.slots.as_slice()));
    }
    if guild.is_changed()
        && let Some(contents) = &guild.contents
    {
        vault.entries = storage_entries(contents.tabs.iter().map(|tab| tab.slots.as_slice()));
    }
}

fn reset_banks(mut bank: ResMut<BankState>, mut guild: ResMut<GuildBankState>) {
    bank.close();
    guild.close();
}

#[cfg(test)]
#[path = "bank_tests.rs"]
mod tests;
