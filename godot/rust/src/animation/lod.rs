//! Distance and visibility based sampling rate for replicated NPC animation, as the
//! original client's `src/rendering/model/animation/lod.rs` ([npc-animation-lod]
//! spec). The animation clock always advances; this only decides on which frames
//! the bone pose is sampled and written. Skipped frames keep the last pose.
//!
//! [npc-animation-lod]: ../../../../docs/specs/npc-animation-lod.md

/// NPCs closer than this to the camera sample every frame.
const FULL_RATE_MAX_YARDS: f32 = 30.0;
/// NPCs farther than this from the camera stop sampling even when on screen.
const FROZEN_MIN_YARDS: f32 = 60.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnimationLod {
    /// Sample every frame.
    Full,
    /// Sample every other frame, staggered by unit so half the NPCs sample each frame.
    Half,
    /// Do not sample; bones keep their last pose.
    Frozen,
}

impl AnimationLod {
    /// `visible`: whether the model was on screen last frame.
    pub fn new(camera_distance: f32, visible: bool) -> Self {
        if !visible || camera_distance > FROZEN_MIN_YARDS {
            Self::Frozen
        } else if camera_distance > FULL_RATE_MAX_YARDS {
            Self::Half
        } else {
            Self::Full
        }
    }

    pub fn samples_frame(self, frame: u64, owner: u64) -> bool {
        match self {
            Self::Full => true,
            Self::Half => frame.wrapping_add(owner) % 2 == 0,
            Self::Frozen => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AnimationLod::{self, Frozen, Full, Half};

    #[test]
    fn thresholds_follow_the_original_client() {
        for (distance, visible, expected) in [
            (0.0, true, Full),
            (30.0, true, Full),
            (30.01, true, Half),
            (60.0, true, Half),
            (60.01, true, Frozen),
            (5.0, false, Frozen),
            (45.0, false, Frozen),
        ] {
            assert_eq!(
                AnimationLod::new(distance, visible),
                expected,
                "{distance} {visible}"
            );
        }
    }

    #[test]
    fn half_rate_alternates_frames_and_staggers_units() {
        let sampled = |owner| {
            (0..6)
                .filter(|frame| Half.samples_frame(*frame, owner))
                .count()
        };
        assert_eq!(sampled(7), 3);
        assert_ne!(Half.samples_frame(10, 7), Half.samples_frame(11, 7));
        assert_ne!(Half.samples_frame(10, 7), Half.samples_frame(10, 8));
        assert!((0..4).all(|frame| Full.samples_frame(frame, 3)));
        assert!((0..4).all(|frame| !Frozen.samples_frame(frame, 3)));
    }
}
