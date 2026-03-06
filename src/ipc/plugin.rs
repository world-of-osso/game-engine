//! Bevy plugin that integrates the IPC server with the render pipeline.

use std::sync::mpsc;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use lightyear::prelude::client::Connected;

use super::{Command, Request, Response, init};
use crate::auction_house::{AuctionHouseState, queue_ipc_request};
use crate::item_info::lookup_item_info;
use crate::mail::{MailState, queue_ipc_request as queue_mail_ipc_request};
use crate::status::{
    CharacterStatsSnapshot, CurrenciesStatusSnapshot, NetworkStatusSnapshot,
    EquippedGearStatusSnapshot, GuildVaultStatusSnapshot, ReputationsStatusSnapshot,
    SoundStatusSnapshot, TerrainStatusSnapshot, WarbankStatusSnapshot,
};
use shared::protocol::AuctionInventorySnapshot;
use crate::ui::plugin::UiState;

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
    connected_query: Query<(), With<Connected>>,
) {
    let (
        network_status,
        terrain_status,
        sound_status,
        currencies_status,
        reputations_status,
    ) = primary_status_snapshots;
    let (
        character_stats,
        guild_vault_status,
        warbank_status,
        equipped_gear_status,
    ) = secondary_status_snapshots;
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
            let _ = cmd
                .respond
                .send(Response::Text(format_network_status(network_status, connected)));
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
            let _ = cmd
                .respond
                .send(Response::Text(format_reputations_status(reputations_status)));
        }
        Request::CharacterStatsStatus => {
            let _ = cmd
                .respond
                .send(Response::Text(format_character_stats_status(character_stats)));
        }
        Request::BagsStatus => {
            let _ = cmd
                .respond
                .send(Response::Text(format_bags_status(auction_house.inventory.as_ref())));
        }
        Request::GuildVaultStatus => {
            let _ = cmd
                .respond
                .send(Response::Text(format_storage_list("guild_vault", &guild_vault_status.entries)));
        }
        Request::WarbankStatus => {
            let _ = cmd
                .respond
                .send(Response::Text(format_storage_list("warbank", &warbank_status.entries)));
        }
        Request::EquippedGearStatus => {
            let _ = cmd
                .respond
                .send(Response::Text(format_equipped_gear_status(equipped_gear_status)));
        }
        Request::ItemInfo { query } => match lookup_item_info(query.item_id) {
            Ok(Some(item)) => {
                let appearance_known = auction_house
                    .inventory
                    .as_ref()
                    .is_some_and(|inventory| inventory.items.iter().any(|entry| entry.item_id == item.item_id));
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
    format!("bags: {}\ngold: {}\n{}", snapshot.items.len(), snapshot.gold, lines)
}

fn format_storage_list(
    title: &str,
    entries: &[crate::status::StorageItemEntry],
) -> String {
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
}
