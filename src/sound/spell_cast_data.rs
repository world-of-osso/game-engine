use std::f32::consts::TAU;

pub const CAST_VOLUME_SCALE: f32 = 0.75;
pub const CAST_SAMPLE_RATE: u32 = 44_100;

/// Observe only confirmed active spell identity; zero and absence both reset the tracker.
pub fn observe_active_spell(last: &mut Option<u32>, spell_id: Option<u32>) -> Option<u32> {
    let active = spell_id.filter(|id| *id != 0);
    if active == *last {
        return None;
    }
    *last = active;
    active
}

/// Legacy 140 ms cast sweep; phase is normalized sample position, not elapsed seconds.
pub fn generate_spell_cast_samples() -> Vec<i16> {
    generate_spell_sweep_samples(140.0, 380.0, 140, 0.22)
}

/// Shared by cast and heal, with the original normalized-phase arithmetic unchanged.
pub fn generate_spell_sweep_samples(
    start_hz: f32,
    end_hz: f32,
    duration_ms: u32,
    amplitude: f32,
) -> Vec<i16> {
    let sample_rate = 44_100.0_f32;
    let sample_count = (sample_rate * duration_ms as f32 / 1000.0) as usize;
    let mut samples = Vec::with_capacity(sample_count);
    for i in 0..sample_count {
        let t = i as f32 / sample_count as f32;
        let hz = start_hz + (end_hz - start_hz) * t;
        let envelope = (1.0 - t).powf(2.0);
        let wave = ((t * hz * TAU).sin() + (t * hz * TAU * 0.5).sin() * 0.35) * envelope;
        samples.push((wave * amplitude * 32_000.0).clamp(-32_767.0, 32_767.0) as i16);
    }
    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observes_only_new_nonzero_spell_ids_and_resets_on_inactive() {
        let mut last = None;
        assert_eq!(observe_active_spell(&mut last, None), None);
        assert_eq!(observe_active_spell(&mut last, Some(0)), None);
        assert_eq!(observe_active_spell(&mut last, Some(133)), Some(133));
        assert_eq!(observe_active_spell(&mut last, Some(133)), None);
        assert_eq!(observe_active_spell(&mut last, Some(42)), Some(42));
        assert_eq!(observe_active_spell(&mut last, Some(0)), None);
        assert_eq!(observe_active_spell(&mut last, Some(42)), Some(42));
        assert_eq!(observe_active_spell(&mut last, None), None);
        assert_eq!(observe_active_spell(&mut last, Some(42)), Some(42));
    }

    #[test]
    fn cast_samples_match_legacy_normalized_sweep() {
        let samples = generate_spell_cast_samples();
        assert_eq!(samples.len(), 6174);
        assert_eq!(samples[0], 0);
        assert_eq!(&samples[1..6], &[1174, 2328, 3440, 4489, 5458]);
        assert_eq!(samples[6173], 0);
        let checksum = samples
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .fold(0xcbf29ce484222325_u64, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
            });
        assert_eq!(checksum, 1_932_001_403_226_046_311);
        assert_eq!(CAST_VOLUME_SCALE, 0.75);
    }
}
