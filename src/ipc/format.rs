//! IPC response formatting helpers.

use crate::status::{
    CharacterStatsSnapshot, CollectionStatusSnapshot, CombatLogEntry, CombatLogEventKind,
    CombatLogStatusSnapshot, CurrenciesStatusSnapshot, EquippedGearStatusSnapshot, GroupRole,
    GroupStatusSnapshot, InventoryItemEntry, InventorySearchSnapshot, MapStatusSnapshot,
    NetworkStatusSnapshot, ProfessionStatusSnapshot, QuestLogStatusSnapshot, QuestRepeatability,
    ReputationsStatusSnapshot, SoundStatusSnapshot, TerrainStatusSnapshot,
};
use crate::targeting::CurrentTarget;
use shared::protocol::AuctionInventorySnapshot;

use super::plugin::DispatchContext;
use super::{Command, Request, Response};

/// Returns true if the request was a status query and was handled.
pub fn dispatch_status_request(cmd: &Command, ctx: &DispatchContext) -> bool {
    let text = match &cmd.request {
        Request::NetworkStatus => format_network_status(ctx.network_status, ctx.connected),
        Request::TerrainStatus => format_terrain_status(ctx.terrain_status),
        Request::SoundStatus => format_sound_status(ctx.sound_status),
        Request::CurrenciesStatus => format_currencies_status(ctx.currencies_status),
        Request::ReputationsStatus | Request::ReputationList => {
            format_reputations_status(ctx.reputations_status)
        }
        Request::CharacterStatsStatus => format_character_stats_status(ctx.character_stats),
        Request::EquippedGearStatus => format_equipped_gear_status(ctx.equipped_gear_status),
        Request::GuildVaultStatus => {
            format_storage_list("guild_vault", &ctx.guild_vault_status.entries)
        }
        Request::WarbankStatus => format_storage_list("warbank", &ctx.warbank_status.entries),
        Request::QuestList => format_quest_list(ctx.quest_status),
        Request::QuestWatch => format_quest_watch(ctx.quest_status),
        Request::QuestShow { quest_id } => format_quest_show(ctx.quest_status, *quest_id),
        Request::GroupRoster => format_group_roster(ctx.group_status),
        Request::GroupStatus => format_group_status(ctx.group_status),
        Request::CombatLog { lines } => format_combat_log(ctx.combat_log_status, *lines),
        Request::CombatRecap { target } => {
            format_combat_recap(ctx.combat_log_status, target.as_deref())
        }
        Request::CollectionMounts { missing } => {
            format_collection_mounts(ctx.collection_status, *missing)
        }
        Request::CollectionPets { missing } => {
            format_collection_pets(ctx.collection_status, *missing)
        }
        Request::ProfessionRecipes { text } => {
            format_profession_recipes(ctx.profession_status, text)
        }
        _ => return false,
    };
    let _ = cmd.respond.send(Response::Text(text));
    true
}

pub fn format_network_status(snapshot: &NetworkStatusSnapshot, connected: bool) -> String {
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

pub fn format_terrain_status(snapshot: &TerrainStatusSnapshot) -> String {
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

pub fn format_sound_status(snapshot: &SoundStatusSnapshot) -> String {
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

pub fn format_currencies_status(snapshot: &CurrenciesStatusSnapshot) -> String {
    if snapshot.entries.is_empty() {
        return "currencies: 0\n-".into();
    }
    let lines = snapshot
        .entries
        .iter()
        .map(|e| format!("{} {} amount={}", e.id, e.name, e.amount))
        .collect::<Vec<_>>()
        .join("\n");
    format!("currencies: {}\n{lines}", snapshot.entries.len())
}

pub fn format_reputations_status(snapshot: &ReputationsStatusSnapshot) -> String {
    if snapshot.entries.is_empty() {
        return "reputations: 0\n-".into();
    }
    let lines = snapshot
        .entries
        .iter()
        .map(|e| {
            format!(
                "{} {} standing={} value={}",
                e.faction_id, e.faction_name, e.standing, e.value
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("reputations: {}\n{lines}", snapshot.entries.len())
}

fn opt_int(value: Option<impl std::fmt::Display>) -> String {
    value.map(|v| v.to_string()).unwrap_or_else(|| "-".into())
}

fn opt_float0(value: Option<f32>) -> String {
    value
        .map(|v| format!("{v:.0}"))
        .unwrap_or_else(|| "-".into())
}

fn opt_float2(value: Option<f32>) -> String {
    value
        .map(|v| format!("{v:.2}"))
        .unwrap_or_else(|| "-".into())
}

pub fn format_character_stats_status(snapshot: &CharacterStatsSnapshot) -> String {
    format!(
        "name: {}\nlevel: {}\nrace: {}\nclass: {}\nhealth: {}/{}\nmana: {}/{}\nmovement_speed: {}\nzone_id: {}",
        snapshot.name.as_deref().unwrap_or("-"),
        opt_int(snapshot.level),
        opt_int(snapshot.race),
        opt_int(snapshot.class),
        opt_float0(snapshot.health_current),
        opt_float0(snapshot.health_max),
        opt_float0(snapshot.mana_current),
        opt_float0(snapshot.mana_max),
        opt_float2(snapshot.movement_speed),
        snapshot.zone_id,
    )
}

pub fn format_bags_status(snapshot: Option<&AuctionInventorySnapshot>) -> String {
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

pub fn format_storage_list(title: &str, entries: &[crate::status::StorageItemEntry]) -> String {
    if entries.is_empty() {
        return format!("{title}: 0\n-");
    }
    let lines = entries
        .iter()
        .map(|e| {
            format!(
                "{} {} {} {} x{}",
                e.slot, e.item_guid, e.item_id, e.name, e.stack_count
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("{title}: {}\n{lines}", entries.len())
}

pub fn format_equipped_gear_status(snapshot: &EquippedGearStatusSnapshot) -> String {
    if snapshot.entries.is_empty() {
        return "equipped_gear: 0\n-".into();
    }
    let lines = snapshot
        .entries
        .iter()
        .map(|e| format!("{} {}", e.slot, e.path))
        .collect::<Vec<_>>()
        .join("\n");
    format!("equipped_gear: {}\n{lines}", snapshot.entries.len())
}

pub fn format_item_info(item: &crate::item_info::ItemStaticInfo, appearance_known: bool) -> String {
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

pub fn format_quest_list(snapshot: &QuestLogStatusSnapshot) -> String {
    if snapshot.entries.is_empty() {
        return "quests: 0\n-".into();
    }
    let lines = snapshot
        .entries
        .iter()
        .map(|e| {
            format!(
                "{} {} zone={} repeat={} completed={} objectives={}",
                e.quest_id,
                e.title,
                e.zone,
                quest_repeatability_label(&e.repeatability),
                e.completed,
                e.objectives.len()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("quests: {}\n{lines}", snapshot.entries.len())
}

pub fn format_quest_watch(snapshot: &QuestLogStatusSnapshot) -> String {
    let watched = snapshot
        .watched_quest_ids
        .iter()
        .filter_map(|id| snapshot.entries.iter().find(|e| e.quest_id == *id))
        .collect::<Vec<_>>();
    if watched.is_empty() {
        return "quest_watch: 0\n-".into();
    }
    let lines = watched
        .iter()
        .map(|e| {
            let obj = e
                .objectives
                .iter()
                .map(|o| format!("{} {}/{}", o.text, o.current, o.required))
                .collect::<Vec<_>>()
                .join("; ");
            format!("{} {} [{}]", e.quest_id, e.title, obj)
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("quest_watch: {}\n{lines}", watched.len())
}

pub fn format_quest_show(snapshot: &QuestLogStatusSnapshot, quest_id: u32) -> String {
    let Some(entry) = snapshot.entries.iter().find(|e| e.quest_id == quest_id) else {
        return format!("quest {quest_id}: not found");
    };
    let objectives = if entry.objectives.is_empty() {
        "-".into()
    } else {
        entry
            .objectives
            .iter()
            .map(|o| {
                format!(
                    "{} {}/{} completed={}",
                    o.text, o.current, o.required, o.completed
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

pub fn format_group_roster(snapshot: &GroupStatusSnapshot) -> String {
    if snapshot.members.is_empty() {
        return "group_roster: 0\n-".into();
    }
    let lines = snapshot
        .members
        .iter()
        .map(|m| {
            format!(
                "{} leader={} role={} online={} subgroup={}",
                m.name,
                m.is_leader,
                group_role_label(&m.role),
                m.online,
                m.subgroup
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("group_roster: {}\n{lines}", snapshot.members.len())
}

pub fn format_group_status(snapshot: &GroupStatusSnapshot) -> String {
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

pub fn format_combat_log(snapshot: &CombatLogStatusSnapshot, lines: u16) -> String {
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
        .map(|e| format_combat_entry(e))
        .collect::<Vec<_>>()
        .join("\n");
    format!("combat_log: {}\n{}", selected.len(), text)
}

pub fn format_combat_recap(snapshot: &CombatLogStatusSnapshot, target: Option<&str>) -> String {
    let target = target.unwrap_or("current").to_ascii_lowercase();
    let filtered = snapshot
        .entries
        .iter()
        .rev()
        .filter(|e| {
            target == "current"
                || e.target.to_ascii_lowercase() == target
                || e.source.to_ascii_lowercase() == target
        })
        .take(10)
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        return format!("combat_recap target={target}: 0\n-");
    }
    let text = filtered
        .iter()
        .map(|e| format_combat_entry(e))
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
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".into()),
        entry.aura.as_deref().unwrap_or("-"),
        entry.text,
    )
}

pub fn format_collection_mounts(snapshot: &CollectionStatusSnapshot, missing: bool) -> String {
    let filtered = snapshot
        .mounts
        .iter()
        .filter(|e| !missing || !e.known)
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        return "mounts: 0\n-".into();
    }
    let lines = filtered
        .iter()
        .map(|e| format!("{} {} known={}", e.mount_id, e.name, e.known))
        .collect::<Vec<_>>()
        .join("\n");
    format!("mounts: {}\n{lines}", filtered.len())
}

pub fn format_collection_pets(snapshot: &CollectionStatusSnapshot, missing: bool) -> String {
    let filtered = snapshot
        .pets
        .iter()
        .filter(|e| !missing || !e.known)
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        return "pets: 0\n-".into();
    }
    let lines = filtered
        .iter()
        .map(|e| format!("{} {} known={}", e.pet_id, e.name, e.known))
        .collect::<Vec<_>>()
        .join("\n");
    format!("pets: {}\n{lines}", filtered.len())
}

pub fn format_profession_recipes(snapshot: &ProfessionStatusSnapshot, text: &str) -> String {
    let needle = text.trim().to_ascii_lowercase();
    let filtered = snapshot
        .recipes
        .iter()
        .filter(|e| needle.is_empty() || e.name.to_ascii_lowercase().contains(&needle))
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        return format!("recipes text={text}: 0\n-");
    }
    let lines = filtered
        .iter()
        .map(|e| {
            format!(
                "{} {} profession={} craftable={} cooldown={}",
                e.spell_id,
                e.name,
                e.profession,
                e.craftable,
                e.cooldown.as_deref().unwrap_or("-")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("recipes text={text}: {}\n{lines}", filtered.len())
}

pub fn format_map_position(snapshot: &MapStatusSnapshot) -> String {
    let waypoint = snapshot
        .waypoint
        .map(|w| format!("{:.2},{:.2}", w.x, w.y))
        .unwrap_or_else(|| "-".into());
    format!(
        "zone_id: {}\nposition: {:.2},{:.2}\nwaypoint: {}",
        snapshot.zone_id, snapshot.player_x, snapshot.player_z, waypoint
    )
}

pub fn format_map_target(
    map_status: &MapStatusSnapshot,
    current_target: &CurrentTarget,
    tree_query: &super::plugin::TreeQuery,
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
        name.map(|v| v.as_str()).unwrap_or("unnamed"),
        entity.to_bits(),
        transform.translation.x,
        transform.translation.z,
        distance,
    )
}

pub fn map_target_none_text() -> String {
    "map_target: none\ndistance: -".into()
}

pub fn build_inventory_entries(
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
    entries.extend(guild_vault.iter().map(|e| InventoryItemEntry {
        storage: "guild_vault".into(),
        slot: e.slot,
        item_guid: e.item_guid,
        item_id: e.item_id,
        name: e.name.clone(),
        stack_count: e.stack_count,
    }));
    entries.extend(warbank.iter().map(|e| InventoryItemEntry {
        storage: "warbank".into(),
        slot: e.slot,
        item_guid: e.item_guid,
        item_id: e.item_id,
        name: e.name.clone(),
        stack_count: e.stack_count,
    }));
    entries
}

pub fn inventory_search_snapshot(
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
            .filter(|e| e.name.to_ascii_lowercase().contains(&needle))
            .cloned()
            .collect(),
    }
}

pub fn format_inventory_list(entries: &[InventoryItemEntry]) -> String {
    if entries.is_empty() {
        return "inventory: 0\n-".into();
    }
    let lines = entries
        .iter()
        .map(|e| {
            format!(
                "{}:{} {} {} {} x{}",
                e.storage, e.slot, e.item_guid, e.item_id, e.name, e.stack_count
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("inventory: {}\n{lines}", entries.len())
}

pub fn format_inventory_search(snapshot: &InventorySearchSnapshot, text: &str) -> String {
    if snapshot.entries.is_empty() {
        return format!("inventory search text={text}: 0\n-");
    }
    let mut lines = Vec::new();
    for storage in ["bags", "guild_vault", "warbank"] {
        let grouped = snapshot
            .entries
            .iter()
            .filter(|e| e.storage == storage)
            .collect::<Vec<_>>();
        if grouped.is_empty() {
            continue;
        }
        lines.push(format!("[{storage}]"));
        lines.extend(grouped.iter().map(|e| {
            format!(
                "{} {} {} {} x{}",
                e.slot, e.item_guid, e.item_id, e.name, e.stack_count
            )
        }));
    }
    format!(
        "inventory search text={text}: {}\n{}",
        snapshot.entries.len(),
        lines.join("\n")
    )
}

pub fn format_inventory_whereis(entries: &[InventoryItemEntry], item_id: u32) -> String {
    let matches = entries
        .iter()
        .filter(|e| e.item_id == item_id)
        .collect::<Vec<_>>();
    if matches.is_empty() {
        return format!("inventory whereis item_id={item_id}: 0\n-");
    }
    let lines = matches
        .iter()
        .map(|e| {
            format!(
                "{}:{} {} {} x{}",
                e.storage, e.slot, e.item_guid, e.name, e.stack_count
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "inventory whereis item_id={item_id}: {}\n{lines}",
        matches.len()
    )
}

pub fn resolve_spell_identifier(spell: &str) -> Result<(Option<u32>, String), String> {
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

pub fn resolve_spell_target(
    selector: Option<&str>,
    current_target: &CurrentTarget,
) -> Result<Option<u64>, String> {
    let Some(selector) = selector else {
        return current_target
            .0
            .map(|e| Some(e.to_bits()))
            .ok_or_else(|| "no current target selected".into());
    };
    if selector.eq_ignore_ascii_case("current") {
        return current_target
            .0
            .map(|e| Some(e.to_bits()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::{
        CollectionStatusSnapshot, CombatLogEntry, CombatLogEventKind, CombatLogStatusSnapshot,
        CurrenciesStatusSnapshot, EquippedGearStatusSnapshot, GroupRole, GroupStatusSnapshot,
        InventoryItemEntry, InventorySearchSnapshot, NetworkStatusSnapshot,
        ProfessionStatusSnapshot, QuestLogStatusSnapshot, QuestRepeatability,
        ReputationsStatusSnapshot, SoundStatusSnapshot, TerrainStatusSnapshot,
    };
    use crate::targeting::CurrentTarget;
    use shared::protocol::{AuctionInventoryItem, AuctionInventorySnapshot};

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
        let text = format_character_stats_status(&crate::status::CharacterStatsSnapshot {
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

    fn make_combat_entry(kind: CombatLogEventKind, text: &str) -> CombatLogEntry {
        CombatLogEntry {
            kind,
            source: "A".into(),
            target: "B".into(),
            spell: None,
            amount: None,
            aura: None,
            text: text.into(),
        }
    }

    fn all_kinds_combat_snapshot() -> CombatLogStatusSnapshot {
        CombatLogStatusSnapshot {
            entries: vec![
                make_combat_entry(CombatLogEventKind::Damage, "hit"),
                make_combat_entry(CombatLogEventKind::Heal, "heal"),
                make_combat_entry(CombatLogEventKind::Interrupt, "interrupt"),
                make_combat_entry(CombatLogEventKind::AuraApplied, "aura"),
                make_combat_entry(CombatLogEventKind::Death, "death"),
            ],
        }
    }

    #[test]
    fn combat_log_formats_damage_heal_interrupt_aura_and_death() {
        let text = format_combat_log(&all_kinds_combat_snapshot(), 10);
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
