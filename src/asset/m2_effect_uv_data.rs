//! Shared authored M2 effect texture-translation sampling.

use crate::asset::m2_format::m2_anim::{AnimTrack, evaluate_vec3_track};

/// Sample both effect texture offsets in authored X/Y coordinates.
/// Local tracks use elapsed time; global tracks wrap their authored duration.
pub fn sample_effect_uv_offsets(
    track_1: Option<&AnimTrack<[f32; 3]>>,
    track_2: Option<&AnimTrack<[f32; 3]>>,
    global_sequences: &[u32],
    elapsed_ms: u32,
) -> ([f32; 2], [f32; 2]) {
    let sample = |track: Option<&AnimTrack<[f32; 3]>>| {
        track
            .and_then(|track| sample_effect_texture_track(track, global_sequences, elapsed_ms))
            .map(|offset| [offset[0], offset[1]])
            .unwrap_or([0.0, 0.0])
    };
    (sample(track_1), sample(track_2))
}

fn sample_effect_texture_track(
    track: &AnimTrack<[f32; 3]>,
    global_sequences: &[u32],
    elapsed_ms: u32,
) -> Option<[f32; 3]> {
    let time_ms = if let Ok(index) = usize::try_from(track.global_sequence) {
        let duration = *global_sequences
            .get(index)
            .expect("M2 texture animation references a missing global sequence");
        if duration == 0 {
            0
        } else {
            elapsed_ms % duration
        }
    } else {
        elapsed_ms
    };
    evaluate_vec3_track(track, 0, time_ms)
}

#[cfg(test)]
mod tests {
    use super::sample_effect_uv_offsets;
    use crate::asset::m2_format::m2_anim::AnimTrack;

    fn track(global_sequence: i16) -> AnimTrack<[f32; 3]> {
        AnimTrack {
            interpolation_type: 1,
            global_sequence,
            sequences: vec![
                (vec![0, 1000], vec![[0.0, 2.0, 99.0], [1.0, 4.0, 88.0]]),
                (vec![0, 1000], vec![[10.0, 20.0, 0.0], [30.0, 40.0, 0.0]]),
            ],
        }
    }

    #[test]
    fn local_tracks_use_unwrapped_elapsed_time_and_sequence_zero() {
        assert_eq!(
            sample_effect_uv_offsets(Some(&track(-1)), None, &[], 1500),
            ([1.0, 4.0], [0.0, 0.0]),
        );
    }

    #[test]
    fn global_tracks_wrap_at_the_authored_duration_and_interpolate() {
        assert_eq!(
            sample_effect_uv_offsets(Some(&track(0)), Some(&track(1)), &[1000, 800], 1250),
            ([0.25, 2.5], [0.45, 2.9]),
        );
    }

    #[test]
    fn zero_duration_samples_at_time_zero() {
        assert_eq!(
            sample_effect_uv_offsets(Some(&track(0)), None, &[0], 750),
            ([0.0, 2.0], [0.0, 0.0]),
        );
    }

    #[test]
    fn absent_or_empty_tracks_yield_zero_offsets() {
        let empty = AnimTrack {
            interpolation_type: 1,
            global_sequence: -1,
            sequences: vec![],
        };
        assert_eq!(
            sample_effect_uv_offsets(Some(&empty), None, &[], 500),
            ([0.0, 0.0], [0.0, 0.0]),
        );
    }

    #[test]
    #[should_panic(expected = "M2 texture animation references a missing global sequence")]
    fn invalid_global_index_fails_explicitly() {
        sample_effect_uv_offsets(Some(&track(2)), None, &[1000], 500);
    }
}
