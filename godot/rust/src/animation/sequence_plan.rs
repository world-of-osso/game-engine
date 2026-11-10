//! Per-sequence bone sampling plans of one model's tracks, shared by every instance.
//!
//! Most bone tracks are empty or hold one keyframe in a given sequence (HumanMale HD:
//! 208 of 216 scale tracks are empty, and 48-121 bones are constant in all three). Sampling
//! still read each track's per-sequence keyframe vectors, a chain of loads that missed
//! cache for 40 crowd characters. A plan resolves such a track once to the value
//! sampling it would return, so only keyframed and global-sequence tracks are read.
//! Keyframed rotations keep their keys unpacked: unpacking each packed component took a
//! sign branch and a divide, eight times for every interpolated rotation.
use super::wow_vec3;
use game_engine_core::{
    asset::m2_format::m2_anim,
    m2::{AnimTrack, BoneAnimTracks},
};
use godot::builtin::{Quaternion, Vector3};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, Weak};

/// One bone component in one sequence: the value every sample returns, or keyframes
/// that sampling interpolates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Component<T> {
    Constant(T),
    Sampled,
}

/// A rotation in one sequence: constant, keyframes on the sequence's own timeline held
/// unpacked, or a global-sequence track sampled from the model.
#[derive(Debug, PartialEq)]
pub(super) enum RotationPlan {
    Constant(Quaternion),
    Keyed(KeyedRotation),
    Sampled,
}

#[derive(Debug, PartialEq)]
pub(super) struct KeyedRotation {
    timestamps: Box<[u32]>,
    values: Box<[[f32; 4]]>,
}

#[derive(Debug, PartialEq)]
pub(super) struct BonePlan {
    pub translation: Component<Vector3>,
    pub rotation: RotationPlan,
    pub scale: Component<Vector3>,
}

/// Lazily built plans for each of a model's own sequences.
pub(super) struct SequencePlans {
    tracks: Arc<Vec<BoneAnimTracks>>,
    sequences: Vec<OnceLock<Vec<BonePlan>>>,
}

type Registry = HashMap<usize, (Weak<Vec<BoneAnimTracks>>, Weak<SequencePlans>)>;

impl SequencePlans {
    /// The plans of `tracks`, shared with every other instance of the same model.
    pub fn shared(tracks: &Arc<Vec<BoneAnimTracks>>, sequence_count: usize) -> Arc<Self> {
        static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
        let mut registry = REGISTRY
            .get_or_init(Default::default)
            .lock()
            .expect("sequence plan registry");
        let key = Arc::as_ptr(tracks) as usize;
        if let Some((known, plans)) = registry.get(&key)
            && known
                .upgrade()
                .is_some_and(|known| Arc::ptr_eq(&known, tracks))
            && let Some(plans) = plans.upgrade()
            && plans.sequences.len() == sequence_count
        {
            return plans;
        }
        registry.retain(|_, (known, plans)| known.strong_count() > 0 && plans.strong_count() > 0);
        let plans = Arc::new(Self {
            tracks: tracks.clone(),
            sequences: (0..sequence_count).map(|_| OnceLock::new()).collect(),
        });
        registry.insert(key, (Arc::downgrade(tracks), Arc::downgrade(&plans)));
        plans
    }

    #[cfg(test)]
    pub fn sequence_count(&self) -> usize {
        self.sequences.len()
    }

    /// The bone plans of sequence `index`, or `None` beyond the model's own sequences.
    pub fn sequence(&self, index: usize) -> Option<&[BonePlan]> {
        let plans = self.sequences.get(index)?;
        Some(plans.get_or_init(|| {
            self.tracks
                .iter()
                .map(|track| BonePlan {
                    translation: plan(&track.translation, index, Vector3::ZERO, |&value| {
                        wow_vec3(value)
                    }),
                    rotation: rotation_plan(&track.rotation, index),
                    scale: plan(&track.scale, index, Vector3::ONE, |s| {
                        Vector3::new(s[0], s[2], s[1])
                    }),
                })
                .collect()
        }))
    }
}

/// What sampling `track` in sequence `index` returns at any time, when that does not
/// depend on time: `absent` without keyframes, the single keyframe's value mapped by
/// `map` (the m2_anim evaluators return it unchanged at every time). A global-sequence
/// track samples its own timeline, so it is never planned.
fn plan<T, U: Copy>(
    track: &AnimTrack<T>,
    index: usize,
    absent: U,
    map: impl Fn(&T) -> U,
) -> Component<U> {
    if track.global_sequence >= 0 {
        return Component::Sampled;
    }
    let Some((timestamps, values)) = track.sequences.get(index) else {
        return Component::Constant(absent);
    };
    match (timestamps.len(), values.first()) {
        (0, _) | (_, None) => Component::Constant(absent),
        (1, Some(value)) => Component::Constant(map(value)),
        _ => Component::Sampled,
    }
}

fn rotation_plan(track: &AnimTrack<[i16; 4]>, index: usize) -> RotationPlan {
    let constant = plan(track, index, Quaternion::IDENTITY, |value| {
        let q = m2_anim::unpack_rotation(value);
        Quaternion::new(q[0], q[1], q[2], q[3]).normalized()
    });
    match constant {
        Component::Constant(rotation) => RotationPlan::Constant(rotation),
        Component::Sampled if track.global_sequence >= 0 => RotationPlan::Sampled,
        Component::Sampled => {
            let (timestamps, values) = &track.sequences[index];
            RotationPlan::Keyed(KeyedRotation {
                timestamps: timestamps.as_slice().into(),
                values: values.iter().map(m2_anim::unpack_rotation).collect(),
            })
        }
    }
}

impl KeyedRotation {
    /// `m2_anim::evaluate_rotation_track` at `time_ms` on these keys, normalized as the
    /// sampler does; keyframe search, slerp and their float operations are copied from
    /// wow_m2 0.1 m2_anim (`sequence_plan_tests` checks them bit for bit).
    pub fn sample(&self, time_ms: u32) -> Quaternion {
        let (i, t) = find_keyframe_pair(&self.timestamps, time_ms);
        let q0 = self.values[i];
        let q = if t == 0.0 || i + 1 >= self.values.len() {
            q0
        } else {
            slerp(&q0, &self.values[i + 1], t)
        };
        Quaternion::new(q[0], q[1], q[2], q[3]).normalized()
    }
}

/// The index of the last timestamp <= `time_ms` and the fraction to the next one, for a
/// track with at least two keys.
fn find_keyframe_pair(timestamps: &[u32], time_ms: u32) -> (usize, f32) {
    if time_ms <= timestamps[0] {
        return (0, 0.0);
    }
    if time_ms >= timestamps[timestamps.len() - 1] {
        return (timestamps.len() - 1, 0.0);
    }
    let i = timestamps
        .partition_point(|&t| t <= time_ms)
        .saturating_sub(1);
    let (t0, t1) = (timestamps[i], timestamps[i + 1]);
    let t = if t1 > t0 {
        (time_ms - t0) as f32 / (t1 - t0) as f32
    } else {
        0.0
    };
    (i, t)
}

fn slerp(a: &[f32; 4], b: &[f32; 4], t: f32) -> [f32; 4] {
    let mut dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
    let mut b = *b;
    if dot < 0.0 {
        b = [-b[0], -b[1], -b[2], -b[3]];
        dot = -dot;
    }
    if dot > 0.9995 {
        let q = [
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
            a[3] + (b[3] - a[3]) * t,
        ];
        let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
        return [q[0] / len, q[1] / len, q[2] / len, q[3] / len];
    }
    let theta = dot.acos();
    let sin_theta = theta.sin();
    let s0 = ((1.0 - t) * theta).sin() / sin_theta;
    let s1 = (t * theta).sin() / sin_theta;
    [
        a[0] * s0 + b[0] * s1,
        a[1] * s0 + b[1] * s1,
        a[2] * s0 + b[2] * s1,
        a[3] * s0 + b[3] * s1,
    ]
}
