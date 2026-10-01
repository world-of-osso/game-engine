//! World and combat requests in the original response text: `map target` and `map
//! waypoint add|clear` (src/ipc/plugin.rs, src/ipc/format.rs `format_map_target`),
//! `group roster|status|invite|uninvite`, `emote`, `spell cast|stop`
//! (src/ipc/plugin/combat.rs, src/ipc/format.rs). The native client has no client-side
//! entity IDs: `map target` reports the target's server entity.
use game_engine_network::ipc_wire::{Request, Response};
use game_engine_ui_model::group_state::{GroupCommand, GroupState};
use shared::protocol::{
    EmoteIntent, GroupMemberState, GroupRoleSnapshot, ReadyCheckAnswer, SpellCastIntent,
};

impl crate::GameClient {
    /// A world request's response, or the request when the client does not serve it.
    pub(crate) fn world_request(&mut self, request: Request) -> Result<Response, Request> {
        let answer = match request {
            Request::MapTarget => self.map_target(),
            Request::MapWaypointAdd { x, y } => {
                self.map_waypoint = Some((x, y));
                self.map_position()
            }
            Request::MapWaypointClear => {
                self.map_waypoint = None;
                self.map_position()
            }
            Request::GroupRoster => Ok(format_group_roster(&self.account.group)),
            Request::GroupStatus => Ok(format_group_status(&self.account.group)),
            Request::GroupInvite { name } => self.group_ipc(
                "group invite",
                GroupCommand::Invite(name.clone()),
                format!("group invite submitted for {name}"),
            ),
            Request::GroupUninvite { name } => self.group_ipc(
                "group uninvite",
                GroupCommand::Uninvite(name.clone()),
                format!("group uninvite submitted for {name}"),
            ),
            Request::Emote { emote } => self.emote_ipc(emote),
            Request::SpellCast { spell, target } => self.spell_cast_ipc(&spell, target.as_deref()),
            Request::SpellStop => self.spell_stop_ipc(),
            request => return Err(request),
        };
        Ok(match answer {
            Ok(text) => Response::Text(text),
            Err(error) => Response::Error(error),
        })
    }

    fn connected(&self) -> bool {
        self.account.link.as_ref().is_some_and(|link| link.connected)
    }

    /// `format_map_target`: the target's name, server entity, position and ground
    /// distance from the local player.
    fn map_target(&self) -> Result<String, String> {
        let Some(target) = self.targeting_target() else {
            return Ok("map_target: none\ndistance: -".into());
        };
        let player = self
            .world
            .local_player_transform()
            .ok_or("map target requires a local player")?
            .origin;
        let (Some(node), Some(name)) = (self.world.unit_node(target), self.world.unit_name(target))
        else {
            return Ok("map_target: missing\ndistance: -".into());
        };
        let position = node.get_global_position();
        let distance = ((position.x - player.x).powi(2) + (position.z - player.z).powi(2)).sqrt();
        Ok(format!(
            "map_target: {name}\nentity: {target}\nposition: {:.2},{:.2}\ndistance: {distance:.2}",
            position.x, position.z
        ))
    }

    fn group_ipc(&self, what: &str, command: GroupCommand, sent: String) -> Result<String, String> {
        if !self.connected() {
            return Err(format!("{what} is unavailable: not connected"));
        }
        self.account
            .send_group(command)
            .map(|()| sent)
            .map_err(|_| format!("{what} sender unavailable"))
    }

    fn emote_ipc(&self, emote: shared::protocol::EmoteKind) -> Result<String, String> {
        if !self.connected() {
            return Err("emote is unavailable: not connected".into());
        }
        self.account
            .send_emote(EmoteIntent { emote })
            .map(|()| format!("emote submitted {emote:?}"))
            .map_err(|_| "emote sender unavailable".into())
    }

    fn spell_cast_ipc(&self, spell: &str, target: Option<&str>) -> Result<String, String> {
        if !self.connected() {
            return Err("spell cast is unavailable: not connected".into());
        }
        let target_entity = resolve_spell_target(target, self.targeting_target())?;
        let (spell_id, spell) = resolve_spell_identifier(spell)?;
        let target_text = target_entity.map_or_else(|| "-".into(), |id| id.to_string());
        self.account
            .send_spell_intent(SpellCastIntent {
                spell_id,
                spell: spell.clone(),
                target_entity,
            })
            .map(|()| format!("spell cast submitted spell={spell} target={target_text}"))
            .map_err(|_| "spell cast is unavailable: not connected".into())
    }

    fn spell_stop_ipc(&self) -> Result<String, String> {
        if !self.connected() {
            return Err("spell stop is unavailable: not connected".into());
        }
        self.account
            .send_stop_cast()
            .map(|()| "spell stop submitted".into())
            .map_err(|_| "spell stop is unavailable: not connected".into())
    }
}

/// `resolve_spell_identifier`: a spell ID or a name token.
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

/// `resolve_spell_target`: the current target (default or `current`), `none`, or a
/// server entity.
fn resolve_spell_target(selector: Option<&str>, current: Option<u64>) -> Result<Option<u64>, String> {
    match selector {
        None => current.map(Some).ok_or_else(|| "no current target selected".into()),
        Some(selector) if selector.eq_ignore_ascii_case("current") => {
            current.map(Some).ok_or_else(|| "no current target selected".into())
        }
        Some(selector) if selector.eq_ignore_ascii_case("none") => Ok(None),
        Some(selector) => selector
            .parse::<u64>()
            .map(Some)
            .map_err(|_| format!("invalid target selector '{selector}'")),
    }
}

/// `format_group_roster`.
fn format_group_roster(group: &GroupState) -> String {
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
                role_label(&m.role),
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

fn format_member_live(live: &GroupMemberState) -> String {
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

/// `format_group_status`.
fn format_group_status(group: &GroupState) -> String {
    let ready = group
        .ready_check
        .as_ref()
        .map(|view| {
            let answered = view
                .update
                .members
                .iter()
                .filter(|m| m.answer == ReadyCheckAnswer::Ready)
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

fn role_label(role: &GroupRoleSnapshot) -> &'static str {
    match role {
        GroupRoleSnapshot::Tank => "tank",
        GroupRoleSnapshot::Healer => "healer",
        GroupRoleSnapshot::Damage => "damage",
        GroupRoleSnapshot::None => "none",
    }
}
