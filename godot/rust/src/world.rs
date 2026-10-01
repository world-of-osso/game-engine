//! Server-identified replicated unit nodes and their authored visual children.

use std::{
    collections::{HashMap, VecDeque},
    f32::consts::PI,
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::{
    animation::{WowAnimationPlayer, lod::AnimationLod},
    lighting::TerrainLight,
    world_models::{
        UnitAppearance, VisualParts, WorldModels, bind_visual_light, place_items,
        place_virtual_items,
    },
};

use crate::replicated::UnitFields;
use crate::terrain::scenery::SceneryDistance;
use crate::wmo::portals::HalfSpace;
use game_engine_core::movement_animation_data::{ANIM_RUN, ANIM_STAND, direction_to_anim_id};
use game_engine_core::movement_input_data::MoveDirection;
use game_engine_core::npc_visibility_data::{npc_should_be_visible, npc_visibility_policy};
use game_engine_core::unit_motion_data::{
    MotionPose, MotionTarget, follow_server_motion, interpolate_remote_motion,
};
use game_engine_network::replica::{Replica, Unit};
use glam::{Affine3A, Vec3};
use godot::{
    builtin::{Transform3D, Vector3},
    classes::Node3D,
    prelude::*,
};
use shared::components::{
    CombatStatus, CreatureMotion, EquipmentAppearance, Health, ModelDisplay, MovementControl,
    MovementSpeed, Npc, Player, PlayerMotion, Position, Rotation, SheathState, UnitPose,
};

#[path = "world_combat.rs"]
pub(crate) mod combat;
use combat::MeleeWeapon;

/// Main-thread time per frame for attaching loaded unit visuals.
const VISUAL_BUDGET: Duration = Duration::from_millis(8);

/// Unit node metadata: the replicated name (Godot renames duplicate siblings `@Node3D@N`).
const UNIT_NAME_META: &str = "unit_name";

struct UnitNode {
    node: Gd<Node3D>,
    name: String,
    is_player: bool,
    motion: UnitMotion,
    /// The appearance of the newest requested visual.
    appearance: Option<UnitAppearance>,
    visual: Option<Gd<Node3D>>,
    /// Visual request still loading and the sheath state its virtual items are placed
    /// for; `visual` shows the previous appearance until it arrives.
    loading: Option<(u64, SheathState)>,
    /// Player race and sex of `visual`'s body, whose playback a re-dress continues.
    visual_player_model: Option<(u8, u8)>,
    death_applied: bool,
    /// Animation ID last selected on `visual` from its `CreatureMotion` and `UnitPose`.
    animation: Option<u16>,
    /// Another player's newest replicated movement flags.
    player_motion: Option<PlayerMotion>,
    /// The replicated pose `pose_anim` was resolved from.
    pose: Option<UnitPose>,
    /// The looping animation `pose` holds while the creature stands still.
    pose_anim: Option<u16>,
    /// The sheath state `visual`'s virtual items are placed for.
    sheath: Option<SheathState>,
    /// Replicated `CombatStatus`: a standing unit holds its Ready stance.
    in_combat: bool,
    /// Main-hand weapon class of `appearance`, for combat clips.
    weapon: MeleeWeapon,
    /// `Item.SubclassID` of the main-hand weapon, for spell visual conditions.
    main_hand_subclass: Option<u8>,
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

fn unit_position(snapshot: Unit) -> Option<Vector3> {
    let position = snapshot.get::<Position>()?;
    Some(Vector3::new(position.x, position.y, position.z))
}

fn unit_yaw(snapshot: Unit, is_new: bool) -> Option<f32> {
    snapshot
        .get::<Rotation>()
        .map(|rotation| rotation.y)
        .or_else(|| is_new.then(|| if snapshot.has::<Player>() { PI } else { 0.0 }))
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
    node.set_meta(UNIT_NAME_META, &name.to_variant());
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
        loading: None,
        visual_player_model: None,
        death_applied: false,
        animation: None,
        player_motion: None,
        pose: None,
        pose_anim: None,
        sheath: None,
        in_combat: false,
        weapon: MeleeWeapon::Unarmed,
        main_hand_subclass: None,
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

fn unit_appearance(snapshot: Unit, native_display: Option<u32>) -> Option<UnitAppearance> {
    let equipment = || {
        snapshot
            .get::<EquipmentAppearance>()
            .cloned()
            .unwrap_or_default()
    };
    if let Some(player) = snapshot.get::<Player>() {
        if let Some(model) = snapshot
            .get::<ModelDisplay>()
            .filter(|model| model.display_id != 0 && Some(model.display_id) != native_display)
        {
            return Some(UnitAppearance::Creature {
                display_id: model.display_id,
                // Player armor and weapons belong to the native humanoid, not its form.
                items: Default::default(),
            });
        }
        return Some(UnitAppearance::Player(player.clone(), equipment()));
    }
    snapshot.get::<Npc>()?;
    let display_id = snapshot.get::<ModelDisplay>()?.display_id;
    (display_id != 0).then(|| UnitAppearance::Creature {
        display_id,
        items: equipment(),
    })
}

/// A unit's replicated sheath state. A player has none replicated: its weapons are drawn
/// in combat and otherwise sheathed (`SHEATH_STATE_UNARMED`, the `SheatheState` a
/// TrinityCore unit starts with, UnitDefines.h:82).
fn unit_sheath(snapshot: Unit) -> SheathState {
    match snapshot.get::<UnitPose>() {
        Some(pose) => pose.sheath_state,
        None if snapshot.has::<Player>() && snapshot.in_combat() => SheathState::Melee,
        None => SheathState::Unarmed,
    }
}

/// Request the visual of a changed appearance; a unit without one loses its visual.
fn request_unit_visual(unit: &mut UnitNode, snapshot: Unit, models: &mut WorldModels) {
    let native_display = match snapshot.get::<Player>().filter(|_| {
        snapshot
            .get::<ModelDisplay>()
            .is_some_and(|model| model.display_id != 0)
    }) {
        Some(player) => match models.player_native_display(player) {
            Ok(display) => Some(display),
            Err(error) => {
                godot_error!("Player {} native display: {error}", snapshot.server_id);
                return;
            }
        },
        None => None,
    };
    let appearance = unit_appearance(snapshot, native_display);
    if unit.appearance == appearance {
        return;
    }
    unit.appearance = appearance;
    unit.loading = unit.appearance.as_ref().map(|appearance| {
        let sheath = unit_sheath(snapshot);
        (models.request(appearance, sheath), sheath)
    });
    if unit.appearance.is_none() {
        unit.animation = None;
        unit.sheath = None;
        if let Some(previous) = unit.visual.take() {
            previous.free();
        }
    }
}

/// Replace `unit`'s visual with the loaded one of its newest request.
fn attach_unit_visual(
    unit: &mut UnitNode,
    server_id: u64,
    loaded: Result<VisualParts, String>,
    sheath: SheathState,
    models: &WorldModels,
    light: Option<&TerrainLight>,
) {
    let appearance = unit
        .appearance
        .as_ref()
        .expect("a loading unit has an appearance");
    let preserve_playback = unit.visual.is_some()
        && unit.visual_player_model.is_some()
        && unit.visual_player_model == appearance.player_model();
    let replacement = loaded.and_then(|parts| {
        models.build_visual(parts, unit.visual.as_ref().filter(|_| preserve_playback))
    });
    unit.animation = None;
    // A creature's load places its virtual items for `sheath`; a player's weapons are
    // placed by the sheath sync that follows the attach.
    unit.sheath = appearance.player_model().is_none().then_some(sheath);
    if let Some(previous) = unit.visual.take() {
        previous.free();
    }
    match replacement {
        Ok(visual) => {
            if let Err(error) = crate::targeting::attach_pick_area(&visual, server_id) {
                godot_error!("{error}");
            }
            bind_visual_light(&visual, light);
            unit.node.add_child(&visual);
            unit.visual = Some(visual);
            unit.visual_player_model = appearance.player_model();
        }
        Err(error) => {
            unit.visual_player_model = None;
            godot_error!("{}: {error}", appearance.describe_unit(server_id));
        }
    }
}

/// Play a dead NPC's death clip once; whether it started now.
fn sync_unit_death(unit: &mut UnitNode, snapshot: Unit) -> bool {
    let alive = snapshot
        .get::<Health>()
        .is_none_or(|health| health.current > 0.0);
    if unit.death_applied || alive {
        return false;
    }
    if !snapshot.has::<Npc>() || unit.is_player {
        return false;
    }
    let Some(animation) = unit
        .visual
        .as_ref()
        .and_then(|visual| visual.get_node_or_null("NpcModel/M2Animation"))
    else {
        return false;
    };
    let mut animation = animation.cast::<WowAnimationPlayer>();
    match animation.bind_mut().play_death() {
        Ok(()) => {
            unit.death_applied = true;
            true
        }
        Err(error) => {
            godot_error!("NPC {} death animation: {error}", snapshot.server_id);
            false
        }
    }
}

/// Original locomotion animation of a replicated creature's motion (Bevy
/// `motion_movement_state`): Still stands, Walk and Run move forward.
pub(crate) fn creature_motion_animation_id(motion: CreatureMotion) -> u16 {
    let (direction, running) = match motion {
        CreatureMotion::Still => (MoveDirection::None, false),
        CreatureMotion::Walk => (MoveDirection::Forward, false),
        CreatureMotion::Run => (MoveDirection::Forward, true),
    };
    direction_to_anim_id(direction, running, false)
}

/// The clip a replicated creature plays (Bevy `switch_animation` precedence): while it
/// walks or runs, its locomotion; while still, the animation its pose holds (sit, sleep,
/// an emote state's stance), else Stand. `None` before any motion or pose arrives.
pub(crate) fn creature_animation_id(
    motion: Option<CreatureMotion>,
    pose_anim: Option<u16>,
) -> Option<u16> {
    match motion {
        Some(CreatureMotion::Walk | CreatureMotion::Run) => {
            motion.map(creature_motion_animation_id)
        }
        _ => pose_anim.or(motion.map(creature_motion_animation_id)),
    }
}

/// The animation ID a snapshot's motion and pose select, or `None` when it is already
/// applied.
pub(crate) fn creature_animation_change(
    applied: Option<u16>,
    motion: Option<CreatureMotion>,
    pose_anim: Option<u16>,
) -> Option<u16> {
    let id = creature_animation_id(motion, pose_anim)?;
    (applied != Some(id)).then_some(id)
}

/// `WowAnimationPlayer::update_locomotion`'s arguments for a player model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Locomotion {
    pub animation_id: u16,
    pub jumping: bool,
    pub running_forward: bool,
}

/// The locomotion another player's replicated Retail `MovementFlags` select, as the local
/// player's own movement selects it (`update_player_animation`): the direction in the
/// local `compute_movement_input` priority (forward, backward, left, right) through the
/// shared direction selector, run unless WALKING, swim clips while SWIMMING, and the jump
/// sequence while FALLING (wow_client `update_animation` animates remote units the same
/// way from their movement flags).
pub(crate) fn player_motion_locomotion(motion: PlayerMotion) -> Locomotion {
    let direction = [
        (PlayerMotion::FORWARD, MoveDirection::Forward),
        (PlayerMotion::BACKWARD, MoveDirection::Backward),
        (PlayerMotion::STRAFE_LEFT, MoveDirection::Left),
        (PlayerMotion::STRAFE_RIGHT, MoveDirection::Right),
    ]
    .into_iter()
    .find_map(|(flag, direction)| motion.contains(flag).then_some(direction))
    .unwrap_or(MoveDirection::None);
    let running = !motion.contains(PlayerMotion::WALKING);
    let swimming = motion.contains(PlayerMotion::SWIMMING);
    Locomotion {
        animation_id: direction_to_anim_id(direction, running, swimming),
        jumping: motion.contains(PlayerMotion::FALLING),
        running_forward: running && direction == MoveDirection::Forward,
    }
}

/// The locomotion a unit follows from its replicated `PlayerMotion`: only another
/// player's. The local player animates from its own predicted movement.
pub(crate) fn remote_player_locomotion(
    is_player: bool,
    is_local: bool,
    motion: Option<PlayerMotion>,
) -> Option<Locomotion> {
    if !is_player || is_local {
        return None;
    }
    motion.map(player_motion_locomotion)
}

/// Resolve the held animation of a changed replicated pose; an unmapped pose holds none.
fn sync_unit_pose(unit: &mut UnitNode, snapshot: Unit, models: &mut WorldModels) {
    let pose = snapshot.get::<UnitPose>().copied();
    if unit.is_player || unit.pose == pose {
        return;
    }
    unit.pose = pose;
    let Some(pose) = pose else {
        unit.pose_anim = None;
        return;
    };
    unit.pose_anim = models
        .gear()
        .and_then(|gear| gear.pose_anim_id(&pose))
        .unwrap_or_else(|error| {
            godot_error!(
                "NPC {} ({}) {pose:?}: {error}",
                snapshot.server_id,
                unit.name
            );
            None
        });
}

/// Move the weapons of a creature or player whose sheath state changed; a visual still
/// loading places them for the current state when it arrives.
fn sync_unit_sheath(unit: &mut UnitNode, snapshot: Unit, models: &mut WorldModels) {
    if unit.loading.is_some() {
        return;
    }
    let sheath = unit_sheath(snapshot);
    let (Some(visual), Some(appearance)) = (&unit.visual, &unit.appearance) else {
        return;
    };
    if unit.sheath == Some(sheath) {
        return;
    }
    unit.sheath = Some(sheath);
    let placed = match appearance {
        UnitAppearance::Creature { items, .. } => models
            .virtual_item_placements(items, sheath)
            .and_then(|placements| place_virtual_items(visual, &placements)),
        UnitAppearance::Player(_, equipment) => models
            .player_weapon_placements(equipment, sheath)
            .and_then(|placements| place_items(visual, &placements)),
    };
    if let Err(error) = placed {
        godot_error!("Unit {} {sheath:?}: {error}", snapshot.server_id);
    }
}

fn sync_unit_animation(unit: &mut UnitNode, snapshot: Unit, fallbacks: &HashMap<u16, u16>) {
    if unit.is_player || unit.death_applied {
        return;
    }
    let Some(mut animation) = unit
        .visual
        .as_ref()
        .and_then(|visual| visual.try_get_node_as::<WowAnimationPlayer>("NpcModel/M2Animation"))
    else {
        return;
    };
    // In combat a creature stands in its Ready stance (or the fallback it has) instead
    // of its held pose.
    let ready = unit
        .in_combat
        .then(|| combat::stance_clip(&animation.bind(), 0, true, unit.weapon, fallbacks));
    // A held pose the model lacks plays its `AnimationData.Fallback` chain (Dead 6 →
    // Death 1 → Stand), as the combat stance does.
    let pose_anim = ready.or_else(|| {
        unit.pose_anim.map(|pose| {
            animation
                .bind()
                .resolve_clip(pose, fallbacks)
                .unwrap_or(ANIM_STAND)
        })
    });
    // The replicated speed of its gait paces the walk and run clips (0: not yet moved).
    let speed = snapshot
        .get::<MovementSpeed>()
        .map(|speed| speed.0)
        .filter(|speed| *speed > 0.0);
    animation.bind_mut().set_locomotion_speed(speed);
    let motion = snapshot.get::<CreatureMotion>().copied();
    let Some(id) = creature_animation_change(unit.animation, motion, pose_anim) else {
        return;
    };
    unit.animation = Some(id);
    if let Err(error) = animation
        .bind_mut()
        .update_locomotion(id, false, id == ANIM_RUN)
    {
        godot_error!("NPC {} animation {id}: {error}", snapshot.server_id);
    }
}

pub struct WorldUnits {
    root: Option<Gd<Node3D>>,
    /// `AnimationData.Fallback`, loaded on first use.
    anim_fallbacks: Option<HashMap<u16, u16>>,
    data_root: PathBuf,
    units: HashMap<u64, UnitNode>,
    selected_name: Option<String>,
    local_player_id: Option<u64>,
    models: WorldModels,
    /// Loaded visuals waiting for main-thread time, oldest first.
    arrived: VecDeque<(u64, Result<VisualParts, String>)>,
    light: Option<TerrainLight>,
    /// Units whose death clip started since `take_deaths`.
    deaths: Vec<u64>,
    /// Visual requests owned by another scene (the paperdoll model), and their arrivals.
    detached: Vec<u64>,
    detached_arrived: Vec<(u64, Result<VisualParts, String>)>,
}

impl WorldUnits {
    pub fn new(data_root: PathBuf) -> Self {
        Self {
            root: None,
            anim_fallbacks: None,
            data_root: data_root.clone(),
            units: HashMap::new(),
            selected_name: None,
            local_player_id: None,
            models: WorldModels::new(data_root),
            arrived: VecDeque::new(),
            light: None,
            deaths: Vec::new(),
            detached: Vec::new(),
            detached_arrived: Vec::new(),
        }
    }

    /// The appearance of the local player's newest requested visual.
    pub fn local_player_appearance(&self) -> Option<&UnitAppearance> {
        self.units.get(&self.local_player_id?)?.appearance.as_ref()
    }

    /// Load `appearance` for a scene outside the world; `take_detached_visual` hands it out.
    pub fn request_detached_visual(&mut self, appearance: &UnitAppearance) -> u64 {
        let id = self.models.request(appearance, SheathState::Unarmed);
        self.detached.push(id);
        id
    }

    /// Main thread: the nodes of detached request `id` once loaded. Superseded arrivals
    /// keep their decoded textures.
    pub fn take_detached_visual(&mut self, id: u64) -> Option<Result<Gd<Node3D>, String>> {
        let mut taken = None;
        for (request, loaded) in std::mem::take(&mut self.detached_arrived) {
            match loaded {
                Ok(parts) if request == id => {
                    taken = Some(self.models.build_visual(parts, None));
                }
                Err(error) if request == id => taken = Some(Err(error)),
                Ok(parts) => self.models.discard(parts),
                Err(_) => {}
            }
        }
        taken
    }

    /// Place a player visual's weapons for `sheath`, as the world sheath sync does.
    pub fn place_player_weapons(
        &mut self,
        visual: &Gd<Node3D>,
        appearance: &UnitAppearance,
        sheath: SheathState,
    ) -> Result<(), String> {
        let UnitAppearance::Player(_, equipment) = appearance else {
            return Ok(());
        };
        let placements = self.models.player_weapon_placements(equipment, sheath)?;
        place_items(visual, &placements)
    }

    pub fn upsert(&mut self, parent: &mut Gd<Node3D>, snapshot: Unit) {
        let Some(position) = unit_position(snapshot) else {
            return;
        };
        let Some(name) = snapshot
            .get::<Player>()
            .map(|player| player.name.as_str())
            .or_else(|| snapshot.get::<Npc>().map(|npc| npc.name.as_str()))
        else {
            return;
        };

        let initial_yaw = unit_yaw(snapshot, true).expect("New unit always has an initial yaw");
        let unit = self.units.entry(snapshot.server_id).or_insert_with(|| {
            spawn_unit(
                &mut self.root,
                parent,
                name,
                snapshot.has::<Player>(),
                position,
                initial_yaw,
            )
        });
        if unit.name != name {
            unit.node.set_name(name);
            unit.node.set_meta(UNIT_NAME_META, &name.to_variant());
            unit.name = name.to_owned();
        }
        unit.is_player = snapshot.has::<Player>();
        unit.player_motion = snapshot.get::<PlayerMotion>().copied();
        unit.in_combat = snapshot
            .get::<CombatStatus>()
            .is_some_and(|status| status.0);
        request_unit_visual(unit, snapshot, &mut self.models);
        (unit.weapon, unit.main_hand_subclass) = combat::unit_weapon_class(unit, &mut self.models);
        sync_unit_sheath(unit, snapshot, &mut self.models);
        sync_unit_pose(unit, snapshot, &mut self.models);
        if sync_unit_death(unit, snapshot) {
            self.deaths.push(snapshot.server_id);
        }
        let fallbacks = combat::load_fallbacks(&mut self.anim_fallbacks, &self.data_root);
        sync_unit_animation(unit, snapshot, fallbacks);
        unit.motion.set_target(
            [position.x, position.y, position.z],
            snapshot.get::<Rotation>().map(|rotation| rotation.y),
            snapshot.get::<MovementControl>().copied(),
        );
    }

    /// Attach arrived visuals within `VISUAL_BUDGET` of main-thread time (at least one
    /// per frame), each brought up to its unit's snapshot.
    pub fn attach_loaded_visuals(&mut self, replica: &Replica) {
        self.arrived.extend(self.models.poll());
        let started = Instant::now();
        while let Some((request, loaded)) = self.arrived.pop_front() {
            if let Some(index) = self.detached.iter().position(|id| *id == request) {
                self.detached.swap_remove(index);
                self.detached_arrived.push((request, loaded));
                continue;
            }
            let loading = self
                .units
                .iter_mut()
                .find(|(_, unit)| unit.loading.is_some_and(|(pending, _)| pending == request));
            let Some((&id, unit)) = loading else {
                // A superseded request, or its unit is gone.
                if let Ok(parts) = loaded {
                    self.models.discard(parts);
                }
                continue;
            };
            let (_, sheath) = unit.loading.take().expect("matched a loading unit");
            attach_unit_visual(unit, id, loaded, sheath, &self.models, self.light.as_ref());
            if let Some(snapshot) = replica.unit(id) {
                sync_unit_sheath(unit, snapshot, &mut self.models);
                if sync_unit_death(unit, snapshot) {
                    self.deaths.push(id);
                }
                let fallbacks = combat::load_fallbacks(&mut self.anim_fallbacks, &self.data_root);
                sync_unit_animation(unit, snapshot, fallbacks);
            }
            if started.elapsed() >= VISUAL_BUDGET {
                break;
            }
        }
    }

    /// Visuals requested and not yet attached.
    pub fn visuals_pending(&self) -> usize {
        self.units
            .values()
            .filter(|unit| unit.loading.is_some())
            .count()
    }

    /// The local player's visual is attached, or failed and was reported.
    pub fn local_visual_settled(&self) -> bool {
        self.local_player_id
            .and_then(|id| self.units.get(&id))
            .is_some_and(|unit| unit.loading.is_none())
    }

    pub fn update_lighting(&mut self, light: Option<TerrainLight>) {
        for unit in self.units.values() {
            if let Some(visual) = &unit.visual {
                bind_visual_light(visual, light.as_ref());
            }
        }
        self.light = light;
    }

    pub fn update_visibility(&mut self, replica: &Replica, minutes: f32) {
        let local_alive = self
            .local_player_id
            .and_then(|id| replica.unit(id))
            .and_then(|snapshot| snapshot.get::<Health>())
            .is_none_or(|health| health.current > 0.0);
        for (id, unit) in &mut self.units {
            let npc = replica.unit(*id).and_then(|snapshot| snapshot.get::<Npc>());
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

    /// Drive every other player's model from its newest replicated flags. Called each
    /// frame after animation time advances, so the jump start, loop and landing play out
    /// as the local player's do; unchanged flags neither restart a clip nor its crossfade.
    /// A model missing a clip is a client failure; the other players still animate.
    pub fn update_remote_locomotion(&mut self) -> Result<(), String> {
        let local = self.local_player_id;
        let mut errors = Vec::new();
        let fallbacks = combat::load_fallbacks(&mut self.anim_fallbacks, &self.data_root);
        for (id, unit) in &mut self.units {
            let Some(locomotion) =
                remote_player_locomotion(unit.is_player, local == Some(*id), unit.player_motion)
            else {
                continue;
            };
            let Some(mut animation) = unit
                .visual
                .as_ref()
                .and_then(|visual| visual.try_get_node_as::<WowAnimationPlayer>("M2Animation"))
            else {
                continue;
            };
            let movement = combat::stance_clip(
                &animation.bind(),
                locomotion.animation_id,
                unit.in_combat,
                unit.weapon,
                fallbacks,
            );
            if let Err(error) = animation.bind_mut().update_locomotion(
                movement,
                locomotion.jumping,
                locomotion.running_forward,
            ) {
                errors.push(format!("Player {} {locomotion:?}: {error}", unit.name));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    /// NPC animation LOD: each NPC model samples its pose at the rate its camera
    /// distance and whether its bounds are in the view frustum allow; players always
    /// sample.
    pub fn apply_animation_lod(&mut self, camera: Vector3, frustum: &[HalfSpace], frame: u64) {
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
            let Some(mut animation) = model.try_get_node_as::<WowAnimationPlayer>("M2Animation")
            else {
                continue;
            };
            let bounds = crate::world_models::mesh_bounds(&model);
            let lod = npc_animation_lod(
                (bounds.position, bounds.end()),
                crate::terrain::objects::affine(model.get_global_transform()),
                Vec3::new(camera.x, camera.y, camera.z),
                frustum,
            );
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
        self.deaths.clear();
        self.local_player_id = None;
        self.selected_name = None;
        if let Some(root) = self.root.take() {
            root.free();
        }
    }

    pub fn root(&self) -> Option<Gd<Node3D>> {
        self.root.clone()
    }

    /// Unit `id`'s creature display (`None` for native players and units without one), whether
    /// a visual for it is loaded, and the locomotion clip last chosen on it.
    pub fn unit_display(&self, id: u64) -> Option<(Option<u32>, bool, Option<u16>)> {
        let unit = self.units.get(&id)?;
        let display_id = match unit.appearance {
            Some(UnitAppearance::Creature { display_id, .. }) => Some(display_id),
            _ => None,
        };
        Some((display_id, unit.visual.is_some(), unit.animation))
    }

    /// The playback rate of an NPC's current clip (movement clips follow its speed).
    pub fn unit_animation_rate(&self, id: u64) -> Option<f32> {
        self.units
            .get(&id)?
            .visual
            .as_ref()?
            .try_get_node_as::<WowAnimationPlayer>("NpcModel/M2Animation")?
            .bind()
            .playback_rate()
    }

    pub fn unit_node(&self, id: u64) -> Option<Gd<Node3D>> {
        Some(self.units.get(&id)?.node.clone())
    }

    pub fn local_player_node(&self) -> Option<Gd<Node3D>> {
        Some(self.units.get(&self.local_player_id?)?.node.clone())
    }

    pub fn local_footstep_phase(&self) -> Option<(u64, Vector3, (usize, u16, f32, f32))> {
        let id = self.local_player_id?;
        let unit = self.units.get(&id)?;
        let animation = unit
            .visual
            .as_ref()?
            .try_get_node_as::<WowAnimationPlayer>("M2Animation")?;
        Some((
            id,
            unit.node.get_global_position(),
            animation.bind().footstep_phase()?,
        ))
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
        let fallbacks = combat::load_fallbacks(&mut self.anim_fallbacks, &self.data_root);
        let movement = combat::stance_clip(
            &animation.bind(),
            animation_id,
            unit.in_combat,
            unit.weapon,
            fallbacks,
        );
        animation
            .bind_mut()
            .update_locomotion(movement, jumping, running_forward)
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

    /// The `MovementControl::epoch` whose position the local player adopted.
    pub fn local_player_epoch(&self) -> Option<u32> {
        self.units.get(&self.local_player_id?)?.motion.adopted_epoch
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

/// The NPC animation rate for model-space `bounds` placed by `world_from_model`: by
/// the model's camera distance and whether its world box is in the view `frustum`,
/// the doodad box-vs-frustum test (`SceneryDistance::box_in_frustum`).
fn npc_animation_lod(
    (min, max): (Vector3, Vector3),
    world_from_model: Affine3A,
    camera: Vec3,
    frustum: &[HalfSpace],
) -> AnimationLod {
    let to_glam = |v: Vector3| Vec3::new(v.x, v.y, v.z);
    let in_frustum =
        SceneryDistance::new(to_glam(min), to_glam(max), world_from_model).box_in_frustum(frustum);
    AnimationLod::new(
        Vec3::from(world_from_model.translation).distance(camera),
        in_frustum,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A camera at the origin looking down -Z with a 90° view, near 0.05, far 1000.
    fn forward_frustum() -> Vec<HalfSpace> {
        [
            (Vec3::new(0.0, 0.0, -1.0), -0.05),
            (Vec3::new(0.0, 0.0, 1.0), 1000.0),
            (Vec3::new(1.0, 0.0, -1.0), 0.0),
            (Vec3::new(-1.0, 0.0, -1.0), 0.0),
            (Vec3::new(0.0, 1.0, -1.0), 0.0),
            (Vec3::new(0.0, -1.0, -1.0), 0.0),
        ]
        .into_iter()
        .map(|(normal, d)| HalfSpace { normal, d })
        .collect()
    }

    /// The rate of a 1-yard NPC box centered at `position` seen from the origin.
    fn lod_at(position: Vec3) -> AnimationLod {
        npc_animation_lod(
            (Vector3::splat(-0.5), Vector3::splat(0.5)),
            Affine3A::from_translation(position),
            Vec3::ZERO,
            &forward_frustum(),
        )
    }

    /// NPC animation LOD from bounds and the view frustum alone, so it is the same with
    /// or without a renderer: in view it samples by distance, out of view or beyond
    /// 60 yd it freezes.
    #[test]
    fn npc_animation_lod_samples_in_the_frustum_and_freezes_outside_or_far() {
        assert_eq!(lod_at(Vec3::new(0.0, 0.0, -10.0)), AnimationLod::Full);
        assert_eq!(lod_at(Vec3::new(0.0, 0.0, -45.0)), AnimationLod::Half);
        assert_eq!(lod_at(Vec3::new(0.0, 0.0, -70.0)), AnimationLod::Frozen);
        assert_eq!(lod_at(Vec3::new(0.0, 0.0, 10.0)), AnimationLod::Frozen);
        assert_eq!(lod_at(Vec3::new(13.0, 0.0, -10.0)), AnimationLod::Frozen);
        // A box straddling the frustum edge is in view.
        assert_eq!(lod_at(Vec3::new(10.3, 0.0, -10.0)), AnimationLod::Full);
        assert!(lod_at(Vec3::new(0.0, 0.0, -10.0)).samples_frame(7, 3));
        assert!(!lod_at(Vec3::new(0.0, 0.0, 10.0)).samples_frame(7, 3));
    }

    const PLAYER_ID: u64 = 42;

    fn player_replica() -> Replica {
        let mut replica = Replica::for_tests();
        replica.insert(
            PLAYER_ID,
            Player {
                name: "Alice".into(),
                race: 1,
                class: 2,
                appearance: Default::default(),
            },
        );
        replica.insert(
            PLAYER_ID,
            Position {
                x: 10.0,
                y: 20.0,
                z: -30.0,
            },
        );
        replica
    }

    #[test]
    fn player_model_display_consumes_cat_bear_and_native_restoration() {
        // Human male ChrModel 1; Cat/Bear SpellShapeshiftForm 1/5 in build 69933.
        let native = 57899;
        let mut replica = player_replica();
        for expected in [native, 115603, 115602, native] {
            replica.insert(
                PLAYER_ID,
                ModelDisplay {
                    display_id: expected,
                },
            );
            let unit = replica.unit(PLAYER_ID).unwrap();
            let actual = match unit_appearance(unit, Some(native)).expect("player appearance") {
                UnitAppearance::Creature { display_id, .. } => display_id,
                UnitAppearance::Player(_, _) => native,
            };
            assert_eq!(actual, expected);
            assert!(
                unit.has::<Player>(),
                "form must not change replicated player identity"
            );
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

    /// Another player swimming at the Northshire lake (seabed 140.07, surface 143.99): its
    /// node follows the server's replicated height, floating or mid-water, never the seabed.
    #[test]
    fn remote_swimmer_follows_replicated_height_not_the_seabed() {
        use game_engine_core::unit_motion_data::MotionPose;

        let seabed = [-8558.0, 140.07, 500.0];
        let floating = [-8558.0, shared::movement::swim_top(143.99), 500.0];
        let mut motion = UnitMotion::new(seabed, 0.0);
        motion.set_target(floating, None, None);
        let mut pose = MotionPose {
            position: seabed.into(),
            rotation: Default::default(),
        };
        for _ in 0..60 {
            pose = motion.advance(pose, false, 1.0 / 30.0);
        }
        assert!(
            (pose.position.y - floating[1]).abs() < 0.001,
            "remote swimmer at {}",
            pose.position
        );
        motion.set_target([-8558.0, 141.5, 500.0], None, None);
        for _ in 0..60 {
            pose = motion.advance(pose, false, 1.0 / 30.0);
        }
        assert!(
            (pose.position.y - 141.5).abs() < 0.001,
            "mid-water {}",
            pose.position
        );
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
        let mut replica = player_replica();
        assert_eq!(
            unit_position(replica.unit(PLAYER_ID).unwrap()),
            Some(Vector3::new(10.0, 20.0, -30.0))
        );
        replica.remove::<Position>(PLAYER_ID);
        assert_eq!(unit_position(replica.unit(PLAYER_ID).unwrap()), None);
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
        let mut replica = player_replica();
        let yaw = |replica: &Replica, is_new| unit_yaw(replica.unit(PLAYER_ID).unwrap(), is_new);
        assert_eq!(yaw(&replica, true), Some(PI));
        assert_eq!(yaw(&replica, false), None);
        replica.remove::<Player>(PLAYER_ID);
        assert_eq!(yaw(&replica, true), Some(0.0));
        replica.insert(
            PLAYER_ID,
            Rotation {
                x: 0.5,
                y: 1.25,
                z: -0.75,
            },
        );
        assert_eq!(yaw(&replica, true), Some(1.25));
        assert_eq!(yaw(&replica, false), Some(1.25));
    }
}
