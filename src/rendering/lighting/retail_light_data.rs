//! Bevy-free Retail exterior light arithmetic in authored RGB space.

/// Sampled LightData colours and fog range, before the sun direction is evaluated.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailLightColors {
    pub ambient: [f32; 3],
    pub horizon_ambient: [f32; 3],
    pub ground_ambient: [f32; 3],
    pub direct: [f32; 3],
    pub fog_color: [f32; 3],
    pub fog_start: f32,
    pub fog_end: f32,
}

/// Exterior scene light; direction is the direction sunlight travels, in Bevy/Godot world axes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailLightData {
    pub ambient: [f32; 3],
    pub horizon_ambient: [f32; 3],
    pub ground_ambient: [f32; 3],
    pub direct: [f32; 3],
    pub sun_direction: [f32; 3],
    pub fog_color: [f32; 3],
    pub fog_start: f32,
    pub fog_end: f32,
}

/// A zero horizon or ground ambient uses the authored ambient instead.
pub fn scene_light(colors: &RetailLightColors, minutes: f32) -> RetailLightData {
    let or_ambient = |color| {
        if color == [0.0; 3] {
            colors.ambient
        } else {
            color
        }
    };
    RetailLightData {
        ambient: colors.ambient,
        horizon_ambient: or_ambient(colors.horizon_ambient),
        ground_ambient: or_ambient(colors.ground_ambient),
        direct: colors.direct,
        sun_direction: sun_direction(minutes),
        fog_color: colors.fog_color,
        fog_start: colors.fog_start,
        fog_end: colors.fog_end,
    }
}

/// MathHelper directionalLightPhiTable / directionalLightThetaTable (fraction of day, radians).
const DIRECTIONAL_LIGHT_PHI: [[f32; 2]; 4] = [
    [0.0, 2.216_568_2],
    [0.25, 1.919_862_2],
    [0.5, 2.216_568_2],
    [0.75, 1.919_862_2],
];
const DIRECTIONAL_LIGHT_THETA: [[f32; 2]; 4] = [
    [0.0, 3.926_990_7],
    [0.25, 3.926_990_7],
    [0.5, 3.926_990_7],
    [0.75, 3.926_990_7],
];

/// MathHelper::InterpTable: cyclic linear interpolation over the day.
pub fn interp_day_table(table: &[[f32; 2]], day: f32) -> f32 {
    let day = if day >= 0.0 { day.min(1.0) } else { day };
    let first = table.iter().position(|entry| day <= entry[0]).unwrap_or(0);
    let second = if first == 0 {
        table.len() - 1
    } else {
        first - 1
    };
    let mut span = table[first][0] - table[second][0];
    if span.abs() < 0.001 {
        return table[second][1];
    }
    if span < 0.0 {
        span += 1.0;
    }
    let mut elapsed = day - table[second][0];
    if elapsed < 0.0 {
        elapsed += 1.0;
    }
    let alpha = elapsed / span;
    table[second][1] + alpha * (table[first][1] - table[second][1])
}

/// Direction the exterior direct light travels, in y-up world coordinates.
pub fn sun_direction(minutes: f32) -> [f32; 3] {
    let day = minutes.rem_euclid(2880.0) / 2880.0;
    let phi = interp_day_table(&DIRECTIONAL_LIGHT_PHI, day);
    let theta = interp_day_table(&DIRECTIONAL_LIGHT_THETA, day);
    // WoW z-up polarToCartesian, then (x, z, -y) into world space.
    normalize([phi.sin() * theta.cos(), phi.cos(), -phi.sin() * theta.sin()])
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let reciprocal = dot(v, v).sqrt().recip();
    v.map(|component| component * reciprocal)
}

fn lerp(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

/// calcLight and applyAndMixAmbients, without point lights, specular or emission.
/// Visibility scales direct sun only; 0 is fully shadowed and 1 is unshadowed.
pub fn shade(
    light: &RetailLightData,
    diffuse: [f32; 3],
    normal: [f32; 3],
    sun_visibility: f32,
) -> [f32; 3] {
    let normal = normalize(normal);
    let n_dot_up = normal[1];
    let sun = normalize(light.sun_direction).map(|component| -component);
    let n_dot_l = dot(normal, sun).clamp(0.0, 1.0);
    let hemisphere = if n_dot_up >= 0.0 {
        lerp(light.horizon_ambient, light.ambient, n_dot_up)
    } else {
        lerp(light.horizon_ambient, light.ground_ambient, -n_dot_up)
    };
    let sky = hemisphere.map(|channel| channel * 1.1);
    let ground = hemisphere.map(|channel| channel * 0.7);
    let ambient = lerp(ground, sky, 0.5 + 0.5 * n_dot_l);
    std::array::from_fn(|i| diffuse[i] * (ambient[i] + light.direct[i] * n_dot_l * sun_visibility))
}
