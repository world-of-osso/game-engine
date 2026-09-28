//! Distance and visibility based sampling rate for replicated NPC and doodad
//! animation, as the original client's `src/rendering/model/animation/lod.rs`
//! ([npc-animation-lod] spec). The animation clock always advances; this only
//! decides on which frames the bone pose is sampled and written. Skipped frames
//! keep the last pose.
//!
//! [npc-animation-lod]: ../../../../docs/specs/npc-animation-lod.md

/// Models closer than this to the camera sample every frame.
const FULL_RATE_MAX_YARDS: f32 = 30.0;
/// Models farther than this from the camera stop sampling even when on screen.
const FROZEN_MIN_YARDS: f32 = 60.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnimationLod {
    /// Sample every frame.
    Full,
    /// Sample every other frame, staggered by owner so half the models sample each frame.
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

/// Animation time owed to a model that is advanced only on the frames it samples
/// (doodads, which have no per-node process): when it samples again it advances by
/// everything it missed, so it resumes at the clock time it would have reached
/// advancing every frame.
#[derive(Debug, Default)]
pub(crate) struct DeferredClock {
    owed_ms: f64,
}

impl DeferredClock {
    /// Accrues `delta_ms`; on a `sampled` frame returns all time owed, split into
    /// advance steps. One advance may cross at most 4,096 variation boundaries
    /// (`VariationFamily::validate_elapsed`), which a doodad frozen for an hour can
    /// exceed, so the owed time is replayed in steps of at most a minute.
    pub fn tick(&mut self, delta_ms: f64, sampled: bool) -> Option<impl Iterator<Item = f64>> {
        self.owed_ms += delta_ms;
        let owed = std::mem::take(&mut self.owed_ms);
        if !sampled {
            self.owed_ms = owed;
            return None;
        }
        let steps = (owed / MAX_ADVANCE_STEP_MS).ceil().max(1.0) as u64;
        Some((0..steps).map(move |step| {
            let done = step as f64 * MAX_ADVANCE_STEP_MS;
            (owed - done).min(MAX_ADVANCE_STEP_MS)
        }))
    }
}

/// Longest single advance replaying owed time: 4,096 boundaries of a 15 ms variation.
const MAX_ADVANCE_STEP_MS: f64 = 60_000.0;

#[cfg(test)]
mod tests {
    use super::AnimationLod::{self, Frozen, Full, Half};
    use super::DeferredClock;

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

    #[test]
    fn deferred_clock_resumes_at_the_every_frame_time() {
        let mut clock = DeferredClock::default();
        let mut advanced = 0.0;
        let schedule = [(Full, 4), (Half, 6), (Frozen, 30), (Half, 3), (Full, 2)];
        let mut frame = 0u64;
        for (lod, frames) in schedule {
            for _ in 0..frames {
                let delta = 16.0 + (frame % 3) as f64;
                let sampled = lod.samples_frame(frame, 5);
                let owed = clock.tick(delta, sampled).map(Iterator::sum::<f64>);
                assert_eq!(owed.is_some(), sampled, "frame {frame} {lod:?}");
                advanced += owed.unwrap_or(0.0);
                frame += 1;
            }
        }
        // Frames 0..45 took 45 * 16 + (0 + 1 + 2) * 15 ms; the last frame sampled.
        assert_eq!(advanced, 45.0 * 16.0 + 45.0);
        // Nothing sampled while frozen, and resuming paid the whole frozen stretch.
        let mut frozen = DeferredClock::default();
        assert!((0..30).all(|_| frozen.tick(20.0, false).is_none()));
        let steps = |clock: &mut DeferredClock, delta| {
            clock.tick(delta, true).map(Iterator::collect::<Vec<_>>)
        };
        assert_eq!(steps(&mut frozen, 20.0), Some(vec![620.0]));
        assert_eq!(steps(&mut frozen, 20.0), Some(vec![20.0]));
    }

    #[test]
    fn long_frozen_time_is_replayed_in_bounded_steps() {
        let mut clock = DeferredClock::default();
        // Two and a half hours off screen at 60 FPS.
        for _ in 0..(150 * 60 * 60) {
            assert!(clock.tick(1000.0 / 60.0, false).is_none());
        }
        let steps: Vec<f64> = clock.tick(1000.0 / 60.0, true).unwrap().collect();
        assert_eq!(steps.len(), 151);
        assert!(steps.iter().all(|step| *step > 0.0 && *step <= 60_000.0));
        let total: f64 = steps.iter().sum();
        assert!((total - 9_000_016.667).abs() < 0.01, "{total}");
    }
}
