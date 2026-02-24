use bevy::prelude::*;
use super::asset::m2_anim::{
    BoneAnimTracks, M2AnimSequence,
    evaluate_vec3_track, evaluate_rotation_track,
};

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

/// Animation player component attached to the model entity.
#[derive(Component)]
pub struct M2AnimPlayer {
    pub current_seq_idx: usize,
    pub time_ms: f32,
    pub looping: bool,
}

fn tick_animation(
    time: Res<Time>,
    anim_data: Option<Res<M2AnimData>>,
    mut players: Query<&mut M2AnimPlayer>,
) {
    let Some(data) = anim_data else { return };
    for mut player in &mut players {
        let Some(seq) = data.sequences.get(player.current_seq_idx) else { continue };
        player.time_ms += time.delta_secs() * 1000.0;
        if player.looping && seq.duration > 0 {
            player.time_ms %= seq.duration as f32;
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

    let seq_idx = player.current_seq_idx;
    let time_ms = player.time_ms as u32;

    for (bone_idx, joint_entity) in data.joint_entities.iter().enumerate() {
        let Some(tracks) = data.bone_tracks.get(bone_idx) else { continue };
        let Ok((mut transform, pivot)) = bone_query.get_mut(*joint_entity) else { continue };

        evaluate_bone(tracks, seq_idx, time_ms, pivot.0, &mut transform);
    }
}

/// Evaluate animation tracks and set bone Transform.
fn evaluate_bone(
    tracks: &BoneAnimTracks,
    seq_idx: usize,
    time_ms: u32,
    pivot: Vec3,
    transform: &mut Transform,
) {
    let trans_wow = evaluate_vec3_track(&tracks.translation, seq_idx, time_ms);
    let rot_raw = evaluate_rotation_track(&tracks.rotation, seq_idx, time_ms);
    let scale_wow = evaluate_vec3_track(&tracks.scale, seq_idx, time_ms);

    // Convert WoW → Bevy coordinates
    let trans = trans_wow
        .map(|t| Vec3::new(t[0], t[2], -t[1]))
        .unwrap_or(Vec3::ZERO);
    let rot = rot_raw
        .map(|r| Quat::from_xyzw(r[0], r[1], r[2], r[3]).normalize())
        .unwrap_or(Quat::IDENTITY);
    let scl = scale_wow
        .map(|s| Vec3::new(s[0], s[2], s[1]))
        .unwrap_or(Vec3::ONE);

    // local = translate(pivot) * SRT * translate(-pivot)
    // Effective: translation = trans + pivot - rot * (scl * pivot)
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
        app.add_systems(Update, (tick_animation, apply_animation).chain());
    }
}
