//! Server-identified replicated unit nodes and their authored visual children.

use std::{collections::HashMap, f32::consts::PI, path::PathBuf};

use crate::{
    animation::{WowAnimationPlayer, lod::AnimationLod},
    lighting::TerrainLight,
    world_models::{UnitAppearance, WorldModels, bind_visual_light},
};

use game_engine_core::npc_visibility_data::{npc_should_be_visible, npc_visibility_policy};
use game_engine_core::unit_motion_data::{
    MotionPose, MotionTarget, follow_server_motion, interpolate_remote_motion,
};
use game_engine_network::UnitSnapshot;
use godot::{
    builtin::{Transform3D, Vector3},
    classes::{Node3D, VisibleOnScreenNotifier3D},
    prelude::*,
};
use shared::components::MovementControl;

struct UnitNode {
    node: Gd<Node3D>,
    name: String,
    is_player: bool,
    motion: UnitMotion,
    appearance: Option<UnitAppearance>,
    visual: Option<Gd<Node3D>>,
    death_applied: bool,
}

struct UnitMotion {
    target: MotionTarget,
    local_yaw: Option<f32>,
    control: Option<MovementControl>,
    adopted_epoch: Option<u32>,
    facing_yaw: f32,
}

impl UnitMotion {
    fn new(position: [f32; 3], yaw: f32) -> Self {
        Self {
            target: MotionTarget {
                position: position.into(),
                yaw: Some(yaw),
            },
            local_yaw: None,
            control: None,
            adopted_epoch: None,
            facing_yaw: PI,
        }
    }

    fn set_target(
        &mut self,
        position: [f32; 3],
        yaw: Option<f32>,
        control: Option<MovementControl>,
    ) {
        self.target.position = position.into();
        if yaw.is_some() {
            self.target.yaw = yaw;
        }
        // Remote targets persist; local authoritative facing requires a present rotation.
        self.local_yaw = yaw;
        self.control = control;
    }

    fn advance(&mut self, pose: MotionPose, is_local: bool, delta: f32) -> MotionPose {
        if !is_local {
            return interpolate_remote_motion(pose, self.target, delta);
        }
        let Some(control) = self.control else {
            return pose;
        };
        let target = MotionTarget {
            yaw: self.local_yaw,
            ..self.target
        };
        let update = follow_server_motion(
            pose,
            target,
            self.adopted_epoch,
            control.epoch,
            control.controlled,
            delta,
        );
        self.adopted_epoch = Some(update.adopted_epoch);
        if let Some(yaw) = update.facing_yaw {
            self.facing_yaw = yaw;
        }
        update.pose
    }
}

fn advance_unit_transform(unit: &mut UnitNode, is_local: bool, delta: f32) {
    let position = unit.node.get_position();
    let rotation = unit.node.get_quaternion();
    let current = MotionPose {
        position: [position.x, position.y, position.z].into(),
        rotation: glam::Quat::from_array([rotation.x, rotation.y, rotation.z, rotation.w]),
    };
    let pose = unit.motion.advance(current, is_local, delta);
    if pose.position != current.position {
        unit.node.set_position(Vector3::new(
            pose.position.x,
            pose.position.y,
            pose.position.z,
        ));
    }
    if pose.rotation != current.rotation {
        let rotation = pose.rotation.to_array();
        unit.node.set_quaternion(Quaternion::new(
            rotation[0],
            rotation[1],
            rotation[2],
            rotation[3],
        ));
    }
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
    position: Vector3,
    yaw: f32,
) -> UnitNode {
    let root = root.get_or_insert_with(|| spawn_root(parent));
    let mut node = Node3D::new_alloc();
    node.set_name(name);
    node.set_position(position);
    node.set_rotation(Vector3::new(0.0, yaw, 0.0));
    root.add_child(&node);
    UnitNode {
        node,
        name: name.to_owned(),
        is_player,
        motion: UnitMotion::new([position.x, position.y, position.z], yaw),
        appearance: None,
        visual: None,
        death_applied: false,
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

fn unit_appearance(snapshot: &UnitSnapshot) -> Option<UnitAppearance> {
    if let Some(player) = &snapshot.player {
        return Some(UnitAppearance::Player(
            player.clone(),
            snapshot.equipment.clone().unwrap_or_default(),
        ));
    }
    snapshot.npc.as_ref()?;
    let display = snapshot.model.as_ref()?.display_id;
    (display != 0).then_some(UnitAppearance::Creature(display))
}

fn sync_unit_visual(
    unit: &mut UnitNode,
    snapshot: &UnitSnapshot,
    models: &mut WorldModels,
    light: Option<&TerrainLight>,
) {
    let appearance = unit_appearance(snapshot);
    if unit.appearance == appearance {
        return;
    }
    let preserve_playback = unit
        .appearance
        .as_ref()
        .zip(appearance.as_ref())
        .is_some_and(|(old, new)| old.same_player_model(new));
    let previous = unit.visual.take();
    unit.appearance = appearance;
    let replacement = unit.appearance.as_ref().map(|appearance| {
        models.load_visual(appearance, previous.as_ref().filter(|_| preserve_playback))
    });
    if let Some(previous) = previous {
        previous.free();
    }
    match replacement {
        Some(Ok(visual)) => {
            bind_visual_light(&visual, light);
            unit.node.add_child(&visual);
            unit.visual = Some(visual);
        }
        Some(Err(error)) => {
            let appearance = unit
                .appearance
                .as_ref()
                .expect("Visual load has appearance");
            godot_error!("{}: {error}", appearance.describe_unit(snapshot.server_id));
        }
        None => {}
    }
}

fn sync_unit_death(unit: &mut UnitNode, snapshot: &UnitSnapshot) {
    let alive = snapshot
        .health
        .as_ref()
        .is_none_or(|health| health.current > 0.0);
    if unit.death_applied || alive {
        return;
    }
    if snapshot.npc.is_none() || unit.is_player {
        return;
    }
    let Some(animation) = unit
        .visual
        .as_ref()
        .and_then(|visual| visual.get_node_or_null("NpcModel/M2Animation"))
    else {
        return;
    };
    let mut animation = animation.cast::<WowAnimationPlayer>();
    match animation.bind_mut().play_death() {
        Ok(()) => unit.death_applied = true,
        Err(error) => godot_error!("NPC {} death animation: {error}", snapshot.server_id),
    }
}

pub struct WorldUnits {
    root: Option<Gd<Node3D>>,
    units: HashMap<u64, UnitNode>,
    selected_name: Option<String>,
    local_player_id: Option<u64>,
    models: WorldModels,
    light: Option<TerrainLight>,
}

impl WorldUnits {
    pub fn new(data_root: PathBuf, cache_root: PathBuf) -> Self {
        Self {
            root: None,
            units: HashMap::new(),
            selected_name: None,
            local_player_id: None,
            models: WorldModels::new(data_root, cache_root),
            light: None,
        }
    }

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

        let initial_yaw = unit_yaw(snapshot, true).expect("New unit always has an initial yaw");
        let unit = self.units.entry(snapshot.server_id).or_insert_with(|| {
            spawn_unit(
                &mut self.root,
                parent,
                name,
                snapshot.player.is_some(),
                position,
                initial_yaw,
            )
        });
        if unit.name != name {
            unit.node.set_name(name);
            unit.name = name.to_owned();
        }
        unit.is_player = snapshot.player.is_some();
        sync_unit_visual(unit, snapshot, &mut self.models, self.light.as_ref());
        sync_unit_death(unit, snapshot);
        unit.motion.set_target(
            [position.x, position.y, position.z],
            snapshot.rotation.map(|rotation| rotation.y),
            snapshot.movement_control,
        );
    }

    pub fn update_lighting(&mut self, light: Option<TerrainLight>) {
        for unit in self.units.values() {
            if let Some(visual) = &unit.visual {
                bind_visual_light(visual, light.as_ref());
            }
        }
        self.light = light;
    }

    pub fn update_visibility(&mut self, snapshots: &HashMap<u64, UnitSnapshot>, minutes: f32) {
        let local_alive = self
            .local_player_id
            .and_then(|id| snapshots.get(&id))
            .and_then(|snapshot| snapshot.health.as_ref())
            .is_none_or(|health| health.current > 0.0);
        for (id, unit) in &mut self.units {
            let npc = snapshots.get(id).and_then(|snapshot| snapshot.npc.as_ref());
            let visible = npc.is_none_or(|npc| {
                npc_should_be_visible(npc_visibility_policy(npc.template_id), local_alive, minutes)
            });
            if unit.node.is_visible() != visible {
                unit.node.set_visible(visible);
            }
        }
    }

    pub fn advance(&mut self, delta: f32) {
        for (id, unit) in &mut self.units {
            advance_unit_transform(unit, self.local_player_id == Some(*id), delta);
        }
    }

    /// NPC animation LOD: each NPC model samples its pose at the rate its camera
    /// distance and last frame's on-screen state allow; players always sample.
    pub fn apply_animation_lod(&mut self, camera: Vector3, frame: u64) {
        for (id, unit) in &self.units {
            if unit.is_player {
                continue;
            }
            let Some(model) = unit
                .visual
                .as_ref()
                .and_then(|visual| visual.try_get_node_as::<Node3D>("NpcModel"))
            else {
                continue;
            };
            let (Some(mut animation), Some(on_screen)) = (
                model.try_get_node_as::<WowAnimationPlayer>("M2Animation"),
                model.try_get_node_as::<VisibleOnScreenNotifier3D>("OnScreen"),
            ) else {
                continue;
            };
            let distance = model.get_global_position().distance_to(camera);
            let lod = AnimationLod::new(distance, on_screen.is_on_screen());
            animation
                .bind_mut()
                .set_sampling(lod.samples_frame(frame, *id));
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
        self.light = None;
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

    pub fn local_player_node(&self) -> Option<Gd<Node3D>> {
        Some(self.units.get(&self.local_player_id?)?.node.clone())
    }

    pub fn update_local_locomotion(
        &mut self,
        animation_id: u16,
        jumping: bool,
        running_forward: bool,
    ) -> Result<(), String> {
        let Some(unit) = self.local_player_id.and_then(|id| self.units.get_mut(&id)) else {
            return Ok(());
        };
        let visual = unit
            .visual
            .as_ref()
            .ok_or_else(|| format!("Local player {} has no authored visual", unit.name))?;
        let mut animation = visual
            .try_get_node_as::<WowAnimationPlayer>("M2Animation")
            .ok_or_else(|| format!("Local player {} has no bone animation", unit.name))?;
        animation
            .bind_mut()
            .update_locomotion(animation_id, jumping, running_forward)
            .map_err(|error| format!("Local player {} animation: {error}", unit.name))
    }

    pub fn local_player_facing(&self) -> Option<f32> {
        Some(self.units.get(&self.local_player_id?)?.motion.facing_yaw)
    }

    pub fn set_local_player_facing(&mut self, yaw: f32) {
        if let Some(unit) = self.local_player_id.and_then(|id| self.units.get_mut(&id)) {
            unit.motion.facing_yaw = yaw;
        }
    }

    pub fn local_player_controlled(&self) -> bool {
        self.local_player_id
            .and_then(|id| self.units.get(&id))
            .and_then(|unit| unit.motion.control)
            .is_some_and(|control| control.controlled)
    }

    pub fn local_player_id(&self) -> Option<u64> {
        self.local_player_id
    }

    /// The newest server-replicated position of the local player.
    pub fn local_player_server_position(&self) -> Option<Vector3> {
        let target = self
            .units
            .get(&self.local_player_id?)?
            .motion
            .target
            .position;
        Some(Vector3::new(target.x, target.y, target.z))
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
            movement_control: None,
        }
    }

    #[test]
    fn local_facing_starts_at_pi_and_changes_only_for_controlled_yaw() {
        let mut motion = UnitMotion::new([0.0; 3], 0.25);
        assert_eq!(motion.facing_yaw, PI);
        let pose = MotionPose {
            position: glam::Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
        };
        motion.set_target(
            [0.0; 3],
            Some(1.25),
            Some(MovementControl {
                epoch: 1,
                controlled: false,
            }),
        );
        motion.advance(pose, true, 0.1);
        assert_eq!(motion.facing_yaw, PI);
        motion.set_target(
            [0.0; 3],
            Some(1.25),
            Some(MovementControl {
                epoch: 2,
                controlled: true,
            }),
        );
        motion.advance(pose, true, 0.1);
        assert_eq!(motion.facing_yaw, 1.25);
        motion.set_target(
            [0.0; 3],
            None,
            Some(MovementControl {
                epoch: 2,
                controlled: true,
            }),
        );
        motion.advance(pose, true, 0.1);
        assert_eq!(motion.facing_yaw, 1.25);
    }

    #[test]
    fn unit_motion_preserves_prediction_then_applies_changed_control_epoch() {
        use game_engine_core::unit_motion_data::MotionPose;
        use shared::components::MovementControl;

        let mut motion = UnitMotion::new([1.0, 2.0, 3.0], 0.0);
        motion.set_target(
            [10.0, 20.0, 30.0],
            Some(1.5),
            Some(MovementControl {
                epoch: 7,
                controlled: false,
            }),
        );
        let predicted = MotionPose {
            position: [4.0, 5.0, 6.0].into(),
            rotation: Default::default(),
        };
        let adopted = motion.advance(predicted, true, 0.05);
        assert_eq!(adopted.position, predicted.position);
        assert_eq!(adopted.rotation, predicted.rotation);
        assert_eq!(motion.adopted_epoch, Some(7));

        motion.set_target(
            [40.0, 50.0, 60.0],
            Some(2.0),
            Some(MovementControl {
                epoch: 7,
                controlled: false,
            }),
        );
        assert_eq!(
            motion.advance(predicted, true, 0.05).position,
            predicted.position
        );
        motion.set_target(
            [40.0, 50.0, 60.0],
            Some(2.0),
            Some(MovementControl {
                epoch: 8,
                controlled: false,
            }),
        );
        let corrected = motion.advance(predicted, true, 0.05);
        assert_eq!(corrected.position.to_array(), [40.0, 50.0, 60.0]);
        assert_eq!(corrected.rotation, predicted.rotation);
        assert_eq!(motion.adopted_epoch, Some(8));
    }

    #[test]
    fn unit_motion_remote_target_survives_missing_yaw_and_local_control_absence() {
        use game_engine_core::unit_motion_data::MotionPose;

        let mut motion = UnitMotion::new([0.0; 3], 0.0);
        motion.set_target([10.0, 0.0, 0.0], Some(1.0), None);
        motion.set_target([20.0, 0.0, 0.0], None, None);
        let current = MotionPose {
            position: [0.0; 3].into(),
            rotation: Default::default(),
        };
        let local = motion.advance(current, true, 0.05);
        assert_eq!(local.position, current.position);
        assert_eq!(motion.adopted_epoch, None);
        let remote = motion.advance(current, false, 0.05);
        assert_eq!(remote.position.to_array(), [10.0, 0.0, 0.0]);
        let rotation = remote.rotation.to_array();
        assert!((rotation[1] - 0.25_f32.sin()).abs() < 0.00001);
        assert!((rotation[3] - 0.25_f32.cos()).abs() < 0.00001);
    }

    #[test]
    fn unit_motion_controlled_follow_does_not_reuse_a_removed_rotation() {
        use game_engine_core::unit_motion_data::MotionPose;
        use shared::components::MovementControl;

        let mut motion = UnitMotion::new([0.0; 3], 0.0);
        let control = Some(MovementControl {
            epoch: 2,
            controlled: true,
        });
        motion.set_target([10.0, 0.0, 0.0], Some(1.0), control);
        let first = motion.advance(
            MotionPose {
                position: [0.0; 3].into(),
                rotation: Default::default(),
            },
            true,
            0.05,
        );
        assert_eq!(first.position.to_array(), [5.0, 0.0, 0.0]);
        motion.set_target([15.0, 0.0, 0.0], None, control);
        let second = motion.advance(first, true, 0.05);
        assert_eq!(second.position.to_array(), [10.0, 0.0, 0.0]);
        assert_eq!(second.rotation, first.rotation);
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
