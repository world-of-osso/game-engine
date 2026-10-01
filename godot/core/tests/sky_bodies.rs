//! Stars alpha over the day (DayNightLightHolder::updatePlanetsAndStars).
use game_engine_core::sky_bodies::{SkyboxDraw, skybox_draws, stars_alpha};

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

/// LightParams 12 (Eastern Kingdoms default, no skybox), 903 (Twilight Highlands Light 2506,
/// LightSkybox 165 `twilighthighlandssky2.m2`, flags 6), 1 and 2 (two other skyboxes).
fn skybox_of(params: u32) -> Option<(u32, u32)> {
    match params {
        903 => Some((451_101, 6)),
        1 => Some((130_636, 0)),
        2 => Some((321_486, 3)),
        _ => None,
    }
}

#[test]
fn a_light_with_a_skybox_draws_it_at_its_weight() {
    assert_eq!(skybox_draws(&[(12, 1.0)], skybox_of), []);
    assert_eq!(
        skybox_draws(&[(12, 1.0), (903, 0.4)], skybox_of),
        [SkyboxDraw {
            fdid: 451_101,
            flags: 6,
            alpha: 0.4
        }]
    );
}

#[test]
fn a_later_skybox_fades_the_earlier_ones_and_a_shared_one_keeps_the_larger_weight() {
    let draws = skybox_draws(&[(1, 1.0), (2, 0.25)], skybox_of);
    assert_eq!(
        draws
            .iter()
            .map(|draw| (draw.fdid, draw.alpha))
            .collect::<Vec<_>>(),
        [(130_636, 0.75), (321_486, 0.25)]
    );
    let shared = |params| (params == 1 || params == 2).then_some((130_636, 0));
    assert_eq!(skybox_draws(&[(1, 0.3), (2, 0.6)], shared)[0].alpha, 0.6);
}
