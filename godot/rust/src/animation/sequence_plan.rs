//! Per-sequence bone sampling plans of one model's tracks, shared by every instance.
//!
//! Most bone tracks are empty or hold one keyframe in a given sequence (HumanMale HD:
//! 208 of 216 scale tracks are empty, and 48-121 bones are constant in all three). Sampling
//! still read each track's per-sequence keyframe vectors, a chain of loads that missed
//! cache for 40 crowd characters. A plan resolves such a track once to the value
//! sampling it would return, so only keyframed and global-sequence tracks are read.
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BonePlan {
    pub translation: Component<Vector3>,
    pub rotation: Component<Quaternion>,
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
                    rotation: plan(&track.rotation, index, Quaternion::IDENTITY, |value| {
                        let q = m2_anim::unpack_rotation(value);
                        Quaternion::new(q[0], q[1], q[2], q[3]).normalized()
                    }),
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
