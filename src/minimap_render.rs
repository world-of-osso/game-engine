//! Minimap image generation: tile rendering, compositing, and UI image helpers.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::asset::adt::ChunkHeightGrid;

/// Create a blank RGBA image of given dimensions.
pub fn create_blank_image(w: u32, h: u32) -> Image {
    let data = vec![0u8; (w * h * 4) as usize];
    Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

/// Create a 200x200 RGBA image with a dark ring at the circle edge.
pub fn create_border_image(size: usize) -> Image {
    let center = size as f32 / 2.0;
    let outer_radius = center;
    let inner_radius = center - 3.0;
    let mut data = vec![0u8; size * size * 4];

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center + 0.5;
            let dy = y as f32 - center + 0.5;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist >= inner_radius && dist <= outer_radius {
                let i = (y * size + x) * 4;
                data[i] = 80;
                data[i + 1] = 60;
                data[i + 2] = 20;
                data[i + 3] = 220;
            }
        }
    }

    Image::new(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

/// Create a 16x16 RGBA image with an upward-pointing yellow triangle.
pub fn create_arrow_image() -> Image {
    let size = 16usize;
    let mut data = vec![0u8; size * size * 4];

    for y in 0..size {
        for x in 0..size {
            if point_in_triangle(x as f32, y as f32, 8.0, 2.0, 3.0, 13.0, 12.0, 13.0) {
                let i = (y * size + x) * 4;
                data[i] = 255;
                data[i + 1] = 220;
                data[i + 2] = 0;
                data[i + 3] = 255;
            }
        }
    }

    Image::new(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

/// Check if point (px, py) is inside triangle defined by three vertices.
#[allow(clippy::too_many_arguments)]
pub fn point_in_triangle(
    px: f32,
    py: f32,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    x3: f32,
    y3: f32,
) -> bool {
    let d1 = (px - x2) * (y1 - y2) - (x1 - x2) * (py - y2);
    let d2 = (px - x3) * (y2 - y3) - (x2 - x3) * (py - y3);
    let d3 = (px - x1) * (y3 - y1) - (x3 - x1) * (py - y1);
    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);
    !(has_neg && has_pos)
}

/// Build a dark-background composite buffer (comp_size x comp_size RGBA).
pub fn build_dark_composite(comp_size: usize) -> Vec<u8> {
    let mut composite = vec![0u8; comp_size * comp_size * 4];
    for i in 0..(comp_size * comp_size) {
        let off = i * 4;
        composite[off] = 20;
        composite[off + 1] = 20;
        composite[off + 2] = 20;
        composite[off + 3] = 255;
    }
    composite
}

/// Copy one tile image (src_w x src_w RGBA) into the composite at (off_x, off_y).
pub fn blit_image(
    dst: &mut [u8],
    dst_w: usize,
    src: &[u8],
    src_w: usize,
    off_x: usize,
    off_y: usize,
) {
    for y in 0..src_w {
        let si_start = y * src_w * 4;
        let di_start = ((off_y + y) * dst_w + off_x) * 4;
        let row_bytes = src_w * 4;
        if si_start + row_bytes <= src.len() && di_start + row_bytes <= dst.len() {
            dst[di_start..di_start + row_bytes]
                .copy_from_slice(&src[si_start..si_start + row_bytes]);
        }
    }
}

/// Crop a display_size window centered on (cx, cy) and apply a circular alpha mask.
pub fn crop_with_circle(
    composite: &[u8],
    comp_size: usize,
    cx: usize,
    cy: usize,
    display_size: u32,
) -> Vec<u8> {
    let ds = display_size as usize;
    let radius = ds as f32 / 2.0;
    let mut out = vec![0u8; ds * ds * 4];

    for y in 0..ds {
        for x in 0..ds {
            let dx = x as f32 - radius + 0.5;
            let dy = y as f32 - radius + 0.5;
            if (dx * dx + dy * dy).sqrt() > radius {
                continue;
            }
            let di = (y * ds + x) * 4;
            let sx = cx as i32 - ds as i32 / 2 + x as i32;
            let sy = cy as i32 - ds as i32 / 2 + y as i32;
            if sx >= 0 && (sx as usize) < comp_size && sy >= 0 && (sy as usize) < comp_size {
                let si = (sy as usize * comp_size + sx as usize) * 4;
                out[di..di + 4].copy_from_slice(&composite[si..si + 4]);
            } else {
                out[di] = 20;
                out[di + 1] = 20;
                out[di + 2] = 20;
                out[di + 3] = 255;
            }
        }
    }
    out
}

/// Draw a 3x3 colored dot at (cx, cy) in an RGBA buffer.
pub fn draw_dot(data: &mut [u8], size: usize, cx: i32, cy: i32, color: &[u8; 4]) {
    for dy in -1..=1i32 {
        for dx in -1..=1i32 {
            let x = cx + dx;
            let y = cy + dy;
            if x >= 0 && y >= 0 && (x as usize) < size && (y as usize) < size {
                let i = (y as usize * size + x as usize) * 4;
                if i + 3 < data.len() {
                    data[i..i + 4].copy_from_slice(color);
                }
            }
        }
    }
}

/// Render a 256x256 RGBA image for one terrain tile from heightmap data.
pub fn render_tile_image(chunks: &[Option<ChunkHeightGrid>], size: usize) -> Image {
    let mut data = vec![0u8; size * size * 4];
    let (min_h, max_h) = find_height_range(chunks);
    let range = (max_h - min_h).max(1.0);
    for chunk in chunks.iter().flatten() {
        fill_chunk_pixels(&mut data, size, chunk, min_h, range);
    }
    Image::new(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

/// Fill pixels for a single chunk within the tile image buffer.
fn fill_chunk_pixels(
    data: &mut [u8],
    size: usize,
    chunk: &ChunkHeightGrid,
    min_h: f32,
    range: f32,
) {
    let cx = chunk.index_x as usize;
    let cy = chunk.index_y as usize;
    let ppc = size / 16;

    for py in 0..ppc {
        for px in 0..ppc {
            let gx = (px * 8 / ppc).min(8);
            let gy = (py * 8 / ppc).min(8);
            let h = chunk.heights[gy * 17 + gx];
            let t = ((h - min_h) / range).clamp(0.0, 1.0);
            let (r, g, b) = height_to_color(t);
            let img_x = cx * ppc + px;
            let img_y = cy * ppc + py;
            let offset = (img_y * size + img_x) * 4;
            if offset + 3 < data.len() {
                data[offset] = r;
                data[offset + 1] = g;
                data[offset + 2] = b;
                data[offset + 3] = 255;
            }
        }
    }
}

/// Map a normalized height (0..1) to an RGB color.
pub fn height_to_color(t: f32) -> (u8, u8, u8) {
    if t < 0.4 {
        let s = t / 0.4;
        (
            (30.0 + s * 50.0) as u8,
            (80.0 + s * 80.0) as u8,
            (20.0 + s * 30.0) as u8,
        )
    } else {
        let s = (t - 0.4) / 0.6;
        (
            (80.0 + s * 80.0) as u8,
            (160.0 - s * 80.0) as u8,
            (50.0 - s * 20.0) as u8,
        )
    }
}

/// Find min/max height across all chunks in a tile.
pub fn find_height_range(chunks: &[Option<ChunkHeightGrid>]) -> (f32, f32) {
    let mut min_h = f32::MAX;
    let mut max_h = f32::MIN;
    for chunk in chunks.iter().flatten() {
        for &h in &chunk.heights {
            min_h = min_h.min(h);
            max_h = max_h.max(h);
        }
    }
    if min_h > max_h {
        (0.0, 1.0)
    } else {
        (min_h, max_h)
    }
}
