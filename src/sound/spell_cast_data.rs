use std::f32::consts::TAU;

pub const CAST_VOLUME_SCALE: f32 = 0.75;
pub const CAST_SAMPLE_RATE: u32 = 44_100;
pub const IMPACT_VOLUME_SCALE: f32 = 1.0;
pub const HEAL_VOLUME_SCALE: f32 = 0.85;
pub const MISS_VOLUME_SCALE: f32 = 0.55;
pub const INTERRUPT_VOLUME_SCALE: f32 = 0.95;

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

pub fn generate_spell_heal_samples() -> Vec<i16> {
    generate_spell_sweep_samples(260.0, 520.0, 180, 0.18)
}

pub fn generate_spell_miss_samples() -> Vec<i16> {
    let sample_rate = 44_100.0_f32;
    let duration_ms = 110;
    let sample_count = (sample_rate * duration_ms as f32 / 1000.0) as usize;
    let mut samples = Vec::with_capacity(sample_count);
    for i in 0..sample_count {
        let t = i as f32 / sample_count as f32;
        let envelope = (1.0 - t).powf(2.6);
        let wave = ((t * TAU * 12.0).sin() * 0.4 + (t * TAU * 27.0).sin() * 0.1) * envelope;
        samples.push((wave * 11_000.0).clamp(-32_767.0, 32_767.0) as i16);
    }
    samples
}

pub fn generate_spell_interrupt_samples() -> Vec<i16> {
    let sample_rate = 44_100.0_f32;
    let duration_ms = 150;
    let sample_count = (sample_rate * duration_ms as f32 / 1000.0) as usize;
    let mut samples = Vec::with_capacity(sample_count);
    for i in 0..sample_count {
        let t = i as f32 / sample_count as f32;
        let envelope = (1.0 - t).powf(1.8);
        let wave = ((90.0 * t * TAU).sin() * 0.6 + (180.0 * t * TAU).sin() * 0.25) * envelope;
        samples.push((wave * 16_000.0).clamp(-32_767.0, 32_767.0) as i16);
    }
    samples
}

pub fn generate_spell_impact_samples() -> Vec<i16> {
    let sample_rate = 44_100.0_f32;
    let duration_ms = 120;
    let sample_count = (sample_rate * duration_ms as f32 / 1000.0) as usize;
    let mut samples = Vec::with_capacity(sample_count);
    let mut rng_state: u32 = 7;
    for i in 0..sample_count {
        rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
        let noise = ((rng_state >> 16) as i32 - 32768) as f32 / 32768.0;
        let t = i as f32 / sample_count as f32;
        let envelope = (1.0 - t).powf(3.5);
        let tone = (t * TAU * 110.0).sin() * 0.45;
        let sample = ((tone + noise * 0.55) * envelope * 18_000.0).clamp(-32_767.0, 32_767.0);
        samples.push(sample as i16);
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
    fn outcome_samples_and_gains_keep_legacy_shapes() {
        assert_eq!(generate_spell_impact_samples().len(), 5292);
        assert_eq!(generate_spell_heal_samples().len(), 7938);
        assert_eq!(generate_spell_miss_samples().len(), 4851);
        assert_eq!(generate_spell_interrupt_samples().len(), 6615);
        for (samples, first) in [
            (generate_spell_impact_samples(), -5172),
            (generate_spell_heal_samples(), 0),
            (generate_spell_miss_samples(), 0),
            (generate_spell_interrupt_samples(), 0),
        ] {
            assert_eq!(samples[0], first);
            assert!(samples.iter().any(|sample| *sample != 0));
        }
        assert_eq!(IMPACT_VOLUME_SCALE, 1.0);
        assert_eq!(HEAL_VOLUME_SCALE, 0.85);
        assert_eq!(MISS_VOLUME_SCALE, 0.55);
        assert_eq!(INTERRUPT_VOLUME_SCALE, 0.95);
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
