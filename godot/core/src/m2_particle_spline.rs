//! Allowed provenance: WebWowViewerCpp C3Spline/CSplineGenerator.
use glam::{Quat, Vec3};

use super::{GeneratorProps, Rng, aim_from_z_source};

// C3Spline::segLength samples twenty intervals per cubic segment.
const LENGTH_INTERVALS: usize = 20;

#[derive(Clone, Debug)]
pub(super) struct Spline {
    points: Vec<Vec3>,
    lengths: Vec<f32>,
    total_length: f32,
    previous_width: f32,
}

pub(super) fn valid_controls(points: &[[f32; 3]]) -> bool {
    points.len() >= 4
        && (points.len() - 1) % 3 == 0
        && points.iter().flatten().all(|v| v.is_finite())
}

impl Spline {
    pub(super) fn new(points: &[[f32; 3]]) -> Option<Self> {
        if !valid_controls(points) {
            return None;
        }
        let points: Vec<Vec3> = points.iter().copied().map(Vec3::from).collect();
        let lengths: Vec<f32> = points
            .windows(4)
            .step_by(3)
            .map(|p| {
                (1..=LENGTH_INTERVALS)
                    .map(|i| {
                        let t = i as f32 / LENGTH_INTERVALS as f32;
                        cubic(p, t).distance(cubic(p, (i - 1) as f32 / LENGTH_INTERVALS as f32))
                    })
                    .sum()
            })
            .collect();
        let total_length = lengths.iter().sum();
        Some(Self {
            points,
            lengths,
            total_length,
            previous_width: 0.0,
        })
    }

    fn sample(&self, fraction: f32) -> (Vec3, Vec3) {
        let mut distance = fraction * self.total_length;
        let mut segment = 0;
        while segment + 1 < self.lengths.len() && distance > self.lengths[segment] {
            distance -= self.lengths[segment];
            segment += 1;
        }
        let t = if self.lengths.len() == 1 {
            fraction
        } else if self.lengths[segment] > 0.0 {
            distance / self.lengths[segment]
        } else {
            0.0
        };
        let p = &self.points[segment * 3..segment * 3 + 4];
        let u = 1.0 - t;
        let tangent = (p[1] - p[0]) * (3.0 * u * u)
            + (p[2] - p[1]) * (6.0 * u * t)
            + (p[3] - p[2]) * (3.0 * t * t);
        (cubic(p, t), tangent.normalize_or_zero())
    }

    pub(super) fn spawn(&mut self, props: &GeneratorProps, rng: &mut Rng) -> (Vec3, Vec3) {
        let lower = props.area_length.clamp(0.0, 1.0);
        let upper = props.area_width.clamp(0.0, 1.0);
        // WWV emits at the new endpoint once when the animated width changes.
        let fraction =
            if (props.area_width - self.previous_width).abs() >= super::DEGENERATE_LENGTH_SQUARED {
                self.previous_width = upper;
                upper
            } else {
                lower + (upper - lower) * rng.uniform_pos()
            };
        let (mut position, tangent) = self.sample(fraction);
        let speed = props.emission_speed * (1.0 + props.speed_variation * rng.uniform());
        let direction = if props.z_source > 0.001 {
            aim_from_z_source(position, props.z_source)
        } else if props.vertical_range != 0.0 {
            let direction =
                Quat::from_axis_angle(tangent, rng.uniform() * props.vertical_range) * Vec3::Z;
            if props.horizontal_range != 0.0 {
                position += Vec3::splat(props.horizontal_range * rng.uniform_pos());
            }
            direction
        } else {
            Vec3::Z
        };
        (position, direction * speed)
    }
}

fn cubic(p: &[Vec3], t: f32) -> Vec3 {
    let u = 1.0 - t;
    p[0] * (u * u * u) + p[1] * (3.0 * u * u * t) + p[2] * (3.0 * u * t * t) + p[3] * (t * t * t)
}
