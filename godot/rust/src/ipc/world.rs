//! World and combat requests in the original response text: `map target` and `map
//! waypoint add|clear` (src/ipc/plugin.rs, src/ipc/format.rs `format_map_target`),
//! `quest interact` and `loot take-all` (src/ipc/plugin.rs `handle_quest_interact`,
//! `handle_loot_take_all`),
//! `group roster|status|invite|uninvite`, `emote`, `spell cast|stop`
//! (src/ipc/plugin/combat.rs, src/ipc/format.rs). The native client has no client-side
//! entity IDs: `map target` reports the target's server entity.
use game_engine_network::ipc_wire::{Request, Response};
use game_engine_ui_model::{
    group_state::{GroupCommand, GroupState},
    loot_data::{NpcRightClick, loot_chat_text, npc_right_click},
};
use shared::{
    components::{Health, Npc, Player},
    protocol::{
        EmoteIntent, GAMEOBJECT_TYPE_MAILBOX, GameObjectInfo, GroupMemberState, GroupRoleSnapshot,
        ReadyCheckAnswer, SpellCastIntent,
    },
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
            Request::QuestInteract { npc } => self.quest_interact(&npc),
            Request::LootTakeAll => self.loot_take_all(),
            request => return self.item_request(request),
        };
        Ok(match answer {
            Ok(text) => Response::Text(text),
            Err(error) => Response::Error(error),
        })
    }

    fn connected(&self) -> bool {
        self.account
            .link
            .as_ref()
            .is_some_and(|link| link.connected)
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

    /// A right-click on the nearest NPC named `name` (auto-loot off; the server checks
    /// range), else a use of the nearest game object of that name, else targeting the
    /// player of that name.
    fn quest_interact(&mut self, name: &str) -> Result<String, String> {
        let player = self
            .world
            .local_player_transform()
            .ok_or("quest interact requires a local player")?
            .origin;
        let distance = |position: godot::prelude::Vector3| {
            (position.x - player.x).powi(2) + (position.z - player.z).powi(2)
        };
        let npc = self
            .replica
            .units()
            .filter_map(|unit| {
                let npc = unit.get::<Npc>()?;
                let node = self.world.unit_node(unit.server_id)?;
                npc.name.eq_ignore_ascii_case(name).then(|| {
                    let dead = unit
                        .get::<Health>()
                        .is_some_and(|health| health.current <= 0.0);
                    (
                        unit.server_id,
                        npc.name.clone(),
                        dead,
                        distance(node.get_global_position()),
                    )
                })
            })
            .min_by(|a, b| a.3.total_cmp(&b.3));
        if let Some((id, npc, dead, _)) = npc {
            self.set_target(Some(id));
            let lootable = self.loot.lootable.contains(&id);
            return match npc_right_click(dead, lootable, false) {
                NpcRightClick::Loot { auto } => self
                    .account
                    .send_loot_unit(id, auto)
                    .map(|()| format!("loot {npc}")),
                NpcRightClick::Target => Ok(format!("target {npc}")),
                NpcRightClick::Interact => self
                    .account
                    .send_interact(id)
                    .map(|()| format!("interact {npc}")),
            }
            .map_err(|error| error.to_string());
        }
        let object = self
            .replica
            .units()
            .filter_map(|unit| {
                let info = unit.get::<GameObjectInfo>()?;
                let position = self.game_objects.position(unit.server_id)?;
                info.name
                    .eq_ignore_ascii_case(name)
                    .then(|| (unit.server_id, info.clone(), distance(position)))
            })
            .min_by(|a, b| a.2.total_cmp(&b.2));
        if let Some((id, info, _)) = object {
            self.account
                .send_use_game_object(id)
                .map_err(|error| error.to_string())?;
            if info.go_type == GAMEOBJECT_TYPE_MAILBOX {
                self.mailbox.session.expect_open(id);
            }
            return Ok(format!("use {}", info.name));
        }
        let player = self.replica.units().find_map(|unit| {
            let player = unit.get::<Player>()?;
            player
                .name
                .eq_ignore_ascii_case(name)
                .then(|| (unit.server_id, player.name.clone()))
        });
        let (id, player) =
            player.ok_or_else(|| format!("no NPC, game object or player named {name}"))?;
        self.set_target(Some(id));
        Ok(format!("target {player}"))
    }

    /// Takes every slot of the open loot window; the answer lists what was on them.
    fn loot_take_all(&self) -> Result<String, String> {
        let corpse = self.loot.state.corpse.ok_or("no loot window open")?;
        for slot in &self.loot.state.slots {
            self.account
                .send_loot_slot(corpse, slot.slot)
                .map_err(|error| error.to_string())?;
        }
        Ok(self
            .loot
            .state
            .slots
            .iter()
            .map(|slot| loot_chat_text(&slot.content))
            .collect::<Vec<_>>()
            .join("\n"))
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
fn resolve_spell_target(
    selector: Option<&str>,
    current: Option<u64>,
) -> Result<Option<u64>, String> {
    match selector {
        None => current
            .map(Some)
            .ok_or_else(|| "no current target selected".into()),
        Some(selector) if selector.eq_ignore_ascii_case("current") => current
            .map(Some)
            .ok_or_else(|| "no current target selected".into()),
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
