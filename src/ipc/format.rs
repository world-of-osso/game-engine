//! IPC response formatting helpers.

#[path = "format_terrain.rs"]
mod format_terrain;

use crate::status::{
    AchievementsStatusSnapshot, BarberShopStatusSnapshot, CharacterStatsSnapshot,
    CollectionStatusSnapshot, CombatLogEntry, CombatLogEventKind, CombatLogStatusSnapshot,
    CurrenciesStatusSnapshot, DeathStateEntry, DeathStatusSnapshot, EncounterJournalStatusSnapshot,
    EquippedGearStatusSnapshot, FriendsStatusSnapshot, GroupRole, GroupStatusSnapshot,
    IgnoreListStatusSnapshot, InventoryItemEntry, InventorySearchSnapshot, LfgStatusSnapshot,
    MapStatusSnapshot, NetworkStatusSnapshot, ProfessionStatusSnapshot, PvpStatusSnapshot,
    QuestLogStatusSnapshot, QuestRepeatability, ReputationsStatusSnapshot, SoundStatusSnapshot,
    WhoStatusSnapshot,
};
use crate::targeting::CurrentTarget;
use shared::protocol::AuctionInventorySnapshot;

use super::plugin::DispatchContext;
use super::{Command, Request, Response};
use format_terrain::format_terrain_status;

/// Returns true if the request was a status query and was handled.
pub fn dispatch_status_request(cmd: &Command, ctx: &DispatchContext) -> bool {
    let text = match &cmd.request {
        Request::AchievementsStatus => format_achievement_status(ctx.achievements_status),
        Request::BarberStatus => format_barber_shop_status(ctx.barber_shop_status),
        Request::DeathStatus => format_death_status(ctx.death_status),
        Request::EncounterJournalStatus => {
            format_encounter_journal_status(ctx.encounter_journal_status)
        }
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
        Request::FriendsStatus => format_friends_status(ctx.friends_status),
        Request::WhoStatus => format_who_status(ctx.who_status),
        Request::IgnoreStatus => format_ignore_list_status(ctx.ignore_list_status),
        Request::LfgStatus => format_lfg_status(ctx.lfg_status),
        Request::PvpStatus => format_pvp_status(ctx.pvp_status),
        Request::ProfessionRecipes { text } => {
            format_profession_recipes(ctx.profession_status, text)
        }
        _ => return false,
    };
    let _ = cmd.respond.send(Response::Text(text));
    true
}

pub fn format_achievement_status(snapshot: &AchievementsStatusSnapshot) -> String {
    let mut lines = vec![format!("achievements: {}", snapshot.earned_ids.len())];
    if let Some(message) = &snapshot.last_server_message {
        lines.push(format!("message: {message}"));
    }
    if let Some(error) = &snapshot.last_error {
        lines.push(format!("error: {error}"));
    }
    if let Some(completed) = &snapshot.last_completed {
        lines.push(format!(
            "completed: {} {} points={}",
            completed.achievement_id, completed.name, completed.points
        ));
    }
    if snapshot.progress.is_empty() {
        lines.push("-".into());
        return lines.join("\n");
    }
    lines.extend(snapshot.progress.iter().map(|entry| {
        format!(
            "{} current={} required={} completed={}",
            entry.achievement_id, entry.current, entry.required, entry.completed
        )
    }));
    lines.join("\n")
}

pub fn format_barber_shop_status(snapshot: &BarberShopStatusSnapshot) -> String {
    let mut lines = vec![
        format!(
            "barber_gold: {}",
            crate::auction_house_data::Money(snapshot.gold as u64).display()
        ),
        format!(
            "pending_cost: {}",
            crate::barber_shop::format_cost(snapshot.pending_cost)
        ),
    ];
    for (index, def) in crate::barber_shop_data::CUSTOMIZATIONS.iter().enumerate() {
        lines.push(format!(
            "{}: current={} pending={}",
            def.label,
            crate::barber_shop::option_value(snapshot.current_appearance, index),
            crate::barber_shop::option_value(snapshot.pending_appearance, index)
        ));
    }
    if let Some(message) = &snapshot.last_server_message {
        lines.push(format!("message: {message}"));
    }
    if let Some(error) = &snapshot.last_error {
        lines.push(format!("error: {error}"));
    }
    lines.join("\n")
}

pub fn format_death_status(snapshot: &DeathStatusSnapshot) -> String {
    let state = snapshot
        .state
        .as_ref()
        .map(|state| match state {
            DeathStateEntry::Alive => "alive",
            DeathStateEntry::Dead => "dead",
            DeathStateEntry::Ghost => "ghost",
            DeathStateEntry::Resurrecting => "resurrecting",
        })
        .unwrap_or("unknown");
    let corpse = snapshot
        .corpse
        .as_ref()
        .map(|corpse| format!("{:.2},{:.2},{:.2}", corpse.x, corpse.y, corpse.z))
        .unwrap_or_else(|| "-".into());
    let graveyard = snapshot
        .graveyard
        .as_ref()
        .map(|graveyard| format!("{:.2},{:.2},{:.2}", graveyard.x, graveyard.y, graveyard.z))
        .unwrap_or_else(|| "-".into());
    let mut lines = vec![
        format!("state: {state}"),
        format!("corpse: {corpse}"),
        format!("graveyard: {graveyard}"),
        format!(
            "can_resurrect_at_corpse: {}",
            snapshot.can_resurrect_at_corpse
        ),
        format!(
            "spirit_healer_available: {}",
            snapshot.spirit_healer_available
        ),
    ];
    if let Some(message) = &snapshot.last_server_message {
        lines.push(format!("message: {message}"));
    }
    if let Some(error) = &snapshot.last_error {
        lines.push(format!("error: {error}"));
    }
    lines.join("\n")
}

pub fn format_encounter_journal_status(snapshot: &EncounterJournalStatusSnapshot) -> String {
    let mut lines = vec![format!("instances: {}", snapshot.instances.len())];
    if let Some(error) = &snapshot.last_error {
        lines.push(format!("error: {error}"));
    }
    if snapshot.instances.is_empty() {
        lines.push("-".into());
        return lines.join("\n");
    }
    for instance in &snapshot.instances {
        lines.push(format!(
            "{} [{}] tier={} source={} bosses={}",
            instance.name,
            instance.instance_type,
            instance.tier,
            instance.source,
            instance.bosses.len()
        ));
        for boss in &instance.bosses {
            lines.push(format!(
                "  {} entry={} level={} - {} rank={} abilities={} loot={}",
                boss.name,
                boss.entry,
                boss.min_level,
                boss.max_level,
                boss.rank,
                boss.ability_count,
                boss.loot_count
            ));
        }
    }
    lines.join("\n")
}

pub fn format_pvp_status(snapshot: &PvpStatusSnapshot) -> String {
    let mut lines = vec![
        format!("honor: {}/{}", snapshot.honor, snapshot.honor_max),
        format!("conquest: {}/{}", snapshot.conquest, snapshot.conquest_max),
        format!("queue: {}", snapshot.queue.as_deref().unwrap_or("-")),
    ];
    if let Some(message) = &snapshot.last_server_message {
        lines.push(format!("message: {message}"));
    }
    if let Some(error) = &snapshot.last_error {
        lines.push(format!("error: {error}"));
    }
    if snapshot.brackets.is_empty() {
        lines.push("brackets: -".into());
        return lines.join("\n");
    }
    lines.push(format!("brackets: {}", snapshot.brackets.len()));
    lines.extend(snapshot.brackets.iter().map(|entry| {
        format!(
            "{} rating={} season={} - {} weekly={} - {}",
            entry.bracket,
            entry.rating,
            entry.season_wins,
            entry.season_losses,
            entry.weekly_wins,
            entry.weekly_losses
        )
    }));
    lines.join("\n")
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

pub fn format_sound_status(snapshot: &SoundStatusSnapshot) -> String {
    format!(
        "enabled: {}\nmuted: {}\nmaster_volume: {:.2}\nambient_volume: {:.2}\nambient_entities: {}\nactive_sinks: {}",
        snapshot.enabled,
        snapshot.muted,
        snapshot.master_volume,
        snapshot.ambient_volume,
        snapshot.ambient_entities,
        snapshot.active_sinks,
    )
}

pub fn format_currencies_status(snapshot: &CurrenciesStatusSnapshot) -> String {
    let mut lines = vec![format!("currencies: {}", snapshot.entries.len())];
    if let Some(message) = &snapshot.last_server_message {
        lines.push(format!("message: {message}"));
    }
    if let Some(error) = &snapshot.last_error {
        lines.push(format!("error: {error}"));
    }
    if snapshot.entries.is_empty() {
        lines.push("-".into());
        return lines.join("\n");
    }
    lines.extend(
        snapshot
            .entries
            .iter()
            .map(|e| format!("{} {} amount={}", e.id, e.name, e.amount)),
    );
    lines.join("\n")
}

pub fn format_reputations_status(snapshot: &ReputationsStatusSnapshot) -> String {
    let mut lines = vec![format!("reputations: {}", snapshot.entries.len())];
    if let Some(message) = &snapshot.last_server_message {
        lines.push(format!("message: {message}"));
    }
    if let Some(error) = &snapshot.last_error {
        lines.push(format!("error: {error}"));
    }
    if snapshot.entries.is_empty() {
        lines.push("-".into());
        return lines.join("\n");
    }
    lines.extend(snapshot.entries.iter().map(|e| {
        format!(
            "{} {} standing={} value={}",
            e.faction_id, e.faction_name, e.standing, e.value
        )
    }));
    lines.join("\n")
}

pub fn format_friends_status(snapshot: &FriendsStatusSnapshot) -> String {
    let mut lines = vec![format!("friends: {}", snapshot.entries.len())];
    if let Some(message) = &snapshot.last_server_message {
        lines.push(format!("message: {message}"));
    }
    if let Some(error) = &snapshot.last_error {
        lines.push(format!("error: {error}"));
    }
    if snapshot.entries.is_empty() {
        lines.push("-".into());
        return lines.join("\n");
    }
    lines.extend(snapshot.entries.iter().map(|entry| {
        format!(
            "{} level={} class={} area={} online={} presence={}",
            entry.name,
            entry.level,
            entry.class_name,
            entry.area,
            entry.online,
            format_presence_state(&entry.presence)
        )
    }));
    lines.join("\n")
}

pub fn format_who_status(snapshot: &WhoStatusSnapshot) -> String {
    let query = if snapshot.query.is_empty() {
        "*"
    } else {
        snapshot.query.as_str()
    };
    let mut lines = vec![
        format!("who_query: {query}"),
        format!("who_results: {}", snapshot.entries.len()),
    ];
    if let Some(message) = &snapshot.last_server_message {
        lines.push(format!("message: {message}"));
    }
    if let Some(error) = &snapshot.last_error {
        lines.push(format!("error: {error}"));
    }
    if snapshot.entries.is_empty() {
        lines.push("-".into());
        return lines.join("\n");
    }
    lines.extend(snapshot.entries.iter().map(|entry| {
        format!(
            "{} level={} class={} area={}",
            entry.name, entry.level, entry.class_name, entry.area
        )
    }));
    lines.join("\n")
}

pub fn format_ignore_list_status(snapshot: &IgnoreListStatusSnapshot) -> String {
    let mut lines = vec![format!("ignored: {}", snapshot.names.len())];
    if let Some(message) = &snapshot.last_server_message {
        lines.push(format!("message: {message}"));
    }
    if let Some(error) = &snapshot.last_error {
        lines.push(format!("error: {error}"));
    }
    if snapshot.names.is_empty() {
        lines.push("-".into());
        return lines.join("\n");
    }
    lines.extend(snapshot.names.iter().cloned());
    lines.join("\n")
}

pub fn format_lfg_status(snapshot: &LfgStatusSnapshot) -> String {
    let mut lines = vec![format!(
        "lfg: queued={} role={}",
        snapshot.queued,
        snapshot
            .selected_role
            .as_ref()
            .map(format_group_role)
            .unwrap_or("-")
    )];
    lines.push(format!(
        "dungeons: {}",
        if snapshot.dungeon_ids.is_empty() {
            "-".into()
        } else {
            snapshot
                .dungeon_ids
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(",")
        }
    ));
    lines.push(format!(
        "queue: size={} avg_wait_secs={} in_demand={}",
        snapshot.queue_size,
        snapshot.average_wait_secs,
        if snapshot.in_demand_roles.is_empty() {
            "-".into()
        } else {
            snapshot
                .in_demand_roles
                .iter()
                .map(format_group_role)
                .collect::<Vec<_>>()
                .join(",")
        }
    ));
    if let Some(role_check) = &snapshot.role_check {
        lines.push(format!(
            "role_check: {} role={} accepted={}/{}",
            role_check.dungeon_name,
            format_group_role(&role_check.assigned_role),
            role_check.accepted_count,
            role_check.total_count
        ));
    }
    if let Some(match_found) = &snapshot.match_found {
        lines.push(format!(
            "match_found: {} role={} members={}",
            match_found.dungeon_name,
            format_group_role(&match_found.assigned_role),
            match_found.members.len()
        ));
        lines.extend(match_found.members.iter().map(|member| {
            format!(
                "member: {} role={}",
                member.name,
                format_group_role(&member.role)
            )
        }));
    }
    if let Some(message) = &snapshot.last_server_message {
        lines.push(format!("message: {message}"));
    }
    if let Some(error) = &snapshot.last_error {
        lines.push(format!("error: {error}"));
    }
    lines.join("\n")
}

fn format_group_role(role: &GroupRole) -> &'static str {
    match role {
        GroupRole::Tank => "tank",
        GroupRole::Healer => "healer",
        GroupRole::Damage => "damage",
        GroupRole::None => "none",
    }
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
        "name: {}\nlevel: {}\nrace: {}\nclass: {}\nhealth: {}/{}\nmana: {}/{}\nsecondary_resource: {}\nmovement_speed: {}\ngold: {}\npresence: {}\nin_combat: {}\nin_rest_area: {}\nrest_area_kind: {}\nrested_xp: {}\nrested_xp_max: {}\nzone_id: {}",
        snapshot.name.as_deref().unwrap_or("-"),
        opt_int(snapshot.level),
        opt_int(snapshot.race),
        opt_int(snapshot.class),
        opt_float0(snapshot.health_current),
        opt_float0(snapshot.health_max),
        opt_float0(snapshot.mana_current),
        opt_float0(snapshot.mana_max),
        format_secondary_resource(snapshot.secondary_resource.as_ref()),
        opt_float2(snapshot.movement_speed),
        crate::auction_house_data::Money(snapshot.gold as u64).display(),
        snapshot
            .presence
            .as_ref()
            .map(format_presence_state)
            .unwrap_or("-"),
        snapshot.in_combat,
        snapshot.in_rest_area,
        snapshot
            .rest_area_kind
            .as_ref()
            .map(|kind| match kind {
                crate::status::RestAreaKindEntry::City => "city",
                crate::status::RestAreaKindEntry::Inn => "inn",
            })
            .unwrap_or("-"),
        snapshot.rested_xp,
        snapshot.rested_xp_max,
        snapshot.zone_id,
    )
}

fn format_secondary_resource(value: Option<&crate::status::SecondaryResourceEntry>) -> String {
    let Some(value) = value else {
        return "-".into();
    };
    format!(
        "{} {}/{}",
        match value.kind {
            crate::status::SecondaryResourceKindEntry::ComboPoints => "combo_points",
            crate::status::SecondaryResourceKindEntry::HolyPower => "holy_power",
            crate::status::SecondaryResourceKindEntry::Chi => "chi",
            crate::status::SecondaryResourceKindEntry::Essence => "essence",
        },
        value.current,
        value.max
    )
}

fn format_presence_state(value: &crate::status::PresenceStateEntry) -> &'static str {
    match value {
        crate::status::PresenceStateEntry::Online => "online",
        crate::status::PresenceStateEntry::Afk => "afk",
        crate::status::PresenceStateEntry::Dnd => "dnd",
        crate::status::PresenceStateEntry::Offline => "offline",
    }
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
    let mut lines = vec![
        format!("equipped_gear: {}", snapshot.entries.len()),
        format!(
            "repair_cost: {}",
            crate::auction_house_data::Money(snapshot.total_repair_cost as u64).display()
        ),
    ];
    if let Some(message) = &snapshot.last_server_message {
        lines.push(format!("message: {message}"));
    }
    if let Some(error) = &snapshot.last_error {
        lines.push(format!("error: {error}"));
    }
    lines.extend(snapshot.entries.iter().map(|entry| {
        let durability = match (entry.durability_current, entry.durability_max) {
            (Some(current), Some(max)) => format!(" durability={current}/{max}"),
            _ => String::new(),
        };
        let broken = if entry.broken { " broken=true" } else { "" };
        let repair_cost = if entry.repair_cost > 0 {
            format!(
                " repair={}",
                crate::auction_house_data::Money(entry.repair_cost as u64).display()
            )
        } else {
            String::new()
        };
        format!(
            "{} {}{}{}{}",
            entry.slot, entry.path, durability, repair_cost, broken
        )
    }));
    lines.join("\n")
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
    format_collection_entries(
        "mounts",
        snapshot.last_server_message.as_deref(),
        snapshot.last_error.as_deref(),
        snapshot.mounts.iter(),
        missing,
        |entry| entry.known,
        |entry| {
            format!(
                "{} {} known={} active={}",
                entry.mount_id, entry.name, entry.known, entry.active
            )
        },
    )
}

pub fn format_collection_pets(snapshot: &CollectionStatusSnapshot, missing: bool) -> String {
    format_collection_entries(
        "pets",
        snapshot.last_server_message.as_deref(),
        snapshot.last_error.as_deref(),
        snapshot.pets.iter(),
        missing,
        |entry| entry.known,
        |entry| {
            format!(
                "{} {} known={} active={}",
                entry.pet_id, entry.name, entry.known, entry.active
            )
        },
    )
}

fn format_collection_entries<'a, T>(
    label: &str,
    message: Option<&str>,
    error: Option<&str>,
    entries: impl Iterator<Item = &'a T>,
    missing: bool,
    is_known: impl Fn(&T) -> bool,
    format_entry: impl Fn(&T) -> String,
) -> String
where
    T: 'a,
{
    let filtered = entries
        .filter(|entry| !missing || !is_known(entry))
        .collect::<Vec<_>>();
    let mut lines = vec![format!("{label}: {}", filtered.len())];
    if let Some(message) = message {
        lines.push(format!("message: {message}"));
    }
    if let Some(error) = error {
        lines.push(format!("error: {error}"));
    }
    if filtered.is_empty() {
        lines.push("-".into());
        return lines.join("\n");
    }
    lines.extend(filtered.into_iter().map(format_entry));
    lines.join("\n")
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
    let graveyard_marker = snapshot
        .graveyard_marker
        .map(|w| format!("{:.2},{:.2}", w.x, w.y))
        .unwrap_or_else(|| "-".into());
    format!(
        "zone_id: {}\nposition: {:.2},{:.2}\nwaypoint: {}\ngraveyard_marker: {}",
        snapshot.zone_id, snapshot.player_x, snapshot.player_z, waypoint, graveyard_marker
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
    let Ok(target_info) = tree_query.get(target) else {
        return "map_target: missing\ndistance: -".into();
    };
    let dx = target_info.transform.translation.x - map_status.player_x;
    let dz = target_info.transform.translation.z - map_status.player_z;
    let distance = (dx * dx + dz * dz).sqrt();
    format!(
        "map_target: {}\nentity: {}\nposition: {:.2},{:.2}\ndistance: {:.2}",
        target_info
            .name
            .map(|value| value.as_str())
            .unwrap_or("unnamed"),
        target_info.entity.to_bits(),
        target_info.transform.translation.x,
        target_info.transform.translation.z,
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
#[path = "../../tests/unit/ipc_format_tests.rs"]
mod tests;
