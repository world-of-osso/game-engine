#[path = "animation/billboard.rs"]
pub mod billboard;
#[path = "animation/runtime.rs"]
mod runtime;

use super::camera::{MoveDirection, MovementState};
use crate::asset::m2_anim::{
    BoneAnimTracks, M2AnimSequence, M2Bone, evaluate_rotation_track, evaluate_vec3_track,
};
use crate::asset::m2_light;
use crate::game_state::GameState;
use bevy::prelude::*;
use shared::protocol::EmoteKind;

use super::m2_spawn::RuntimeM2PointLight;
pub use billboard::propagate_spherical_billboards;
#[cfg(test)]
pub(crate) use runtime::{
    advance_player_time, direction_to_anim_id, switch_jump, turn_in_place_direction,
};
use runtime::{apply_emote_animation, switch_animation, sync_turn_in_place_state, tick_animation};

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
const ANIM_SWIM_IDLE: u16 = 41;
const ANIM_SWIM: u16 = 42;
const ANIM_SWIM_LEFT: u16 = 43;
const ANIM_SWIM_RIGHT: u16 = 44;
const ANIM_SWIM_BACKWARDS: u16 = 45;
const ANIM_JUMP_START: u16 = 37;
const ANIM_JUMP: u16 = 38; // airborne loop
const ANIM_JUMP_END: u16 = 39;
const ANIM_JUMP_LAND_RUN: u16 = 187;
const ANIM_SPELL_CAST_DIRECTED: u16 = 51;
const ANIM_SPELL_CAST_OMNI: u16 = 52;
const ANIM_READY_SPELL_DIRECTED: u16 = 55;
const ANIM_READY_SPELL_OMNI: u16 = 56;
const ANIM_CHANNEL: u16 = 76;
const ANIM_WAVE: u16 = 67;
const ANIM_DANCE: u16 = 69;
const ANIM_KNEEL: u16 = 75;
const ANIM_SIT_GROUND: u16 = 97;
const ANIM_SLEEP: u16 = 100;
const ANIM_ATTACK_1H: u16 = 46;
const ANIM_ATTACK_2H: u16 = 48;
const ANIM_ATTACK_OFF: u16 = 47;
const ANIM_PARRY_1H: u16 = 49;
const ANIM_PARRY_2H: u16 = 50;
const ANIM_READY_1H: u16 = 53;
const ANIM_READY_2H: u16 = 54;
const TURN_IN_PLACE_THRESHOLD: f32 = 0.02;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurnDirection {
    Left,
    Right,
}

#[derive(Component, Clone, Debug)]
pub struct TurnInPlaceState {
    pub last_yaw: f32,
    pub direction: Option<TurnDirection>,
}

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

/// Reason a cast/attack animation was cancelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimCancelReason {
    /// Player started moving — return to walk/run.
    Movement,
    /// Spell was interrupted (counterspell, stun, etc.) — return to idle.
    Interrupt,
    /// Cast completed naturally — return to idle.
    Complete,
}

/// Crossfade blend time in milliseconds for animation cancels.
const CANCEL_BLEND_MS_FAST: f32 = 100.0;
const CANCEL_BLEND_MS_NORMAL: f32 = 200.0;

/// Determines the target animation and blend time when cancelling a cast/attack.
pub fn cancel_anim_params(
    reason: AnimCancelReason,
    is_moving: bool,
    is_running: bool,
) -> (u16, f32) {
    let target_anim = match reason {
        AnimCancelReason::Movement => {
            if is_running {
                ANIM_RUN
            } else {
                ANIM_WALK
            }
        }
        AnimCancelReason::Interrupt => ANIM_STAND,
        AnimCancelReason::Complete => {
            if is_moving {
                if is_running { ANIM_RUN } else { ANIM_WALK }
            } else {
                ANIM_STAND
            }
        }
    };
    let blend_ms = match reason {
        AnimCancelReason::Interrupt => CANCEL_BLEND_MS_FAST,
        _ => CANCEL_BLEND_MS_NORMAL,
    };
    (target_anim, blend_ms)
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

/// Component that triggers a social emote animation on a model.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct EmoteAnimState {
    pub kind: EmoteKind,
    started: bool,
}

impl EmoteAnimState {
    pub fn new(kind: EmoteKind) -> Self {
        Self {
            kind,
            started: false,
        }
    }

    pub fn anim_id(&self) -> u16 {
        emote_anim_id(self.kind)
    }

    pub fn loops_until_interrupted(&self) -> bool {
        looping_emote(self.kind)
    }
}

fn emote_anim_id(kind: EmoteKind) -> u16 {
    match kind {
        EmoteKind::Dance => ANIM_DANCE,
        EmoteKind::Wave => ANIM_WAVE,
        EmoteKind::Sit => ANIM_SIT_GROUND,
        EmoteKind::Sleep => ANIM_SLEEP,
        EmoteKind::Kneel => ANIM_KNEEL,
    }
}

fn looping_emote(kind: EmoteKind) -> bool {
    matches!(kind, EmoteKind::Sit | EmoteKind::Sleep | EmoteKind::Kneel)
}

fn movement_interrupts_looping_emote(movement: &MovementState) -> bool {
    movement.direction != MoveDirection::None || movement.jumping || movement.swimming
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
                | GameState::M2Debug
                | GameState::SkyboxDebug
        )
    )
}

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                sync_turn_in_place_state,
                apply_emote_animation,
                switch_animation,
                tick_animation,
                apply_animation,
            )
                .chain()
                .run_if(animation_active_state),
        )
        .add_systems(Update, apply_billboard_rotation.after(apply_animation))
        .add_systems(Update, sync_model_lights.run_if(animation_active_state));
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/animation_tests/mod.rs"]
mod tests;
