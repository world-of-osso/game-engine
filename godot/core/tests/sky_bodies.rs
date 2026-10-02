//! Stars alpha over the day (DayNightLightHolder::updatePlanetsAndStars).
use game_engine_core::sky_bodies::{
    PLANET_FDIDS, PlanetDraw, SkyboxDraw, planet_draws, skybox_draws, stars_alpha,
};

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

fn assert_direction(actual: [f32; 3], expected: [f32; 3]) {
    for axis in 0..3 {
        assert!(
            (actual[axis] - expected[axis]).abs() < 1e-3,
            "{actual:?} vs {expected:?}"
        );
    }
}

#[test]
fn noon_sun_stands_near_the_zenith_at_unit_scale() {
    let planets = planet_draws(1440.0, 0.0, 0.0, 0.0);
    let fdids: Vec<_> = planets.iter().map(|planet| planet.fdid).collect();
    assert_eq!(fdids, PLANET_FDIDS);
    // sunPhiTable at day 0.5: phi 0.0873, theta 0.7854 -> WoW (0.0617, 0.0617, 0.9962).
    assert_direction(planets[0].direction, [0.0617, 0.9962, -0.0617]);
    assert_eq!(planets[0].scale, 1.0);
    // The moons sit 100 degrees from the zenith (below the horizon) at moonScale 1.5.
    assert!(planets[1].direction[1] < -0.17);
    assert!((planets[1].scale - 1.5 * 2.2).abs() < 1e-5);
    assert!((planets[2].scale - 1.5 * 1.2).abs() < 1e-5);
}

#[test]
fn midnight_moons_rise_on_either_side_and_the_sun_is_below() {
    let planets = planet_draws(0.0, 0.0, 0.0, 0.0);
    // moonPhiTable 0.6109 with moonTheta 0.7854 and moon2Theta 2.3562.
    assert_direction(planets[1].direction, [0.4056, 0.8192, -0.4056]);
    assert_direction(planets[2].direction, [-0.4056, 0.8192, -0.4056]);
    assert!((planets[1].scale - 2.2).abs() < 1e-5);
    assert!((planets[2].scale - 1.2).abs() < 1e-5);
    assert!(planets[0].direction[1] < -0.17);
}

#[test]
fn light_params_flags_hide_the_sun_the_moons_or_every_planet() {
    let fdids = |planets: Vec<PlanetDraw>| planets.iter().map(|p| p.fdid).collect::<Vec<_>>();
    // 0x4 hides the sun, 0x8 both moons; a blend under one half still shows them.
    assert_eq!(fdids(planet_draws(0.0, 1.0, 0.0, 0.0)), PLANET_FDIDS[1..]);
    assert_eq!(fdids(planet_draws(0.0, 0.0, 1.0, 0.0)), PLANET_FDIDS[..1]);
    assert_eq!(fdids(planet_draws(0.0, 0.4, 0.4, 0.0)), PLANET_FDIDS);
    // 0x100 (custom sun position) sets every planet's alpha to zero.
    assert!(planet_draws(0.0, 0.0, 0.0, 1.0).is_empty());
}
