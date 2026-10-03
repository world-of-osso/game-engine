//! Bevy-free procedural sky profile and linear RGBA16F cubemap pixels.

/// Dome points from the top pole to the bottom pole, in half-turn fractions.
const SKY_DOME_RING_LATITUDES: [f32; SKY_DOME_POINT_COUNT] =
    [0.0, 0.17, 0.2, 0.23, 0.24, 0.25, 1.0];

pub const SKY_DOME_POINT_COUNT: usize = 7;
pub const SKY_BAND_MAX: f32 = (SKY_DOME_POINT_COUNT - 1) as f32;
pub const ENV_MAP_SIZE: u32 = 32;

/// One dome point for a unit dome: horizontal radius and height above the eye.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyDomePoint {
    pub horizontal: f32,
    pub height: f32,
}

impl SkyDomePoint {
    pub fn elevation(self) -> f32 {
        self.height.atan2(self.horizontal)
    }
}

/// The client's periodic cubic stand-in for cos(π·phase).
fn client_periodic_cosine(phase: f64) -> f64 {
    let period = if phase <= 0.0 {
        phase as i32 - 1
    } else {
        phase as i32
    };
    let fraction = phase - f64::from(period);
    let value = 1.0 - (6.0 - 4.0 * fraction) * fraction * fraction;
    if period & 1 != 0 { -value } else { value }
}

pub fn sky_dome_profile() -> [SkyDomePoint; SKY_DOME_POINT_COUNT] {
    let lowered_by = std::f64::consts::FRAC_PI_4.cos();
    SKY_DOME_RING_LATITUDES.map(|latitude| {
        let phase = f64::from(latitude);
        SkyDomePoint {
            horizontal: client_periodic_cosine(phase - 0.5).abs() as f32,
            height: (client_periodic_cosine(phase) - lowered_by) as f32,
        }
    })
}

/// Band coordinate seen along a view elevation, interpolated across dome edges.
pub fn sky_band_at_elevation(elevation: f32) -> f32 {
    let profile = sky_dome_profile();
    let (sin, cos) = elevation.sin_cos();
    for (index, pair) in profile.windows(2).enumerate() {
        let (upper, lower) = (pair[0], pair[1]);
        if elevation > lower.elevation() || index + 2 == SKY_DOME_POINT_COUNT {
            return index as f32 + edge_fraction(upper, lower, sin, cos);
        }
    }
    SKY_BAND_MAX
}

fn edge_fraction(upper: SkyDomePoint, lower: SkyDomePoint, sin: f32, cos: f32) -> f32 {
    let dh = lower.horizontal - upper.horizontal;
    let dz = lower.height - upper.height;
    let denominator = dh * sin - dz * cos;
    if denominator.abs() <= f32::EPSILON {
        return 0.0;
    }
    ((upper.height * cos - upper.horizontal * sin) / denominator).clamp(0.0, 1.0)
}

/// Linear RGB of the dome at a band coordinate. Stops are top pole first.
pub fn sky_gradient_color(stops: &[[f32; 3]; SKY_DOME_POINT_COUNT], band: f32) -> [f32; 3] {
    let band = band.clamp(0.0, SKY_BAND_MAX);
    let index = (band.floor() as usize).min(SKY_DOME_POINT_COUNT - 2);
    let t = band - index as f32;
    let a = stops[index];
    let b = stops[index + 1];
    std::array::from_fn(|channel| a[channel] + (b[channel] - a[channel]) * t)
}

/// Normalized direction through a pixel center. Face order: +X, -X, +Y, -Y, +Z, -Z.
pub fn cubemap_direction(face: u32, x: u32, y: u32) -> [f32; 3] {
    let u = (x as f32 + 0.5) / ENV_MAP_SIZE as f32 * 2.0 - 1.0;
    let v = (y as f32 + 0.5) / ENV_MAP_SIZE as f32 * 2.0 - 1.0;
    let dir: [f32; 3] = match face {
        0 => [1.0, -v, -u],
        1 => [-1.0, -v, u],
        2 => [u, 1.0, v],
        3 => [u, -1.0, -v],
        4 => [u, -v, 1.0],
        _ => [-u, -v, -1.0],
    };
    let inverse_length = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2])
        .sqrt()
        .recip();
    dir.map(|channel| channel * inverse_length)
}

/// Six contiguous 32×32 faces, pixel-center sampled, linear RGBA16F little-endian;
/// no mip levels. Each input stop is linear RGB, including the bottom pole.
pub fn build_sky_cubemap(stops: &[[f32; 3]; SKY_DOME_POINT_COUNT]) -> Vec<u8> {
    let face_pixels = (ENV_MAP_SIZE * ENV_MAP_SIZE) as usize;
    let mut data = vec![0u8; face_pixels * 6 * 8];
    for face in 0..6u32 {
        let offset = face as usize * face_pixels * 8;
        fill_cubemap_face(&mut data[offset..offset + face_pixels * 8], face, stops);
    }
    data
}

fn fill_cubemap_face(data: &mut [u8], face: u32, stops: &[[f32; 3]; SKY_DOME_POINT_COUNT]) {
    for y in 0..ENV_MAP_SIZE {
        for x in 0..ENV_MAP_SIZE {
            let dir = cubemap_direction(face, x, y);
            let color = sky_gradient_color(stops, sky_band_at_elevation(dir[1].asin()));
            let pixel_offset = ((y * ENV_MAP_SIZE + x) as usize) * 8;
            write_rgba16f(&mut data[pixel_offset..pixel_offset + 8], color);
        }
    }
}

fn write_rgba16f(dst: &mut [u8], rgb: [f32; 3]) {
    for (index, value) in rgb.into_iter().chain([1.0]).enumerate() {
        let bytes = half::f16::from_f32(value).to_le_bytes();
        dst[index * 2..index * 2 + 2].copy_from_slice(&bytes);
    }
}
