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
    CharacterStatsSnapshot, CollectionStatusSnapshot, CombatLogEntry, CombatLogEventKind,
    CombatLogStatusSnapshot, CurrenciesStatusSnapshot, EquippedGearStatusSnapshot, GroupRole,
    GroupStatusSnapshot, GuildVaultStatusSnapshot, InventoryItemEntry, InventorySearchSnapshot,
    MapStatusSnapshot, NetworkStatusSnapshot, ProfessionStatusSnapshot, QuestLogStatusSnapshot,
    QuestRepeatability, ReputationsStatusSnapshot, SoundStatusSnapshot, TerrainStatusSnapshot,
    WarbankStatusSnapshot, Waypoint,
};
use crate::targeting::CurrentTarget;
use crate::ui::plugin::UiState;
use shared::protocol::{
    AuctionInventorySnapshot, CombatChannel, GroupInviteIntent, GroupUninviteIntent,
    SpellCastIntent, StopSpellCast,
};

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
    expanded_status_snapshots: (
        Res<QuestLogStatusSnapshot>,
        Res<GroupStatusSnapshot>,
        Res<CombatLogStatusSnapshot>,
        Res<CollectionStatusSnapshot>,
        Res<ProfessionStatusSnapshot>,
        ResMut<MapStatusSnapshot>,
    ),
    current_target: Res<CurrentTarget>,
    mut spell_cast_senders: Query<&mut MessageSender<SpellCastIntent>>,
    mut spell_stop_senders: Query<&mut MessageSender<StopSpellCast>>,
    mut group_invite_senders: Query<&mut MessageSender<GroupInviteIntent>>,
    mut group_uninvite_senders: Query<&mut MessageSender<GroupUninviteIntent>>,
    connected_query: Query<(), With<Connected>>,
) {
    let (network_status, terrain_status, sound_status, currencies_status, reputations_status) =
        primary_status_snapshots;
    let (character_stats, guild_vault_status, warbank_status, equipped_gear_status) =
        secondary_status_snapshots;
    let (
        quest_status,
        group_status,
        combat_log_status,
        collection_status,
        profession_status,
        mut map_status,
    ) = expanded_status_snapshots;
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
            &quest_status,
            &group_status,
            &combat_log_status,
            &collection_status,
            &profession_status,
            map_status.as_mut(),
            &current_target,
            &mut spell_cast_senders,
            &mut spell_stop_senders,
            &mut group_invite_senders,
            &mut group_uninvite_senders,
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
    quest_status: &QuestLogStatusSnapshot,
    group_status: &GroupStatusSnapshot,
    combat_log_status: &CombatLogStatusSnapshot,
    collection_status: &CollectionStatusSnapshot,
    profession_status: &ProfessionStatusSnapshot,
    map_status: &mut MapStatusSnapshot,
    current_target: &CurrentTarget,
    spell_cast_senders: &mut Query<&mut MessageSender<SpellCastIntent>>,
    spell_stop_senders: &mut Query<&mut MessageSender<StopSpellCast>>,
    group_invite_senders: &mut Query<&mut MessageSender<GroupInviteIntent>>,
    group_uninvite_senders: &mut Query<&mut MessageSender<GroupUninviteIntent>>,
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
            let (spell_id, spell_token) = match resolve_spell_identifier(&spell) {
                Ok(value) => value,
                Err(error) => {
                    let _ = cmd.respond.send(Response::Error(error));
                    return;
                }
            };
            let intent = SpellCastIntent {
                spell_id,
                spell: spell_token,
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
                .send(Response::Text(format_quest_list(quest_status)));
        }
        Request::QuestWatch => {
            let _ = cmd
                .respond
                .send(Response::Text(format_quest_watch(quest_status)));
        }
        Request::QuestShow { quest_id } => {
            let _ = cmd
                .respond
                .send(Response::Text(format_quest_show(quest_status, quest_id)));
        }
        Request::GroupRoster => {
            let _ = cmd
                .respond
                .send(Response::Text(format_group_roster(group_status)));
        }
        Request::GroupStatus => {
            let _ = cmd
                .respond
                .send(Response::Text(format_group_status(group_status)));
        }
        Request::GroupInvite { name } => {
            if !connected {
                let _ = cmd.respond.send(Response::Error(
                    "group invite is unavailable: not connected".into(),
                ));
            } else if send_combat_message(
                group_invite_senders,
                GroupInviteIntent { name: name.clone() },
            ) {
                let _ = cmd
                    .respond
                    .send(Response::Text(format!("group invite submitted for {name}")));
            } else {
                let _ = cmd
                    .respond
                    .send(Response::Error("group invite sender unavailable".into()));
            }
        }
        Request::GroupUninvite { name } => {
            if !connected {
                let _ = cmd.respond.send(Response::Error(
                    "group uninvite is unavailable: not connected".into(),
                ));
            } else if send_combat_message(
                group_uninvite_senders,
                GroupUninviteIntent { name: name.clone() },
            ) {
                let _ = cmd.respond.send(Response::Text(format!(
                    "group uninvite submitted for {name}"
                )));
            } else {
                let _ = cmd
                    .respond
                    .send(Response::Error("group uninvite sender unavailable".into()));
            }
        }
        Request::CombatLog { lines } => {
            let _ = cmd
                .respond
                .send(Response::Text(format_combat_log(combat_log_status, lines)));
        }
        Request::CombatRecap { target } => {
            let _ = cmd.respond.send(Response::Text(format_combat_recap(
                combat_log_status,
                target.as_deref(),
            )));
        }
        Request::ReputationList => {
            let _ = cmd.respond.send(Response::Text(format_reputations_status(
                reputations_status,
            )));
        }
        Request::CollectionMounts { missing } => {
            let _ = cmd.respond.send(Response::Text(format_collection_mounts(
                collection_status,
                missing,
            )));
        }
        Request::CollectionPets { missing } => {
            let _ = cmd.respond.send(Response::Text(format_collection_pets(
                collection_status,
                missing,
            )));
        }
        Request::ProfessionRecipes { text } => {
            let _ = cmd.respond.send(Response::Text(format_profession_recipes(
                profession_status,
                &text,
            )));
        }
        Request::MapPosition => {
            let _ = cmd
                .respond
                .send(Response::Text(format_map_position(map_status)));
        }
        Request::MapTarget => {
            let _ = cmd.respond.send(Response::Text(format_map_target(
                map_status,
                current_target,
                tree_query,
            )));
        }
        Request::MapWaypointAdd { x, y } => {
            map_status.waypoint = Some(Waypoint { x, y });
            let _ = cmd
                .respond
                .send(Response::Text(format_map_position(map_status)));
        }
        Request::MapWaypointClear => {
            map_status.waypoint = None;
            let _ = cmd
                .respond
                .send(Response::Text(format_map_position(map_status)));
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

fn format_quest_list(snapshot: &QuestLogStatusSnapshot) -> String {
    if snapshot.entries.is_empty() {
        return "quests: 0\n-".into();
    }
    let lines = snapshot
        .entries
        .iter()
        .map(|entry| {
            format!(
                "{} {} zone={} repeat={} completed={} objectives={}",
                entry.quest_id,
                entry.title,
                entry.zone,
                quest_repeatability_label(&entry.repeatability),
                entry.completed,
                entry.objectives.len()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("quests: {}\n{lines}", snapshot.entries.len())
}

fn format_quest_watch(snapshot: &QuestLogStatusSnapshot) -> String {
    let watched = snapshot
        .watched_quest_ids
        .iter()
        .filter_map(|id| snapshot.entries.iter().find(|entry| entry.quest_id == *id))
        .collect::<Vec<_>>();
    if watched.is_empty() {
        return "quest_watch: 0\n-".into();
    }
    let lines = watched
        .iter()
        .map(|entry| {
            let objective = entry
                .objectives
                .iter()
                .map(|obj| format!("{} {}/{}", obj.text, obj.current, obj.required))
                .collect::<Vec<_>>()
                .join("; ");
            format!("{} {} [{}]", entry.quest_id, entry.title, objective)
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("quest_watch: {}\n{lines}", watched.len())
}

fn format_quest_show(snapshot: &QuestLogStatusSnapshot, quest_id: u32) -> String {
    let Some(entry) = snapshot
        .entries
        .iter()
        .find(|entry| entry.quest_id == quest_id)
    else {
        return format!("quest {quest_id}: not found");
    };
    let objectives = if entry.objectives.is_empty() {
        "-".into()
    } else {
        entry
            .objectives
            .iter()
            .map(|obj| {
                format!(
                    "{} {}/{} completed={}",
                    obj.text, obj.current, obj.required, obj.completed
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        "quest_id: {}\ntitle: {}\nzone: {}\nrepeatability: {}\ncompleted: {}\nobjectives:\n{}",
        entry.quest_id,
        entry.title,
        entry.zone,
        quest_repeatability_label(&entry.repeatability),
        entry.completed,
        objectives,
    )
}

fn quest_repeatability_label(value: &QuestRepeatability) -> &'static str {
    match value {
        QuestRepeatability::Normal => "normal",
        QuestRepeatability::Daily => "daily",
        QuestRepeatability::Weekly => "weekly",
    }
}

fn format_group_roster(snapshot: &GroupStatusSnapshot) -> String {
    if snapshot.members.is_empty() {
        return "group_roster: 0\n-".into();
    }
    let lines = snapshot
        .members
        .iter()
        .map(|member| {
            format!(
                "{} leader={} role={} online={} subgroup={}",
                member.name,
                member.is_leader,
                group_role_label(&member.role),
                member.online,
                member.subgroup
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("group_roster: {}\n{lines}", snapshot.members.len())
}

fn format_group_status(snapshot: &GroupStatusSnapshot) -> String {
    format!(
        "in_group: {}\nis_raid: {}\nmembers: {}\nready: {}/{}\nlast_message: {}",
        !snapshot.members.is_empty(),
        snapshot.is_raid,
        snapshot.members.len(),
        snapshot.ready_count,
        snapshot.total_count,
        snapshot.last_server_message.as_deref().unwrap_or("-")
    )
}

fn group_role_label(role: &GroupRole) -> &'static str {
    match role {
        GroupRole::Tank => "tank",
        GroupRole::Healer => "healer",
        GroupRole::Damage => "damage",
        GroupRole::None => "none",
    }
}

fn format_combat_log(snapshot: &CombatLogStatusSnapshot, lines: u16) -> String {
    if snapshot.entries.is_empty() {
        return "combat_log: 0\n-".into();
    }
    let take_count = usize::from(lines).max(1);
    let selected = snapshot
        .entries
        .iter()
        .rev()
        .take(take_count)
        .collect::<Vec<_>>();
    let text = selected
        .iter()
        .map(|entry| format_combat_entry(entry))
        .collect::<Vec<_>>()
        .join("\n");
    format!("combat_log: {}\n{}", selected.len(), text)
}

fn format_combat_recap(snapshot: &CombatLogStatusSnapshot, target: Option<&str>) -> String {
    let target = target.unwrap_or("current").to_ascii_lowercase();
    let filtered = snapshot
        .entries
        .iter()
        .rev()
        .filter(|entry| {
            target == "current"
                || entry.target.to_ascii_lowercase() == target
                || entry.source.to_ascii_lowercase() == target
        })
        .take(10)
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        return format!("combat_recap target={target}: 0\n-");
    }
    let text = filtered
        .iter()
        .map(|entry| format_combat_entry(entry))
        .collect::<Vec<_>>()
        .join("\n");
    format!("combat_recap target={target}: {}\n{text}", filtered.len())
}

fn format_combat_entry(entry: &CombatLogEntry) -> String {
    let kind = match entry.kind {
        CombatLogEventKind::Damage => "damage",
        CombatLogEventKind::Heal => "heal",
        CombatLogEventKind::Interrupt => "interrupt",
        CombatLogEventKind::AuraApplied => "aura",
        CombatLogEventKind::Death => "death",
    };
    format!(
        "{} src={} dst={} spell={} amount={} aura={} text={}",
        kind,
        entry.source,
        entry.target,
        entry.spell.as_deref().unwrap_or("-"),
        entry
            .amount
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".into()),
        entry.aura.as_deref().unwrap_or("-"),
        entry.text,
    )
}

fn format_collection_mounts(snapshot: &CollectionStatusSnapshot, missing: bool) -> String {
    let filtered = snapshot
        .mounts
        .iter()
        .filter(|entry| !missing || !entry.known)
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        return "mounts: 0\n-".into();
    }
    let lines = filtered
        .iter()
        .map(|entry| format!("{} {} known={}", entry.mount_id, entry.name, entry.known))
        .collect::<Vec<_>>()
        .join("\n");
    format!("mounts: {}\n{lines}", filtered.len())
}

fn format_collection_pets(snapshot: &CollectionStatusSnapshot, missing: bool) -> String {
    let filtered = snapshot
        .pets
        .iter()
        .filter(|entry| !missing || !entry.known)
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        return "pets: 0\n-".into();
    }
    let lines = filtered
        .iter()
        .map(|entry| format!("{} {} known={}", entry.pet_id, entry.name, entry.known))
        .collect::<Vec<_>>()
        .join("\n");
    format!("pets: {}\n{lines}", filtered.len())
}

fn format_profession_recipes(snapshot: &ProfessionStatusSnapshot, text: &str) -> String {
    let needle = text.trim().to_ascii_lowercase();
    let filtered = snapshot
        .recipes
        .iter()
        .filter(|entry| needle.is_empty() || entry.name.to_ascii_lowercase().contains(&needle))
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        return format!("recipes text={text}: 0\n-");
    }
    let lines = filtered
        .iter()
        .map(|entry| {
            format!(
                "{} {} profession={} craftable={} cooldown={}",
                entry.spell_id,
                entry.name,
                entry.profession,
                entry.craftable,
                entry.cooldown.as_deref().unwrap_or("-")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("recipes text={text}: {}\n{lines}", filtered.len())
}

fn format_map_position(snapshot: &MapStatusSnapshot) -> String {
    let waypoint = snapshot
        .waypoint
        .map(|waypoint| format!("{:.2},{:.2}", waypoint.x, waypoint.y))
        .unwrap_or_else(|| "-".into());
    format!(
        "zone_id: {}\nposition: {:.2},{:.2}\nwaypoint: {}",
        snapshot.zone_id, snapshot.player_x, snapshot.player_z, waypoint
    )
}

fn format_map_target(
    map_status: &MapStatusSnapshot,
    current_target: &CurrentTarget,
    tree_query: &Query<(
        Entity,
        Option<&Name>,
        Option<&Children>,
        Option<&Visibility>,
        &Transform,
    )>,
) -> String {
    let Some(target) = current_target.0 else {
        return map_target_none_text();
    };
    let Ok((entity, name, _, _, transform)) = tree_query.get(target) else {
        return "map_target: missing\ndistance: -".into();
    };
    let dx = transform.translation.x - map_status.player_x;
    let dz = transform.translation.z - map_status.player_z;
    let distance = (dx * dx + dz * dz).sqrt();
    format!(
        "map_target: {}\nentity: {}\nposition: {:.2},{:.2}\ndistance: {:.2}",
        name.map(|value| value.as_str()).unwrap_or("unnamed"),
        entity.to_bits(),
        transform.translation.x,
        transform.translation.z,
        distance,
    )
}

fn map_target_none_text() -> String {
    "map_target: none\ndistance: -".into()
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

fn resolve_spell_identifier(spell: &str) -> Result<(Option<u32>, String), String> {
    let trimmed = spell.trim();
    if trimmed.is_empty() {
        return Err("spell identifier cannot be empty".into());
    }
    if let Ok(spell_id) = trimmed.parse::<u32>() {
        return Ok((Some(spell_id), trimmed.to_string()));
    }
    let valid_token = trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == ' ' || ch == '-');
    if !valid_token {
        return Err(format!("invalid spell identifier '{trimmed}'"));
    }
    Ok((None, trimmed.to_string()))
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

    #[test]
    fn quest_list_formats_daily_and_objective_counters() {
        let snapshot = QuestLogStatusSnapshot {
            entries: vec![crate::status::QuestEntry {
                quest_id: 101,
                title: "Defend the Farm".into(),
                zone: "Westfall".into(),
                completed: false,
                repeatability: QuestRepeatability::Daily,
                objectives: vec![crate::status::QuestObjectiveEntry {
                    text: "Harvest Watchers slain".into(),
                    current: 3,
                    required: 8,
                    completed: false,
                }],
            }],
            watched_quest_ids: vec![],
        };

        let text = format_quest_list(&snapshot);

        assert!(text.contains("repeat=daily"));
        assert!(text.contains("objectives=1"));
    }

    #[test]
    fn group_roster_formatter_shows_leader_role_online_and_subgroup() {
        let snapshot = GroupStatusSnapshot {
            is_raid: false,
            members: vec![crate::status::GroupMemberEntry {
                name: "Thrall".into(),
                role: GroupRole::Healer,
                is_leader: true,
                online: true,
                subgroup: 1,
            }],
            ready_count: 1,
            total_count: 1,
            last_server_message: None,
        };

        let text = format_group_roster(&snapshot);

        assert!(text.contains("leader=true"));
        assert!(text.contains("role=healer"));
        assert!(text.contains("online=true"));
        assert!(text.contains("subgroup=1"));
    }

    #[test]
    fn combat_log_formats_damage_heal_interrupt_aura_and_death() {
        let snapshot = CombatLogStatusSnapshot {
            entries: vec![
                CombatLogEntry {
                    kind: CombatLogEventKind::Damage,
                    source: "A".into(),
                    target: "B".into(),
                    spell: Some("Strike".into()),
                    amount: Some(10),
                    aura: None,
                    text: "hit".into(),
                },
                CombatLogEntry {
                    kind: CombatLogEventKind::Heal,
                    source: "A".into(),
                    target: "B".into(),
                    spell: Some("Heal".into()),
                    amount: Some(8),
                    aura: None,
                    text: "heal".into(),
                },
                CombatLogEntry {
                    kind: CombatLogEventKind::Interrupt,
                    source: "A".into(),
                    target: "B".into(),
                    spell: Some("Kick".into()),
                    amount: None,
                    aura: None,
                    text: "interrupt".into(),
                },
                CombatLogEntry {
                    kind: CombatLogEventKind::AuraApplied,
                    source: "A".into(),
                    target: "B".into(),
                    spell: Some("Buff".into()),
                    amount: None,
                    aura: Some("Power".into()),
                    text: "aura".into(),
                },
                CombatLogEntry {
                    kind: CombatLogEventKind::Death,
                    source: "A".into(),
                    target: "B".into(),
                    spell: None,
                    amount: None,
                    aura: None,
                    text: "death".into(),
                },
            ],
        };

        let text = format_combat_log(&snapshot, 10);

        assert!(text.contains("damage"));
        assert!(text.contains("heal"));
        assert!(text.contains("interrupt"));
        assert!(text.contains("aura"));
        assert!(text.contains("death"));
    }

    #[test]
    fn combat_recap_orders_newest_first() {
        let snapshot = CombatLogStatusSnapshot {
            entries: vec![
                CombatLogEntry {
                    kind: CombatLogEventKind::Damage,
                    source: "A".into(),
                    target: "B".into(),
                    spell: Some("First".into()),
                    amount: Some(1),
                    aura: None,
                    text: "first".into(),
                },
                CombatLogEntry {
                    kind: CombatLogEventKind::Damage,
                    source: "A".into(),
                    target: "B".into(),
                    spell: Some("Second".into()),
                    amount: Some(2),
                    aura: None,
                    text: "second".into(),
                },
            ],
        };

        let text = format_combat_recap(&snapshot, Some("B"));
        let first = text.find("Second").expect("second entry present");
        let second = text.find("First").expect("first entry present");

        assert!(first < second);
    }

    #[test]
    fn collection_mounts_missing_filters_known_entries() {
        let snapshot = CollectionStatusSnapshot {
            mounts: vec![
                crate::status::CollectionMountEntry {
                    mount_id: 1,
                    name: "Horse".into(),
                    known: true,
                },
                crate::status::CollectionMountEntry {
                    mount_id: 2,
                    name: "Wolf".into(),
                    known: false,
                },
            ],
            pets: vec![],
        };

        let text = format_collection_mounts(&snapshot, true);

        assert!(!text.contains("Horse"));
        assert!(text.contains("Wolf"));
    }

    #[test]
    fn profession_recipes_filters_by_text() {
        let snapshot = ProfessionStatusSnapshot {
            recipes: vec![crate::status::ProfessionRecipeEntry {
                spell_id: 100,
                profession: "Alchemy".into(),
                name: "Major Healing Potion".into(),
                craftable: true,
                cooldown: None,
            }],
        };

        let text = format_profession_recipes(&snapshot, "potion");

        assert!(text.contains("Major Healing Potion"));
    }

    #[test]
    fn map_target_none_formatter_is_clear() {
        let text = map_target_none_text();

        assert_eq!(text, "map_target: none\ndistance: -");
    }
}
