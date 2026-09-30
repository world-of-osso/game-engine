//! Retail world-space floating combat text over the target (docs/specs/spellbook-action-bar.md).
//!
//! The engine CVars (wowless `data/products/wow/cvars.yaml`, retail 12.0.7.68275) drive it:
//! `floatingCombatTextFloatMode_v2 = 1` scrolls up (`COMBAT_TEXT_SCROLL_UP`,
//! Blizzard_SettingsDefinitions_Frame/Mainline/CombatOverrides.lua:55-58);
//! `WorldTextStartPosRandomness_v2 = 1.0` (:1626) and `WorldTextRandomZMin_v2 = 0.8` /
//! `WorldTextRandomZMax_v2 = 1.5` (:1622-1623) randomise where each number starts, so
//! simultaneous numbers do not stack; `WorldTextRampDuration_v2 = 1.0` with
//! `WorldTextRampPow_v2 = 1.9` / `WorldTextRampPowCrit_v2 = 8.0` (:1618-1620) shape the size
//! ramp, a crit popping larger and settling faster. No source has the engine formula;
//! the mapping of each CVar here is read from its name.

use godot::prelude::*;

/// `WorldTextStartPosRandomness_v2`: yards of sideways start spread either side.
const START_POS_RANDOMNESS: f32 = 1.0;
/// `WorldTextRandomZMin_v2` / `WorldTextRandomZMax_v2`, yards.
const RANDOM_Z_MIN: f32 = 0.8;
const RANDOM_Z_MAX: f32 = 1.5;
/// `WorldTextRampDuration_v2`, seconds.
const RAMP_SECS: f32 = 1.0;
/// `WorldTextRampPow_v2` / `WorldTextRampPowCrit_v2`.
const RAMP_POW: f32 = 1.9;
const RAMP_POW_CRIT: f32 = 8.0;
/// Extra size at spawn, decaying over the ramp.
const RAMP_POP: f32 = 0.5;

/// R2 low-discrepancy sequence (plastic number): consecutive spawns land far apart in
/// both axes, so numbers spawned together never share a start.
const R2: [f32; 2] = [0.754_877_7, 0.569_840_3];

/// Where the `index`th number starts relative to the anchor over the target's head:
/// sideways along the camera's right, and up by the random Z range.
pub(crate) fn start_offset(index: u32, camera_right: Vector3) -> Vector3 {
    let [u, v] = R2.map(|alpha| (0.5 + alpha * index as f32).fract());
    let side = camera_right.normalized() * (START_POS_RANDOMNESS * (2.0 * u - 1.0));
    side + Vector3::UP * ((RANDOM_Z_MAX - RANDOM_Z_MIN) * v)
}

/// Size multiplier `age` seconds after spawn: pops larger, then ramps down to 1.
pub(crate) fn ramp_scale(age: f32, crit: bool) -> f32 {
    let remaining = (1.0 - age / RAMP_SECS).clamp(0.0, 1.0);
    let pow = if crit { RAMP_POW_CRIT } else { RAMP_POW };
    1.0 + RAMP_POP * remaining.powf(pow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_spawned_together_start_apart() {
        // Showcase: a Frostbolt hit and a crit landed on the same frame, drawn on top of
        // each other. Any four numbers in a row start at least half a yard apart.
        let right = Vector3::new(0.6, 0.0, -0.8);
        for first in 0..1000 {
            let starts: Vec<_> = (first..first + 4)
                .map(|index| start_offset(index, right))
                .collect();
            for (i, a) in starts.iter().enumerate() {
                for b in &starts[i + 1..] {
                    assert!(a.distance_to(*b) >= 0.5, "{first}: {a} vs {b}");
                }
            }
        }
    }

    #[test]
    fn starts_stay_in_the_camera_plane_within_the_random_ranges() {
        let right = Vector3::new(0.0, 0.0, 2.0);
        for index in 0..200 {
            let offset = start_offset(index, right);
            assert_eq!(offset.x, 0.0);
            assert!(offset.z.abs() <= START_POS_RANDOMNESS);
            assert!((0.0..=RANDOM_Z_MAX - RANDOM_Z_MIN).contains(&offset.y));
        }
    }

    #[test]
    fn crits_pop_and_settle_faster_than_hits() {
        assert_eq!(ramp_scale(0.0, true), 1.0 + RAMP_POP);
        assert_eq!(ramp_scale(0.0, false), 1.0 + RAMP_POP);
        assert!(ramp_scale(0.3, true) < ramp_scale(0.3, false));
        assert!(ramp_scale(0.3, true) < 1.03);
        assert_eq!(ramp_scale(RAMP_SECS, false), 1.0);
        assert_eq!(ramp_scale(2.0, true), 1.0);
    }
}
