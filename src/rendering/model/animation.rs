#[path = "animation/billboard.rs"]
pub mod billboard;

use super::camera::{MoveDirection, MovementState};
use crate::asset::m2_anim::{
    BoneAnimTracks, M2AnimSequence, M2Bone, evaluate_rotation_track, evaluate_vec3_track,
};
use crate::asset::m2_light;
use crate::game_state::GameState;
use bevy::prelude::*;

use super::m2_spawn::RuntimeM2PointLight;
pub use billboard::propagate_spherical_billboards;

/// Marker component for bone entities, storing their local pivot relative to the parent bone.
#[derive(Component)]
pub struct BonePivot(pub Vec3);

/// Bone flag: spherical billboard — bone always faces the camera.
pub const M2_BONE_SPHERICAL_BILLBOARD: u32 = 0x8;

/// Marker for bones that should always face the camera (M2 bone flag 0x8).
#[derive(Component)]
pub struct SphericalBillboard {
    /// Bone pivot in Bevy coordinates — the point the bone rotates around.
    pub pivot: Vec3,
}

/// All animation data for a single animated M2 model root.
#[derive(Component)]
pub struct M2AnimData {
    pub bones: Vec<M2Bone>,
    pub spherical_billboards: Vec<bool>,
    pub sequences: Vec<M2AnimSequence>,
    pub bone_tracks: Vec<BoneAnimTracks>,
    pub joint_entities: Vec<Entity>,
}

/// Active crossfade between two animation sequences.
pub struct AnimTransition {
    pub from_seq_idx: usize,
    pub from_time_ms: f32,
    pub blend_duration_ms: f32,
    pub blend_elapsed_ms: f32,
}

/// Animation player component attached to the model entity.
#[derive(Component)]
pub struct M2AnimPlayer {
    pub current_seq_idx: usize,
    pub time_ms: f32,
    pub looping: bool,
    pub transition: Option<AnimTransition>,
}

// WoW animation IDs
const ANIM_STAND: u16 = 0;
const ANIM_WALK: u16 = 4;
const ANIM_RUN: u16 = 5;
const ANIM_SHUFFLE_LEFT: u16 = 11;
const ANIM_SHUFFLE_RIGHT: u16 = 12;
const ANIM_WALK_BACKWARDS: u16 = 13;
const ANIM_JUMP_START: u16 = 37;
const ANIM_JUMP: u16 = 38; // airborne loop
const ANIM_JUMP_END: u16 = 39;
const ANIM_SPELL_CAST_DIRECTED: u16 = 51;
const ANIM_SPELL_CAST_OMNI: u16 = 52;
const ANIM_READY_SPELL_DIRECTED: u16 = 55;
const ANIM_READY_SPELL_OMNI: u16 = 56;
const ANIM_CHANNEL: u16 = 76;
const ANIM_ATTACK_1H: u16 = 46;
const ANIM_ATTACK_2H: u16 = 48;
const ANIM_ATTACK_OFF: u16 = 47;
const ANIM_PARRY_1H: u16 = 49;
const ANIM_PARRY_2H: u16 = 50;
const ANIM_READY_1H: u16 = 53;
const ANIM_READY_2H: u16 = 54;

/// Melee weapon type, determines which attack animation to play.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeleeWeaponKind {
    OneHand,
    TwoHand,
    OffHand,
}

impl MeleeWeaponKind {
    /// Attack swing animation ID.
    pub fn attack_anim_id(self) -> u16 {
        match self {
            Self::OneHand => ANIM_ATTACK_1H,
            Self::TwoHand => ANIM_ATTACK_2H,
            Self::OffHand => ANIM_ATTACK_OFF,
        }
    }

    /// Parry animation ID.
    pub fn parry_anim_id(self) -> u16 {
        match self {
            Self::OneHand | Self::OffHand => ANIM_PARRY_1H,
            Self::TwoHand => ANIM_PARRY_2H,
        }
    }

    /// Ready/combat stance animation ID.
    pub fn ready_anim_id(self) -> u16 {
        match self {
            Self::OneHand | Self::OffHand => ANIM_READY_1H,
            Self::TwoHand => ANIM_READY_2H,
        }
    }
}

/// Component that triggers a melee attack animation.
/// Plays the attack swing once, then returns to ready stance.
#[derive(Component, Clone, Debug)]
pub struct AttackAnimState {
    pub weapon: MeleeWeaponKind,
    /// Time remaining for the swing animation.
    pub remaining: f32,
}

impl AttackAnimState {
    pub fn new(weapon: MeleeWeaponKind, swing_time: f32) -> Self {
        Self {
            weapon,
            remaining: swing_time,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.remaining <= 0.0
    }

    pub fn tick(&mut self, dt: f32) {
        self.remaining = (self.remaining - dt).max(0.0);
    }

    /// The animation ID to play.
    pub fn anim_id(&self) -> u16 {
        self.weapon.attack_anim_id()
    }
}

/// An animation override for a specific ability/spell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimOverride {
    /// The WoW animation ID to play instead of the default.
    pub anim_id: u16,
    /// Whether this override loops (like a channel) or plays once.
    pub looping: bool,
}

/// Registry mapping spell IDs to animation overrides.
#[derive(Default, Clone, Debug)]
pub struct AnimOverrideRegistry {
    entries: Vec<(u32, AnimOverride)>,
}

impl AnimOverrideRegistry {
    /// Register an override for a spell ID.
    pub fn insert(&mut self, spell_id: u32, anim: AnimOverride) {
        if let Some(entry) = self.entries.iter_mut().find(|(id, _)| *id == spell_id) {
            entry.1 = anim;
        } else {
            self.entries.push((spell_id, anim));
        }
    }

    /// Look up an override for a spell.
    pub fn get(&self, spell_id: u32) -> Option<&AnimOverride> {
        self.entries
            .iter()
            .find(|(id, _)| *id == spell_id)
            .map(|(_, anim)| anim)
    }

    /// Resolve the animation ID for a spell, falling back to the cast kind default.
    pub fn resolve(&self, spell_id: u32, default_kind: CastAnimKind) -> u16 {
        self.get(spell_id)
            .map(|o| o.anim_id)
            .unwrap_or_else(|| default_kind.cast_anim_id())
    }

    /// Resolve whether the animation should loop.
    pub fn resolve_looping(&self, spell_id: u32, default_kind: CastAnimKind) -> bool {
        self.get(spell_id)
            .map(|o| o.looping)
            .unwrap_or_else(|| default_kind.is_looping())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Whether a spell has a target (directed) or is area-effect (omni).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CastAnimKind {
    #[default]
    Directed,
    Omni,
    Channel,
}

impl CastAnimKind {
    /// The WoW animation ID for the cast wind-up.
    pub fn cast_anim_id(self) -> u16 {
        match self {
            Self::Directed => ANIM_SPELL_CAST_DIRECTED,
            Self::Omni => ANIM_SPELL_CAST_OMNI,
            Self::Channel => ANIM_CHANNEL,
        }
    }

    /// The ready/hold animation for sustained casts.
    pub fn ready_anim_id(self) -> u16 {
        match self {
            Self::Directed => ANIM_READY_SPELL_DIRECTED,
            Self::Omni => ANIM_READY_SPELL_OMNI,
            Self::Channel => ANIM_CHANNEL,
        }
    }

    /// Whether this animation should loop until cancelled.
    /// Channels loop; directed/omni casts play once then hold ready pose.
    pub fn is_looping(self) -> bool {
        matches!(self, Self::Channel)
    }
}

/// Component that triggers a cast animation on a character model.
/// Added when a spell cast starts, removed when it ends.
#[derive(Component, Clone, Debug)]
pub struct CastAnimState {
    pub kind: CastAnimKind,
    /// Remaining cast/channel time (for knowing when to transition back to Stand).
    pub remaining: f32,
    /// Whether the animation is in the "hold" phase (ready pose after cast wind-up).
    pub holding: bool,
}

impl CastAnimState {
    pub fn new(kind: CastAnimKind, duration: f32) -> Self {
        Self {
            kind,
            remaining: duration,
            holding: false,
        }
    }

    /// Create a channel animation state (loops until duration expires).
    pub fn channel(duration: f32) -> Self {
        Self {
            kind: CastAnimKind::Channel,
            remaining: duration,
            holding: false,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.remaining <= 0.0
    }

    /// Whether the M2AnimPlayer should loop the current animation.
    pub fn should_loop(&self) -> bool {
        self.kind.is_looping() && !self.is_finished()
    }

    /// The animation ID to play right now.
    pub fn current_anim_id(&self) -> u16 {
        if self.holding {
            self.kind.ready_anim_id()
        } else {
            self.kind.cast_anim_id()
        }
    }

    pub fn tick(&mut self, dt: f32) {
        self.remaining = (self.remaining - dt).max(0.0);
    }
}

/// Map movement direction to a WoW animation ID.
fn direction_to_anim_id(dir: MoveDirection, running: bool) -> u16 {
    match dir {
        MoveDirection::None => ANIM_STAND,
        MoveDirection::Forward => {
            if running {
                ANIM_RUN
            } else {
                ANIM_WALK
            }
        }
        MoveDirection::Backward => ANIM_WALK_BACKWARDS,
        MoveDirection::Left => ANIM_SHUFFLE_LEFT,
        MoveDirection::Right => ANIM_SHUFFLE_RIGHT,
    }
}

/// Find the sequence index for an animation ID, or None if the model lacks it.
fn find_seq_idx(sequences: &[M2AnimSequence], anim_id: u16) -> Option<usize> {
    sequences.iter().position(|s| s.id == anim_id)
}

const MIN_MOVEMENT_BLEND_MS: f32 = 150.0;

/// Start a crossfade transition to a new sequence.
/// If already mid-transition, blends from the current blended pose (not the raw source).
fn start_transition(player: &mut M2AnimPlayer, target_idx: usize, blend_ms: f32) {
    let blend_duration = blend_ms.max(MIN_MOVEMENT_BLEND_MS);

    // If mid-transition, keep blending from current pose by preserving from_* as-is
    // but update the blend progress proportionally so the pose doesn't jump.
    if let Some(ref existing) = player.transition {
        let progress = (existing.blend_elapsed_ms / existing.blend_duration_ms).clamp(0.0, 1.0);
        // Start the new blend from where the old blend currently is
        player.transition = Some(AnimTransition {
            from_seq_idx: player.current_seq_idx,
            from_time_ms: player.time_ms,
            blend_duration_ms: blend_duration,
            // Start partway through so the outgoing pose weight matches current blend
            blend_elapsed_ms: blend_duration * (1.0 - progress) * 0.5,
        });
    } else {
        player.transition = Some(AnimTransition {
            from_seq_idx: player.current_seq_idx,
            from_time_ms: player.time_ms,
            blend_duration_ms: blend_duration,
            blend_elapsed_ms: 0.0,
        });
    }
    player.current_seq_idx = target_idx;
    player.time_ms = 0.0;
}

fn is_jump_anim(id: u16) -> bool {
    matches!(id, ANIM_JUMP_START | ANIM_JUMP | ANIM_JUMP_END)
}

fn switch_animation(mut players: Query<(&mut M2AnimPlayer, Option<&MovementState>, &M2AnimData)>) {
    for (mut player, movement, data) in &mut players {
        let current_id = data.sequences.get(player.current_seq_idx).map(|s| s.id);
        let in_jump = current_id.is_some_and(is_jump_anim);
        let default_movement = MovementState::default();
        let movement = movement.unwrap_or(&default_movement);

        // Jump state machine: enter on jumping flag, stay until JumpEnd finishes
        if movement.jumping || in_jump {
            switch_jump(&mut player, movement, current_id, &data.sequences);
            continue;
        }

        let target_id = direction_to_anim_id(movement.direction, movement.running);
        if current_id == Some(target_id) {
            continue;
        }
        let Some(target_idx) = find_seq_idx(&data.sequences, target_id) else {
            continue;
        };
        let blend_ms = data.sequences[target_idx].blend_time as f32;
        start_transition(&mut player, target_idx, blend_ms);
    }
}

/// Blend time for jump transitions — short to avoid floaty arm interpolation.
const JUMP_BLEND_MS: f32 = 80.0;

/// Handle jump state machine: JumpStart (once) → Jump (loop) → JumpEnd (once) → done.
fn switch_jump(
    player: &mut M2AnimPlayer,
    movement: &MovementState,
    current_id: Option<u16>,
    sequences: &[M2AnimSequence],
) {
    match current_id {
        // Not yet in any jump anim → start JumpStart
        Some(id) if id != ANIM_JUMP_START && id != ANIM_JUMP && id != ANIM_JUMP_END => {
            if let Some(idx) = find_seq_idx(sequences, ANIM_JUMP_START) {
                start_transition(player, idx, JUMP_BLEND_MS);
                player.looping = false;
            }
        }
        // JumpStart finished playing → transition to airborne loop
        Some(ANIM_JUMP_START) if anim_finished(player, sequences) => {
            if let Some(idx) = find_seq_idx(sequences, ANIM_JUMP) {
                start_transition(player, idx, JUMP_BLEND_MS);
                player.looping = true;
            }
        }
        // Airborne → wait for physics to land (camera.rs controls timing via jump_elapsed)
        Some(ANIM_JUMP) if !movement.jumping => {
            if let Some(idx) = find_seq_idx(sequences, ANIM_JUMP_END) {
                start_transition(player, idx, JUMP_BLEND_MS);
                player.looping = false;
            }
        }
        // JumpEnd finished → return to movement anim with normal blend
        Some(ANIM_JUMP_END) if anim_finished(player, sequences) => {
            player.looping = true;
            let target_id = direction_to_anim_id(movement.direction, movement.running);
            if let Some(idx) = find_seq_idx(sequences, target_id) {
                let blend_ms = sequences[idx].blend_time as f32;
                start_transition(player, idx, blend_ms);
            }
        }
        _ => {}
    }
}

/// Check if the current (non-looping) animation has played through.
fn anim_finished(player: &M2AnimPlayer, sequences: &[M2AnimSequence]) -> bool {
    sequences
        .get(player.current_seq_idx)
        .is_some_and(|seq| player.time_ms >= seq.duration as f32)
}

fn valid_next_sequence_idx(sequences: &[M2AnimSequence], seq_idx: usize) -> Option<usize> {
    let next_idx = sequences.get(seq_idx)?.next_animation;
    let next_idx = usize::try_from(next_idx).ok()?;
    sequences.get(next_idx)?;
    Some(next_idx)
}

pub(crate) fn advance_player_time(player: &mut M2AnimPlayer, data: &M2AnimData, delta_ms: f32) {
    let Some(seq) = data.sequences.get(player.current_seq_idx) else {
        return;
    };
    player.time_ms += delta_ms;
    if seq.duration > 0 {
        if player.looping {
            if player.time_ms >= seq.duration as f32 {
                if let Some(next_idx) =
                    valid_next_sequence_idx(&data.sequences, player.current_seq_idx)
                        .filter(|next_idx| *next_idx != player.current_seq_idx)
                {
                    let overflow = player.time_ms - seq.duration as f32;
                    player.time_ms = seq.duration as f32;
                    let blend_ms = data.sequences[next_idx].blend_time as f32;
                    start_transition(player, next_idx, blend_ms);
                    player.time_ms = overflow;
                } else {
                    player.time_ms %= seq.duration as f32;
                }
            }
        } else {
            player.time_ms = player.time_ms.min(seq.duration as f32);
        }
    }
}

fn tick_animation(time: Res<Time>, mut players: Query<(&mut M2AnimPlayer, &M2AnimData)>) {
    let delta_ms = time.delta_secs() * 1000.0;
    for (mut player, data) in &mut players {
        advance_player_time(&mut player, data, delta_ms);

        let mut clear_transition = false;
        if let Some(ref mut tr) = player.transition {
            tr.blend_elapsed_ms += delta_ms;
            if let Some(from_seq) = data.sequences.get(tr.from_seq_idx) {
                tr.from_time_ms += delta_ms;
                if from_seq.duration > 0 {
                    // Clamp at end — wrapping would snap to frame 0 mid-blend
                    tr.from_time_ms = tr.from_time_ms.min(from_seq.duration as f32);
                }
            }
            if tr.blend_elapsed_ms >= tr.blend_duration_ms {
                clear_transition = true;
            }
        }
        if clear_transition {
            player.transition = None;
        }
    }
}

fn blended_bone_components(
    player: &M2AnimPlayer,
    data: &M2AnimData,
    bone_idx: usize,
) -> Option<(Vec3, Quat, Vec3)> {
    let tracks = data.bone_tracks.get(bone_idx)?;
    let current = evaluate_bone_components(tracks, player.current_seq_idx, player.time_ms as u32);
    Some(if let Some(ref tr) = player.transition {
        let from = evaluate_bone_components(tracks, tr.from_seq_idx, tr.from_time_ms as u32);
        let t = (tr.blend_elapsed_ms / tr.blend_duration_ms).clamp(0.0, 1.0);
        (
            from.0.lerp(current.0, t),
            from.1.slerp(current.1, t),
            from.2.lerp(current.2, t),
        )
    } else {
        current
    })
}

fn apply_animation_to_model(
    player: &M2AnimPlayer,
    data: &M2AnimData,
    bone_query: &mut Query<(&mut Transform, &BonePivot)>,
) {
    for (bone_idx, joint_entity) in data.joint_entities.iter().enumerate() {
        let Some((trans, rot, scl)) = blended_bone_components(player, data, bone_idx) else {
            continue;
        };
        let Ok((mut transform, pivot)) = bone_query.get_mut(*joint_entity) else {
            continue;
        };
        let effective_trans = trans + pivot.0 - rot * (scl * pivot.0);
        *transform = Transform {
            translation: effective_trans,
            rotation: rot,
            scale: scl,
        };
    }
}

fn apply_animation(
    players: Query<(&M2AnimPlayer, &M2AnimData)>,
    mut bone_query: Query<(&mut Transform, &BonePivot)>,
) {
    for (player, data) in &players {
        apply_animation_to_model(player, data, &mut bone_query);
    }
}

/// Evaluate animation tracks and return (translation, rotation, scale) in Bevy coordinates.
pub fn evaluate_bone_components(
    tracks: &BoneAnimTracks,
    seq_idx: usize,
    time_ms: u32,
) -> (Vec3, Quat, Vec3) {
    let trans_wow = evaluate_vec3_track(&tracks.translation, seq_idx, time_ms);
    let rot_raw = evaluate_rotation_track(&tracks.rotation, seq_idx, time_ms);
    let scale_wow = evaluate_vec3_track(&tracks.scale, seq_idx, time_ms);

    let trans = trans_wow
        .map(|t| Vec3::new(t[0], t[2], -t[1]))
        .unwrap_or(Vec3::ZERO);
    // Quaternion already in Bevy space from unpack_rotation() in m2_anim.rs
    let rot = rot_raw
        .map(|r| Quat::from_xyzw(r[0], r[1], r[2], r[3]).normalize())
        .unwrap_or(Quat::IDENTITY);
    let scl = scale_wow
        .map(|s| Vec3::new(s[0], s[2], s[1]))
        .unwrap_or(Vec3::ONE);

    (trans, rot, scl)
}

fn apply_billboard_rotation(
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    mut bones: Query<(&mut Transform, &SphericalBillboard)>,
) {
    let Some(camera_gt) = camera_query.iter().next() else {
        return;
    };
    let Some(bb_rot) = billboard_rotation_from_camera(camera_gt.rotation()) else {
        return;
    };
    for (mut transform, bb) in &mut bones {
        transform.translation = bb.pivot;
        transform.rotation = bb_rot;
    }
}

fn billboard_rotation_from_camera(camera_rotation: Quat) -> Option<Quat> {
    let view_dir = camera_rotation * -Vec3::Z;
    let right = view_dir.cross(Vec3::Y);
    if right.length_squared() < 1.0e-6 {
        return None;
    }
    let right = right.normalize();
    let toward_camera = -view_dir;
    let up = toward_camera.cross(right).normalize();
    Some(
        Quat::from_mat3(&Mat3::from_cols(right, up, toward_camera))
            * Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
    )
}

fn sync_model_lights(
    players: Query<&M2AnimPlayer>,
    mut lights: Query<(&RuntimeM2PointLight, &mut PointLight, &mut Visibility)>,
) {
    for (runtime, mut point_light, mut visibility) in &mut lights {
        let (seq_idx, time_ms) = players
            .get(runtime.anim_owner)
            .map(|player| (player.current_seq_idx, player.time_ms as u32))
            .unwrap_or((0, 0));
        let authored = m2_light::evaluate_light(&runtime.light, seq_idx, time_ms);
        point_light.color =
            Color::linear_rgb(authored.color[0], authored.color[1], authored.color[2]);
        point_light.intensity = authored.intensity;
        point_light.range = authored.attenuation_end;
        point_light.radius = authored.attenuation_start.min(authored.attenuation_end);
        *visibility = if authored.visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

pub struct AnimationPlugin;

fn animation_active_state(state: Option<Res<State<GameState>>>) -> bool {
    matches!(
        state.as_deref().map(State::get),
        Some(
            GameState::InWorld
                | GameState::InWorldSelectionDebug
                | GameState::CharSelect
                | GameState::CharCreate
                | GameState::DebugCharacter
                | GameState::SkyboxDebug
        )
    )
}

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (switch_animation, tick_animation, apply_animation)
                .chain()
                .run_if(animation_active_state),
        )
        .add_systems(Update, apply_billboard_rotation.after(apply_animation))
        .add_systems(Update, sync_model_lights.run_if(animation_active_state));
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/animation_tests.rs"]
mod tests;
