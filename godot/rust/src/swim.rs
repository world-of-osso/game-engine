//! Swim height rules on shared-protocol's `SWIM_DEPTH` threshold (`is_swimming`,
//! `swim_top`), the ones the server bounds reported swim heights with.

use shared::movement::{SWIM_EPSILON, swim_top};

/// Whether a swimmer at `feet_y` floats at the surface.
pub(crate) fn at_swim_surface(feet_y: f32, surface_y: f32) -> bool {
    feet_y >= swim_top(surface_y) - SWIM_EPSILON
}

/// A swimmer's height after `rise`, this frame's intended vertical travel (ascend/descend
/// and pitched movement). A swimmer floating at the surface stays on it while not diving;
/// none rises above it or sinks through the ground.
pub(crate) fn swim_height(
    feet_y: f32,
    rise: f32,
    surface_y: f32,
    ground_y: Option<f32>,
    at_surface: bool,
) -> f32 {
    let top = swim_top(surface_y);
    let y = if at_surface && rise >= 0.0 {
        top
    } else {
        (feet_y + rise).min(top)
    };
    ground_y.map_or(y, |ground| y.max(ground))
}
