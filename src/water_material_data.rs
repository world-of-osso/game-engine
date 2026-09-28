#[cfg(test)]
#[path = "water_material_data_tests.rs"]
mod tests;

pub const WATER_NORMAL_SIZE: u32 = 256;
const HASH_F32_OUTPUT_MASK: u32 = 0xFFFF;

/// Generate the tileable procedural water normal map as RGBA8 linear bytes.
pub fn generate_water_normal_rgba() -> Vec<u8> {
    let size = WATER_NORMAL_SIZE;
    let mut heightmap = vec![0.0f32; (size * size) as usize];
    fbm_tileable_noise(&mut heightmap, size);

    let mut data = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let (nx, ny) = heightmap_normal(&heightmap, x, y, size);
            let nz = (1.0 - nx * nx - ny * ny).max(0.0).sqrt();
            let idx = ((y * size + x) * 4) as usize;
            data[idx] = ((nx * 0.5 + 0.5) * 255.0) as u8;
            data[idx + 1] = ((ny * 0.5 + 0.5) * 255.0) as u8;
            data[idx + 2] = (nz * 255.0) as u8;
            data[idx + 3] = 255;
        }
    }
    data
}

/// Tileable FBM noise: 5 octaves of value noise, each tileable at `size`.
fn fbm_tileable_noise(out: &mut [f32], size: u32) {
    let octaves = [
        (4.0, 0.4),
        (8.0, 0.3),
        (16.0, 0.15),
        (32.0, 0.1),
        (64.0, 0.05),
    ];
    for (freq, amp) in octaves {
        for y in 0..size {
            for x in 0..size {
                let v = tileable_value_noise(
                    x as f32 * freq / size as f32,
                    y as f32 * freq / size as f32,
                    freq,
                );
                out[(y * size + x) as usize] += v * amp;
            }
        }
    }
}

/// Value noise that tiles at integer period `period`.
fn tileable_value_noise(x: f32, y: f32, period: f32) -> f32 {
    let p = period as u32;
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let fx = x - x.floor();
    let fy = y - y.floor();
    let sx = fx * fx * (3.0 - 2.0 * fx); // smoothstep
    let sy = fy * fy * (3.0 - 2.0 * fy);

    let v00 = hash_f32(wrap(ix, p), wrap(iy, p));
    let v10 = hash_f32(wrap(ix + 1, p), wrap(iy, p));
    let v01 = hash_f32(wrap(ix, p), wrap(iy + 1, p));
    let v11 = hash_f32(wrap(ix + 1, p), wrap(iy + 1, p));

    let a = v00 + (v10 - v00) * sx;
    let b = v01 + (v11 - v01) * sx;
    a + (b - a) * sy
}

fn wrap(v: i32, period: u32) -> u32 {
    ((v % period as i32 + period as i32) % period as i32) as u32
}

/// Hash two u32 coords to a float in [-1, 1].
fn hash_f32(x: u32, y: u32) -> f32 {
    let mut h = x
        .wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    (h & HASH_F32_OUTPUT_MASK) as f32 / 32767.5 - 1.0
}

/// Compute normal from heightmap via central differences (wrapping for tileability).
fn heightmap_normal(heightmap: &[f32], x: u32, y: u32, size: u32) -> (f32, f32) {
    let s = size as usize;
    let h = |xi: u32, yi: u32| heightmap[(yi % size) as usize * s + (xi % size) as usize];
    let xp = if x + 1 >= size { 0 } else { x + 1 };
    let xm = if x == 0 { size - 1 } else { x - 1 };
    let yp = if y + 1 >= size { 0 } else { y + 1 };
    let ym = if y == 0 { size - 1 } else { y - 1 };
    let scale = 0.8; // controls normal intensity
    let dx = (h(xp, y) - h(xm, y)) * scale;
    let dy = (h(x, yp) - h(x, ym)) * scale;
    (dx.clamp(-1.0, 1.0), dy.clamp(-1.0, 1.0))
}
