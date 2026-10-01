//! Stars alpha over the day (DayNightLightHolder::updatePlanetsAndStars).
use game_engine_core::sky_bodies::stars_alpha;

#[test]
fn stars_shine_at_midnight_and_vanish_by_day() {
    // Midnight: brightness 1, byte 255.
    assert_eq!(stars_alpha(0.0, 0.0), Some(1.0));
    // Noon and 06:00: brightness 0, byte 1, not drawn.
    assert_eq!(stars_alpha(1440.0, 0.0), None);
    assert_eq!(stars_alpha(720.0, 0.0), None);
}

#[test]
fn stars_ramp_in_after_dusk_and_hide_by_light_params_flag() {
    // 20:45 (2490 half-minutes) is halfway up the 20:00-21:30 ramp: byte 128.
    let alpha = stars_alpha(2490.0, 0.0).unwrap();
    assert!((alpha - 128.0 / 255.0).abs() < 1e-3, "{alpha}");
    // A Light with LightParams flag 0x10 at full weight hides them.
    assert_eq!(stars_alpha(0.0, 1.0), None);
    assert_eq!(stars_alpha(0.0, 0.5), Some(0.5));
}
