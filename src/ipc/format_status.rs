use crate::ipc::Request;
use crate::ipc::plugin::DispatchContext;
use crate::status::{
    AchievementsStatusSnapshot, BarberShopStatusSnapshot, CalendarSignupStateEntry,
    CalendarStatusSnapshot, CharacterStatsSnapshot, CurrenciesStatusSnapshot, DeathStateEntry,
    DeathStatusSnapshot, EncounterJournalStatusSnapshot, FriendsStatusSnapshot, GroupRole,
    GuildStatusSnapshot, IgnoreListStatusSnapshot, LfgStatusSnapshot, NetworkStatusSnapshot,
    PvpStatusSnapshot, ReputationsStatusSnapshot, SoundStatusSnapshot, WhoStatusSnapshot,
};

pub fn status_request_text(request: &Request, ctx: &DispatchContext) -> Option<String> {
    format_core_status_request(request, ctx)
        .or_else(|| format_social_status_request(request, ctx))
        .or_else(|| format_progression_status_request(request, ctx))
}

fn format_core_status_request(request: &Request, ctx: &DispatchContext) -> Option<String> {
    match request {
        Request::AchievementsStatus => Some(format_achievement_status(ctx.achievements_status)),
        Request::DeathStatus => Some(format_death_status(ctx.death_status)),
        Request::EncounterJournalStatus => Some(format_encounter_journal_status(
            ctx.encounter_journal_status,
        )),
        Request::CalendarStatus => Some(format_calendar_status(ctx.calendar_status)),
        Request::NetworkStatus => Some(format_network_status(ctx.network_status, ctx.connected)),
        Request::SoundStatus => Some(format_sound_status(ctx.sound_status)),
        Request::CurrenciesStatus => Some(format_currencies_status(ctx.currencies_status)),
        Request::ReputationsStatus | Request::ReputationList => {
            Some(format_reputations_status(ctx.reputations_status))
        }
        Request::CharacterStatsStatus => Some(format_character_stats_status(ctx.character_stats)),
        _ => None,
    }
}

fn format_social_status_request(request: &Request, ctx: &DispatchContext) -> Option<String> {
    match request {
        Request::GuildStatus => Some(format_guild_status(ctx.guild_status)),
        Request::FriendsStatus => Some(format_friends_status(ctx.friends_status)),
        Request::WhoStatus => Some(format_who_status(ctx.who_status)),
        Request::IgnoreStatus => Some(format_ignore_list_status(ctx.ignore_list_status)),
        _ => None,
    }
}

fn format_progression_status_request(request: &Request, ctx: &DispatchContext) -> Option<String> {
    match request {
        Request::LfgStatus => Some(format_lfg_status(ctx.lfg_status)),
        Request::PvpStatus => Some(format_pvp_status(ctx.pvp_status)),
        _ => None,
    }
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
    let state = format_death_state(snapshot.state.as_ref());
    let corpse = format_death_marker(snapshot.corpse.as_ref());
    let graveyard = format_death_marker(snapshot.graveyard.as_ref());
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

fn format_death_state(state: Option<&DeathStateEntry>) -> &'static str {
    match state {
        Some(DeathStateEntry::Alive) => "alive",
        Some(DeathStateEntry::Dead) => "dead",
        Some(DeathStateEntry::Ghost) => "ghost",
        Some(DeathStateEntry::Resurrecting) => "resurrecting",
        None => "unknown",
    }
}

fn format_death_marker(position: Option<&crate::status::DeathPositionEntry>) -> String {
    position
        .map(|position| format!("{:.2},{:.2},{:.2}", position.x, position.y, position.z))
        .unwrap_or_else(|| "-".into())
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

pub fn format_guild_status(snapshot: &GuildStatusSnapshot) -> String {
    let mut lines = vec![
        format!("guild_id: {}", snapshot.guild_id.unwrap_or_default()),
        format!("guild_name: {}", snapshot.guild_name),
        format!("guild_members: {}", snapshot.entries.len()),
        format!("guild_motd: {}", snapshot.motd),
        format!("guild_info: {}", snapshot.info_text),
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
            "{} level={} class={} rank={} online={} officer_note={} last_online={}",
            entry.character_name,
            entry.level,
            entry.class_name,
            entry.rank_name,
            entry.online,
            entry.officer_note,
            entry.last_online
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

pub fn format_calendar_status(snapshot: &CalendarStatusSnapshot) -> String {
    let mut lines = vec![format!("calendar_events: {}", snapshot.events.len())];
    if let Some(message) = &snapshot.last_server_message {
        lines.push(format!("message: {message}"));
    }
    if let Some(error) = &snapshot.last_error {
        lines.push(format!("error: {error}"));
    }
    if snapshot.events.is_empty() {
        lines.push("-".into());
        return lines.join("\n");
    }
    lines.extend(snapshot.events.iter().map(format_calendar_event));
    lines.join("\n")
}

fn format_calendar_event(event: &crate::status::CalendarEventEntry) -> String {
    let (confirmed, tentative, declined) = calendar_signup_counts(event);
    format!(
        "{} title={} organizer={} raid={} starts_at={} confirmed={}/{} tentative={} declined={}",
        event.event_id,
        event.title,
        event.organizer_name,
        event.is_raid,
        event.starts_at_unix_secs,
        confirmed,
        event.max_signups,
        tentative,
        declined
    )
}

fn calendar_signup_counts(event: &crate::status::CalendarEventEntry) -> (usize, usize, usize) {
    let confirmed = count_calendar_signups(event, CalendarSignupStateEntry::Confirmed);
    let tentative = count_calendar_signups(event, CalendarSignupStateEntry::Tentative);
    let declined = count_calendar_signups(event, CalendarSignupStateEntry::Declined);
    (confirmed, tentative, declined)
}

fn count_calendar_signups(
    event: &crate::status::CalendarEventEntry,
    status: CalendarSignupStateEntry,
) -> usize {
    event
        .signups
        .iter()
        .filter(|signup| signup.status == status)
        .count()
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
    let mut lines = format_lfg_header_lines(snapshot);
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
        lines.extend(format_lfg_match_found_lines(match_found));
    }
    if let Some(message) = &snapshot.last_server_message {
        lines.push(format!("message: {message}"));
    }
    if let Some(error) = &snapshot.last_error {
        lines.push(format!("error: {error}"));
    }
    lines.join("\n")
}

fn format_lfg_header_lines(snapshot: &LfgStatusSnapshot) -> Vec<String> {
    vec![
        format!(
            "lfg: queued={} role={}",
            snapshot.queued,
            snapshot
                .selected_role
                .as_ref()
                .map(format_group_role)
                .unwrap_or("-")
        ),
        format!("dungeons: {}", format_lfg_dungeons(snapshot)),
        format!(
            "queue: size={} avg_wait_secs={} in_demand={}",
            snapshot.queue_size,
            snapshot.average_wait_secs,
            format_lfg_roles(&snapshot.in_demand_roles)
        ),
    ]
}

fn format_lfg_match_found_lines(match_found: &crate::status::LfgMatchFoundEntry) -> Vec<String> {
    let mut lines = vec![format!(
        "match_found: {} role={} members={}",
        match_found.dungeon_name,
        format_group_role(&match_found.assigned_role),
        match_found.members.len()
    )];
    lines.extend(match_found.members.iter().map(|member| {
        format!(
            "member: {} role={}",
            member.name,
            format_group_role(&member.role)
        )
    }));
    lines
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
            .map(format_rest_area_kind)
            .unwrap_or("-"),
        snapshot.rested_xp,
        snapshot.rested_xp_max,
        snapshot.zone_id,
    )
}

fn format_group_role(role: &GroupRole) -> &'static str {
    match role {
        GroupRole::Tank => "tank",
        GroupRole::Healer => "healer",
        GroupRole::Damage => "damage",
        GroupRole::None => "none",
    }
}

fn format_lfg_dungeons(snapshot: &LfgStatusSnapshot) -> String {
    if snapshot.dungeon_ids.is_empty() {
        return "-".into();
    }
    snapshot
        .dungeon_ids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn format_lfg_roles(roles: &[GroupRole]) -> String {
    if roles.is_empty() {
        return "-".into();
    }
    roles
        .iter()
        .map(format_group_role)
        .collect::<Vec<_>>()
        .join(",")
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

fn format_rest_area_kind(kind: &crate::status::RestAreaKindEntry) -> &'static str {
    match kind {
        crate::status::RestAreaKindEntry::City => "city",
        crate::status::RestAreaKindEntry::Inn => "inn",
    }
}
