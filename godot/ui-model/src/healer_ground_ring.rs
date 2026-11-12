//! Healer ground rings: the per-party-member health arc, its dispel ring, scope rules and
//! the screen-edge chevron (PLAN USER 11-05).
//!
//! Health is shown as arc length, the primary cue: a full ring at full health whose gap
//! grows clockwise from 12 o'clock as health drops. A colour ramp is the secondary cue
//! (colour-blind-safe by default). The renderer draws the ring on the visible ground
//! surface and keeps this module free of Godot types, so the rules are unit-testable.

use game_engine_core::client_options_data::{HealerGroundRingOptions, MAX_RAID_LOWEST};
use game_engine_core::dispel_data::{DispelSchool, DispelSchools};

use std::f32::consts::TAU;

/// Rings stay hidden at or above this health fraction and fade in over the band below it.
pub const HIDE_ABOVE_HEALTH: f32 = 0.92;
/// Health width of the fade-in band; full opacity at `HIDE_ABOVE_HEALTH - FADE_IN_BAND`.
pub const FADE_IN_BAND: f32 = 0.02;

/// Ring radii, in yards, from the member's center.
pub const HEALTH_INNER_RADIUS: f32 = 0.82;
pub const HEALTH_OUTER_RADIUS: f32 = 1.12;
/// The pulsing dispel ring sits just outside the health ring.
pub const DISPEL_INNER_RADIUS: f32 = 1.19;
pub const DISPEL_OUTER_RADIUS: f32 = 1.33;
/// Dispel-ring pulse, cycles per second.
pub const DISPEL_PULSE_HZ: f32 = 1.3;
/// Angular stations the draped mesh samples; one station is 360/48 = 7.5 degrees.
/// The arc ends interpolate between stations, so the health step stays smooth.
pub const ARC_STATIONS: usize = 48;
/// Lift above the sampled visible surface, in yards, to keep the ring off z-fighting.
pub const RING_LIFT: f32 = 0.02;
/// Screen-edge chevron inset from the viewport edge, in pixels.
pub const CHEVRON_MARGIN: f32 = 24.0;

/// Which members show a ring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RingScope {
    Off,
    All,
    LowestN(usize),
}

/// `HealerGroundRingOptions` plus the group and specialization facts decide the scope.
pub fn ring_scope(
    options: &HealerGroundRingOptions,
    spec_is_healer: bool,
    in_party: bool,
    in_raid: bool,
) -> RingScope {
    if !options.enabled {
        return RingScope::Off;
    }
    if in_raid {
        let lowest = usize::from(options.raid_lowest.min(MAX_RAID_LOWEST));
        return if lowest == 0 {
            RingScope::Off
        } else {
            RingScope::LowestN(lowest)
        };
    }
    if in_party && (spec_is_healer || options.all_roles) {
        RingScope::All
    } else {
        RingScope::Off
    }
}

/// One group member as the ring rules see it.
pub struct RingMember<'a> {
    pub name: &'a str,
    /// 0..1 current health.
    pub health_fraction: f32,
    /// `SpellDispelType` ids of the member's harmful visible auras.
    pub debuff_types: &'a [u8],
    pub dead: bool,
    pub offline: bool,
}

/// The ring for one member.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RingDraw {
    pub index: usize,
    /// Fade-in alpha, 0 above the threshold.
    pub alpha: f32,
    /// Visible arc start, radians clockwise from 12 o'clock.
    pub arc_start: f32,
    /// Visible arc end, always [`TAU`] for a drawn ring.
    pub arc_end: f32,
    /// Secondary health-ramp colour.
    pub color: [f32; 3],
    /// The school of the highest-priority dispellable debuff the player can remove.
    pub dispel: Option<DispelSchool>,
}

/// Whether the member has a non-empty health ring and which members it is.
pub fn ring_draws(
    members: &[RingMember],
    scope: RingScope,
    schools: DispelSchools,
    colorblind_ramp: bool,
) -> Vec<RingDraw> {
    let selected = selected_indexes(members, scope);
    selected
        .into_iter()
        .filter_map(|index| {
            let member = &members[index];
            if member.dead || member.offline {
                return None;
            }
            let alpha = ring_alpha(member.health_fraction);
            let dispel = dispellable_school(member.debuff_types, schools);
            // A dispellable debuff is actionable at any health, so its outer ring does
            // not wait for the health arc to fade in.
            if alpha <= 0.0 && dispel.is_none() {
                return None;
            }
            let (arc_start, arc_end) = arc_span(member.health_fraction);
            Some(RingDraw {
                index,
                alpha,
                arc_start,
                arc_end,
                color: health_color(member.health_fraction, colorblind_ramp),
                dispel,
            })
        })
        .collect()
}

/// The surviving member indexes in roster order, limited to the lowest N when in a raid.
fn selected_indexes(members: &[RingMember], scope: RingScope) -> Vec<usize> {
    let alive: Vec<usize> = (0..members.len())
        .filter(|&index| !members[index].dead && !members[index].offline)
        .collect();
    match scope {
        RingScope::Off => Vec::new(),
        RingScope::All => alive,
        RingScope::LowestN(count) => {
            let mut ranked = alive;
            ranked.sort_by(|&left, &right| {
                members[left]
                    .health_fraction
                    .total_cmp(&members[right].health_fraction)
                    .then(left.cmp(&right))
            });
            ranked.truncate(count);
            ranked.sort_unstable();
            ranked
        }
    }
}

/// Fade-in alpha: hidden at or above [`HIDE_ABOVE_HEALTH`].
pub fn ring_alpha(health_fraction: f32) -> f32 {
    ((HIDE_ABOVE_HEALTH - health_fraction) / FADE_IN_BAND).clamp(0.0, 1.0)
}

/// The visible arc as `(start, end)` radians clockwise from 12 o'clock. The empty gap
/// grows clockwise from 12 o'clock, so the remaining arc keeps its end at 12 and its
/// start advances clockwise as health drops.
pub fn arc_span(health_fraction: f32) -> (f32, f32) {
    let health = health_fraction.clamp(0.0, 1.0);
    (TAU * (1.0 - health), TAU)
}

/// The secondary health ramp. `colorblind_ramp` keeps hue and lightness separable for
/// deuteranopia (blue at full health to orange at empty) instead of red/green.
pub fn health_color(health_fraction: f32, colorblind_ramp: bool) -> [f32; 3] {
    let health = health_fraction.clamp(0.0, 1.0);
    if colorblind_ramp {
        // Orange -> amber -> blue.
        if health <= 0.5 {
            lerp_rgb([0.95, 0.42, 0.05], [0.98, 0.85, 0.20], health / 0.5)
        } else {
            lerp_rgb([0.98, 0.85, 0.20], [0.16, 0.55, 1.0], (health - 0.5) / 0.5)
        }
    } else {
        // Red -> yellow -> green.
        if health <= 0.5 {
            lerp_rgb([0.86, 0.12, 0.08], [0.96, 0.82, 0.12], health / 0.5)
        } else {
            lerp_rgb([0.96, 0.82, 0.12], [0.16, 0.82, 0.24], (health - 0.5) / 0.5)
        }
    }
}

fn lerp_rgb(from: [f32; 3], to: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        from[0] + (to[0] - from[0]) * t,
        from[1] + (to[1] - from[1]) * t,
        from[2] + (to[2] - from[2]) * t,
    ]
}

/// The highest-priority dispellable school on the member, in [`DispelSchool::ALL`] order.
pub fn dispellable_school(debuff_types: &[u8], schools: DispelSchools) -> Option<DispelSchool> {
    DispelSchool::ALL
        .into_iter()
        .find(|school| schools.contains(*school) && debuff_types.contains(&school.id()))
}

/// Pulse multiplier for the outer dispel ring, `0.55..1.0`.
pub fn dispel_pulse(elapsed_secs: f32) -> f32 {
    0.775 + 0.225 * (elapsed_secs * DISPEL_PULSE_HZ * TAU).sin()
}

/// A screen-edge chevron for one off-screen ally.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChevronPlacement {
    /// Viewport pixels.
    pub position: [f32; 2],
    /// Screen-space radians, 0 pointing right, positive turning down.
    pub angle: f32,
}

/// Places a chevron inside `viewport` at the edge nearest an ally whose clip-space
/// position is `clip` (x right, y up, both -1..1). `behind` marks an ally behind the
/// camera, whose projection is mirrored. Returns `None` when the ally is on screen.
pub fn chevron_placement(
    clip: [f32; 2],
    behind: bool,
    viewport: [f32; 2],
    margin: f32,
) -> Option<ChevronPlacement> {
    let center = [viewport[0] * 0.5, viewport[1] * 0.5];
    let half = [center[0] - margin, center[1] - margin];
    if half[0] <= 0.0 || half[1] <= 0.0 {
        return None;
    }
    let mut direction = [clip[0] * half[0], -clip[1] * half[1]];
    if behind {
        direction = [-direction[0], -direction[1]];
    }
    if direction[0] == 0.0 && direction[1] == 0.0 {
        return None;
    }
    let inside = direction[0].abs() <= half[0] && direction[1].abs() <= half[1] && !behind;
    if inside {
        return None;
    }
    let scale_x = if direction[0] == 0.0 {
        f32::INFINITY
    } else {
        half[0] / direction[0].abs()
    };
    let scale_y = if direction[1] == 0.0 {
        f32::INFINITY
    } else {
        half[1] / direction[1].abs()
    };
    let scale = scale_x.min(scale_y);
    let position = [
        center[0] + direction[0] * scale,
        center[1] + direction[1] * scale,
    ];
    Some(ChevronPlacement {
        position,
        angle: direction[1].atan2(direction[0]),
    })
}

/// The nearest drawn ring the ground ray `origin + dir * t` crosses. `rings` hold
/// `(center, inner, outer, arc_start)`: the point must lie in the annulus and, clockwise
/// from 12 o'clock, at or past the arc's start so the empty gap is not clickable.
pub fn ring_ray_hit(
    origin: [f32; 3],
    dir: [f32; 3],
    rings: &[([f32; 3], f32, f32, f32)],
) -> Option<(usize, f32)> {
    if dir[1].abs() < 1e-6 {
        return None;
    }
    let mut best: Option<(usize, f32)> = None;
    for (index, (center, inner, outer, arc_start)) in rings.iter().enumerate() {
        let plane = center[1] + RING_LIFT;
        let t = (plane - origin[1]) / dir[1];
        if t <= 0.0 {
            continue;
        }
        let x = origin[0] + dir[0] * t - center[0];
        let z = origin[2] + dir[2] * t - center[2];
        let distance = (x * x + z * z).sqrt();
        if distance < *inner || distance > *outer {
            continue;
        }
        let angle = x.atan2(-z).rem_euclid(TAU);
        if angle + 1e-4 < *arc_start {
            continue;
        }
        if best.is_none_or(|(_, nearest)| t < nearest) {
            best = Some((index, t));
        }
    }
    best
}

/// Radial offset `(x, z)` of an angle clockwise from 12 o'clock.
pub fn station_offset(angle: f32, radius: f32) -> (f32, f32) {
    (angle.sin() * radius, -angle.cos() * radius)
}

/// The angle 12 o'clock is at, for renderer documentation and tests.
pub const UP_ANGLE: f32 = 0.0;

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine_core::client_options_data::HealerGroundRingOptions;
    use game_engine_core::dispel_data::{
        DISPEL_CURSE, DISPEL_DISEASE, DISPEL_MAGIC, DISPEL_POISON,
    };
    use std::f32::consts::PI;

    fn member<'a>(name: &'a str, health: f32) -> RingMember<'a> {
        RingMember {
            name,
            health_fraction: health,
            debuff_types: &[],
            dead: false,
            offline: false,
        }
    }

    #[test]
    fn full_health_is_a_full_ring_and_the_gap_grows_clockwise() {
        assert_eq!(arc_span(1.0), (0.0, TAU));
        let (start, end) = arc_span(0.75);
        assert!((start - TAU * 0.25).abs() < 1e-5);
        assert_eq!(end, TAU);
        let (start, end) = arc_span(0.0);
        assert!((start - TAU).abs() < 1e-5);
        assert_eq!(end, TAU);
        // Out-of-range health is clamped, not wrapped.
        assert_eq!(arc_span(-1.0), arc_span(0.0));
        assert_eq!(arc_span(2.0), arc_span(1.0));
    }

    #[test]
    fn rings_hide_above_ninety_two_percent_and_fade_in_below() {
        assert_eq!(ring_alpha(1.0), 0.0);
        assert_eq!(ring_alpha(0.93), 0.0);
        assert_eq!(ring_alpha(HIDE_ABOVE_HEALTH), 0.0);
        assert!((ring_alpha(0.91) - 0.5).abs() < 1e-6);
        assert_eq!(ring_alpha(0.90), 1.0);
        assert_eq!(ring_alpha(0.25), 1.0);
    }

    #[test]
    fn colour_ramps_reach_their_ends_and_differ_between_palettes() {
        let near = |color: [f32; 3], expected: [f32; 3]| {
            color
                .iter()
                .zip(expected.iter())
                .all(|(left, right)| (left - right).abs() < 1e-6)
        };
        assert!(near(health_color(0.0, false), [0.86, 0.12, 0.08]));
        assert!(near(health_color(1.0, false), [0.16, 0.82, 0.24]));
        assert!(near(health_color(0.0, true), [0.95, 0.42, 0.05]));
        assert!(near(health_color(1.0, true), [0.16, 0.55, 1.0]));
        assert_ne!(health_color(0.5, true), health_color(0.5, false));
    }

    #[test]
    fn scope_defaults_on_for_a_healer_party_and_off_for_other_roles() {
        let options = HealerGroundRingOptions::default();
        assert_eq!(ring_scope(&options, true, true, false), RingScope::All);
        assert_eq!(ring_scope(&options, false, true, false), RingScope::Off);
        assert_eq!(ring_scope(&options, true, false, false), RingScope::Off);
        assert_eq!(ring_scope(&options, true, false, true), RingScope::Off);

        let mut opted_in = options;
        opted_in.all_roles = true;
        assert_eq!(ring_scope(&opted_in, false, true, false), RingScope::All);
        // Raids need the lowest-N mode even when opted in.
        assert_eq!(ring_scope(&opted_in, true, true, true), RingScope::Off);

        let mut raid = options;
        raid.raid_lowest = 3;
        assert_eq!(ring_scope(&raid, false, true, true), RingScope::LowestN(3));

        let disabled = HealerGroundRingOptions {
            enabled: false,
            ..options
        };
        assert_eq!(ring_scope(&disabled, true, true, false), RingScope::Off);
        assert_eq!(ring_scope(&disabled, true, true, true), RingScope::Off);
    }

    #[test]
    fn draws_skip_dead_offline_and_healthy_members() {
        let debuffs = [DISPEL_MAGIC];
        let members = [
            RingMember {
                name: "healthy",
                health_fraction: 1.0,
                debuff_types: &debuffs,
                dead: false,
                offline: false,
            },
            member("hurt", 0.4),
            RingMember {
                name: "dead",
                health_fraction: 0.0,
                debuff_types: &[],
                dead: true,
                offline: false,
            },
            RingMember {
                name: "offline",
                health_fraction: 0.2,
                debuff_types: &[],
                dead: false,
                offline: true,
            },
        ];
        let draws = ring_draws(&members, RingScope::All, DispelSchools::default(), true);
        assert_eq!(draws.len(), 1);
        assert_eq!(draws[0].index, 1);
        assert_eq!(draws[0].alpha, 1.0);
        assert!(draws[0].color[0] > draws[0].color[2]);
    }

    #[test]
    fn lowest_n_picks_the_lowest_health_members_in_roster_order() {
        let members = [
            member("a", 0.8),
            member("b", 0.2),
            member("c", 0.5),
            member("d", 0.1),
        ];
        let draws = ring_draws(
            &members,
            RingScope::LowestN(2),
            DispelSchools::default(),
            true,
        );
        let indexes: Vec<usize> = draws.iter().map(|draw| draw.index).collect();
        assert_eq!(indexes, vec![1, 3]);
    }

    #[test]
    fn dispel_ring_shows_only_schools_the_spec_can_remove() {
        let cure = DispelSchools::from_schools([DispelSchool::Magic, DispelSchool::Curse]);
        let members = [
            RingMember {
                name: "curse",
                health_fraction: 0.5,
                debuff_types: &[DISPEL_CURSE],
                dead: false,
                offline: false,
            },
            RingMember {
                name: "poison",
                health_fraction: 0.5,
                debuff_types: &[DISPEL_POISON],
                dead: false,
                offline: false,
            },
            RingMember {
                name: "both",
                health_fraction: 0.5,
                debuff_types: &[DISPEL_DISEASE, DISPEL_MAGIC],
                dead: false,
                offline: false,
            },
        ];
        let draws = ring_draws(&members, RingScope::All, cure, true);
        assert_eq!(draws[0].dispel, Some(DispelSchool::Curse));
        assert_eq!(draws[1].dispel, None);
        assert_eq!(draws[2].dispel, Some(DispelSchool::Magic));
    }

    #[test]
    fn a_dispellable_debuff_keeps_its_outer_ring_at_full_health() {
        let schools = DispelSchools::from_schools([DispelSchool::Magic]);
        let debuffs = [DISPEL_MAGIC];
        let members = [RingMember {
            name: "full",
            health_fraction: 1.0,
            debuff_types: &debuffs,
            dead: false,
            offline: false,
        }];
        let draws = ring_draws(&members, RingScope::All, schools, true);
        assert_eq!(draws.len(), 1);
        assert_eq!(draws[0].alpha, 0.0);
        assert_eq!(draws[0].dispel, Some(DispelSchool::Magic));
    }

    #[test]
    fn the_pulse_stays_inside_its_alpha_band() {
        for step in 0..200 {
            let alpha = dispel_pulse(step as f32 * 0.017);
            assert!((0.55..=1.0).contains(&alpha), "{alpha}");
        }
    }

    #[test]
    fn an_on_screen_ally_gets_no_chevron_and_off_screen_allies_clamp_to_the_edge() {
        let viewport = [1280.0, 720.0];
        assert_eq!(
            chevron_placement([0.0, 0.0], false, viewport, CHEVRON_MARGIN),
            None
        );
        let right = chevron_placement([2.0, 0.0], false, viewport, CHEVRON_MARGIN).unwrap();
        assert!((right.position[0] - (1280.0 - CHEVRON_MARGIN)).abs() < 1e-3);
        assert!((right.position[1] - 360.0).abs() < 1e-3);
        assert_eq!(right.angle, 0.0);
        let top = chevron_placement([0.0, 2.0], false, viewport, CHEVRON_MARGIN).unwrap();
        assert!((top.position[1] - CHEVRON_MARGIN).abs() < 1e-3);
        assert!((top.angle + PI / 2.0).abs() < 1e-4);
        // A target behind the camera points the opposite way.
        let behind = chevron_placement([0.5, 0.0], true, viewport, CHEVRON_MARGIN).unwrap();
        assert!((behind.position[0] - CHEVRON_MARGIN).abs() < 1e-3);
        assert!((behind.angle - PI).abs() < 1e-4);
    }

    #[test]
    fn ring_ray_hits_only_the_drawn_annulus() {
        const ARC_START: f32 = 0.25 * TAU;
        let origin = [0.0, 10.0, 0.0];
        let down = [0.0, -1.0, 0.0];
        let rings = [(
            [0.0, 0.0, 0.0],
            HEALTH_INNER_RADIUS,
            HEALTH_OUTER_RADIUS,
            ARC_START,
        )];
        // Center is inside the hole.
        assert_eq!(ring_ray_hit(origin, down, &rings), None);
        // The drawn quarter of the band hits.
        let hit = ring_ray_hit([0.97, 10.0, 0.0], down, &rings).unwrap();
        assert_eq!(hit.0, 0);
        assert!((hit.1 - (10.0 - RING_LIFT)).abs() < 1e-4);
        // Outside the outer radius misses.
        assert_eq!(ring_ray_hit([1.5, 10.0, 0.0], down, &rings), None);
        // A ray pointing up never reaches the ground.
        assert_eq!(ring_ray_hit(origin, [0.0, 1.0, 0.0], &rings), None);
        // The empty clockwise gap is not clickable: 45 degrees is before the start.
        let gap = [0.97 * (TAU / 8.0).sin(), 10.0, -0.97 * (TAU / 8.0).cos()];
        assert_eq!(ring_ray_hit(gap, down, &rings), None);
        // Just inside the drawn arc still hits.
        let drawn = [0.97 * (TAU / 3.0).sin(), 10.0, -0.97 * (TAU / 3.0).cos()];
        assert!(ring_ray_hit(drawn, down, &rings).is_some());
    }

    #[test]
    fn station_offset_is_clockwise_from_twelve_oclock() {
        let (x, z) = station_offset(UP_ANGLE, 1.0);
        assert!(x.abs() < 1e-6 && (z + 1.0).abs() < 1e-6);
        let (x, z) = station_offset(TAU * 0.25, 1.0);
        assert!((x - 1.0).abs() < 1e-6 && z.abs() < 1e-6);
    }
}
