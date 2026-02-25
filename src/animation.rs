use bevy::prelude::*;
use super::asset::m2_anim::{
    BoneAnimTracks, M2AnimSequence,
    evaluate_vec3_track, evaluate_rotation_track,
};
use super::camera::{MoveDirection, MovementState};

/// Marker component for bone entities, storing their pivot in Bevy coordinates.
#[derive(Component)]
pub struct BonePivot(pub Vec3);

/// All animation data for the loaded M2 model.
#[derive(Resource)]
pub struct M2AnimData {
    pub sequences: Vec<M2AnimSequence>,
    pub bone_tracks: Vec<BoneAnimTracks>,
    pub global_sequences: Vec<u32>,
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
const ANIM_SHUFFLE_LEFT: u16 = 11;
const ANIM_SHUFFLE_RIGHT: u16 = 12;
const ANIM_WALK_BACKWARDS: u16 = 13;
const ANIM_JUMP_START: u16 = 37;
const ANIM_JUMP: u16 = 38;  // airborne loop
const ANIM_JUMP_END: u16 = 39;

/// Map movement direction to a WoW animation ID.
fn direction_to_anim_id(dir: MoveDirection) -> u16 {
    match dir {
        MoveDirection::None => ANIM_STAND,
        MoveDirection::Forward => ANIM_WALK,
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

fn switch_animation(
    anim_data: Option<Res<M2AnimData>>,
    mut players: Query<(&mut M2AnimPlayer, &mut MovementState)>,
) {
    let Some(data) = anim_data else { return };
    for (mut player, mut movement) in &mut players {
        let current_id = data.sequences.get(player.current_seq_idx).map(|s| s.id);
        let in_jump = current_id.is_some_and(is_jump_anim);

        // Jump state machine: enter on jumping flag, stay until JumpEnd finishes
        if movement.jumping || in_jump {
            switch_jump(&mut player, &mut movement, current_id, &data.sequences);
            continue;
        }

        let target_id = direction_to_anim_id(movement.direction);
        if current_id == Some(target_id) {
            continue;
        }
        let Some(target_idx) = find_seq_idx(&data.sequences, target_id) else { continue };
        let blend_ms = data.sequences[target_idx].blend_time as f32;
        start_transition(&mut player, target_idx, blend_ms);
    }
}

/// Blend time for jump transitions — short to avoid floaty arm interpolation.
const JUMP_BLEND_MS: f32 = 80.0;

/// Handle jump state machine: JumpStart (once) → Jump (loop) → JumpEnd (once) → done.
fn switch_jump(
    player: &mut M2AnimPlayer,
    movement: &mut MovementState,
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
            let target_id = direction_to_anim_id(movement.direction);
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

fn tick_animation(
    time: Res<Time>,
    anim_data: Option<Res<M2AnimData>>,
    mut players: Query<&mut M2AnimPlayer>,
) {
    let Some(data) = anim_data else { return };
    let delta_ms = time.delta_secs() * 1000.0;
    for mut player in &mut players {
        let Some(seq) = data.sequences.get(player.current_seq_idx) else { continue };
        player.time_ms += delta_ms;
        if seq.duration > 0 {
            if player.looping {
                player.time_ms %= seq.duration as f32;
            } else {
                player.time_ms = player.time_ms.min(seq.duration as f32);
            }
        }

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

fn apply_animation(
    anim_data: Option<Res<M2AnimData>>,
    players: Query<&M2AnimPlayer>,
    mut bone_query: Query<(&mut Transform, &BonePivot)>,
) {
    let Some(data) = anim_data else { return };
    let Some(player) = players.iter().next() else { return };

    for (bone_idx, joint_entity) in data.joint_entities.iter().enumerate() {
        let Some(tracks) = data.bone_tracks.get(bone_idx) else { continue };
        let Ok((mut transform, pivot)) = bone_query.get_mut(*joint_entity) else { continue };
        let current = evaluate_bone_components(tracks, player.current_seq_idx, player.time_ms as u32);

        let (trans, mut rot, scl) = if let Some(ref tr) = player.transition {
            let from = evaluate_bone_components(tracks, tr.from_seq_idx, tr.from_time_ms as u32);
            let t = (tr.blend_elapsed_ms / tr.blend_duration_ms).clamp(0.0, 1.0);
            (
                from.0.lerp(current.0, t),
                from.1.slerp(current.1, t),
                from.2.lerp(current.2, t),
            )
        } else {
            current
        };

        apply_bone_transform(pivot.0, trans, rot, scl, &mut transform);
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

/// Apply SRT with pivot offset to a bone Transform.
fn apply_bone_transform(pivot: Vec3, trans: Vec3, rot: Quat, scl: Vec3, transform: &mut Transform) {
    // local = translate(pivot) * SRT * translate(-pivot)
    let effective_trans = trans + pivot - rot * (scl * pivot);
    *transform = Transform {
        translation: effective_trans,
        rotation: rot,
        scale: scl,
    };
}

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (switch_animation, tick_animation, apply_animation).chain());
    }
}
