use bevy::prelude::*;
use super::asset::m2_anim::{
    BoneAnimTracks, M2AnimSequence,
    evaluate_vec3_track, evaluate_rotation_track,
};
use super::camera::MovementState;

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

const ANIM_ID_STAND: u16 = 0;
const ANIM_ID_WALK: u16 = 4;

fn switch_animation(
    anim_data: Option<Res<M2AnimData>>,
    mut players: Query<(&mut M2AnimPlayer, &MovementState)>,
) {
    let Some(data) = anim_data else { return };
    for (mut player, movement) in &mut players {
        let current_id = data.sequences.get(player.current_seq_idx).map(|s| s.id);
        let target_id = if movement.moving { ANIM_ID_WALK } else { ANIM_ID_STAND };

        if current_id == Some(target_id) {
            continue;
        }

        let Some(target_idx) = data.sequences.iter().position(|s| s.id == target_id) else {
            continue;
        };

        let blend_time = data.sequences[target_idx].blend_time as f32;
        let blend_duration = if blend_time > 0.0 { blend_time } else { 150.0 };

        player.transition = Some(AnimTransition {
            from_seq_idx: player.current_seq_idx,
            from_time_ms: player.time_ms,
            blend_duration_ms: blend_duration,
            blend_elapsed_ms: 0.0,
        });
        player.current_seq_idx = target_idx;
        player.time_ms = 0.0;
    }
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
        if player.looping && seq.duration > 0 {
            player.time_ms %= seq.duration as f32;
        }

        let mut clear_transition = false;
        if let Some(ref mut tr) = player.transition {
            tr.blend_elapsed_ms += delta_ms;
            if let Some(from_seq) = data.sequences.get(tr.from_seq_idx) {
                tr.from_time_ms += delta_ms;
                if from_seq.duration > 0 {
                    tr.from_time_ms %= from_seq.duration as f32;
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

        let (trans, rot, scl) = if let Some(ref tr) = player.transition {
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
fn evaluate_bone_components(
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
