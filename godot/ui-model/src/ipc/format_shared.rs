//! Engine-free IPC response text shared by the original engine (`src/ipc/format.rs`)
//! and the native client: group roster and status, and the spell identifier of
//! `spell cast`.

use crate::group_state::GroupState;

pub fn format_group_roster(group: &GroupState) -> String {
    if group.members.is_empty() {
        return "group_roster: 0\n-".into();
    }
    let lines = group
        .members
        .iter()
        .map(|m| {
            format!(
                "{} leader={} role={} online={} subgroup={} level={}{}{}",
                m.name,
                m.is_leader,
                group_snapshot_role_label(&m.role),
                m.online,
                m.subgroup,
                m.level,
                group
                    .live
                    .get(&m.name)
                    .map(format_member_live)
                    .unwrap_or_default(),
                group
                    .ready_mark(&m.name)
                    .map(|mark| format!(" ready={mark:?}"))
                    .unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("group_roster: {}\n{lines}", group.members.len())
}

fn format_member_live(live: &shared::protocol::GroupMemberState) -> String {
    let power = live
        .power
        .as_ref()
        .map(|p| format!(" power={:?}:{}/{}", p.power, p.current, p.max))
        .unwrap_or_default();
    let debuffs = live
        .debuffs
        .iter()
        .map(|d| format!("{}:{}", d.spell_id, d.dispel_type))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        " hp={}/{}{power} death={:?} pos={:.1},{:.1},{:.1} debuffs=[{debuffs}]",
        live.health, live.max_health, live.death, live.position.x, live.position.y, live.position.z
    )
}

pub fn format_group_status(group: &GroupState) -> String {
    let ready = group
        .ready_check
        .as_ref()
        .map(|view| {
            let answered = view
                .update
                .members
                .iter()
                .filter(|m| m.answer == shared::protocol::ReadyCheckAnswer::Ready)
                .count();
            format!(
                "{answered}/{} finished={}",
                view.update.members.len(),
                view.update.finished
            )
        })
        .unwrap_or_else(|| "-".into());
    format!(
        "in_group: {}\nis_raid: {}\nmembers: {}\nready_check: {ready}\npending_invite: {}\nlast_message: {}",
        group.in_group(),
        group.is_raid,
        group.members.len(),
        group.pending_invite.as_deref().unwrap_or("-"),
        group.last_server_message.as_deref().unwrap_or("-")
    )
}

fn group_snapshot_role_label(role: &shared::protocol::GroupRoleSnapshot) -> &'static str {
    use shared::protocol::GroupRoleSnapshot;
    match role {
        GroupRoleSnapshot::Tank => "tank",
        GroupRoleSnapshot::Healer => "healer",
        GroupRoleSnapshot::Damage => "damage",
        GroupRoleSnapshot::None => "none",
    }
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
