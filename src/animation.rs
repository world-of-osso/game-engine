pub mod billboard;

use super::asset::m2_anim::{
    BoneAnimTracks, M2AnimSequence, M2Bone, evaluate_rotation_track, evaluate_vec3_track,
};
use super::camera::{MoveDirection, MovementState};
use super::game_state::GameState;
use bevy::prelude::*;

pub use billboard::{compute_bone_stages, propagate_spherical_billboards};

/// Marker component for bone entities, storing their local pivot relative to the parent bone.
#[derive(Component)]
pub struct BonePivot(pub Vec3);

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
    camera_rotation: Option<Quat>,
    bone_query: &mut Query<&mut Transform>,
) {
    let mut pre_billboard_stage = vec![Mat4::IDENTITY; data.joint_entities.len()];
    let mut post_billboard_stage = vec![Mat4::IDENTITY; data.joint_entities.len()];
    let billboard_world_rotation = camera_rotation.and_then(billboard_world_rotation_from_camera);

    for (bone_idx, joint_entity) in data.joint_entities.iter().enumerate() {
        apply_animation_to_bone(
            player,
            data,
            bone_idx,
            *joint_entity,
            billboard_world_rotation,
            &mut pre_billboard_stage,
            &mut post_billboard_stage,
            bone_query,
        );
    }
}

fn billboard_world_rotation_from_camera(camera_rotation: Quat) -> Option<Quat> {
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

#[allow(clippy::too_many_arguments)]
fn apply_animation_to_bone(
    player: &M2AnimPlayer,
    data: &M2AnimData,
    bone_idx: usize,
    joint_entity: Entity,
    billboard_world_rotation: Option<Quat>,
    pre_billboard_stage: &mut [Mat4],
    post_billboard_stage: &mut [Mat4],
    bone_query: &mut Query<&mut Transform>,
) {
    let Some((trans, rot, scl)) = blended_bone_components(player, data, bone_idx) else {
        return;
    };
    let Some((parent_idx, pre_stage, post_stage)) = compute_bone_stages(
        &data.bones,
        &data.spherical_billboards,
        bone_idx,
        trans,
        rot,
        scl,
        billboard_world_rotation,
        pre_billboard_stage,
        post_billboard_stage,
    ) else {
        return;
    };
    pre_billboard_stage[bone_idx] = pre_stage;
    post_billboard_stage[bone_idx] = post_stage;

    let Ok(mut transform) = bone_query.get_mut(joint_entity) else {
        return;
    };
    let local = if let Some(parent_idx) = parent_idx {
        post_billboard_stage[parent_idx].inverse() * post_stage
    } else {
        post_stage
    };
    let (scale, rotation, translation) = local.to_scale_rotation_translation();
    *transform = Transform {
        translation,
        rotation,
        scale,
    };
}

pub(crate) fn apply_animation(
    players: Query<(&M2AnimPlayer, &M2AnimData)>,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    mut bone_query: Query<&mut Transform>,
) {
    let camera_rotation = camera_query.iter().next().map(GlobalTransform::rotation);
    for (player, data) in &players {
        apply_animation_to_model(player, data, camera_rotation, &mut bone_query);
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
                | GameState::ParticleDebug
        )
    )
}

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (switch_animation, tick_animation)
                .chain()
                .run_if(animation_active_state),
        )
        .add_systems(
            Update,
            apply_animation
                .after(tick_animation)
                .after(crate::orbit_camera::orbit_camera_system)
                .run_if(animation_active_state),
        );
    }
}

#[cfg(test)]
#[path = "animation_tests.rs"]
mod tests;
