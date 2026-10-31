//! Sequence selection and pose sampling for an external parent skeleton.
use super::{BonePose, keyframed, wow_vec3};
use game_engine_core::{asset::m2_format::m2_anim, m2};
use godot::builtin::{Quaternion, Vector3};
use std::sync::Arc;

pub(super) struct ParentBank {
    start: usize,
    source_indices: Vec<usize>,
    tracks: Arc<Vec<m2::BoneAnimTracks>>,
    globals: Vec<u32>,
    bones: Vec<m2::Bone>,
    child_bones: Vec<m2::Bone>,
}

impl ParentBank {
    pub(super) fn new(
        model: &m2::Model,
        sequences: &mut Vec<m2::Sequence>,
        animated: &mut Vec<bool>,
    ) -> Result<Option<Self>, String> {
        let Some(parent) = &model.parent_animation else {
            return Ok(None);
        };
        if parent.bones.len() != model.bones.len() || parent.bones.len() != parent.bone_tracks.len()
        {
            return Err("Parent skeleton does not provide a track for each model bone".into());
        }
        if parent
            .bones
            .iter()
            .zip(&model.bones)
            .any(|(parent, child)| parent.parent_bone_id != child.parent_bone_id)
        {
            return Err("Parent skeleton has a different hierarchy".into());
        }
        let globals = parent.global_sequences.len();
        if parent.bone_tracks.iter().any(|track| {
            [
                track.translation.global_sequence,
                track.rotation.global_sequence,
                track.scale.global_sequence,
            ]
            .iter()
            .any(|&global| usize::try_from(global).is_ok_and(|index| index >= globals))
        }) {
            return Err(format!(
                "Parent bone track names a global sequence beyond the parent's {globals}"
            ));
        }
        let start = sequences.len();
        let source_indices: Vec<usize> = parent
            .sequences
            .iter()
            .enumerate()
            .filter(|(_, sequence)| {
                !parent.blacklist.contains(&sequence.id)
                    && !model
                        .sequences
                        .iter()
                        .any(|child| child.id == sequence.id && child.variation_id == 0)
            })
            .map(|(index, _)| index)
            .collect();
        for &source in &source_indices {
            let mut sequence = parent.sequences[source].clone();
            sequence.variation_next = usize::try_from(sequence.variation_next)
                .ok()
                .and_then(|next| source_indices.iter().position(|&index| index == next))
                .and_then(|next| i16::try_from(start + next).ok())
                .unwrap_or(-1);
            sequences.push(sequence);
            animated.push(parent.bone_tracks.iter().any(|track| {
                keyframed(&track.translation, source)
                    || keyframed(&track.rotation, source)
                    || keyframed(&track.scale, source)
            }));
        }
        Ok(Some(Self {
            start,
            source_indices,
            tracks: parent.bone_tracks.clone(),
            globals: parent.global_sequences.clone(),
            bones: parent.bones.clone(),
            child_bones: model.bones.clone(),
        }))
    }

    pub(super) fn contains(&self, index: usize) -> bool {
        index >= self.start && index < self.start + self.source_indices.len()
    }

    pub(super) fn sample(&self, index: usize, time_ms: f64, global_ms: f64) -> Vec<BonePose> {
        let source = self.source_indices[index - self.start];
        let track_time = |track: &m2_anim::AnimTrack<[f32; 3]>| {
            m2::global_track_time(track, &self.globals, global_ms)
                .unwrap_or((source, time_ms as u32))
        };
        self.bones
            .iter()
            .zip(self.tracks.iter())
            .enumerate()
            .map(|(index, (bone, track))| {
                let pivot = wow_vec3(bone.pivot);
                let parent_index = usize::try_from(bone.parent_bone_id).ok();
                let parent_pivot = parent_index
                    .map(|parent| wow_vec3(self.bones[parent].pivot))
                    .unwrap_or(Vector3::ZERO);
                let parent_offset = parent_index
                    .map(|parent| {
                        wow_vec3(self.child_bones[parent].pivot)
                            - wow_vec3(self.bones[parent].pivot)
                    })
                    .unwrap_or(Vector3::ZERO);
                let (timeline, time) = track_time(&track.translation);
                let translation = m2_anim::evaluate_vec3_track(&track.translation, timeline, time)
                    .map(wow_vec3)
                    .unwrap_or(Vector3::ZERO);
                let (timeline, time) =
                    m2::global_track_time(&track.rotation, &self.globals, global_ms)
                        .unwrap_or((source, time_ms as u32));
                let rotation = m2_anim::evaluate_rotation_track(&track.rotation, timeline, time)
                    .map(|q| Quaternion::new(q[0], q[1], q[2], q[3]).normalized())
                    .unwrap_or(Quaternion::IDENTITY);
                let (timeline, time) = track_time(&track.scale);
                let scale = m2_anim::evaluate_vec3_track(&track.scale, timeline, time)
                    .map(|s| Vector3::new(s[0], s[2], s[1]))
                    .unwrap_or(Vector3::ONE);
                let bind_offset = wow_vec3(self.child_bones[index].pivot) - pivot;
                let rotated_offset =
                    super::Basis::from_quaternion(rotation).scaled(scale) * bind_offset;
                BonePose {
                    position: pivot - parent_pivot + translation - parent_offset + rotated_offset,
                    rotation,
                    scale,
                }
            })
            .collect()
    }
}
