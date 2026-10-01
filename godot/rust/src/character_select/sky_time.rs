//! Fixed authored-sky bone phase; preserve original f32 conversion before remainder.
pub(super) fn fixed_sequence_phase_ms(time_ms: u32, duration_ms: u32) -> f64 {
    if duration_ms > 0 {
        f64::from(time_ms as f32 % duration_ms as f32)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::fixed_sequence_phase_ms;

    #[test]
    fn positive_duration_preserves_original_wrapped_phase() {
        assert_eq!(fixed_sequence_phase_ms(2500, 1000), 500.0);
        assert_eq!(fixed_sequence_phase_ms(1000, 1000), 0.0);
    }

    #[test]
    fn wide_u32_preserves_original_f32_rounding_before_remainder() {
        // u32::MAX rounds to 4294967296 as f32, then wraps to 296, not 295.
        assert_eq!(fixed_sequence_phase_ms(u32::MAX, 1000), 296.0);
    }

    #[test]
    fn zero_duration_retains_requested_original_phase() {
        assert_eq!(fixed_sequence_phase_ms(1234, 0), 1234.0);
        assert_eq!(fixed_sequence_phase_ms(u32::MAX, 0), 4294967296.0);
    }
}
