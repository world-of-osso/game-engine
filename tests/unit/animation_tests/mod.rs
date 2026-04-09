use super::*;
use crate::animation::billboard::compute_bone_stages;
use crate::asset::m2_anim::AnimTrack;

mod billboard;
mod cast_attack;
mod core;
mod emote;
mod movement;

pub(super) fn single_key_vec3_track(value: [f32; 3]) -> AnimTrack<[f32; 3]> {
    AnimTrack {
        interpolation_type: 0,
        global_sequence: -1,
        sequences: vec![(vec![0], vec![value])],
    }
}

pub(super) fn single_key_rot_track() -> AnimTrack<[i16; 4]> {
    AnimTrack {
        interpolation_type: 0,
        global_sequence: -1,
        sequences: vec![(vec![0], vec![[0, 0, 0, i16::MAX]])],
    }
}

pub(super) fn stationary_bone(translation: [f32; 3]) -> BoneAnimTracks {
    BoneAnimTracks {
        translation: single_key_vec3_track(translation),
        rotation: single_key_rot_track(),
        scale: single_key_vec3_track([1.0, 1.0, 1.0]),
    }
}

pub(super) fn stand_sequence() -> M2AnimSequence {
    M2AnimSequence {
        id: 0,
        variation_id: 0,
        duration: 1000,
        movespeed: 0.0,
        flags: 0,
        blend_time: 0,
        next_animation: -1,
    }
}

pub(super) fn sequence(anim_id: u16, duration: u32) -> M2AnimSequence {
    M2AnimSequence {
        id: anim_id,
        variation_id: 0,
        duration,
        movespeed: 0.0,
        flags: 0,
        blend_time: 150,
        next_animation: -1,
    }
}

pub(super) fn stand_sequence_with_next(
    duration: u32,
    variation_id: u16,
    next_animation: i16,
) -> M2AnimSequence {
    M2AnimSequence {
        id: 0,
        variation_id,
        duration,
        movespeed: 0.0,
        flags: 0,
        blend_time: 150,
        next_animation,
    }
}

pub(super) fn single_root_bone() -> Vec<M2Bone> {
    vec![M2Bone {
        key_bone_id: -1,
        flags: 0,
        parent_bone_id: -1,
        submesh_id: 0,
        pivot: [0.0, 0.0, 0.0],
    }]
}
