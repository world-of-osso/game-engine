use super::*;
use std::collections::HashMap;

const SEAM_SMOOTHING_RINGS: &[(usize, f32)] = &[];

pub(super) fn stitch_chunk_edges(parsed: &mut [McnkData]) {
    let indices: HashMap<(u32, u32), usize> = parsed
        .iter()
        .enumerate()
        .map(|(i, chunk)| ((chunk.index_x, chunk.index_y), i))
        .collect();

    for i in 0..parsed.len() {
        let (index_x, index_y) = (parsed[i].index_x, parsed[i].index_y);
        if let Some(&neighbor) = indices.get(&(index_x, index_y + 1)) {
            stitch_vertical_border(parsed, i, neighbor);
        }
        if let Some(&neighbor) = indices.get(&(index_x + 1, index_y)) {
            stitch_horizontal_border(parsed, i, neighbor);
        }
    }

    stitch_chunk_corners(parsed, &indices);
}

fn stitch_horizontal_border(parsed: &mut [McnkData], top: usize, bottom: usize) {
    let (top_chunk, bottom_chunk) = split_two_mut(parsed, top, bottom);
    for col in 0..=8 {
        let stitched = average_height_pair(
            top_chunk.pos[2],
            &mut top_chunk.heights,
            vertex_index(16, col),
            bottom_chunk.pos[2],
            &mut bottom_chunk.heights,
            vertex_index(0, col),
        );
        smooth_horizontal_interior(top_chunk, bottom_chunk, col, stitched);
    }
}

fn stitch_vertical_border(parsed: &mut [McnkData], left: usize, right: usize) {
    let (left_chunk, right_chunk) = split_two_mut(parsed, left, right);
    for row in 0..=8 {
        let stitched = average_height_pair(
            left_chunk.pos[2],
            &mut left_chunk.heights,
            vertex_index(row * 2, 8),
            right_chunk.pos[2],
            &mut right_chunk.heights,
            vertex_index(row * 2, 0),
        );
        smooth_vertical_interior(left_chunk, right_chunk, row, stitched);
    }
}

fn stitch_chunk_corners(parsed: &mut [McnkData], indices: &HashMap<(u32, u32), usize>) {
    for (&(index_x, index_y), &top_left) in indices {
        let Some(&top_right) = indices.get(&(index_x, index_y + 1)) else {
            continue;
        };
        let Some(&bottom_left) = indices.get(&(index_x + 1, index_y)) else {
            continue;
        };
        let Some(&bottom_right) = indices.get(&(index_x + 1, index_y + 1)) else {
            continue;
        };
        average_height_quad(
            parsed,
            [
                (top_left, vertex_index(16, 8)),
                (top_right, vertex_index(16, 0)),
                (bottom_left, vertex_index(0, 8)),
                (bottom_right, vertex_index(0, 0)),
            ],
        );
    }
}

fn split_two_mut<T>(slice: &mut [T], a: usize, b: usize) -> (&mut T, &mut T) {
    assert!(a != b, "indices must be distinct");
    if a < b {
        let (left, right) = slice.split_at_mut(b);
        (&mut left[a], &mut right[0])
    } else {
        let (left, right) = slice.split_at_mut(a);
        (&mut right[0], &mut left[b])
    }
}

fn average_height_pair(
    base_a: f32,
    heights_a: &mut [f32; MCVT_COUNT],
    idx_a: usize,
    base_b: f32,
    heights_b: &mut [f32; MCVT_COUNT],
    idx_b: usize,
) -> f32 {
    let absolute_a = base_a + heights_a[idx_a];
    let absolute_b = base_b + heights_b[idx_b];
    let avg = (absolute_a + absolute_b) * 0.5;
    heights_a[idx_a] = avg - base_a;
    heights_b[idx_b] = avg - base_b;
    avg
}

fn average_height_quad(parsed: &mut [McnkData], vertices: [(usize, usize); 4]) {
    let average = vertices
        .iter()
        .map(|&(chunk_idx, vertex_idx)| {
            parsed[chunk_idx].pos[2] + parsed[chunk_idx].heights[vertex_idx]
        })
        .sum::<f32>()
        / vertices.len() as f32;

    for (chunk_idx, vertex_idx) in vertices {
        let chunk = &mut parsed[chunk_idx];
        chunk.heights[vertex_idx] = average - chunk.pos[2];
    }
}

fn smooth_horizontal_interior(
    top_chunk: &mut McnkData,
    bottom_chunk: &mut McnkData,
    col: usize,
    stitched_height: f32,
) {
    if col == 0 || col == 8 {
        return;
    }
    for &(ring, weight) in SEAM_SMOOTHING_RINGS {
        if ring > 8 {
            break;
        }
        blend_absolute_height(
            top_chunk,
            vertex_index(16 - ring * 2, col),
            stitched_height,
            weight,
        );
        blend_absolute_height(
            bottom_chunk,
            vertex_index(ring * 2, col),
            stitched_height,
            weight,
        );
    }
}

fn smooth_vertical_interior(
    left_chunk: &mut McnkData,
    right_chunk: &mut McnkData,
    row: usize,
    stitched_height: f32,
) {
    if row == 0 || row == 8 {
        return;
    }
    for &(ring, weight) in SEAM_SMOOTHING_RINGS {
        if ring > 8 {
            break;
        }
        blend_absolute_height(
            left_chunk,
            vertex_index(row * 2, 8 - ring),
            stitched_height,
            weight,
        );
        blend_absolute_height(
            right_chunk,
            vertex_index(row * 2, ring),
            stitched_height,
            weight,
        );
    }
}

fn blend_absolute_height(chunk: &mut McnkData, vertex_idx: usize, target_height: f32, weight: f32) {
    let current = chunk.pos[2] + chunk.heights[vertex_idx];
    let blended = current + (target_height - current) * weight;
    chunk.heights[vertex_idx] = blended - chunk.pos[2];
}

pub(super) fn center_surface_position(
    chunks: &[McnkData],
    tile_coords: Option<(u32, u32)>,
) -> [f32; 3] {
    let center_chunk = chunks
        .iter()
        .find(|c| c.index_x == 8 && c.index_y == 8)
        .unwrap_or(&chunks[chunks.len() / 2]);
    let (origin_x, origin_z) = chunk_origin_bevy(center_chunk, tile_coords);
    vertex_position_from_origin(
        9,
        4,
        origin_x,
        origin_z,
        center_chunk.pos[2],
        &center_chunk.heights,
    )
}
