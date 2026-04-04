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
    let mut data = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let u = x as f32 / width as f32;
            let v = y as f32 / height as f32;
            let noise = fbm_simplex(u * 7.0, v * 9.0, seed);
            let ridges = fbm_simplex(
                u * 15.0 + 17.3,
                v * 13.0 - 11.1,
                seed ^ CLOUD_RIDGE_SEED_MIX,
            );
            let combined =
                (noise * 0.72 + (1.0 - (ridges * 2.0 - 1.0).abs()) * 0.28).clamp(0.0, 1.0);
            let softened = combined.powf(1.35);
            let value = (softened * 255.0).round() as u8;
            let idx = ((y * width + x) * 4) as usize;
            data[idx] = value;
            data[idx + 1] = value;
            data[idx + 2] = value;
            data[idx + 3] = 255;
        }
    }

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

fn fbm_simplex(x: f32, y: f32, seed: u32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 0.55;
    let mut freq = 1.0;
    let mut norm = 0.0;
    for octave in 0..CLOUD_OCTAVES {
        let ox = seed as f32 * 0.017 + octave as f32 * 13.1;
        let oy = seed as f32 * 0.023 - octave as f32 * 9.7;
        sum += simplex2(
            x * freq + ox,
            y * freq + oy,
            seed.wrapping_add(octave as u32 * 31),
        ) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    ((sum / norm) * 0.5 + 0.5).clamp(0.0, 1.0)
}

fn simplex2(x: f32, y: f32, seed: u32) -> f32 {
    const F2: f32 = 0.366_025_4;
    const G2: f32 = 0.211_324_87;

    let s = (x + y) * F2;
    let i = (x + s).floor() as i32;
    let j = (y + s).floor() as i32;
    let t = (i + j) as f32 * G2;
    let x0 = x - (i as f32 - t);
    let y0 = y - (j as f32 - t);

    let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };
    let x1 = x0 - i1 as f32 + G2;
    let y1 = y0 - j1 as f32 + G2;
    let x2 = x0 - 1.0 + 2.0 * G2;
    let y2 = y0 - 1.0 + 2.0 * G2;

    let n0 = simplex_corner(i, j, x0, y0, seed);
    let n1 = simplex_corner(i + i1, j + j1, x1, y1, seed);
    let n2 = simplex_corner(i + 1, j + 1, x2, y2, seed);

    70.0 * (n0 + n1 + n2)
}

fn simplex_corner(i: i32, j: i32, x: f32, y: f32, seed: u32) -> f32 {
    let t = 0.5 - x * x - y * y;
    if t <= 0.0 {
        return 0.0;
    }
    let grad = gradient(hash2(i, j, seed));
    let t2 = t * t;
    t2 * t2 * (grad.x * x + grad.y * y)
}

fn hash2(i: i32, j: i32, seed: u32) -> u32 {
    let mut h = seed
        .wrapping_add((i as u32).wrapping_mul(0x8DA6_B343))
        .wrapping_add((j as u32).wrapping_mul(0xD816_3841));
    h ^= h >> 13;
    h = h.wrapping_mul(0x85EB_CA6B);
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
        assert!(data.iter().any(|&value| value != first));
    }

    #[test]
    fn cloud_buffer_index_cycles_through_three_buffers() {
        assert_eq!(next_cloud_buffer_index(0), 1);
        assert_eq!(next_cloud_buffer_index(1), 2);
        assert_eq!(next_cloud_buffer_index(2), 0);
    }
}
