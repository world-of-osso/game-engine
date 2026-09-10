use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::rendering::image_sampler::repeat_linear_sampler;

pub const CLOUD_TEXTURE_WIDTH: u32 = 512;
pub const CLOUD_TEXTURE_HEIGHT: u32 = 1024;
pub const CLOUD_REGEN_SECONDS: f32 = 5.0;
const CLOUD_OCTAVES: usize = 6;
const CLOUD_RIDGE_SEED_MIX: u32 = 0x9E37_79B9;
const GRADIENT_HASH_X_MIX: u32 = 0x8DA6_B343;
const GRADIENT_HASH_Y_MIX: u32 = 0xD816_3841;
const GRADIENT_HASH_FINAL_MIX: u32 = 0x85EB_CA6B;

#[derive(Resource)]
pub struct ProceduralCloudMaps {
    pub handles: [Handle<Image>; 3],
    pub active_index: usize,
    pub next_seed: u32,
    pub regen_timer: Timer,
}

impl ProceduralCloudMaps {
    pub fn active_handle(&self) -> Handle<Image> {
        self.handles[self.active_index].clone()
    }
}

pub fn create_procedural_cloud_maps(images: &mut Assets<Image>) -> ProceduralCloudMaps {
    let handles = [
        images.add(generate_procedural_cloud_image(0)),
        images.add(generate_procedural_cloud_image(1)),
        images.add(generate_procedural_cloud_image(2)),
    ];
    ProceduralCloudMaps {
        handles,
        active_index: 0,
        next_seed: 3,
        regen_timer: Timer::from_seconds(CLOUD_REGEN_SECONDS, TimerMode::Repeating),
    }
}

pub fn next_cloud_buffer_index(current: usize) -> usize {
    (current + 1) % 3
}

pub fn generate_procedural_cloud_image(seed: u32) -> Image {
    let width = CLOUD_TEXTURE_WIDTH;
    let height = CLOUD_TEXTURE_HEIGHT;
    let data = generate_cloud_pixels(width, height, seed);

    let mut image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = repeat_linear_sampler();
    image
}

fn generate_cloud_pixels(width: u32, height: u32, seed: u32) -> Vec<u8> {
    let mut data = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let value = cloud_density_byte(x, y, width, height, seed);
            let idx = ((y * width + x) * 4) as usize;
            data[idx] = value;
            data[idx + 1] = value;
            data[idx + 2] = value;
            data[idx + 3] = 255;
        }
    }
    data
}

fn cloud_density_byte(x: u32, y: u32, width: u32, height: u32, seed: u32) -> u8 {
    let u = x as f32 / width as f32;
    let v = y as f32 / height as f32;
    (cloud_density(u, v, seed) * 255.0).round() as u8
}

fn cloud_density(u: f32, v: f32, seed: u32) -> f32 {
    let noise = fbm_periodic(Vec2::new(u * 7.0, v * 9.0), IVec2::new(7, 9), seed);
    let ridges = fbm_periodic(
        Vec2::new(u * 15.0 + 17.3, v * 13.0 - 11.1),
        IVec2::new(15, 13),
        seed ^ CLOUD_RIDGE_SEED_MIX,
    );
    let combined = (noise * 0.72 + (1.0 - (ridges * 2.0 - 1.0).abs()) * 0.28).clamp(0.0, 1.0);
    combined.powf(1.35)
}

fn fbm_periodic(point: Vec2, period: IVec2, seed: u32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 0.55;
    let mut frequency = 1;
    let mut norm = 0.0;
    for octave in 0..CLOUD_OCTAVES {
        let phase = Vec2::new(octave as f32 * 13.1, octave as f32 * -9.7);
        sum += periodic_gradient_noise(
            point * frequency as f32 + phase,
            period * frequency,
            seed.wrapping_add(octave as u32 * 31),
        ) * amp;
        norm += amp;
        amp *= 0.5;
        frequency *= 2;
    }
    ((sum / norm) * 0.5 + 0.5).clamp(0.0, 1.0)
}

fn periodic_gradient_noise(point: Vec2, period: IVec2, seed: u32) -> f32 {
    let cell = point.floor();
    let offset = point - cell;
    let x0 = (cell.x as i32).rem_euclid(period.x);
    let y0 = (cell.y as i32).rem_euclid(period.y);
    let x1 = if x0 + 1 == period.x { 0 } else { x0 + 1 };
    let y1 = if y0 + 1 == period.y { 0 } else { y0 + 1 };
    let n00 = gradient(hash2(x0, y0, seed)).dot(offset);
    let n10 = gradient(hash2(x1, y0, seed)).dot(offset - Vec2::X);
    let n01 = gradient(hash2(x0, y1, seed)).dot(offset - Vec2::Y);
    let n11 = gradient(hash2(x1, y1, seed)).dot(offset - Vec2::ONE);
    let weight = Vec2::new(noise_blend_weight(offset.x), noise_blend_weight(offset.y));
    let lower = n00 + weight.x * (n10 - n00);
    let upper = n01 + weight.x * (n11 - n01);
    lower + weight.y * (upper - lower)
}

fn noise_blend_weight(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn hash2(i: i32, j: i32, seed: u32) -> u32 {
    let mut h = seed
        .wrapping_add((i as u32).wrapping_mul(GRADIENT_HASH_X_MIX))
        .wrapping_add((j as u32).wrapping_mul(GRADIENT_HASH_Y_MIX));
    h ^= h >> 13;
    h = h.wrapping_mul(GRADIENT_HASH_FINAL_MIX);
    h ^ (h >> 16)
}

fn gradient(hash: u32) -> Vec2 {
    match hash & 7 {
        0 => Vec2::new(1.0, 1.0),
        1 => Vec2::new(-1.0, 1.0),
        2 => Vec2::new(1.0, -1.0),
        3 => Vec2::new(-1.0, -1.0),
        4 => Vec2::new(1.0, 0.0),
        5 => Vec2::new(-1.0, 0.0),
        6 => Vec2::new(0.0, 1.0),
        _ => Vec2::new(0.0, -1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_density_repeats_across_positive_and_negative_uv_periods() {
        for seed in [0, 1, 2, 11] {
            for (u, v) in [(0.0, 0.0), (0.125, 0.25), (0.3125, 0.6875)] {
                let expected = cloud_density(u, v, seed);
                for (du, dv) in [(1.0, 0.0), (0.0, 1.0), (-2.0, 3.0), (3.0, -2.0)] {
                    let actual = cloud_density(u + du, v + dv, seed);
                    assert!(
                        (actual - expected).abs() < 0.00002,
                        "seed {seed}, UV ({u}, {v}) shifted by ({du}, {dv} changed density from {expected} to {actual}"
                    );
                }
            }
        }
    }

    #[test]
    fn high_bit_seeds_preserve_fractional_coordinate_detail() {
        for seed in [0x8000_0000, u32::MAX] {
            for (u, v) in [(0.125, 0.25), (0.3125, 0.6875)] {
                let center = cloud_density(u, v, seed);
                let neighbor = cloud_density(u + 1.0 / 1024.0, v + 1.0 / 1024.0, seed);
                assert!(
                    (center - neighbor).abs() > 0.000001,
                    "seed {seed} lost fractional detail at ({u}, {v}): {center} versus {neighbor}"
                );
            }
        }
    }

    #[test]
    fn cloud_pixel_wrap_steps_are_comparable_to_interior_steps() {
        for seed in [0, 1, 2, 11] {
            let pixels = generate_cloud_pixels(CLOUD_TEXTURE_WIDTH, CLOUD_TEXTURE_HEIGHT, seed);
            let (interior, wrapped) = measure_neighbor_steps(&pixels);
            for (axis, inside, edge) in [("U", interior.x, wrapped.x), ("V", interior.y, wrapped.y)]
            {
                assert!(
                    edge <= inside * 3.0 + 1.0,
                    "seed {seed} {axis} wrap has mean step {edge}, versus interior {inside}"
                );
            }
        }
    }

    fn measure_neighbor_steps(pixels: &[u8]) -> (Vec2, Vec2) {
        let width = CLOUD_TEXTURE_WIDTH as usize;
        let height = CLOUD_TEXTURE_HEIGHT as usize;
        let sample = |x, y| f32::from(pixels[(y * width + x) * 4]);
        let mut interior = Vec2::ZERO;
        let mut wrapped = Vec2::ZERO;
        for y in 0..height {
            for x in 0..width {
                let value = sample(x, y);
                let dx = (sample((x + 1) % width, y) - value).abs();
                let dy = (sample(x, (y + 1) % height) - value).abs();
                if x + 1 == width {
                    wrapped.x += dx;
                } else {
                    interior.x += dx;
                }
                if y + 1 == height {
                    wrapped.y += dy;
                } else {
                    interior.y += dy;
                }
            }
        }
        interior /= Vec2::new(((width - 1) * height) as f32, (width * (height - 1)) as f32);
        wrapped /= Vec2::new(height as f32, width as f32);
        (interior, wrapped)
    }

    #[test]
    fn cloud_pixels_are_deterministic_and_change_with_seed() {
        let pixels = generate_cloud_pixels(64, 64, 11);
        assert_eq!(pixels, generate_cloud_pixels(64, 64, 11));
        assert_ne!(pixels, generate_cloud_pixels(64, 64, 12));
    }

    #[test]
    fn generated_cloud_image_has_expected_size() {
        let image = generate_procedural_cloud_image(7);
        assert_eq!(image.texture_descriptor.size.width, CLOUD_TEXTURE_WIDTH);
        assert_eq!(image.texture_descriptor.size.height, CLOUD_TEXTURE_HEIGHT);
        assert_eq!(
            image.data.as_ref().unwrap().len(),
            (CLOUD_TEXTURE_WIDTH * CLOUD_TEXTURE_HEIGHT * 4) as usize
        );
    }

    #[test]
    fn generated_cloud_image_contains_variation() {
        let image = generate_procedural_cloud_image(11);
        let data = image.data.as_ref().unwrap();
        let first = data[0];
        assert!(data.chunks_exact(4).any(|pixel| pixel[0] != first));
    }

    #[test]
    fn cloud_buffer_index_cycles_through_three_buffers() {
        assert_eq!(next_cloud_buffer_index(0), 1);
        assert_eq!(next_cloud_buffer_index(1), 2);
        assert_eq!(next_cloud_buffer_index(2), 0);
    }
}
