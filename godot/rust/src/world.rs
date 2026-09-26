//! Server-identified replicated unit nodes. Visual models and world readiness are separate.

use std::{collections::HashMap, f32::consts::PI};

use game_engine_network::UnitSnapshot;
use godot::{
    builtin::{Transform3D, Vector3},
    classes::Node3D,
    prelude::*,
};

struct UnitNode {
    node: Gd<Node3D>,
    name: String,
    is_player: bool,
}

fn unit_position(snapshot: &UnitSnapshot) -> Option<Vector3> {
    let position = snapshot.position?;
    Some(Vector3::new(position.x, position.y, position.z))
}

fn unit_yaw(snapshot: &UnitSnapshot, is_new: bool) -> Option<f32> {
    snapshot
        .rotation
        .map(|rotation| rotation.y)
        .or_else(|| is_new.then(|| if snapshot.player.is_some() { PI } else { 0.0 }))
}

fn newest_matching_player<'a>(
    units: impl Iterator<Item = (u64, &'a str, bool, i32)>,
    selected_name: &str,
) -> (Option<u64>, usize) {
    let matches: Vec<_> = units
        .filter(|(_, name, is_player, _)| *is_player && *name == selected_name)
        .collect();
    let count = matches.len();
    let chosen = matches
        .into_iter()
        .max_by_key(|(_, _, _, child_index)| *child_index)
        .map(|(id, _, _, _)| id);
    (chosen, count)
}

fn spawn_root(parent: &mut Gd<Node3D>) -> Gd<Node3D> {
    let mut root = Node3D::new_alloc();
    root.set_name("WorldUnits");
    parent.add_child(&root);
    root
}

fn spawn_unit(
    root: &mut Option<Gd<Node3D>>,
    parent: &mut Gd<Node3D>,
    name: &str,
    is_player: bool,
) -> UnitNode {
    let root = root.get_or_insert_with(|| spawn_root(parent));
    let mut node = Node3D::new_alloc();
    node.set_name(name);
    root.add_child(&node);
    UnitNode {
        node,
        name: name.to_owned(),
        is_player,
    }
}

fn resolve_selected_player(
    units: &HashMap<u64, UnitNode>,
    current_id: Option<u64>,
    name: &str,
) -> Option<u64> {
    let existing_match = current_id.filter(|id| {
        units
            .get(id)
            .is_some_and(|unit| unit.is_player && unit.name == name)
    });
    if existing_match.is_some() {
        return existing_match;
    }

    // Godot uniquifies duplicate sibling names. The last-created child is the
    // closest equivalent to the original render world's newest matching entity.
    let (chosen, match_count) = newest_matching_player(
        units.iter().map(|(id, unit)| {
            (
                *id,
                unit.name.as_str(),
                unit.is_player,
                unit.node.get_index(),
            )
        }),
        name,
    );
    if match_count > 1 {
        godot_warn!(
            "Found {match_count} replicated players named '{}'; choosing newest entity as local",
            name
        );
    }
    chosen
}

#[derive(Default)]
pub struct WorldUnits {
    root: Option<Gd<Node3D>>,
    units: HashMap<u64, UnitNode>,
    selected_name: Option<String>,
    local_player_id: Option<u64>,
}

impl WorldUnits {
    pub fn upsert(&mut self, parent: &mut Gd<Node3D>, snapshot: &UnitSnapshot) {
        let Some(position) = unit_position(snapshot) else {
            return;
        };
        let Some(name) = snapshot
            .player
            .as_ref()
            .map(|player| player.name.as_str())
            .or_else(|| snapshot.npc.as_ref().map(|npc| npc.name.as_str()))
        else {
            return;
        };

        let is_new = !self.units.contains_key(&snapshot.server_id);
        let unit = self
            .units
            .entry(snapshot.server_id)
            .or_insert_with(|| spawn_unit(&mut self.root, parent, name, snapshot.player.is_some()));
        if unit.name != name {
            unit.node.set_name(name);
            unit.name = name.to_owned();
        }
        unit.is_player = snapshot.player.is_some();
        unit.node.set_position(position);
        if let Some(yaw) = unit_yaw(snapshot, is_new) {
            unit.node.set_rotation(Vector3::new(0.0, yaw, 0.0));
        }
    }

    pub fn remove(&mut self, id: u64) {
        if let Some(unit) = self.units.remove(&id) {
            unit.node.free();
        }
        if self.local_player_id == Some(id) {
            self.local_player_id = None;
        }
    }

    pub fn reset(&mut self) {
        self.units.clear();
        self.local_player_id = None;
        self.selected_name = None;
        if let Some(root) = self.root.take() {
            root.free();
        }
    }

    pub fn root(&self) -> Option<Gd<Node3D>> {
        self.root.clone()
    }

    pub fn local_player_transform(&self) -> Option<Transform3D> {
        let unit = self.units.get(&self.local_player_id?)?;
        Some(unit.node.get_transform())
    }

    /// Match the selected character's exact name. Keep an existing match when a duplicate arrives.
    pub fn select_local_player(&mut self, selected_name: Option<&str>) {
        if self.selected_name.as_deref() != selected_name {
            self.selected_name = selected_name.map(str::to_owned);
            self.local_player_id = None;
        }
        let Some(name) = selected_name else {
            return;
        };
        self.local_player_id = resolve_selected_player(&self.units, self.local_player_id, name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::components::{Player, Position, Rotation};

    fn player_snapshot() -> UnitSnapshot {
        UnitSnapshot {
            server_id: 42,
            player: Some(Player {
                name: "Alice".into(),
                race: 1,
                class: 2,
                appearance: Default::default(),
            }),
            npc: None,
            position: Some(Position {
                x: 10.0,
                y: 20.0,
                z: -30.0,
            }),
            rotation: None,
            health: None,
            mana: None,
            model: None,
            level: None,
            equipment: None,
        }
    }

    #[test]
    fn replicated_position_uses_authoritative_axes_without_conversion() {
        let mut snapshot = player_snapshot();
        assert_eq!(
            unit_position(&snapshot),
            Some(Vector3::new(10.0, 20.0, -30.0))
        );
        snapshot.position = None;
        assert_eq!(unit_position(&snapshot), None);
    }

    #[test]
    fn newest_matching_node_wins_initial_duplicate_selection_not_largest_server_id() {
        let units = [
            (900, "Alice", true, 0),
            (2, "Alice", true, 1),
            (7, "Alice", false, 2),
            (11, "Bob", true, 3),
        ];
        assert_eq!(
            newest_matching_player(units.into_iter(), "Alice"),
            (Some(2), 2)
        );
        assert_eq!(
            newest_matching_player(units.into_iter(), "Bob"),
            (Some(11), 1)
        );
        assert_eq!(
            newest_matching_player(units.into_iter(), "Missing"),
            (None, 0)
        );
    }

    #[test]
    fn yaw_uses_wire_y_and_original_spawn_defaults_without_resetting_updates() {
        let mut snapshot = player_snapshot();
        assert_eq!(unit_yaw(&snapshot, true), Some(PI));
        assert_eq!(unit_yaw(&snapshot, false), None);
        snapshot.player = None;
        assert_eq!(unit_yaw(&snapshot, true), Some(0.0));
        snapshot.rotation = Some(Rotation {
            x: 0.5,
            y: 1.25,
            z: -0.75,
        });
        assert_eq!(unit_yaw(&snapshot, true), Some(1.25));
        assert_eq!(unit_yaw(&snapshot, false), Some(1.25));
    }
}
