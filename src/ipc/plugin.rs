//! Bevy plugin that integrates the IPC server with the render pipeline.

use std::sync::mpsc;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use lightyear::prelude::MessageSender;
use lightyear::prelude::client::Connected;

use super::{Command, Request, Response, init};
use crate::auction_house::{AuctionHouseState, queue_ipc_request};
use crate::item_info::lookup_item_info;
use crate::mail::{MailState, queue_ipc_request as queue_mail_ipc_request};
use crate::status::{
    CharacterStatsSnapshot, CurrenciesStatusSnapshot, EquippedGearStatusSnapshot,
    GuildVaultStatusSnapshot, InventoryItemEntry, InventorySearchSnapshot, NetworkStatusSnapshot,
    ReputationsStatusSnapshot, SoundStatusSnapshot, TerrainStatusSnapshot, WarbankStatusSnapshot,
};
use crate::targeting::CurrentTarget;
use crate::ui::plugin::UiState;
use shared::protocol::{AuctionInventorySnapshot, CombatChannel, SpellCastIntent, StopSpellCast};

/// Channel sender to reply to an IPC caller waiting for a screenshot.
#[derive(Component)]
struct ScreenshotReply(mpsc::Sender<Response>);

pub struct IpcPlugin;

impl Plugin for IpcPlugin {
    fn build(&self, app: &mut App) {
        let (receiver, guard) = init();

        app.insert_non_send_resource(receiver)
            .insert_non_send_resource(guard)
            .add_systems(Update, poll_ipc);
    }
}

/// Poll IPC commands each frame and dispatch them.
#[allow(clippy::type_complexity)]
fn poll_ipc(
    receiver: NonSend<mpsc::Receiver<Command>>,
    mut commands: Commands,
    tree_query: Query<(
        Entity,
        Option<&Name>,
        Option<&Children>,
        Option<&Visibility>,
        &Transform,
    )>,
    parent_query: Query<&ChildOf>,
    ui_state: Res<UiState>,
    mut auction_house: ResMut<AuctionHouseState>,
    mut mail: ResMut<MailState>,
    primary_status_snapshots: (
        Res<NetworkStatusSnapshot>,
        Res<TerrainStatusSnapshot>,
        Res<SoundStatusSnapshot>,
        Res<CurrenciesStatusSnapshot>,
        Res<ReputationsStatusSnapshot>,
    ),
    secondary_status_snapshots: (
        Res<CharacterStatsSnapshot>,
        Res<GuildVaultStatusSnapshot>,
        Res<WarbankStatusSnapshot>,
        Res<EquippedGearStatusSnapshot>,
    ),
    current_target: Res<CurrentTarget>,
    mut spell_cast_senders: Query<&mut MessageSender<SpellCastIntent>>,
    mut spell_stop_senders: Query<&mut MessageSender<StopSpellCast>>,
    connected_query: Query<(), With<Connected>>,
) {
    let (network_status, terrain_status, sound_status, currencies_status, reputations_status) =
        primary_status_snapshots;
    let (character_stats, guild_vault_status, warbank_status, equipped_gear_status) =
        secondary_status_snapshots;
    while let Ok(cmd) = receiver.try_recv() {
        dispatch(
            cmd,
            &mut commands,
            &tree_query,
            &parent_query,
            &ui_state,
            &mut auction_house,
            mail.as_mut(),
            &network_status,
            &terrain_status,
            &sound_status,
            &currencies_status,
            &reputations_status,
            &character_stats,
            &guild_vault_status,
            &warbank_status,
            &equipped_gear_status,
            &current_target,
            &mut spell_cast_senders,
            &mut spell_stop_senders,
            !connected_query.is_empty(),
        );
    }
}

#[allow(clippy::type_complexity)]
fn dispatch(
    cmd: Command,
    commands: &mut Commands,
    tree_query: &Query<(
        Entity,
        Option<&Name>,
        Option<&Children>,
        Option<&Visibility>,
        &Transform,
    )>,
    parent_query: &Query<&ChildOf>,
    ui_state: &UiState,
    auction_house: &mut AuctionHouseState,
    mail: &mut MailState,
    network_status: &NetworkStatusSnapshot,
    terrain_status: &TerrainStatusSnapshot,
    sound_status: &SoundStatusSnapshot,
    currencies_status: &CurrenciesStatusSnapshot,
    reputations_status: &ReputationsStatusSnapshot,
    character_stats: &CharacterStatsSnapshot,
    guild_vault_status: &GuildVaultStatusSnapshot,
    warbank_status: &WarbankStatusSnapshot,
    equipped_gear_status: &EquippedGearStatusSnapshot,
    current_target: &CurrentTarget,
    spell_cast_senders: &mut Query<&mut MessageSender<SpellCastIntent>>,
    spell_stop_senders: &mut Query<&mut MessageSender<StopSpellCast>>,
    connected: bool,
) {
    if queue_ipc_request(auction_house, &cmd.request, cmd.respond.clone()) {
        return;
    }
    if queue_mail_ipc_request(mail, &cmd.request, cmd.respond.clone()) {
        return;
    }
    match cmd.request {
        Request::Ping => {
            let _ = cmd.respond.send(Response::Pong);
        }
        Request::Screenshot => {
            commands
                .spawn(Screenshot::primary_window())
                .insert(ScreenshotReply(cmd.respond))
                .observe(on_screenshot_captured);
        }
        Request::DumpTree { filter } => {
            let tree = crate::dump::build_tree(tree_query, parent_query, filter.as_deref());
            let _ = cmd.respond.send(Response::Tree(tree));
        }
        Request::DumpUiTree { filter } => {
            let tree = crate::dump::build_ui_tree(&ui_state.registry, filter.as_deref());
            let _ = cmd.respond.send(Response::Tree(tree));
        }
        Request::NetworkStatus => {
            let _ = cmd.respond.send(Response::Text(format_network_status(
                network_status,
                connected,
            )));
        }
        Request::TerrainStatus => {
            let _ = cmd
                .respond
                .send(Response::Text(format_terrain_status(terrain_status)));
        }
        Request::SoundStatus => {
            let _ = cmd
                .respond
                .send(Response::Text(format_sound_status(sound_status)));
        }
        Request::CurrenciesStatus => {
            let _ = cmd
                .respond
                .send(Response::Text(format_currencies_status(currencies_status)));
        }
        Request::ReputationsStatus => {
            let _ = cmd.respond.send(Response::Text(format_reputations_status(
                reputations_status,
            )));
        }
        Request::CharacterStatsStatus => {
            let _ = cmd
                .respond
                .send(Response::Text(format_character_stats_status(
                    character_stats,
                )));
        }
        Request::BagsStatus => {
            let _ = cmd.respond.send(Response::Text(format_bags_status(
                auction_house.inventory.as_ref(),
            )));
        }
        Request::GuildVaultStatus => {
            let _ = cmd.respond.send(Response::Text(format_storage_list(
                "guild_vault",
                &guild_vault_status.entries,
            )));
        }
        Request::WarbankStatus => {
            let _ = cmd.respond.send(Response::Text(format_storage_list(
                "warbank",
                &warbank_status.entries,
            )));
        }
        Request::EquippedGearStatus => {
            let _ = cmd.respond.send(Response::Text(format_equipped_gear_status(
                equipped_gear_status,
            )));
        }
        Request::ItemInfo { query } => match lookup_item_info(query.item_id) {
            Ok(Some(item)) => {
                let appearance_known = auction_house.inventory.as_ref().is_some_and(|inventory| {
                    inventory
                        .items
                        .iter()
                        .any(|entry| entry.item_id == item.item_id)
                });
                let _ = cmd
                    .respond
                    .send(Response::Text(format_item_info(&item, appearance_known)));
            }
            Ok(None) => {
                let _ = cmd
                    .respond
                    .send(Response::Error(format!("item {} not found", query.item_id)));
            }
            Err(error) => {
                let _ = cmd.respond.send(Response::Error(error));
            }
        },
        Request::InventoryList => {
            let entries = build_inventory_entries(
                auction_house.inventory.as_ref(),
                &guild_vault_status.entries,
                &warbank_status.entries,
            );
            let _ = cmd
                .respond
                .send(Response::Text(format_inventory_list(&entries)));
        }
        Request::InventorySearch { text } => {
            let entries = build_inventory_entries(
                auction_house.inventory.as_ref(),
                &guild_vault_status.entries,
                &warbank_status.entries,
            );
            let snapshot = inventory_search_snapshot(&entries, &text);
            let _ = cmd
                .respond
                .send(Response::Text(format_inventory_search(&snapshot, &text)));
        }
        Request::InventoryWhereis { item_id } => {
            let entries = build_inventory_entries(
                auction_house.inventory.as_ref(),
                &guild_vault_status.entries,
                &warbank_status.entries,
            );
            let _ = cmd
                .respond
                .send(Response::Text(format_inventory_whereis(&entries, item_id)));
        }
        Request::SpellCast { spell, target } => {
            if !connected {
                let _ = cmd.respond.send(Response::Error(
                    "spell cast is unavailable: not connected".into(),
                ));
                return;
            }
            let target_bits = match resolve_spell_target(target.as_deref(), current_target) {
                Ok(bits) => bits,
                Err(error) => {
                    let _ = cmd.respond.send(Response::Error(error));
                    return;
                }
            };
            let intent = SpellCastIntent {
                spell_id: parse_spell_id(&spell),
                spell: spell.clone(),
                target_entity: target_bits,
            };
            if send_combat_message(spell_cast_senders, intent.clone()) {
                let target_text = intent
                    .target_entity
                    .map(|bits| bits.to_string())
                    .unwrap_or_else(|| "-".into());
                let _ = cmd.respond.send(Response::Text(format!(
                    "spell cast submitted spell={} target={target_text}",
                    intent.spell,
                )));
            } else {
                let _ = cmd.respond.send(Response::Error(
                    "spell cast is unavailable: not connected".into(),
                ));
            }
        }
        Request::SpellStop => {
            if !connected {
                let _ = cmd.respond.send(Response::Error(
                    "spell stop is unavailable: not connected".into(),
                ));
                return;
            }
            if send_combat_message(spell_stop_senders, StopSpellCast) {
                let _ = cmd
                    .respond
                    .send(Response::Text("spell stop submitted".into()));
            } else {
                let _ = cmd.respond.send(Response::Error(
                    "spell stop is unavailable: not connected".into(),
                ));
            }
        }
        Request::QuestList => {
            let _ = cmd
                .respond
                .send(Response::Text("quest list: unavailable".into()));
        }
        Request::QuestWatch => {
            let _ = cmd
                .respond
                .send(Response::Text("quest watch: unavailable".into()));
        }
        Request::QuestShow { quest_id } => {
            let _ = cmd.respond.send(Response::Text(format!(
                "quest show id={quest_id}: unavailable"
            )));
        }
        Request::GroupRoster => {
            let _ = cmd
                .respond
                .send(Response::Text("group roster: unavailable".into()));
        }
        Request::GroupStatus => {
            let _ = cmd
                .respond
                .send(Response::Text("group status: unavailable".into()));
        }
        Request::GroupInvite { name } => {
            let _ = cmd.respond.send(Response::Text(format!(
                "group invite {name}: pending server support"
            )));
        }
        Request::GroupUninvite { name } => {
            let _ = cmd.respond.send(Response::Text(format!(
                "group uninvite {name}: pending server support"
            )));
        }
        Request::CombatLog { lines } => {
            let _ = cmd.respond.send(Response::Text(format!(
                "combat log lines={lines}: unavailable"
            )));
        }
        Request::CombatRecap { target } => {
            let target = target.unwrap_or_else(|| "current".into());
            let _ = cmd.respond.send(Response::Text(format!(
                "combat recap target={target}: unavailable"
            )));
        }
        Request::ReputationList => {
            let _ = cmd
                .respond
                .send(Response::Text("reputation list: unavailable".into()));
        }
        Request::CollectionMounts { missing } => {
            let _ = cmd.respond.send(Response::Text(format!(
                "collection mounts missing={missing}: unavailable"
            )));
        }
        Request::CollectionPets { missing } => {
            let _ = cmd.respond.send(Response::Text(format!(
                "collection pets missing={missing}: unavailable"
            )));
        }
        Request::ProfessionRecipes { text } => {
            let _ = cmd.respond.send(Response::Text(format!(
                "profession recipes text={text}: unavailable"
            )));
        }
        Request::MapPosition => {
            let _ = cmd
                .respond
                .send(Response::Text("map position: unavailable".into()));
        }
        Request::MapTarget => {
            let _ = cmd
                .respond
                .send(Response::Text("map target: unavailable".into()));
        }
        Request::MapWaypointAdd { x, y } => {
            let _ = cmd.respond.send(Response::Text(format!(
                "map waypoint add x={x:.2} y={y:.2}: unavailable"
            )));
        }
        Request::MapWaypointClear => {
            let _ = cmd
                .respond
                .send(Response::Text("map waypoint clear: unavailable".into()));
        }
        Request::AuctionOpen
        | Request::AuctionBrowse { .. }
        | Request::AuctionOwned
        | Request::AuctionBids
        | Request::AuctionInventory
        | Request::AuctionMailbox
        | Request::AuctionCreate { .. }
        | Request::AuctionBid { .. }
        | Request::AuctionBuyout { .. }
        | Request::AuctionCancel { .. }
        | Request::AuctionClaimMail { .. }
        | Request::AuctionStatus
        | Request::MailSend { .. }
        | Request::MailList { .. }
        | Request::MailRead { .. }
        | Request::MailClaim { .. }
        | Request::MailDelete { .. }
        | Request::MailStatus => {}
    }
}

fn format_network_status(snapshot: &NetworkStatusSnapshot, connected: bool) -> String {
    format!(
        "server_addr: {}\ngame_state: {}\nconnected: {}\nconnected_links: {}\nlocal_client_id: {}\nzone_id: {}\nremote_entities: {}\nlocal_players: {}\nchat_messages: {}",
        snapshot.server_addr.as_deref().unwrap_or("-"),
        snapshot.game_state,
        connected || snapshot.connected,
        snapshot.connected_links,
        snapshot
            .local_client_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "-".into()),
        snapshot.zone_id,
        snapshot.remote_entities,
        snapshot.local_players,
        snapshot.chat_messages,
    )
}

fn format_terrain_status(snapshot: &TerrainStatusSnapshot) -> String {
    format!(
        "map_name: {}\ninitial_tile: {},{}\nload_radius: {}\nloaded_tiles: {}\npending_tiles: {}\nfailed_tiles: {}\nserver_requested_tiles: {}\nheightmap_tiles: {}",
        if snapshot.map_name.is_empty() {
            "-"
        } else {
            &snapshot.map_name
        },
        snapshot.initial_tile.0,
        snapshot.initial_tile.1,
        snapshot.load_radius,
        snapshot.loaded_tiles,
        snapshot.pending_tiles,
        snapshot.failed_tiles,
        snapshot.server_requested_tiles,
        snapshot.heightmap_tiles,
    )
}

fn format_sound_status(snapshot: &SoundStatusSnapshot) -> String {
    format!(
        "enabled: {}\nmuted: {}\nmaster_volume: {:.2}\nfootstep_volume: {:.2}\nambient_volume: {:.2}\nambient_entities: {}\nactive_sinks: {}",
        snapshot.enabled,
        snapshot.muted,
        snapshot.master_volume,
        snapshot.footstep_volume,
        snapshot.ambient_volume,
        snapshot.ambient_entities,
        snapshot.active_sinks,
    )
}

fn format_currencies_status(snapshot: &CurrenciesStatusSnapshot) -> String {
    if snapshot.entries.is_empty() {
        return "currencies: 0\n-".into();
    }

    let lines = snapshot
        .entries
        .iter()
        .map(|entry| format!("{} {} amount={}", entry.id, entry.name, entry.amount))
        .collect::<Vec<_>>()
        .join("\n");
    format!("currencies: {}\n{lines}", snapshot.entries.len())
}

fn format_reputations_status(snapshot: &ReputationsStatusSnapshot) -> String {
    if snapshot.entries.is_empty() {
        return "reputations: 0\n-".into();
    }

    let lines = snapshot
        .entries
        .iter()
        .map(|entry| {
            format!(
                "{} {} standing={} value={}",
                entry.faction_id, entry.faction_name, entry.standing, entry.value
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("reputations: {}\n{lines}", snapshot.entries.len())
}

fn format_character_stats_status(snapshot: &CharacterStatsSnapshot) -> String {
    format!(
        "name: {}\nlevel: {}\nrace: {}\nclass: {}\nhealth: {}/{}\nmana: {}/{}\nmovement_speed: {}\nzone_id: {}",
        snapshot.name.as_deref().unwrap_or("-"),
        snapshot
            .level
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".into()),
        snapshot
            .race
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".into()),
        snapshot
            .class
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".into()),
        snapshot
            .health_current
            .map(|value| format!("{value:.0}"))
            .unwrap_or_else(|| "-".into()),
        snapshot
            .health_max
            .map(|value| format!("{value:.0}"))
            .unwrap_or_else(|| "-".into()),
        snapshot
            .mana_current
            .map(|value| format!("{value:.0}"))
            .unwrap_or_else(|| "-".into()),
        snapshot
            .mana_max
            .map(|value| format!("{value:.0}"))
            .unwrap_or_else(|| "-".into()),
        snapshot
            .movement_speed
            .map(|value| format!("{value:.2}"))
            .unwrap_or_else(|| "-".into()),
        snapshot.zone_id,
    )
}

fn format_bags_status(snapshot: Option<&AuctionInventorySnapshot>) -> String {
    let Some(snapshot) = snapshot else {
        return "bags: unavailable\n-".into();
    };
    if snapshot.items.is_empty() {
        return format!("bags: 0\ngold: {}\n-", snapshot.gold);
    }
    let lines = snapshot
        .items
        .iter()
        .enumerate()
        .map(|(slot, item)| {
            format!(
                "{} {} {} x{} q{} lvl{}",
                slot,
                item.item_guid,
                item.name,
                item.stack_count,
                item.quality,
                item.required_level
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "bags: {}\ngold: {}\n{}",
        snapshot.items.len(),
        snapshot.gold,
        lines
    )
}

fn format_storage_list(title: &str, entries: &[crate::status::StorageItemEntry]) -> String {
    if entries.is_empty() {
        return format!("{title}: 0\n-");
    }
    let lines = entries
        .iter()
        .map(|entry| {
            format!(
                "{} {} {} {} x{}",
                entry.slot, entry.item_guid, entry.item_id, entry.name, entry.stack_count
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("{title}: {}\n{lines}", entries.len())
}

fn format_equipped_gear_status(snapshot: &EquippedGearStatusSnapshot) -> String {
    if snapshot.entries.is_empty() {
        return "equipped_gear: 0\n-".into();
    }
    let lines = snapshot
        .entries
        .iter()
        .map(|entry| format!("{} {}", entry.slot, entry.path))
        .collect::<Vec<_>>()
        .join("\n");
    format!("equipped_gear: {}\n{lines}", snapshot.entries.len())
}

fn format_item_info(item: &crate::item_info::ItemStaticInfo, appearance_known: bool) -> String {
    format!(
        "item_id: {}\nname: {}\nquality: {}\nitem_level: {}\nrequired_level: {}\ninventory_type: {}\nsell_price: {}\nstackable: {}\nbonding: {}\nexpansion_id: {}\nappearance_known: {}",
        item.item_id,
        item.name,
        item.quality,
        item.item_level,
        item.required_level,
        item.inventory_type,
        item.sell_price,
        item.stackable,
        item.bonding,
        item.expansion_id,
        appearance_known,
    )
}

fn build_inventory_entries(
    bags: Option<&AuctionInventorySnapshot>,
    guild_vault: &[crate::status::StorageItemEntry],
    warbank: &[crate::status::StorageItemEntry],
) -> Vec<InventoryItemEntry> {
    let mut entries = Vec::new();

    if let Some(snapshot) = bags {
        for (index, item) in snapshot.items.iter().enumerate() {
            entries.push(InventoryItemEntry {
                storage: "bags".into(),
                slot: index as u32,
                item_guid: item.item_guid,
                item_id: item.item_id,
                name: item.name.clone(),
                stack_count: item.stack_count,
            });
        }
    }

    entries.extend(guild_vault.iter().map(|entry| InventoryItemEntry {
        storage: "guild_vault".into(),
        slot: entry.slot,
        item_guid: entry.item_guid,
        item_id: entry.item_id,
        name: entry.name.clone(),
        stack_count: entry.stack_count,
    }));

    entries.extend(warbank.iter().map(|entry| InventoryItemEntry {
        storage: "warbank".into(),
        slot: entry.slot,
        item_guid: entry.item_guid,
        item_id: entry.item_id,
        name: entry.name.clone(),
        stack_count: entry.stack_count,
    }));

    entries
}

fn inventory_search_snapshot(
    entries: &[InventoryItemEntry],
    text: &str,
) -> InventorySearchSnapshot {
    let needle = text.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return InventorySearchSnapshot {
            entries: entries.to_vec(),
        };
    }
    InventorySearchSnapshot {
        entries: entries
            .iter()
            .filter(|entry| entry.name.to_ascii_lowercase().contains(&needle))
            .cloned()
            .collect(),
    }
}

fn format_inventory_list(entries: &[InventoryItemEntry]) -> String {
    if entries.is_empty() {
        return "inventory: 0\n-".into();
    }
    let lines = entries
        .iter()
        .map(|entry| {
            format!(
                "{}:{} {} {} {} x{}",
                entry.storage,
                entry.slot,
                entry.item_guid,
                entry.item_id,
                entry.name,
                entry.stack_count
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("inventory: {}\n{lines}", entries.len())
}

fn format_inventory_search(snapshot: &InventorySearchSnapshot, text: &str) -> String {
    if snapshot.entries.is_empty() {
        return format!("inventory search text={text}: 0\n-");
    }
    let mut lines = Vec::new();
    for storage in ["bags", "guild_vault", "warbank"] {
        let grouped = snapshot
            .entries
            .iter()
            .filter(|entry| entry.storage == storage)
            .collect::<Vec<_>>();
        if grouped.is_empty() {
            continue;
        }
        lines.push(format!("[{storage}]"));
        lines.extend(grouped.iter().map(|entry| {
            format!(
                "{} {} {} {} x{}",
                entry.slot, entry.item_guid, entry.item_id, entry.name, entry.stack_count
            )
        }));
    }
    format!(
        "inventory search text={text}: {}\n{}",
        snapshot.entries.len(),
        lines.join("\n")
    )
}

fn format_inventory_whereis(entries: &[InventoryItemEntry], item_id: u32) -> String {
    let matches = entries
        .iter()
        .filter(|entry| entry.item_id == item_id)
        .collect::<Vec<_>>();
    if matches.is_empty() {
        return format!("inventory whereis item_id={item_id}: 0\n-");
    }
    let lines = matches
        .iter()
        .map(|entry| {
            format!(
                "{}:{} {} {} x{}",
                entry.storage, entry.slot, entry.item_guid, entry.name, entry.stack_count
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "inventory whereis item_id={item_id}: {}\n{lines}",
        matches.len()
    )
}

fn parse_spell_id(spell: &str) -> Option<u32> {
    spell.parse().ok()
}

fn resolve_spell_target(
    selector: Option<&str>,
    current_target: &CurrentTarget,
) -> Result<Option<u64>, String> {
    let Some(selector) = selector else {
        return current_target
            .0
            .map(|entity| Some(entity.to_bits()))
            .ok_or_else(|| "no current target selected".into());
    };
    if selector.eq_ignore_ascii_case("current") {
        return current_target
            .0
            .map(|entity| Some(entity.to_bits()))
            .ok_or_else(|| "no current target selected".into());
    }
    if selector.eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    selector
        .parse::<u64>()
        .map(Some)
        .map_err(|_| format!("invalid target selector '{selector}'"))
}

fn send_combat_message<T: Clone + lightyear::prelude::Message>(
    senders: &mut Query<&mut MessageSender<T>>,
    message: T,
) -> bool {
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        sender.send::<CombatChannel>(message.clone());
        sent = true;
    }
    sent
}

/// Per-entity observer triggered when this screenshot is captured.
fn on_screenshot_captured(
    trigger: On<ScreenshotCaptured>,
    query: Query<&ScreenshotReply>,
    mut commands: Commands,
) {
    let entity = trigger.event_target();
    let Ok(reply) = query.get(entity) else {
        return;
    };

    let response = encode_screenshot(&trigger.image);
    let _ = reply.0.send(response);

    commands.entity(entity).despawn();
}

fn encode_screenshot(img: &bevy::image::Image) -> Response {
    let Some(data) = img.data.as_ref() else {
        return Response::Error("screenshot has no pixel data".into());
    };
    let size = img.size();
    let encoder = webp::Encoder::from_rgba(data, size.x, size.y);
    let webp_data = encoder.encode(15.0);
    Response::Screenshot(webp_data.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::AuctionInventoryItem;

    #[test]
    fn formats_network_status_snapshot() {
        let text = format_network_status(
            &NetworkStatusSnapshot {
                server_addr: Some("127.0.0.1:8085".into()),
                game_state: "InWorld".into(),
                connected: false,
                connected_links: 1,
                local_client_id: Some(42),
                zone_id: 12,
                remote_entities: 7,
                local_players: 1,
                chat_messages: 3,
            },
            true,
        );

        assert!(text.contains("connected: true"));
        assert!(text.contains("server_addr: 127.0.0.1:8085"));
        assert!(text.contains("remote_entities: 7"));
    }

    #[test]
    fn formats_terrain_status_snapshot() {
        let text = format_terrain_status(&TerrainStatusSnapshot {
            map_name: "azeroth".into(),
            initial_tile: (32, 48),
            load_radius: 1,
            loaded_tiles: 9,
            pending_tiles: 1,
            failed_tiles: 2,
            server_requested_tiles: 0,
            heightmap_tiles: 9,
        });

        assert!(text.contains("map_name: azeroth"));
        assert!(text.contains("loaded_tiles: 9"));
        assert!(text.contains("pending_tiles: 1"));
    }

    #[test]
    fn formats_sound_status_snapshot() {
        let text = format_sound_status(&SoundStatusSnapshot {
            enabled: true,
            muted: false,
            master_volume: 0.8,
            footstep_volume: 0.5,
            ambient_volume: 0.3,
            ambient_entities: 1,
            active_sinks: 2,
        });

        assert!(text.contains("enabled: true"));
        assert!(text.contains("master_volume: 0.80"));
        assert!(text.contains("active_sinks: 2"));
    }

    #[test]
    fn formats_empty_currencies_status_snapshot() {
        let text = format_currencies_status(&CurrenciesStatusSnapshot::default());

        assert_eq!(text, "currencies: 0\n-");
    }

    #[test]
    fn formats_empty_reputations_status_snapshot() {
        let text = format_reputations_status(&ReputationsStatusSnapshot::default());

        assert_eq!(text, "reputations: 0\n-");
    }

    #[test]
    fn formats_character_stats_status_snapshot() {
        let text = format_character_stats_status(&CharacterStatsSnapshot {
            name: Some("Thrall".into()),
            level: Some(12),
            race: Some(2),
            class: Some(7),
            health_current: Some(120.0),
            health_max: Some(150.0),
            mana_current: Some(80.0),
            mana_max: Some(100.0),
            movement_speed: Some(7.0),
            zone_id: 12,
        });

        assert!(text.contains("name: Thrall"));
        assert!(text.contains("health: 120/150"));
        assert!(text.contains("movement_speed: 7.00"));
    }

    #[test]
    fn formats_unavailable_bags_status() {
        let text = format_bags_status(None);

        assert_eq!(text, "bags: unavailable\n-");
    }

    #[test]
    fn formats_empty_guild_vault_status_snapshot() {
        let text = format_storage_list("guild_vault", &[]);

        assert_eq!(text, "guild_vault: 0\n-");
    }

    #[test]
    fn formats_empty_warbank_status_snapshot() {
        let text = format_storage_list("warbank", &[]);

        assert_eq!(text, "warbank: 0\n-");
    }

    #[test]
    fn formats_empty_equipped_gear_status_snapshot() {
        let text = format_equipped_gear_status(&EquippedGearStatusSnapshot::default());

        assert_eq!(text, "equipped_gear: 0\n-");
    }

    #[test]
    fn formats_item_info_with_appearance_state() {
        let text = format_item_info(
            &crate::item_info::ItemStaticInfo {
                item_id: 2589,
                name: "Linen Cloth".into(),
                quality: 1,
                item_level: 5,
                required_level: 1,
                inventory_type: 0,
                sell_price: 13,
                stackable: 200,
                bonding: 0,
                expansion_id: 0,
            },
            true,
        );

        assert!(text.contains("item_id: 2589"));
        assert!(text.contains("name: Linen Cloth"));
        assert!(text.contains("appearance_known: true"));
    }

    #[test]
    fn inventory_search_groups_entries_by_storage() {
        let snapshot = InventorySearchSnapshot {
            entries: vec![
                InventoryItemEntry {
                    storage: "bags".into(),
                    slot: 4,
                    item_guid: 101,
                    item_id: 25,
                    name: "Worn Shortsword".into(),
                    stack_count: 1,
                },
                InventoryItemEntry {
                    storage: "guild_vault".into(),
                    slot: 7,
                    item_guid: 202,
                    item_id: 2589,
                    name: "Linen Cloth".into(),
                    stack_count: 12,
                },
            ],
        };

        let text = format_inventory_search(&snapshot, "lin");

        assert!(text.contains("[bags]"));
        assert!(text.contains("[guild_vault]"));
        assert!(text.contains("Linen Cloth"));
    }

    #[test]
    fn inventory_search_empty_result_formats_placeholder() {
        let text = format_inventory_search(&InventorySearchSnapshot::default(), "torch");

        assert_eq!(text, "inventory search text=torch: 0\n-");
    }

    #[test]
    fn resolve_spell_target_requires_current_selection() {
        let target = CurrentTarget(None);

        let err = resolve_spell_target(Some("current"), &target).expect_err("missing target");

        assert_eq!(err, "no current target selected");
    }

    #[test]
    fn build_inventory_entries_reads_bag_snapshot() {
        let entries = build_inventory_entries(
            Some(&AuctionInventorySnapshot {
                gold: 0,
                items: vec![AuctionInventoryItem {
                    item_guid: 42,
                    item_id: 2589,
                    name: "Linen Cloth".into(),
                    quality: 1,
                    required_level: 1,
                    stack_count: 7,
                    vendor_sell_price: 13,
                }],
            }),
            &[],
            &[],
        );

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].storage, "bags");
        assert_eq!(entries[0].item_id, 2589);
    }
}
