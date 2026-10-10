//! World and combat requests in the original response text: `map target` and `map
//! waypoint add|clear` (src/ipc/plugin.rs, src/ipc/format.rs `format_map_target`),
//! `quest interact` and `loot take-all` (src/ipc/plugin.rs `handle_quest_interact`,
//! `handle_loot_take_all`),
//! `group roster|status|invite|uninvite`, `emote`, `spell cast|stop`
//! (src/ipc/plugin/combat.rs, src/ipc/format.rs). The native client has no client-side
//! entity IDs: `map target` reports the target's server entity.
use game_engine_network::ipc_wire::{Request, Response};
use game_engine_ui_model::{
    group_state::GroupCommand,
    ipc_format::{format_group_roster, format_group_status, resolve_spell_identifier},
    loot_data::{NpcRightClick, loot_chat_text, npc_right_click},
};
use shared::{
    components::{Health, Npc, Player},
    protocol::{EmoteIntent, GAMEOBJECT_TYPE_MAILBOX, GameObjectInfo, SpellCastIntent},
};

impl crate::GameClient {
    /// A world request's response, or the request when the client does not serve it.
    pub(crate) fn world_request(&mut self, request: Request) -> Result<Response, Request> {
        let answer = match request {
            Request::DeathStatus => format_death_snapshot(self.death_flow.snapshot.as_ref()),
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
            Request::GroupSubgroup { name, subgroup } => self.group_ipc(
                "group subgroup",
                GroupCommand::SetSubgroup {
                    name: name.clone(),
                    subgroup,
                },
                format!("group subgroup submitted for {name}: {subgroup}"),
            ),
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

    /// The original `format_map_target` (src/ipc/format.rs:465, over Bevy queries): the
    /// target's name, server entity, position and ground distance from the local player.
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

    /// The original `handle_quest_interact` (src/ipc/plugin.rs:761, over Bevy queries):
    /// a right-click on the nearest NPC named `name` (auto-loot off; the server checks
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

    /// The original `handle_loot_take_all` (src/ipc/plugin.rs:742): takes every slot of
    /// the open loot window; the answer lists what was on them.
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

    fn spell_cast_ipc(&mut self, spell: &str, target: Option<&str>) -> Result<String, String> {
        if !self.connected() {
            return Err("spell cast is unavailable: not connected".into());
        }
        let target_entity = resolve_spell_target(target, self.targeting_target())?;
        let (spell_id, spell) = resolve_spell_identifier(spell)?;
        let target_text = target_entity.map_or_else(|| "-".into(), |id| id.to_string());
        let witness = self.cast_witness(target_entity)?;
        let witness_text = witness.map_or_else(
            || "-".into(),
            |ray| format!("{:?}->{:?}", ray.start, ray.end),
        );
        self.account
            .send_spell_intent(SpellCastIntent {
                spell_id,
                spell: spell.clone(),
                target_entity,
                witness,
                destination: None,
            })
            .map(|()| {
                format!(
                    "spell cast submitted spell={spell} target={target_text} witness={witness_text}"
                )
            })
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

fn format_death_snapshot(
    snapshot: Option<&shared::protocol::DeathSnapshot>,
) -> Result<String, String> {
    let snapshot = snapshot.ok_or("No authoritative death snapshot received")?;
    serde_json::to_string_pretty(snapshot)
        .map_err(|error| format!("Serialize authoritative death snapshot: {error}"))
}

#[cfg(test)]
mod death_status_tests {
    use super::format_death_snapshot;
    use shared::protocol::{DeathPositionSnapshot, DeathSnapshot, DeathStateSnapshot};

    #[test]
    fn deathstate_ipc_status_preserves_authoritative_phase_and_positions() {
        let snapshot = DeathSnapshot {
            state: DeathStateSnapshot::Ghost,
            corpse: Some(DeathPositionSnapshot {
                map_id: 0,
                x: -9464.0,
                y: 56.0,
                z: -62.0,
            }),
            graveyard: None,
            can_resurrect_at_corpse: false,
            spirit_healer_available: true,
        };
        let json: serde_json::Value =
            serde_json::from_str(&format_death_snapshot(Some(&snapshot)).unwrap()).unwrap();
        assert_eq!(json["state"], "Ghost");
        assert_eq!(json["corpse"]["x"], -9464.0);
        assert_eq!(json["corpse"]["y"], 56.0);
        assert_eq!(json["corpse"]["z"], -62.0);
        assert_eq!(json["corpse"]["map_id"], 0);
        assert_eq!(json["graveyard"], serde_json::Value::Null);
        assert_eq!(json["can_resurrect_at_corpse"], false);
        assert_eq!(json["spirit_healer_available"], true);
        assert!(format_death_snapshot(None).is_err());
    }
}

/// The original `resolve_spell_target` (src/ipc/format.rs:622), which reads the
/// Bevy `CurrentTarget` entity: the current target (default or `current`), `none`, or a
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
